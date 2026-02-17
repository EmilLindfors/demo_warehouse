# Magasin — Fyllingsgrad

Trender, historiske bånd, og år-over-år sammenligning.

```sql area_options
select distinct area_name
from databricks.reservoir_trends
order by area_name
```

```sql year_options
select distinct cast(cast(iso_year as integer) as varchar) as iso_year
from databricks.reservoir_trends
order by iso_year desc
```

```sql year_range
select
    cast(min(iso_year) as varchar) as min_year,
    cast(max(iso_year) as varchar) as max_year
from databricks.reservoir_trends
```

<Dropdown name="selected_area" data={area_options} value="area_name" title="El-område" defaultValue="Øst-Norge" />
<Dropdown name="from_year" data={year_options} value="iso_year" title="Fra år" />
<Dropdown name="to_year" data={year_options} value="iso_year" title="Til år" />

## Fyllingsgrad over tid

```sql reservoir_timeline
select
    observation_date,
    fill_pct,
    hist_min_fill_pct,
    hist_max_fill_pct,
    hist_median_fill_pct,
    fill_pct_12w_avg,
    fill_pct_prev_year,
    fill_pct_change,
    fill_pct_yoy_change,
    fill_status
from databricks.reservoir_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
order by observation_date
```

<LineChart
    data={reservoir_timeline}
    x="observation_date"
    y={["fill_pct", "hist_median_fill_pct"]}
    yAxisTitle="Fyllingsgrad (%)"
    title="Fyllingsgrad vs. historisk median"
    yMin={0}
    yFmt="num0"
/>

## Historisk bånd (min/maks)

<LineChart
    data={reservoir_timeline}
    x="observation_date"
    y={["fill_pct", "hist_max_fill_pct", "hist_median_fill_pct", "hist_min_fill_pct"]}
    yAxisTitle="Fyllingsgrad (%)"
    title="Fyllingsgrad innenfor historisk min/maks-bånd"
    yMin={0}
    yFmt="num0"
/>

## 12-ukers glidende snitt

<LineChart
    data={reservoir_timeline}
    x="observation_date"
    y={["fill_pct", "fill_pct_12w_avg"]}
    yAxisTitle="Fyllingsgrad (%)"
    title="Fyllingsgrad med 12-ukers glidende snitt"
    yMin={0}
    yFmt="num0"
/>

## Ukentlig endring i fyllingsgrad

<BarChart
    data={reservoir_timeline}
    x="observation_date"
    y="fill_pct_change"
    yAxisTitle="Endring (prosentpoeng)"
    title="Ukentlig endring i fyllingsgrad"
    yFmt="num1"
/>

## Nøkkeltall for valgt periode

```sql reservoir_stats
select
    min(fill_pct) as min_fill,
    max(fill_pct) as max_fill,
    round(avg(fill_pct), 1) as avg_fill,
    min(observation_date) as period_start,
    max(observation_date) as period_end
from databricks.reservoir_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
```

<BigValue data={reservoir_stats} value="min_fill" title="Minimum fyllingsgrad" fmt="num1" />
<BigValue data={reservoir_stats} value="max_fill" title="Maksimum fyllingsgrad" fmt="num1" />
<BigValue data={reservoir_stats} value="avg_fill" title="Gjennomsnitt fyllingsgrad" fmt="num1" />

## Fyllingsgrad per uke — alle år sammenstilt

```sql multi_year
select
    iso_week,
    cast(cast(iso_year as integer) as varchar) as iso_year,
    fill_pct
from databricks.reservoir_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
  and iso_week <= 53
order by iso_year, iso_week
```

<LineChart
    data={multi_year}
    x="iso_week"
    y="fill_pct"
    series="iso_year"
    xAxisTitle="Uke"
    yAxisTitle="Fyllingsgrad (%)"
    title="Fyllingsgrad per uke — sammenligning over år"
    xMin={1}
    xMax={53}
    yMin={0}
    yFmt="num0"
/>

## År-over-år endring

<BarChart
    data={reservoir_timeline}
    x="observation_date"
    y="fill_pct_yoy_change"
    yAxisTitle="Endring fra forrige år (pp)"
    title="Fyllingsgrad vs. forrige år (prosentpoeng)"
    yFmt="num1"
/>
