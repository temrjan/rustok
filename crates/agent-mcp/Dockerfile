# Multi-stage build for rustok-agent-mcp
# Builder: compile the Rust binary
# Runtime: minimal Debian image with CA certificates

FROM rust:slim-bookworm AS builder

WORKDIR /build

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Create a minimal stub for the excluded workspace member so cargo can resolve
# the workspace graph without requiring app/src-tauri source files.
RUN mkdir -p app/src-tauri/src && \
    printf '[package]\nname = "rustok-desktop"\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.85"\nlicense = "AGPL-3.0-or-later"\npublish = false\n\n[lib]\nname = "rustok_desktop_lib"\ncrate-type = ["rlib"]\n' > app/src-tauri/Cargo.toml && \
    touch app/src-tauri/src/lib.rs

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
