mod config;
mod csv_writer;
mod databricks_client;
mod error;
mod frost_client;

use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::thread;

use clap::{Parser, Subcommand, ValueEnum};
use config::{yearly_chunks, stations_for_areas, ElArea, Station};
use databricks_client::DatabricksClient;
use error::Result;
use frost_client::{FrostClient, PrecipitationRow};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "frost", about = "Fetch precipitation data from frost.met.no and load into Databricks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fetch precipitation data and load into Databricks or CSV
    Ingest {
        /// Start date (inclusive), e.g. 2024-01-01
        #[arg(long)]
        from: String,

        /// End date (exclusive), e.g. 2024-02-01
        #[arg(long)]
        to: String,

        /// Electricity areas to fetch (comma-separated: NO1,NO2,...). Defaults to all.
        #[arg(long, value_delimiter = ',')]
        areas: Option<Vec<ElArea>>,

        /// Output destination: databricks or csv
        #[arg(long, default_value = "databricks")]
        output: Output,

        /// CSV output file path (used when --output csv)
        #[arg(long, default_value = "precipitation.csv")]
        csv_path: PathBuf,

        /// Fetch in parallel: one thread per station per year
        #[arg(long)]
        parallel: bool,
    },

    /// List available precipitation weather stations from the Frost API
    Stations {
        /// Filter to specific electricity areas (comma-separated: NO1,NO2,...). Defaults to all.
        #[arg(long, value_delimiter = ',')]
        areas: Option<Vec<ElArea>>,

        /// Only show currently active stations
        #[arg(long, default_value = "true")]
        active_only: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Output {
    Databricks,
    Csv,
}

fn fetch_sequential(
    frost: &FrostClient,
    stations: &[&Station],
    from: &str,
    to: &str,
) -> Result<Vec<PrecipitationRow>> {
    frost.fetch_precipitation(stations, from, to)
}

/// Simple counting semaphore using stdlib primitives.
struct Semaphore {
    state: Mutex<usize>,
    cond: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(permits),
            cond: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.state.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        *self.state.lock().unwrap() += 1;
        self.cond.notify_one();
    }
}

/// Max concurrent Frost API requests (their limit is 5, we stay at 4 for safety).
const MAX_CONCURRENT_REQUESTS: usize = 4;

