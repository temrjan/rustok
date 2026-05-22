# Multi-stage build for rustok-agent-mcp
# Builder: compile the Rust binary
# Runtime: minimal Debian image with CA certificates

FROM rust:1.85-slim-bookworm AS builder

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
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for running the agent wallet
RUN groupadd -r rustok && useradd -r -g rustok -m rustok

# Pre-create data directory so named volumes inherit correct ownership
RUN mkdir -p /home/rustok/.rustok/agent && chown -R rustok:rustok /home/rustok/.rustok

COPY --from=builder --chown=rustok:rustok /build/target/release/rustok-agent-mcp /usr/local/bin/rustok-agent-mcp

USER rustok
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD bash -c 'printf "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n" > /dev/tcp/localhost/3000' || exit 1

ENTRYPOINT ["/usr/local/bin/rustok-agent-mcp"]
CMD ["--host", "0.0.0.0", "--port", "3000"]
