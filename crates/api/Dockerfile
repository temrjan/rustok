# Stage 1: Build
FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Stub for workspace member app/src-tauri (not needed for API, but workspace requires it).
COPY app/src-tauri/Cargo.toml app/src-tauri/Cargo.toml
RUN mkdir -p app/src-tauri/src && \
    echo "fn main(){}" > app/src-tauri/src/main.rs && \
    echo "" > app/src-tauri/src/lib.rs

# Build only the API binary in release mode.
RUN cargo build --release -p rustok-api && \
    strip target/release/rustok-api

# Stage 2: Minimal runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rustok-api /usr/local/bin/rustok-api

ENV RUST_LOG=info
EXPOSE 3000

CMD ["rustok-api"]
