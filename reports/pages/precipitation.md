# Nedbør

Trender, kumulativ nedbør, og sammenligning med forrige år.

```sql area_options
select distinct area_name
from databricks.precipitation_trends
order by area_name
```

```sql year_options
select distinct cast(cast(iso_year as integer) as varchar) as iso_year
from databricks.precipitation_trends
order by iso_year desc
```

<Dropdown name="selected_area" data={area_options} value="area_name" title="El-område" defaultValue="Øst-Norge" />
<Dropdown name="from_year" data={year_options} value="iso_year" title="Fra år" />
<Dropdown name="to_year" data={year_options} value="iso_year" title="Til år" />

## Ukentlig nedbør

```sql weekly_precip
select
    week_start_date,
    weekly_precipitation_mm,
    precip_12w_avg_mm,
    precip_mm_prev_year,
    max_daily_precipitation_mm,
    avg_rainy_days
from databricks.precipitation_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
order by week_start_date
```

<BarChart
    data={weekly_precip}
    x="week_start_date"
    y="weekly_precipitation_mm"
    yAxisTitle="Nedbør (mm)"
    title="Ukentlig nedbør"
/>

## 12-ukers glidende snitt

<LineChart
    data={weekly_precip}
    x="week_start_date"
    y={["weekly_precipitation_mm", "precip_12w_avg_mm"]}
    yAxisTitle="Nedbør (mm)"
    title="Ukentlig nedbør med 12-ukers glidende snitt"
/>

## Maks daglig nedbør per uke

<LineChart
    data={weekly_precip}
    x="week_start_date"
    y="max_daily_precipitation_mm"
    yAxisTitle="Nedbør (mm)"
    title="Høyeste daglige nedbør per uke"
/>

## Kumulativ nedbør per år

```sql cumulative
select
    iso_week,
    cast(cast(iso_year as integer) as varchar) as iso_year,
    ytd_cumulative_precipitation_mm
from databricks.precipitation_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
  and iso_week <= 53
order by iso_year, iso_week
```

<LineChart
    data={cumulative}
    x="iso_week"
    y="ytd_cumulative_precipitation_mm"
    series="iso_year"
    xAxisTitle="Uke"
    yAxisTitle="Kumulativ nedbør (mm)"
    title="Kumulativ nedbør per uke — sammenligning over år"
    xMin={1}
    xMax={53}
/>

## Nøkkeltall for valgt periode

```sql precip_stats
select
    round(sum(weekly_precipitation_mm), 0) as total_mm,
    round(max(weekly_precipitation_mm), 1) as max_week_mm,
    round(avg(weekly_precipitation_mm), 1) as avg_week_mm,
    round(max(max_daily_precipitation_mm), 1) as max_day_mm,
    round(avg(avg_rainy_days), 1) as avg_rainy_days_per_week
from databricks.precipitation_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
```

<BigValue data={precip_stats} value="total_mm" title="Total nedbør (mm)" />
<BigValue data={precip_stats} value="max_week_mm" title="Våteste uke (mm)" />
<BigValue data={precip_stats} value="max_day_mm" title="Våteste dag (mm)" />
<BigValue data={precip_stats} value="avg_week_mm" title="Ukesnitt (mm)" />
<BigValue data={precip_stats} value="avg_rainy_days_per_week" title="Snitt regndager/uke" />

## År-over-år endring

```sql yoy
select
    week_start_date,
    precip_yoy_change_mm
from databricks.precipitation_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
  and precip_yoy_change_mm is not null
order by week_start_date
```

<BarChart
    data={yoy}
    x="week_start_date"
    y="precip_yoy_change_mm"
    yAxisTitle="Endring fra forrige år (mm)"
    title="Ukentlig nedbørsendring vs. forrige år"
/>

## Ukentlig nedbør per år — sammenstilt

```sql precip_by_week
select
    iso_week,
    cast(cast(iso_year as integer) as varchar) as iso_year,
    weekly_precipitation_mm
from databricks.precipitation_trends
where area_name = '${inputs.selected_area.value}'
  and cast(cast(iso_year as integer) as varchar) >= '${inputs.from_year.value}'
  and cast(cast(iso_year as integer) as varchar) <= '${inputs.to_year.value}'
  and iso_week <= 53
order by iso_year, iso_week
```

<LineChart
    data={precip_by_week}
    x="iso_week"
    y="weekly_precipitation_mm"
    series="iso_year"
    xAxisTitle="Uke"
    yAxisTitle="Nedbør (mm)"
    title="Ukentlig nedbør per uke — sammenligning over år"
    xMin={1}
    xMax={53}
/>
