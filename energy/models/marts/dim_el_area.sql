select
    el_area,
    area_number,
    area_name,
    area_description,
    weather_station_count

from {{ ref('int_dimensions') }}
