-- Combines reservoir fill changes and precipitation peaks into a single
-- table for the "biggest changes" report page. Ranking is done globally
-- per area (not per year) so the report can pick the true top N.

with

reservoir as (

    select * from {{ ref('fct_reservoir_weekly') }}

),

precip_weekly as (

    select * from {{ ref('int_precipitation_weekly') }}

),

area_info as (

    select * from {{ ref('dim_el_area') }}

),

-- rank weeks by absolute reservoir fill change per area (global)
reservoir_changes as (

    select
        r.el_area,
        a.area_name,
        r.iso_year,
        r.iso_week,
        r.observation_date,
        r.fill_pct,
        r.fill_pct_change,
        'reservoir_fill_change' as change_type

    from reservoir r
    left join area_info a on r.el_area = a.el_area
    where r.fill_pct_change is not null

),

-- rank weeks by precipitation per area (global)
precip_peaks as (

    select
        p.el_area,
        a.area_name,
        p.iso_year,
        p.iso_week,
        p.week_start_date     as observation_date,
        cast(null as double)  as fill_pct,
        cast(null as double)  as fill_pct_change,
        p.weekly_precipitation_mm,
        'precipitation_peak'  as change_type

    from precip_weekly p
    left join area_info a on p.el_area = a.el_area

)

select
    el_area,
    area_name,
    iso_year,
    iso_week,
    observation_date,
    change_type,
    fill_pct_change,
    fill_pct,
    cast(null as double) as weekly_precipitation_mm

from reservoir_changes

union all

select
    el_area,
    area_name,
    iso_year,
    iso_week,
    observation_date,
    change_type,
    fill_pct_change,
    fill_pct,
    weekly_precipitation_mm

from precip_peaks
