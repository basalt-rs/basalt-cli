FROM rust:1.84 as basalt-compilation

RUN git clone https://github.com/basalt-rs/basalt-server

WORKDIR /basalt-server

RUN cargo build --release

# DO NOT EDIT UNLESS YOU KNOW WHAT YOU'RE DOING
FROM fedora:rawhide as setup

WORKDIR /setup

COPY install.sh .
RUN chmod +x install.sh
RUN ./install.sh

FROM setup as execution

WORKDIR /execution

COPY --from=basalt-compilation /basalt-server/target/release/basalt-server .

COPY config.toml .
COPY entrypoint.sh .
RUN chmod +x ./entrypoint.sh

EXPOSE 9090
ENTRYPOINT [ "./entrypoint.sh" ]
CMD [ "./basalt-server", "run", "--port", "9090", "./config.toml" ]
