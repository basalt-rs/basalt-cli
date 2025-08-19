FROM ghcr.io/basalt-rs/basalt-server:{{ server_tag }} AS basalt-server-base
{% if web_client %}
FROM ghcr.io/basalt-rs/basalt-web:{{ web_tag }} AS basalt-web-base
{% endif %}

# DO NOT EDIT UNLESS YOU KNOW WHAT YOU'RE DOING
FROM fedora:rawhide as setup

WORKDIR /setup

COPY install.sh .
RUN chmod +x install.sh && ./install.sh

FROM setup as execution

WORKDIR /execution

COPY --from=basalt-server-base /usr/local/bin/basalt-server /usr/local/bin/basalt-server
{% if web_client %}
COPY --from=basalt-web-base /web ./web/
{% endif %}

COPY config.toml .
COPY entrypoint.sh .
RUN chmod +x ./entrypoint.sh

EXPOSE 9090
# the CMD will be executed within the context of the execution of the ENTRYPOINT
ENTRYPOINT [ "./entrypoint.sh" ]
{% if web_client %}
CMD [ "basalt-server", "run", "--port", "9090", "./config.toml", "-w", "./web/" ]
{% else %}
CMD [ "basalt-server", "run", "--port", "9090", "./config.toml" ]
{% endif %}
