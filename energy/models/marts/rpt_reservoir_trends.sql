with

reservoir as (

    select * from {{ ref('fct_reservoir_weekly') }}

),

area_info as (

    select * from {{ ref('dim_el_area') }}

),

trends as (

    select

        ---------- dimensions
        r.el_area,
        a.area_name,
        r.iso_year,
        r.iso_week,
        r.observation_date,

        ---------- current values
        r.fill_pct,
        r.fill_twh,
        r.capacity_twh,
        r.fill_pct_change,
        r.fill_status,

        ---------- historical bands
        r.hist_min_fill_pct,
        r.hist_max_fill_pct,
        r.hist_median_fill_pct,
        r.fill_pct_vs_median,

        ---------- year-over-year
        r.fill_pct_prev_year,
        r.fill_pct_yoy_change,

        ---------- running stats within year
        min(r.fill_pct) over (
            partition by r.el_area, r.iso_year
            order by r.iso_week
            rows between unbounded preceding and current row
        ) as ytd_min_fill_pct,
        max(r.fill_pct) over (
            partition by r.el_area, r.iso_year
            order by r.iso_week
            rows between unbounded preceding and current row
        ) as ytd_max_fill_pct,

        ---------- 12-week moving average
        round(avg(r.fill_pct) over (
            partition by r.el_area
            order by r.observation_date
            rows between 11 preceding and current row
        ), 2) as fill_pct_12w_avg

    from reservoir r

    left join area_info a
        on r.el_area = a.el_area

)

select * from trends