fn fetch_parallel(
    frost: &FrostClient,
    stations: &[&'static Station],
    from: &str,
    to: &str,
) -> Result<Vec<PrecipitationRow>> {
    let chunks = yearly_chunks(from, to)?;

    // Build work items: (station, chunk_from, chunk_to)
    let work: Vec<(&Station, String, String)> = stations
        .iter()
        .flat_map(|&station| {
            chunks
                .iter()
                .map(move |(f, t)| (station, f.clone(), t.clone()))
        })
        .collect();

    info!(
        tasks = work.len(),
        stations = stations.len(),
        chunks = chunks.len(),
        max_concurrent = MAX_CONCURRENT_REQUESTS,
        "Parallel fetch"
    );

    let semaphore = Semaphore::new(MAX_CONCURRENT_REQUESTS);
    let all_rows: Mutex<Vec<PrecipitationRow>> = Mutex::new(Vec::new());
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let sem_ref = &semaphore;
    let all_rows_ref = &all_rows;
    let errors_ref = &errors;

    thread::scope(|s| {
        for (station, chunk_from, chunk_to) in &work {
            s.spawn(move || {
                sem_ref.acquire();
                let single = &[*station];
                let result = frost.fetch_precipitation(single, chunk_from, chunk_to);
                sem_ref.release();

                match result {
                    Ok(rows) => {
                        all_rows_ref.lock().unwrap().extend(rows);
                    }
                    Err(e) => {
                        error!(
                            station = station.id,
                            from = %chunk_from,
                            to = %chunk_to,
                            error = %e,
                            "Fetch failed"
                        );
                        errors_ref.lock().unwrap().push(format!(
                            "{} {chunk_from}..{chunk_to}: {e}",
                            station.id
                        ));
                    }
                }
            });
        }
    });

    let errs = errors.into_inner().unwrap();
    if !errs.is_empty() {
        return Err(error::FrostCliError::config(format!(
            "{} fetch(es) failed:\n  {}",
            errs.len(),
            errs.join("\n  ")
        )));
    }

    let rows = all_rows.into_inner().unwrap();
    info!(rows = rows.len(), "All parallel fetches complete");
    Ok(rows)
}

fn run_ingest(
    from: String,
    to: String,
    areas: Option<Vec<ElArea>>,
    output: Output,
    csv_path: PathBuf,
    parallel: bool,
) -> Result<()> {
    let config = config::Config::load()?;

    let areas: Vec<ElArea> = areas
        .unwrap_or_else(|| vec![ElArea::NO1, ElArea::NO2, ElArea::NO3, ElArea::NO4, ElArea::NO5]);

    let stations = stations_for_areas(&areas);
    if stations.is_empty() {
        return Err(error::FrostCliError::config(
            "No stations matched the selected areas",
        ));
    }

    let area_str = areas
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let station_str = stations.iter().map(|s| s.id).collect::<Vec<_>>().join(", ");
    info!(
        from = %from,
        to = %to,
        areas = %area_str,
        stations = %station_str,
        parallel = parallel,
        "Starting Frost CLI ingest"
    );

    let frost = FrostClient::new(config.frost_client_id.clone());
    let rows = if parallel {
        fetch_parallel(&frost, &stations, &from, &to)?
    } else {
        fetch_sequential(&frost, &stations, &from, &to)?
    };

    if rows.is_empty() {
        info!("No precipitation data returned. Nothing to do.");
        return Ok(());
    }

    match output {
        Output::Csv => {
            let written = csv_writer::write_csv(&rows, &csv_path)?;
            info!(rows = written, path = %csv_path.display(), "Done — wrote CSV");
        }
        Output::Databricks => {
            let db = DatabricksClient::new(&config);
            db.create_schema()?;
            db.create_table()?;
            db.delete_existing(&from, &to)?;
            let inserted = db.insert_precipitation(&rows)?;
            info!(
                rows = inserted,
                table = format_args!(
                    "{}.raw_frost.precipitation",
                    config.databricks_catalog
                ),
                "Done — inserted into Databricks"
            );
        }
    }

    Ok(())
}

fn run_stations(areas: Option<Vec<ElArea>>, active_only: bool) -> Result<()> {
    let config = config::Config::load_frost_only()?;
    let frost = FrostClient::new(config.frost_client_id);

    let all_stations = frost.list_precipitation_stations()?;

    let area_filter: Option<Vec<ElArea>> = areas;

    let mut filtered: Vec<_> = all_stations
        .into_iter()
        .filter(|s| {
            if active_only && !s.is_active {
                return false;
            }
            match (&area_filter, &s.el_area) {
                (Some(areas), Some(area)) => areas.contains(area),
                (Some(_), None) => false,
                (None, _) => true,
            }
        })
        .collect();

    filtered.sort_by(|a, b| {
        a.el_area
            .map(|e| e.to_string())
            .cmp(&b.el_area.map(|e| e.to_string()))
            .then(a.name.cmp(&b.name))
    });

    let mut current_area: Option<String> = None;
    let mut area_count = 0;

    for station in &filtered {
        let area_str = station
            .el_area
            .map_or("??".to_string(), |a| a.to_string());

        if current_area.as_deref() != Some(&area_str) {
            if current_area.is_some() {
                println!("  ({area_count} stations)\n");
            }
            println!("===== {area_str} =====");
            current_area = Some(area_str);
            area_count = 0;
        }

        let active_marker = if station.is_active { " " } else { "*" };
        println!(
            " {active_marker} {id:<12} {name:<42} {county:<25} {muni}",
            id = station.id,
            name = station.name,
            county = station.county,
            muni = station.municipality,
        );
        area_count += 1;
    }

    if current_area.is_some() {
        println!("  ({area_count} stations)");
    }

    println!("\nTotal: {} stations", filtered.len());
    if !active_only {
        println!("  (* = inactive station)");
    }

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Ingest {
            from,
            to,
            areas,
            output,
            csv_path,
            parallel,
        } => run_ingest(from, to, areas, output, csv_path, parallel),
        Command::Stations {
            areas,
            active_only,
        } => run_stations(areas, active_only),
    }
}
