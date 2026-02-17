# /// script
# dependencies = [
#   "dlt[databricks]",
#   "pyarrow"
# ]
# ///

import dlt
from dlt.sources.rest_api import rest_api_source

BASE_URL = "https://biapi.nve.no/magasinstatistikk"


def nve_magasin_source():
    """
    Declarative REST API source for all NVE magasinstatistikk endpoints.
    dlt handles pagination, schema inference, and state tracking.
    """
    return rest_api_source(
        {
            "client": {
                "base_url": BASE_URL,
            },
            "resource_defaults": {
                "write_disposition": "merge",
            },
            "resources": [
                {
                    "name": "reservoir_stats",
                    "endpoint": {
                        "path": "/api/Magasinstatistikk/HentOffentligData",
                    },
                    "primary_key": ["dato_Id", "omrType", "omrnr"],
                },
                {
                    "name": "reservoir_stats_latest_week",
                    "endpoint": {
                        "path": "/api/Magasinstatistikk/HentOffentligDataSisteUke",
                    },
                    "write_disposition": "replace",  # always just the latest week
                    "primary_key": ["dato_Id", "omrType", "omrnr"],
                },
                {
                    "name": "reservoir_min_max_median",
                    "endpoint": {
                        "path": "/api/Magasinstatistikk/HentOffentligDataMinMaxMedian",
                    },
                    "primary_key": ["omrType", "omrnr", "iso_uke"],
                },
                {
                    "name": "areas",
                    "endpoint": {
                        "path": "/api/Magasinstatistikk/HentOmråder",
                    },
                    "write_disposition": "replace",  # small reference data
                },
            ],
        }
    )


if __name__ == "__main__":
    # --- Configure the Databricks destination ---
    # For Free Edition with Direct Load (no external staging bucket needed):
    from dlt.destinations import databricks

    destination = databricks(
        # If running inside a Databricks notebook, credentials come from context.
        # If running locally, they come from .dlt/secrets.toml or env vars.
    )

    pipeline = dlt.pipeline(
        pipeline_name="nve_magasin",
        destination=destination,
        dataset_name="raw_nve",  # This becomes the schema in Databricks
    )

    source = nve_magasin_source()

    load_info = pipeline.run(source)
    print(load_info)

    # Quick verification
    print("\n--- Row counts ---")
    with pipeline.sql_client() as client:
        for table in [
            "reservoir_stats",
            "reservoir_stats_latest_week",
            "reservoir_min_max_median",
            "areas",
        ]:
            try:
                with client.execute_sql(f"SELECT COUNT(*) FROM {table}") as cursor:
                    count = cursor.fetchone()[0]
                    print(f"  {table}: {count} rows")
            except Exception as e:
                print(f"  {table}: {e}")
