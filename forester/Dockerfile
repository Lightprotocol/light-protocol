FROM rust:1.79 AS builder
WORKDIR /app
RUN apt-get update && apt install -y clang lld
COPY .. .
RUN cargo build --release --package forester

FROM debian:12-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/config
COPY --from=builder /app/target/release/forester /usr/local/bin/forester
WORKDIR /app

RUN echo '#!/bin/sh' > /usr/local/bin/start.sh && \
    echo 'echo "Starting Forester service..."' >> /usr/local/bin/start.sh && \
    echo 'echo "Arguments received: $@"' >> /usr/local/bin/start.sh && \
    echo 'exec forester "$@"' >> /usr/local/bin/start.sh && \
    chmod +x /usr/local/bin/start.sh

ENTRYPOINT ["/usr/local/bin/start.sh"]