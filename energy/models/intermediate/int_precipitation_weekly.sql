with

daily as (

    select * from {{ ref('stg_precipitation') }}

),

station_weekly as (

    select

        el_area,
        station_id,
        cast(case
            when weekofyear(observation_date) = 1 and month(observation_date) = 12
                then year(observation_date) + 1
            when weekofyear(observation_date) >= 52 and month(observation_date) = 1
                then year(observation_date) - 1
            else year(observation_date)
        end as int)                                         as iso_year,
        cast(weekofyear(observation_date) as int)           as iso_week,
        min(observation_date)                        as week_start_date,
        max(observation_date)                        as week_end_date,
        count(*)                                     as days_with_data,
        round(sum(precipitation_mm), 1)              as weekly_precipitation_mm,
        round(avg(precipitation_mm), 1)              as avg_daily_precipitation_mm,
        round(max(precipitation_mm), 1)              as max_daily_precipitation_mm,
        sum(case when precipitation_mm > 0 then 1 else 0 end) as rainy_days

    from daily
    where is_verified
    group by
        el_area,
        station_id,
        case
            when weekofyear(observation_date) = 1 and month(observation_date) = 12
                then year(observation_date) + 1
            when weekofyear(observation_date) >= 52 and month(observation_date) = 1
                then year(observation_date) - 1
            else year(observation_date)
        end,
        weekofyear(observation_date)

),

area_weekly as (

    select

        el_area,
        iso_year,
        iso_week,
        min(week_start_date)                             as week_start_date,
        max(week_end_date)                               as week_end_date,
        count(distinct station_id)                       as station_count,
        round(avg(weekly_precipitation_mm), 1)           as weekly_precipitation_mm,
        round(avg(avg_daily_precipitation_mm), 1)        as avg_daily_precipitation_mm,
        round(max(max_daily_precipitation_mm), 1)        as max_daily_precipitation_mm,
        round(avg(rainy_days), 1)                        as avg_rainy_days,
        round(min(weekly_precipitation_mm), 1)           as min_station_weekly_mm,
        round(max(weekly_precipitation_mm), 1)           as max_station_weekly_mm,
        round(avg(days_with_data), 1)                    as avg_days_with_data

    from station_weekly
    group by
        el_area,
        iso_year,
        iso_week

)

select * from area_weekly
