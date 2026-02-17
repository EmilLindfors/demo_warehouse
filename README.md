# DyrWatt — Analyse av vannmagasin og nedbør i Norge

Datapipeline som kombinerer fyllingsgrad i norske vannmagasin (NVE) med nedbørsdata (Frost/MET) for analyse på tvers av Norges fem el-områder (NO1–NO5).

## Forutsetninger

- [just](https://just.systems/) — kommandokjører (alle kommandoer kjøres herfra)
- [Rust](https://rustup.rs/) — for Frost CLI
- [Python 3.10+](https://python.org/) med [uv](https://docs.astral.sh/uv/) — for NVE-innhenting
- [dbt Fusion](https://www.getdbt.com/) (`dbt`) — for SQL-transformasjoner
- [Bun](https://bun.sh/) — for Evidence.dev-rapporter
- Databricks-workspace med SQL-warehouse

## Konfigurasjon

Opprett `.env` i prosjektroten:

```env
FROST_CLIENT_ID="din-frost-client-id"
DATABRICKS_HOSTNAME="din-instans.cloud.databricks.com"
DATABRICKS_HTTP_PATH="/sql/1.0/warehouses/din-warehouse-id"
DATABRICKS_CATALOG="workspace"
DATABRICKS_ACCESS_TOKEN="din-token"
```

dbt-profil i `~/.dbt/profiles.yml`:

```yaml
energy:
  target: dev
  outputs:
    dev:
      type: databricks
      catalog: workspace
      schema: default
      host: "din-instans.cloud.databricks.com"
      http_path: "/sql/1.0/warehouses/din-warehouse-id"
      token: "din-token"
```

dlt-hemmeligheter i `.dlt/secrets.toml`:

```toml
[destination.databricks.credentials]
server_hostname = "din-instans.cloud.databricks.com"
http_path = "/sql/1.0/warehouses/din-warehouse-id"
access_token = "din-token"
catalog = "workspace"
```

---

## Kommandoer

Alt kjøres via `just` fra prosjektroten. Kjør `just` uten argumenter for å se alle tilgjengelige kommandoer.

### Full pipeline

```bash
just pipeline           # Start warehouse → hent data → kjør dbt
```

Dette starter warehouse, henter NVE-data, henter gårsdagens Frost-data, og kjører dbt build.

### Databricks-warehouse

```bash
just start              # Start SQL-warehouse
just stop               # Stopp warehouse
just status             # Vis warehouse-status
```

### Datainnhenting

```bash
just ingest-nve         # Hent magasindata fra NVE
just ingest-frost-latest # Hent gårsdagens nedbørsdata fra Frost
just ingest-frost-all   # Hent full historikk (2015–i dag, parallell)
```

Frost CLI har også egne kommandoer via `just` i `frost/`-mappen:

```bash
cd frost
just stations           # List alle 25 værstasjoner
just stations-area NO1  # List stasjoner for ett område
just ingest 2024-01-01 2024-06-01  # Egendefinert datoperiode
just debug-latest       # Eksporter til CSV (feilsøking)
```

### dbt-transformasjoner

```bash
just build              # Kjør modeller + tester (starter warehouse automatisk)
just run                # Kjør kun modeller
just test               # Kjør kun tester
just compile            # Kompiler SQL uten å kjøre
```

### Rapporter (Evidence.dev)

```bash
just sources            # Oppdater kildedata fra Databricks
just dev                # Start utviklingsserver (http://localhost:3000)
just reports-build      # Bygg statisk nettside
just reports-preview    # Forhåndsvis bygget nettside
```

### SQL-spørringer

```bash
just sql "SELECT COUNT(*) FROM workspace.raw_frost.precipitation"
just schemas            # List alle skjemaer
just tables raw_nve     # List tabeller i et skjema
just drop staging       # Slett et skjema (forsiktig!)
```

### Telemetri (dbt-kjøringer)

```bash
just logs               # Vis siste dbt-telemetri
just logs-slowest       # Tregest modeller sortert etter varighet
just logs-sql           # Alle SQL-spørringer sortert etter varighet
```

---

## Værstasjoner

25 stasjoner — 5 per el-område:

| Område | Stasjoner |
|--------|-----------|
| NO1 (Øst-Norge) | Oslo-Blindern, Rygge, Lillehammer, Nesbyen, Færder Fyr |
| NO2 (Sør-Norge) | Kjevik, Sola, Nelaug, Lista Fyr, Landvik |
| NO3 (Midt-Norge) | Trondheim-Voll, Molde, Sunndalsøra, Værnes, Veiholmen |
| NO4 (Nord-Norge) | Tromsø, Bodø, Andøya, Hammerfest, Skrova Fyr |
| NO5 (Vest-Norge) | Bergen-Florida, Flesland, Vossavangen, Ytterøyane Fyr, Slåtterøy Fyr |

## Docker

Begge innhentingene har Dockerfiler for containerisert kjøring:

```bash
# Bygg
docker build -f nve.Dockerfile -t nve-ingest .
docker build -f frost.Dockerfile -t frost-ingest .

# Kjør
docker run --env-file .env nve-ingest
docker run --env-file .env frost-ingest ingest --from 2015-01-01 --to 2026-01-01 --parallel
```

## Prosjektstruktur

```
fraktal/
├── justfile                        # Alle kommandoer samlet
├── nve.py                          # NVE-innhenting (Python + dlt)
├── frost/                          # Frost-innhenting (Rust CLI)
│   ├── justfile
│   └── src/
│       ├── main.rs                 # CLI-dispatch
│       ├── config.rs               # Stasjoner og konfigurasjon
│       ├── frost_client.rs         # HTTP-klient mot Frost API
│       ├── databricks_client.rs    # SQL-eksekvering mot Databricks
│       ├── csv_writer.rs           # CSV-eksport (feilsøking)
│       └── error.rs                # Feiltyper
├── energy/                         # dbt-prosjekt
│   ├── dbt_project.yml
│   └── models/
│       ├── staging/
│       │   ├── stg_reservoir_stats.sql
│       │   ├── stg_reservoir_min_max_median.sql
│       │   ├── stg_precipitation.sql
│       │   └── stg_areas.sql
│       ├── intermediate/
│       │   ├── int_dimensions.sql
│       │   ├── int_stations.sql
│       │   ├── int_precipitation_weekly.sql
│       │   └── int_reservoir_enriched.sql
│       └── marts/
│           ├── dim_el_area.sql
│           ├── fct_reservoir_weekly.sql
│           ├── rpt_reservoir_trends.sql
│           ├── rpt_precipitation_trends.sql
│           └── rpt_biggest_changes.sql
├── reports/                        # Evidence.dev-dashbord
│   └── pages/
│       ├── index.md                # Oversikt
│       ├── reservoir.md            # Magasin-detaljer
│       ├── precipitation.md        # Nedbør-detaljer
│       ├── biggest-changes.md      # Største endringer
│       └── methodology.md          # Visualiseringsmetodikk
├── nve.Dockerfile
├── frost.Dockerfile
└── .env                            # Hemmeligheter (ikke committet)
```
