FROM python:3.12-slim

WORKDIR /app

# Install uv for fast Python package management
RUN pip install --no-cache-dir uv

# Copy the pipeline script
COPY nve.py .

# Pre-install dependencies so container startup is fast
RUN uv pip install --system "dlt[databricks]" pyarrow

# Credentials are passed at runtime via:
#   - Environment variables (DATABRICKS_HOSTNAME, etc.), or
#   - Mounting .dlt/secrets.toml into /app/.dlt/secrets.toml
#
# Example:
#   docker run --env-file .env nve-ingest
#   docker run -v $(pwd)/.dlt:/app/.dlt nve-ingest

ENTRYPOINT ["python", "nve.py"]
