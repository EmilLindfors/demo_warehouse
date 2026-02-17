FROM rust:1.85-slim AS builder

WORKDIR /build

# Copy manifest first for dependency caching
COPY frost/Cargo.toml frost/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/deps/frost*

# Copy real source and build
COPY frost/src ./src
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/frost /usr/local/bin/frost

# Credentials are passed at runtime via environment variables:
#   FROST_CLIENT_ID, DATABRICKS_HOSTNAME, DATABRICKS_HTTP_PATH,
#   DATABRICKS_CATALOG, DATABRICKS_ACCESS_TOKEN
#
# Example:
#   docker run --env-file .env frost-ingest ingest --from 2024-01-01 --to 2025-01-01 --parallel
#   docker run --env-file .env frost-ingest ingest --from 2024-01-01 --to 2025-01-01 --output csv --csv-path /data/out.csv

ENTRYPOINT ["frost"]
