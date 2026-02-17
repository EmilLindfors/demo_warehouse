-- Pulls from dbt mart: rpt_precipitation_trends
-- Weekly precipitation with YoY comparison and moving averages
select
    el_area,
    area_name,
    cast(iso_year as int) as iso_year,
    cast(iso_week as int) as iso_week,
    week_start_date,
    week_end_date,
    station_count,
    weekly_precipitation_mm,
    avg_daily_precipitation_mm,
    max_daily_precipitation_mm,
    avg_rainy_days,
    precip_mm_prev_year,
    precip_yoy_change_mm,
    ytd_cumulative_precipitation_mm,
    precip_12w_avg_mm
from marts.rpt_precipitation_trends
order by el_area, week_start_date
