#!/bin/sh
echo "ENTRYPOINT: Initializing"
echo "INIT: Running base init"
{{ base_init }}

{% if custom_init %}
echo "INIT: Running custom init"
{{ custom_init }}
{% endif %}

LOG_DIR="/var/log/basalt"
mkdir -p "$LOG_DIR"

DATE=$(date +%Y-%m-%d_%H-%M-%S)
STDOUT_LOG="$LOG_DIR/$DATE.log"

echo "ENTRYPOINT: Executing"
exec "$@" | tee -a "$STDOUT_LOG"
