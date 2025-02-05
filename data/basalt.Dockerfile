FROM rust:1.80 as basalt-compilation

RUN git clone https://github.com/basalt-rs/basalt-server

RUN apt-get update
RUN apt-get install -y protobuf-compiler

WORKDIR /basalt-server

RUN cargo build --release

# DO NOT EDIT UNLESS YOU KNOW WHAT YOU'RE DOING
FROM fedora:rawhide as setup

WORKDIR /setup

RUN echo "{{ installsh }}" > install.sh
RUN chmod +x install.sh
RUN ./install.sh

FROM setup as execution

WORKDIR /execution

COPY --from=basalt-compilation /basalt-server/target/release/basalt-server .

RUN touch ./init.sh
RUN echo "{{ initsh }}" > ./init.sh
RUN chmod +x ./init.sh
RUN touch ./entrypoint.sh
RUN echo "{{ entrypointsh }}" > ./entrypoint.sh
RUN chmod +x ./entrypoint.sh

EXPOSE 9090
ENTRYPOINT [ "./entrypoint.sh" ]
CMD [ "./basalt-server", "run", "9090"  ]
