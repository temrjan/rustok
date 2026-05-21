# Multi-stage build for rustok-agent-mcp
# Builder: compile the Rust binary
# Runtime: minimal Debian image with CA certificates

FROM rust:1.85-slim-bookworm AS builder

WORKDIR /build

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build only the agent-mcp crate and its dependencies
RUN cargo build --release -p rustok-agent-mcp

# ─── Runtime stage ───────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/rustok-agent-mcp /usr/local/bin/rustok-agent-mcp

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/rustok-agent-mcp"]
