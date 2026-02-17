-- Pulls from dbt mart: dim_el_area
select
    el_area,
    area_number,
    area_name,
    area_description,
    weather_station_count
from marts.dim_el_area
order by el_area
