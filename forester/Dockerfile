FROM rust:1.79 AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y \
    clang \
    lld \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY .. .
RUN cargo build --release --package forester

FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/config
COPY --from=builder /app/target/release/forester /usr/local/bin/forester
WORKDIR /app

ENTRYPOINT ["/usr/local/bin/forester"]
CMD []