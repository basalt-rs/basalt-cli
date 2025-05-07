FROM rust:1.84 as basalt-compilation

RUN touch /redocly
RUN chmod +x /redocly
ENV PATH=/:$PATH
RUN git clone https://github.com/basalt-rs/basalt-server

WORKDIR /basalt-server
RUN cargo build --release --no-default-features

{% if web_client %}
FROM node:22 as web-compilation

RUN git clone https://github.com/basalt-rs/basalt /basalt

WORKDIR /basalt
RUN git checkout Sync-Leaderboard
WORKDIR /basalt/client
RUN npm ci
RUN npm run build
{% endif %}

# DO NOT EDIT UNLESS YOU KNOW WHAT YOU'RE DOING
FROM fedora:rawhide as setup

WORKDIR /setup

COPY install.sh .
RUN chmod +x install.sh
RUN ./install.sh

FROM setup as execution

WORKDIR /execution

COPY --from=basalt-compilation /basalt-server/target/release/basalt-server .
{% if web_client %}
COPY --from=web-compilation /basalt/client/out ./web/
{% endif %}

COPY config.toml .
COPY entrypoint.sh .
RUN chmod +x ./entrypoint.sh

EXPOSE 9090
ENTRYPOINT [ "./entrypoint.sh" ]
{% if web_client %}
CMD [ "./basalt-server", "run", "--port", "9090", "./config.toml", "-w", "./web/" ]
{% else %}
CMD [ "./basalt-server", "run", "--port", "9090", "./config.toml" ]
{% endif %}
