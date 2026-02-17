with

reservoir as (

    select * from {{ ref('stg_reservoir_stats') }}

),

historical as (

    select * from {{ ref('stg_reservoir_min_max_median') }}

),

-- makes every previous year match with this year, leading to an efficient year-over-year comparison. not my idea, but pretty cool.
prev_year as (

    select
        el_area,
        iso_year + 1       as compare_year,
        iso_week,
        fill_pct           as fill_pct_prev_year,
        fill_twh           as fill_twh_prev_year

    from reservoir

),

enriched as (

    select

        ---------- ids & time
        r.el_area,
        r.area_number,
        r.observation_date,
        r.iso_year,
        r.iso_week,

        ---------- current measures
        r.fill_pct,
        r.fill_pct_prev_week,
        r.fill_pct_change,
        r.capacity_twh,
        r.fill_twh,

        ---------- historical context
        h.min_fill_pct          as hist_min_fill_pct,
        h.max_fill_pct          as hist_max_fill_pct,
        h.median_fill_pct       as hist_median_fill_pct,

        ---------- year-over-year
        py.fill_pct_prev_year,
        round(r.fill_pct - py.fill_pct_prev_year, 2) as fill_pct_yoy_change,

        ---------- derived
        round(r.fill_pct - h.median_fill_pct, 2) as fill_pct_vs_median,
        case
            when r.fill_pct <= h.min_fill_pct then 'historic_low'
            when r.fill_pct < h.median_fill_pct - 10 then 'below_normal'
            when r.fill_pct <= h.median_fill_pct + 10 then 'normal'
            when r.fill_pct < h.max_fill_pct then 'above_normal'
            else 'historic_high'
        end as fill_status

    from reservoir r

    left join historical h
        on r.el_area = h.el_area
        and r.iso_week = h.iso_week

    left join prev_year py
        on r.el_area = py.el_area
        and r.iso_year = py.compare_year
        and r.iso_week = py.iso_week

)

select * from enriched
