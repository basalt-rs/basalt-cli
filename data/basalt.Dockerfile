FROM ghcr.io/basalt-rs/basalt-server:{{ server_tag }} AS basalt-server-base
{% if web_dir %}
FROM ghcr.io/basalt-rs/basalt-web:{{ web_tag }} AS basalt-web-base
{% endif %}

# TODO: build packet

FROM fedora:rawhide as setup

WORKDIR /setup

COPY install.sh .
RUN chmod +x install.sh && ./install.sh

FROM setup as execution

WORKDIR /execution

COPY --from=basalt-server-base /usr/local/bin/basalt-server /usr/local/bin/basalt-server
{% if web_dir %}
COPY --from=basalt-web-base /web {{ web_dir }}
{% endif %}

COPY . .
RUN chmod +x ./entrypoint.sh

EXPOSE 9090
# the CMD will be executed within the context of the execution of the ENTRYPOINT
ENTRYPOINT [ "./entrypoint.sh" ]

CMD {{ exec_command }}
