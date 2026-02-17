with

weekly as (

    select * from {{ ref('int_precipitation_weekly') }}

),

area_info as (

    select * from {{ ref('dim_el_area') }}

),

prev_year as (

    select
        el_area,
        iso_year + 1                        as compare_year,
        iso_week,
        weekly_precipitation_mm             as precip_mm_prev_year

    from weekly

),

trends as (

    select

        ---------- dimensions
        w.el_area,
        a.area_name,
        w.iso_year,
        w.iso_week,
        w.week_start_date,
        w.week_end_date,

        ---------- measures
        w.station_count,
        w.weekly_precipitation_mm,
        w.avg_daily_precipitation_mm,
        w.max_daily_precipitation_mm,
        w.avg_rainy_days,
        w.avg_days_with_data,

        ---------- year-over-year
        py.precip_mm_prev_year,
        round(w.weekly_precipitation_mm - py.precip_mm_prev_year, 1) as precip_yoy_change_mm,

        ---------- running totals within year
        round(sum(w.weekly_precipitation_mm) over (
            partition by w.el_area, w.iso_year
            order by w.iso_week
            rows between unbounded preceding and current row
        ), 1) as ytd_cumulative_precipitation_mm,

        ---------- 12-week moving average
        round(avg(w.weekly_precipitation_mm) over (
            partition by w.el_area
            order by w.iso_year, w.iso_week
            rows between 11 preceding and current row
        ), 1) as precip_12w_avg_mm

    from weekly w

    left join area_info a
        on w.el_area = a.el_area

    left join prev_year py
        on w.el_area = py.el_area
        and w.iso_year = py.compare_year
        and w.iso_week = py.iso_week

)

select * from trends
