FROM rust:1.79 AS builder
WORKDIR /app
RUN apt-get update && apt install -y clang lld pkgconf libssl-dev
ENV OPENSSL_DIR=/usr
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include

COPY .. .
RUN cargo build --release --package forester

FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/config
COPY --from=builder /app/target/release/forester /usr/local/bin/forester
WORKDIR /app

ENTRYPOINT ["/usr/local/bin/forester"]
CMD []