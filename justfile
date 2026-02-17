# DyrWatt — Norwegian Hydroelectric Reservoir & Precipitation Analytics

set dotenv-load
set shell := ["C:/Program Files/PowerShell/7/pwsh.exe", "-NoProfile", "-Command"]

warehouse_id := "66071776da4067e0"
dbt := "~/.local/bin/dbt.exe"
otel := "--otel-parquet-file-name telemetry.parquet"

export DATABRICKS_HOST := "https://" + env("DATABRICKS_HOSTNAME")
export DATABRICKS_TOKEN := env("DATABRICKS_ACCESS_TOKEN")

# Default: show available recipes
default:
    @just --list

# --- Databricks Warehouse ---

# Start the SQL warehouse
start:
    ./databricks.exe warehouses start {{ warehouse_id }}

# Stop the SQL warehouse
stop:
    ./databricks.exe warehouses stop {{ warehouse_id }}

# Show warehouse status
status:
    @./databricks.exe warehouses list

# Run a SQL statement against the warehouse (handles single quotes in SQL)
sql statement:
    $body = @{ warehouse_id = '{{ warehouse_id }}'; statement = '{{ replace(statement, "'", "''") }}'; wait_timeout = '30s' } | ConvertTo-Json -Compress; ./databricks.exe api post /api/2.0/sql/statements --json $body

# List all schemas in the workspace catalog
schemas:
    $body = @{ warehouse_id = '{{ warehouse_id }}'; statement = 'SHOW SCHEMAS IN workspace'; wait_timeout = '30s' } | ConvertTo-Json -Compress; ./databricks.exe api post /api/2.0/sql/statements --json $body

# List tables in a schema (e.g. just tables raw_nve)
tables schema:
    $body = @{ warehouse_id = '{{ warehouse_id }}'; statement = 'SHOW TABLES IN workspace.{{ schema }}'; wait_timeout = '30s' } | ConvertTo-Json -Compress; ./databricks.exe api post /api/2.0/sql/statements --json $body

# Drop a schema and all its tables (e.g. just drop bronze_frost)
drop schema:
    $body = @{ warehouse_id = '{{ warehouse_id }}'; statement = 'DROP SCHEMA workspace.{{ schema }} CASCADE'; wait_timeout = '30s' } | ConvertTo-Json -Compress; ./databricks.exe api post /api/2.0/sql/statements --json $body

# --- dbt (run in energy/) ---

# Start warehouse then run dbt build (models + tests)
build: start
    Push-Location energy && {{ dbt }} build {{ otel }} && Pop-Location

# Start warehouse then run dbt models only
run: start
    Push-Location energy && {{ dbt }} run {{ otel }} && Pop-Location

# Start warehouse then run dbt tests only
test: start
    Push-Location energy && {{ dbt }} test {{ otel }} && Pop-Location

# Compile dbt SQL without executing
compile:
    Push-Location energy && {{ dbt }} compile && Pop-Location

# --- Telemetry / Logs ---

telemetry := "energy/target/metadata/telemetry.parquet"

# Query dbt telemetry parquet with DuckDB
logs query="SELECT * FROM read_parquet('energy/target/metadata/telemetry.parquet') LIMIT 20":
    duckdb -c "{{ query }}"

# Show slowest models and tests by duration
logs-slowest:
    duckdb -c "SELECT span_name, attributes.node_outcome as outcome, attributes.materialization as mat, attributes.duration_ms as ms, attributes.rows_affected as rows FROM read_parquet('{{ telemetry }}') WHERE record_type = 'SpanEnd' AND attributes.duration_ms IS NOT NULL ORDER BY ms DESC"

# Show all SQL queries sent to the warehouse, sorted by duration
logs-sql:
    duckdb -c "SELECT span_name as query, attributes.query_outcome as outcome, epoch_ms(end_time_unix_nano) - epoch_ms(start_time_unix_nano) as ms FROM read_parquet('{{ telemetry }}') WHERE span_name LIKE 'Query executed%' AND record_type = 'SpanEnd' ORDER BY ms DESC"

# --- Reports (Evidence.dev) ---

# Refresh Evidence.dev source data from Databricks
sources:
    Push-Location reports && bun run sources && Pop-Location

# Start the Evidence.dev dev server
dev:
    Push-Location reports && bun run dev && Pop-Location

# Build static Evidence.dev site
reports-build:
    Push-Location reports && bun run build && Pop-Location

# Preview built Evidence.dev site
reports-preview:
    Push-Location reports && bun run preview && Pop-Location

# --- Data Ingestion ---

# Ingest reservoir data from NVE
ingest-nve:
    uv run nve.py

# Ingest yesterday's precipitation from Frost
ingest-frost-latest:
    Push-Location frost && just ingest-latest && Pop-Location

# Ingest full precipitation history from Frost
ingest-frost-all:
    Push-Location frost && just ingest-all && Pop-Location

# Full pipeline: start warehouse, ingest all sources, build dbt
pipeline: start ingest-nve ingest-frost-latest build
