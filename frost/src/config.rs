use crate::error::{FrostCliError, Result};
use std::fmt;

/// Split a date range into yearly chunks: [(from, to), ...]
/// Dates are "YYYY-MM-DD" strings. Each chunk starts on Jan 1st.
pub fn yearly_chunks(from: &str, to: &str) -> Result<Vec<(String, String)>> {
    let parse_year = |s: &str| -> Result<i32> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(FrostCliError::config(format!("Invalid date: {s}")));
        }
        parts[0].parse().map_err(|_| FrostCliError::config(format!("Invalid year in: {s}")))
    };

    let from_y = parse_year(from)?;
    let _to_y = parse_year(to)?;

    let mut chunks = Vec::new();
    let mut chunk_start = from.to_string();

    let mut y = from_y;
    loop {
        let next_boundary = format!("{:04}-01-01", y + 1);

        if next_boundary.as_str() >= to {
            if chunk_start.as_str() < to {
                chunks.push((chunk_start, to.to_string()));
            }
            break;
        }

        chunks.push((chunk_start, next_boundary.clone()));
        chunk_start = next_boundary;
        y += 1;
    }

    Ok(chunks)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElArea {
    NO1,
    NO2,
    NO3,
    NO4,
    NO5,
}

impl fmt::Display for ElArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElArea::NO1 => write!(f, "NO1"),
            ElArea::NO2 => write!(f, "NO2"),
            ElArea::NO3 => write!(f, "NO3"),
            ElArea::NO4 => write!(f, "NO4"),
            ElArea::NO5 => write!(f, "NO5"),
        }
    }
}

