-- Pulls from dbt mart: rpt_biggest_changes
-- All reservoir fill changes and precipitation peaks per area
select
    el_area,
    area_name,
    cast(iso_year as int) as iso_year,
    cast(iso_week as int) as iso_week,
    observation_date,
    change_type,
    fill_pct_change,
    fill_pct,
    weekly_precipitation_mm
from marts.rpt_biggest_changes
order by el_area, observation_date
