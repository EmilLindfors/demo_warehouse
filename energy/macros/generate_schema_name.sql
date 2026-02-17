{#
    Override dbt's default schema naming which concatenates <target_schema>_<custom_schema>.
    Without this, +schema: staging in dbt_project.yml would produce "default_staging" instead of just "staging".
#}
{% macro generate_schema_name(custom_schema_name, node) -%}

    {%- set default_schema = target.schema -%}

    {%- if custom_schema_name is none -%}

        {{ default_schema }}

    {%- else -%}

        {{ custom_schema_name | trim }}

    {%- endif -%}

{%- endmacro %}
