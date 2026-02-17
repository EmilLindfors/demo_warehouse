# Største endringer

Perioder med størst endring i fyllingsgrad og nedbør per el-område.

```sql area_options
select distinct area_name
from databricks.biggest_changes
order by area_name
```

```sql year_options
select distinct cast(cast(iso_year as integer) as varchar) as iso_year
from databricks.biggest_changes
order by iso_year desc
```

<Dropdown name="selected_area" data={area_options} value="area_name" title="El-område" defaultValue="Øst-Norge" />
<Dropdown name="from_year" data={year_options} value="iso_year" title="Fra år" />
<Dropdown name="to_year" data={year_options} value="iso_year" title="Til år" />

## Topp 20 — største endringer i magasinfylling

```sql top_reservoir
select
    row_number() over (order by abs(fill_pct_change) desc) as rang,
    cast(cast(iso_year as integer) as varchar) as år,
    observation_date as dato,
    'Uke ' || cast(cast(iso_week as integer) as varchar) as uke,
    fill_pct_change,
    fill_pct
from databricks.biggest_changes
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
  and change_type = 'reservoir_fill_change'
order by abs(fill_pct_change) desc
limit 20
```

<DataTable data={top_reservoir}>
    <Column id="rang" title="#" />
    <Column id="år" title="År" />
    <Column id="dato" title="Dato" />
    <Column id="uke" title="Uke" />
    <Column id="fill_pct_change" title="Endring (pp)" fmt="num1" />
    <Column id="fill_pct" title="Fyllingsgrad %" fmt="num1" />
</DataTable>

<ScatterPlot
    data={top_reservoir}
    x="dato"
    y="fill_pct_change"
    yAxisTitle="Endring (prosentpoeng)"
    title="Topp 20 ukentlige magasinendringer"
    yFmt="num1"
/>

## Topp 20 — høyest ukentlig nedbør

```sql top_precip
select
    row_number() over (order by weekly_precipitation_mm desc) as rang,
    cast(cast(iso_year as integer) as varchar) as år,
    observation_date as dato,
    'Uke ' || cast(cast(iso_week as integer) as varchar) as uke,
    weekly_precipitation_mm
from databricks.biggest_changes
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
  and change_type = 'precipitation_peak'
order by weekly_precipitation_mm desc
limit 20
```

<DataTable data={top_precip}>
    <Column id="rang" title="#" />
    <Column id="år" title="År" />
    <Column id="dato" title="Dato" />
    <Column id="uke" title="Uke" />
    <Column id="weekly_precipitation_mm" title="Nedbør (mm)" fmt="num1" />
</DataTable>

<ScatterPlot
    data={top_precip}
    x="dato"
    y="weekly_precipitation_mm"
    yAxisTitle="Nedbør (mm)"
    title="Topp 20 nedbørsuker"
/>

## Tidslinje — fyllingsgrad med endringer

```sql timeline
select
    observation_date,
    fill_pct,
    fill_pct_change
from databricks.reservoir_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
order by observation_date
```

<LineChart
    data={timeline}
    x="observation_date"
    y="fill_pct"
    yAxisTitle="Fyllingsgrad (%)"
    title="Fyllingsgrad over valgt periode"
    yMin={0}
    yMax={100}
    yFmt="num0"
/>

<BarChart
    data={timeline}
    x="observation_date"
    y="fill_pct_change"
    yAxisTitle="Endring (pp)"
    title="Ukentlig endring i fyllingsgrad"
    yFmt="num1"
/>
