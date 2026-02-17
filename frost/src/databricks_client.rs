use crate::config::Config;
use crate::error::{FrostCliError, Result};
use crate::frost_client::PrecipitationRow;
use serde::Deserialize;
use tracing::{debug, info};

const BATCH_SIZE: usize = 5_000;

pub struct DatabricksClient {
    client: reqwest::blocking::Client,
    sql_url: String,
    warehouse_id: String,
    access_token: String,
    catalog: String,
}

#[derive(Debug, Deserialize)]
struct SqlResponse {
    status: Option<SqlStatus>,
    #[allow(dead_code)]
    manifest: Option<serde_json::Value>,
    #[allow(dead_code)]
    result: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SqlStatus {
    state: String,
    error: Option<SqlError>,
}

#[derive(Debug, Deserialize)]
struct SqlError {
    message: Option<String>,
}

impl DatabricksClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            sql_url: config.databricks_sql_url(),
            warehouse_id: config.warehouse_id().to_string(),
            access_token: config.databricks_access_token.clone(),
            catalog: config.databricks_catalog.clone(),
        }
    }

    fn execute_sql(&self, sql: &str) -> Result<SqlResponse> {
        debug!(sql_len = sql.len(), "Executing SQL statement");

        let body = serde_json::json!({
            "warehouse_id": self.warehouse_id,
            "catalog": self.catalog,
            "schema": "raw_frost",
            "statement": sql,
            "wait_timeout": "30s",
            "disposition": "INLINE",
        });

        let resp = self
            .client
            .post(&self.sql_url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(FrostCliError::databricks(format!(
                "HTTP {status}: {text}"
            )));
        }

        let sql_resp: SqlResponse = resp.json()?;

        if let Some(ref st) = sql_resp.status
            && st.state == "FAILED"
        {
            let msg = st
                .error
                .as_ref()
                .and_then(|e| e.message.as_deref())
                .unwrap_or("Unknown SQL error");
            return Err(FrostCliError::databricks(msg));
        }

        Ok(sql_resp)
    }

    pub fn create_schema(&self) -> Result<()> {
        info!(catalog = %self.catalog, "Creating schema if not exists");
        self.execute_sql(&format!(
            "CREATE SCHEMA IF NOT EXISTS {}.raw_frost",
            self.catalog
        ))?;
        Ok(())
    }

    pub fn create_table(&self) -> Result<()> {
        info!("Creating table if not exists");
        let sql = format!(
            r#"CREATE TABLE IF NOT EXISTS {catalog}.raw_frost.precipitation (
    station_id       STRING  NOT NULL,
    station_name     STRING  NOT NULL,
    el_area          STRING  NOT NULL,
    reference_time   DATE    NOT NULL,
    precipitation_mm DOUBLE,
    quality_code     INT,
    ingested_at      TIMESTAMP
)"#,
            catalog = self.catalog
        );
        self.execute_sql(&sql)?;
        Ok(())
    }

    pub fn delete_existing(&self, from: &str, to: &str) -> Result<()> {
        info!(from, to, "Deleting existing rows for date range");
        let sql = format!(
            "DELETE FROM {catalog}.raw_frost.precipitation \
             WHERE reference_time >= '{from}' AND reference_time < '{to}'",
            catalog = self.catalog,
        );
        self.execute_sql(&sql)?;
        Ok(())
    }

    pub fn insert_precipitation(&self, rows: &[PrecipitationRow]) -> Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;

        for (batch_idx, chunk) in rows.chunks(BATCH_SIZE).enumerate() {
            let values: Vec<String> = chunk
                .iter()
                .map(|r| {
                    let precip = match r.precipitation_mm {
                        Some(v) => v.to_string(),
                        None => "NULL".to_string(),
                    };
                    let quality = match r.quality_code {
                        Some(v) => v.to_string(),
                        None => "NULL".to_string(),
                    };
                    format!(
                        "('{station_id}', '{station_name}', '{el_area}', '{ref_time}', {precip}, {quality}, CURRENT_TIMESTAMP())",
                        station_id = r.station_id,
                        station_name = r.station_name.replace('\'', "''"),
                        el_area = r.el_area,
                        ref_time = r.reference_time,
                    )
                })
                .collect();

            let sql = format!(
                "INSERT INTO {catalog}.raw_frost.precipitation \
                 (station_id, station_name, el_area, reference_time, precipitation_mm, quality_code, ingested_at) \
                 VALUES {values}",
                catalog = self.catalog,
                values = values.join(", "),
            );

            info!(batch = batch_idx + 1, rows = chunk.len(), "Inserting batch");
            self.execute_sql(&sql)?;
            total_inserted += chunk.len();
        }

        Ok(total_inserted)
    }
}
