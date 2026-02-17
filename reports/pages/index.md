# DyrWatt AS — Energirapporter

Oversikt over vannmagasin-fyllingsgrad og nedbør for Norges el-områder.

```sql latest_reservoir
select
    el_area,
    area_name,
    observation_date,
    fill_pct,
    fill_pct_prev_year,
    fill_pct_yoy_change,
    fill_status,
    hist_median_fill_pct
from databricks.reservoir_trends
where observation_date = (
    select max(observation_date) from databricks.reservoir_trends
)
order by el_area
```

```sql latest_precip
select
    el_area,
    area_name,
    week_start_date,
    weekly_precipitation_mm,
    precip_mm_prev_year,
    precip_yoy_change_mm,
    ytd_cumulative_precipitation_mm
from databricks.precipitation_trends
where week_start_date = (
    select max(week_start_date) from databricks.precipitation_trends
)
order by el_area
```

## Siste status per el-område

<DataTable data={latest_reservoir}>
    <Column id="area_name" title="Område" />
    <Column id="observation_date" title="Dato" />
    <Column id="fill_pct" title="Fyllingsgrad %" fmt="num1" />
    <Column id="fill_pct_prev_year" title="Forrige år %" fmt="num1" />
    <Column id="fill_pct_yoy_change" title="Endring YoY" fmt="num1" />
    <Column id="fill_status" title="Status" />
</DataTable>

## Fyllingsgrad og nedbør — kombinert

```sql area_options
select distinct area_name
from databricks.reservoir_trends
order by area_name
```

<Dropdown name="selected_area" data={area_options} value="area_name" title="El-område" defaultValue="Øst-Norge" />

```sql combined_data
select
    r.observation_date,
    r.fill_pct,
    p.weekly_precipitation_mm
from databricks.reservoir_trends r
inner join databricks.precipitation_trends p
    on r.el_area = p.el_area
    and r.iso_year = p.iso_year
    and r.iso_week = p.iso_week
where r.area_name = '${inputs.selected_area.value}'
  and r.iso_year >= (select max(iso_year) - 2 from databricks.reservoir_trends)
order by r.observation_date
```

<LineChart
    data={combined_data}
    x="observation_date"
    y="fill_pct"
    y2="weekly_precipitation_mm"
    y2SeriesType="bar"
    yAxisTitle="Fyllingsgrad (%)"
    y2AxisTitle="Nedbør (mm)"
    yMin={0}
    yFmt="num0"
    title="Fyllingsgrad og ukentlig nedbør"
/>

## Fyllingsgrad — alle områder (siste 3 år)

```sql fill_overview
select
    observation_date,
    area_name,
    fill_pct
from databricks.reservoir_trends
where iso_year >= (select max(iso_year) - 2 from databricks.reservoir_trends)
order by observation_date
```

<LineChart
    data={fill_overview}
    x="observation_date"
    y="fill_pct"
    series="area_name"
    yAxisTitle="Fyllingsgrad (%)"
    title="Magasinfyllingsgrad per el-område"
    yMin={0}
    yFmt="num0"
/>

## Ukentlig nedbør — alle områder

```sql precip_overview
select
    week_start_date,
    area_name,
    weekly_precipitation_mm
from databricks.precipitation_trends
order by week_start_date
```

<BarChart
    data={precip_overview}
    x="week_start_date"
    y="weekly_precipitation_mm"
    series="area_name"
    yAxisTitle="Nedbør (mm)"
    title="Ukentlig nedbør per el-område"
/>

## Kumulativ nedbør per år — alle områder

```sql cumulative_all
select
    iso_week,
    cast(cast(iso_year as integer) as varchar) || ' ' || area_name as series_label,
    ytd_cumulative_precipitation_mm
from databricks.precipitation_trends
where iso_week <= 53
order by iso_year, iso_week
```

<LineChart
    data={cumulative_all}
    x="iso_week"
    y="ytd_cumulative_precipitation_mm"
    series="series_label"
    xAxisTitle="Uke"
    yAxisTitle="Kumulativ nedbør (mm)"
    title="Kumulativ nedbør per år og område"
    xMin={1}
    xMax={53}
/>

---

<a href="/reservoir">Magasin-detaljer</a> ·
<a href="/precipitation">Nedbør-detaljer</a> ·
<a href="/biggest-changes">Største endringer</a> ·
<a href="/methodology">Visualiseringsmetodikk</a>
