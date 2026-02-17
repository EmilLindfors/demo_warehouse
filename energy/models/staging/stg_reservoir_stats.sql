with

source as (

    select * from {{ source('raw_nve', 'reservoir_stats') }}

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
        cast(omrnr as int)                      as area_number,

        ---------- time
        cast(dato_id as date)                   as observation_date,
        cast(iso_aar as int)                    as iso_year,
        cast(iso_uke as int)                    as iso_week,

        ---------- measures
        round(fyllingsgrad * 100, 2)            as fill_pct,
        round(fyllingsgrad_forrige_uke * 100, 2) as fill_pct_prev_week,
        round(endring_fyllingsgrad * 100, 2)    as fill_pct_change,
        round(kapasitet_t_wh, 3)                as capacity_twh,
        round(fylling_t_wh, 3)                  as fill_twh

    from electricity_areas

)

select * from renamed
