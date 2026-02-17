with

source as (

    select * from {{ source('raw_nve', 'reservoir_min_max_median') }}

),

electricity_areas as (

    select
        *,
        concat('NO', omrnr) as el_area

    from source
    where omr_type = 'EL'

),

renamed as (

    select

        ---------- ids
        el_area,

        ---------- time
        cast(iso_uke as int)                            as iso_week,

        ---------- measures (NVE pre-computed historical bands, converted to %)
        round(min_fyllingsgrad * 100, 2)                as min_fill_pct,
        round(max_fyllingsgrad * 100, 2)                as max_fill_pct,
        round(median_fyllings_grad * 100, 2)            as median_fill_pct

    from electricity_areas

)

select * from renamed
