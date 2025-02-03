#!usr/env/sh
echo "INSTALL: running base install"
{{ base_install }}

{% if custom_install %}
echo "INSTALL: running custom install"
{{ custom_install }}
{% endif %}
