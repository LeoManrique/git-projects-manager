# Build stage
FROM rust:1.95-slim-bookworm AS builder
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/gpm-sync-server /usr/local/bin/gpm-sync-server

ENV BIND_ADDR=0.0.0.0:8787
ENV DATABASE_URL=/data/sync.db
EXPOSE 8787

VOLUME ["/data"]
CMD ["gpm-sync-server"]
