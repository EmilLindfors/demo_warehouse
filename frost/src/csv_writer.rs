use crate::error::Result;
use crate::frost_client::PrecipitationRow;
use std::path::Path;
use tracing::info;

pub fn write_csv(rows: &[PrecipitationRow], path: &Path) -> Result<usize> {
    info!(path = %path.display(), rows = rows.len(), "Writing CSV");

    let mut wtr = csv::Writer::from_path(path)?;

    wtr.write_record([
        "station_id",
        "station_name",
        "el_area",
        "reference_time",
        "precipitation_mm",
        "quality_code",
    ])?;

    for row in rows {
        let precip = row
            .precipitation_mm
            .map_or(String::new(), |v| v.to_string());
        let quality = row.quality_code.map_or(String::new(), |v| v.to_string());

        wtr.write_record([
            &row.station_id,
            &row.station_name,
            &row.el_area,
            &row.reference_time,
            &precip,
            &quality,
        ])?;
    }

    wtr.flush()?;
    info!(rows = rows.len(), "CSV written successfully");
    Ok(rows.len())
}