impl std::str::FromStr for ElArea {
    type Err = FrostCliError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "NO1" => Ok(ElArea::NO1),
            "NO2" => Ok(ElArea::NO2),
            "NO3" => Ok(ElArea::NO3),
            "NO4" => Ok(ElArea::NO4),
            "NO5" => Ok(ElArea::NO5),
            _ => Err(FrostCliError::config(format!("Unknown electricity area: {s}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Station {
    pub id: &'static str,
    pub name: &'static str,
    pub el_area: ElArea,
}

/// 5 well-known stations per electricity area (25 total).
/// Station IDs and names verified against Frost API sources endpoint.
pub const STATIONS: &[Station] = &[
    // NO1 — Øst-Norge (Eastern Norway)
    Station { id: "SN18700", name: "Oslo - Blindern",              el_area: ElArea::NO1 },
    Station { id: "SN17150", name: "Rygge",                        el_area: ElArea::NO1 },
    Station { id: "SN12680", name: "Lillehammer - Sætherengen",    el_area: ElArea::NO1 },
    Station { id: "SN24890", name: "Nesbyen - Todokk",             el_area: ElArea::NO1 },
    Station { id: "SN27500", name: "Færder Fyr",                   el_area: ElArea::NO1 },
    // NO2 — Sør-Norge (Southern Norway)
    Station { id: "SN39040", name: "Kjevik",                       el_area: ElArea::NO2 },
    Station { id: "SN44560", name: "Sola",                         el_area: ElArea::NO2 },
    Station { id: "SN36560", name: "Nelaug",                       el_area: ElArea::NO2 },
    Station { id: "SN42160", name: "Lista Fyr",                    el_area: ElArea::NO2 },
    Station { id: "SN38140", name: "Landvik",                      el_area: ElArea::NO2 },
    // NO3 — Midt-Norge (Central Norway)
    Station { id: "SN68860", name: "Trondheim - Voll",             el_area: ElArea::NO3 },
    Station { id: "SN62290", name: "Molde - Nøisomhed",            el_area: ElArea::NO3 },
    Station { id: "SN63420", name: "Sunndalsøra III",              el_area: ElArea::NO3 },
    Station { id: "SN69100", name: "Værnes",                       el_area: ElArea::NO3 },
    Station { id: "SN65310", name: "Veiholmen",                    el_area: ElArea::NO3 },
    // NO4 — Nord-Norge (Northern Norway)
    Station { id: "SN90450", name: "Tromsø",                       el_area: ElArea::NO4 },
    Station { id: "SN82290", name: "Bodø VI",                      el_area: ElArea::NO4 },
    Station { id: "SN87110", name: "Andøya",                       el_area: ElArea::NO4 },
    Station { id: "SN94280", name: "Hammerfest Lufthavn",          el_area: ElArea::NO4 },
    Station { id: "SN85380", name: "Skrova Fyr",                   el_area: ElArea::NO4 },
    // NO5 — Vest-Norge (Western Norway)
    Station { id: "SN50540", name: "Bergen - Florida",             el_area: ElArea::NO5 },
    Station { id: "SN50500", name: "Flesland",                     el_area: ElArea::NO5 },
    Station { id: "SN51530", name: "Vossavangen",                  el_area: ElArea::NO5 },
    Station { id: "SN57770", name: "Ytterøyane Fyr",               el_area: ElArea::NO5 },
    Station { id: "SN48330", name: "Slåtterøy Fyr",                el_area: ElArea::NO5 },
];

pub fn stations_for_areas(areas: &[ElArea]) -> Vec<&'static Station> {
    STATIONS.iter().filter(|s| areas.contains(&s.el_area)).collect()
}

pub fn station_by_id(id: &str) -> Option<&'static Station> {
    STATIONS.iter().find(|s| s.id == id)
}

/// Map Norwegian county names (both old and new) to electricity areas.
pub fn county_to_el_area(county: &str) -> Option<ElArea> {
    let c = county.to_uppercase();
    // NO1 — Eastern Norway
    if c.contains("OSLO") || c.contains("AKERSHUS") || c.contains("ØSTFOLD")
        || c.contains("BUSKERUD") || c.contains("HEDMARK") || c.contains("OPPLAND")
        || c.contains("VESTFOLD") || c.contains("TELEMARK")
        || c.contains("VIKEN") || c.contains("INNLANDET")
    {
        return Some(ElArea::NO1);
    }
    // NO2 — Southern Norway
    if c.contains("AGDER") || c.contains("ROGALAND") {
        return Some(ElArea::NO2);
    }
    // NO3 — Central Norway
    if c.contains("TRØNDELAG") || c.contains("MØRE OG ROMSDAL") {
        return Some(ElArea::NO3);
    }
    // NO4 — Northern Norway
    if c.contains("NORDLAND") || c.contains("TROMS") || c.contains("FINNMARK") {
        return Some(ElArea::NO4);
    }
    // NO5 — Western Norway
    if c.contains("HORDALAND") || c.contains("SOGN OG FJORDANE") || c.contains("VESTLAND") {
        return Some(ElArea::NO5);
    }
    None
}

#[derive(Debug)]
pub struct Config {
    pub frost_client_id: String,
    pub databricks_hostname: String,
    pub databricks_http_path: String,
    pub databricks_catalog: String,
    pub databricks_access_token: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv()).ok();

        let get = |key: &str| -> Result<String> {
            std::env::var(key).map_err(|_| FrostCliError::EnvVar(key.to_string()))
        };

        Ok(Config {
            frost_client_id: get("FROST_CLIENT_ID")?,
            databricks_hostname: get("DATABRICKS_HOSTNAME")?,
            databricks_http_path: get("DATABRICKS_HTTP_PATH")?,
            databricks_catalog: get("DATABRICKS_CATALOG")?,
            databricks_access_token: get("DATABRICKS_ACCESS_TOKEN")?,
        })
    }

    /// Load only the Frost client ID (for station listing without Databricks).
    pub fn load_frost_only() -> Result<Self> {
        dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv()).ok();

        let frost_client_id = std::env::var("FROST_CLIENT_ID")
            .map_err(|_| FrostCliError::EnvVar("FROST_CLIENT_ID".to_string()))?;

        Ok(Config {
            frost_client_id,
            databricks_hostname: String::new(),
            databricks_http_path: String::new(),
            databricks_catalog: String::new(),
            databricks_access_token: String::new(),
        })
    }

    pub fn databricks_sql_url(&self) -> String {
        format!(
            "https://{}/api/2.0/sql/statements",
            self.databricks_hostname
        )
    }

    pub fn warehouse_id(&self) -> &str {
        self.databricks_http_path
            .rsplit('/')
            .next()
            .unwrap_or(&self.databricks_http_path)
    }
}
