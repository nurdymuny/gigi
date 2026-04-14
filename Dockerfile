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
RUN cargo build --release --bin gigi-stream

# Stage 2: Runtime
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/gigi-stream /usr/local/bin/
RUN useradd -m gigi
USER gigi
EXPOSE 3142
CMD ["gigi-stream"]
