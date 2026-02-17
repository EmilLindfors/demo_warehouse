with

stations as (

    select distinct
        station_id,
        station_name,
        el_area
    from {{ ref('stg_precipitation') }}

)

select
    station_id,
    station_name,
    el_area

from stations
