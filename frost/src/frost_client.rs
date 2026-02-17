use crate::config::{county_to_el_area, station_by_id, ElArea, Station};
use crate::error::{FrostCliError, Result};
use serde::Deserialize;
use tracing::{info, warn};

const FROST_BASE_URL: &str = "https://frost.met.no/observations/v0.jsonld";
const FROST_SOURCES_URL: &str = "https://frost.met.no/sources/v0.jsonld";

pub struct FrostClient {
    client: reqwest::blocking::Client,
    client_id: String,
}

// --- Frost API response types (observations) ---

#[derive(Debug, Deserialize)]
pub struct FrostResponse {
    #[serde(rename = "@type")]
    #[allow(dead_code)]
    pub response_type: Option<String>,
    pub data: Option<Vec<FrostObservationData>>,
    pub error: Option<FrostErrorBody>,
}

#[derive(Debug, Deserialize)]
pub struct FrostErrorBody {
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrostObservationData {
    pub source_id: String,
    pub reference_time: String,
    pub observations: Vec<FrostObservation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrostObservation {
    pub element_id: String,
    pub value: Option<f64>,
    pub quality_code: Option<i32>,
}

// --- Frost API response types (sources/stations) ---

#[derive(Debug, Deserialize)]
pub struct FrostSourcesResponse {
    pub data: Option<Vec<FrostSource>>,
    pub error: Option<FrostErrorBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrostSource {
    pub id: String,
    pub name: Option<String>,
    pub county: Option<String>,
    pub municipality: Option<String>,
    #[allow(dead_code)]
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

// --- Flattened output row ---

#[derive(Debug, Clone)]
pub struct PrecipitationRow {
    pub station_id: String,
    pub station_name: String,
    pub el_area: String,
    pub reference_time: String,
    pub precipitation_mm: Option<f64>,
    pub quality_code: Option<i32>,
}

/// A discovered station from the Frost API.
#[derive(Debug)]
pub struct DiscoveredStation {
    pub id: String,
    pub name: String,
    pub county: String,
    pub municipality: String,
    pub el_area: Option<ElArea>,
    pub is_active: bool,
}

impl FrostClient {
    pub fn new(client_id: String) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            client_id,
        }
    }

    pub fn fetch_precipitation(
        &self,
        stations: &[&Station],
        from: &str,
        to: &str,
    ) -> Result<Vec<PrecipitationRow>> {
        let sources = stations
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>()
            .join(",");

        let reference_time = format!("{from}/{to}");

        info!(stations = %sources, period = %reference_time, "Fetching precipitation from Frost API");

        let resp = self
            .client
            .get(FROST_BASE_URL)
            .basic_auth(&self.client_id, Some(""))
            .query(&[
                ("sources", sources.as_str()),
                ("elements", "sum(precipitation_amount P1D)"),
                ("referencetime", reference_time.as_str()),
                ("timeoffsets", "PT6H"),
            ])
            .send()?;

        let status = resp.status();
        let body: FrostResponse = resp.json()?;

        if let Some(err) = body.error {
            let reason = err.reason.unwrap_or_default();
            // 404 "Not found" and 412 "No time series found" mean no data — not real errors
            if reason == "Not found"
                || status.as_u16() == 404
                || status.as_u16() == 412
            {
                warn!(stations = %sources, period = %reference_time, "No data available (skipping)");
                return Ok(Vec::new());
            }
            return Err(FrostCliError::frost_api(reason, err.message.unwrap_or_default()));
        }

        if !status.is_success() {
            return Err(FrostCliError::frost_api(
                status.as_str(),
                "Unexpected error from Frost API",
            ));
        }

        let data = body.data.unwrap_or_default();
        let rows = flatten_observations(&data);

        info!(rows = rows.len(), "Received observation data");
        Ok(rows)
    }

    /// List all stations in Norway that report daily precipitation.
    pub fn list_precipitation_stations(&self) -> Result<Vec<DiscoveredStation>> {
        info!("Fetching available precipitation stations from Frost API");

        let resp = self
            .client
            .get(FROST_SOURCES_URL)
            .basic_auth(&self.client_id, Some(""))
            .query(&[
                ("types", "SensorSystem"),
                ("elements", "sum(precipitation_amount P1D)"),
                ("country", "NO"),
            ])
            .send()?;

        let status = resp.status();
        let body: FrostSourcesResponse = resp.json()?;

        if let Some(err) = body.error {
            return Err(FrostCliError::frost_api(
                err.reason.unwrap_or_default(),
                err.message.unwrap_or_default(),
            ));
        }

        if !status.is_success() {
            return Err(FrostCliError::frost_api(
                status.as_str(),
                "Unexpected error from Frost sources API",
            ));
        }

        let sources = body.data.unwrap_or_default();
        info!(total = sources.len(), "Received station list");

        let stations: Vec<DiscoveredStation> = sources
            .into_iter()
            .map(|s| {
                let county = s.county.unwrap_or_default();
                let el_area = county_to_el_area(&county);
                let is_active = s.valid_to.is_none()
                    || s.valid_to.as_deref().is_some_and(|v| v > "2024-01-01");

                DiscoveredStation {
                    id: s.id,
                    name: s.name.unwrap_or_default(),
                    county,
                    municipality: s.municipality.unwrap_or_default(),
                    el_area,
                    is_active,
                }
            })
            .collect();

        Ok(stations)
    }
}

fn flatten_observations(data: &[FrostObservationData]) -> Vec<PrecipitationRow> {
    let mut rows = Vec::new();

    for entry in data {
        // source_id looks like "SN18700:0" — strip the ":0" suffix
        let station_id = entry
            .source_id
            .split(':')
            .next()
            .unwrap_or(&entry.source_id);

        let station = station_by_id(station_id);
        let station_name = station.map_or("Unknown", |s| s.name);
        let el_area = station.map_or_else(
            || {
                warn!(station_id, "Unknown station, cannot determine el_area");
                "??".to_string()
            },
            |s| s.el_area.to_string(),
        );

        // reference_time from Frost is ISO8601 like "2024-01-01T06:00:00.000Z"
        // Extract just the date part
        let date = entry
            .reference_time
            .split('T')
            .next()
            .unwrap_or(&entry.reference_time);

        for obs in &entry.observations {
            if obs.element_id == "sum(precipitation_amount P1D)" {
                rows.push(PrecipitationRow {
                    station_id: station_id.to_string(),
                    station_name: station_name.to_string(),
                    el_area: el_area.clone(),
                    reference_time: date.to_string(),
                    precipitation_mm: obs.value,
                    quality_code: obs.quality_code,
                });
            }
        }
    }

    rows
}
