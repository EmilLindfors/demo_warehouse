with

source as (

    select * from {{ source('raw_nve', 'areas__elspot') }}

),

renamed as (

    select

        ---------- ids
        concat('NO', omrnr)                             as el_area,
        cast(omrnr as int)                               as area_number,

        ---------- names (area name is the first word before ". " in beskrivelse)
        trim(split_part(beskrivelse, '.', 1))            as area_name,
        beskrivelse                                      as area_description

    from source

)

select * from renamed
