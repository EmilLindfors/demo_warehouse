-- Pulls from dbt mart: rpt_reservoir_trends
-- Reservoir fill % with historical bands, YoY, and moving averages
select
    el_area,
    area_name,
    cast(iso_year as int) as iso_year,
    cast(iso_week as int) as iso_week,
    observation_date,
    fill_pct,
    fill_twh,
    capacity_twh,
    fill_pct_change,
    fill_status,
    hist_min_fill_pct,
    hist_max_fill_pct,
    hist_median_fill_pct,
    fill_pct_vs_median,
    fill_pct_prev_year,
    fill_pct_yoy_change,
    ytd_min_fill_pct,
    ytd_max_fill_pct,
    fill_pct_12w_avg
from marts.rpt_reservoir_trends
order by el_area, observation_date
