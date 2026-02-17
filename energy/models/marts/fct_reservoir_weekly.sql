with

reservoir as (

    select * from {{ ref('int_reservoir_enriched') }}

),

final as (

    select

        ---------- keys
        el_area,
        observation_date,
        iso_year,
        iso_week,

        ---------- current measures
        fill_pct,
        fill_twh,
        capacity_twh,
        fill_pct_change,

        ---------- historical context
        hist_min_fill_pct,
        hist_max_fill_pct,
        hist_median_fill_pct,
        fill_pct_vs_median,
        fill_status,

        ---------- year-over-year
        fill_pct_prev_year,
        fill_pct_yoy_change

    from reservoir

)

select * from final
