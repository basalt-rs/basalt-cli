#!/bin/sh
echo "ENTRYPOINT: Initializing"
echo "INIT: Running base init"
{{base_init}}

{% if custom_init %}
echo "INIT: Running custom init"
{{ custom_init }}
{% endif %}

echo "ENTRYPOINT: Executing"
exec "$@"
