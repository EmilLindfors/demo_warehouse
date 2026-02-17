with

source as (

    select * from {{ source('raw_frost', 'precipitation') }}

),

renamed as (

    select

        ---------- ids
        station_id,
        el_area,

        ---------- text
        station_name,

        ---------- measures
        reference_time                          as observation_date,
        precipitation_mm,
        quality_code,
        -- we only include precipitation data that is verified (e.g. code = 0)
        quality_code = 0                        as is_verified,

        ---------- metadata
        ingested_at

    from source
    where precipitation_mm is not null

)

select * from renamed
