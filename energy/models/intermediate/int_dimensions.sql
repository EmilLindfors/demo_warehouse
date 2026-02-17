with

areas as (

    select * from {{ ref('stg_areas') }}

),

station_counts as (

    select
        el_area,
        count(distinct station_id) as weather_station_count
    from {{ ref('stg_precipitation') }}
    group by el_area

)

select
    a.el_area,
    a.area_number,
    a.area_name,
    a.area_description,
    coalesce(sc.weather_station_count, 0) as weather_station_count

from areas a

left join station_counts sc
    on a.el_area = sc.el_area
