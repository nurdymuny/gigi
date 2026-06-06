# Stage 1: Build
FROM rust:1.92-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY openapi.json ./
COPY src/ ./src/
COPY benches/ ./benches/
COPY examples/ ./examples/
COPY dashboard/ ./dashboard/
RUN cargo build --release --features "kahler imagine sharded transactions patterns" --bin gigi-stream

# Stage 2: Runtime
FROM debian:trixie-slim
# awscli: needed by gigi-stream's Tigris S3 push at startup. Tigris is
# fly.io's S3-compatible storage (NOT Amazon — credentials in fly.toml
# point at fly's TIGRIS endpoint). The aws CLI is just a generic S3
# client we shell out to for the /data/ → gigi-snapshots/ sync.
# Without it, the push silently fails on every startup and the only
# copy of production data is the local fly volume.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates awscli && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/gigi-stream /usr/local/bin/
RUN useradd -m gigi
USER gigi
EXPOSE 3142
CMD ["gigi-stream"]
