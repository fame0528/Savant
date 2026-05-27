# Stage 1: Build
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Copy workspace manifests and lock file
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY lib/ lib/

# Build the savant binary in release mode
RUN cargo build --release --bin savant

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/savant /usr/local/bin/savant

# Copy config and create data directories
COPY config/ /app/config/
RUN mkdir -p /app/data /app/workspaces

WORKDIR /app

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:8080/live || exit 1

ENTRYPOINT ["savant"]
CMD ["start"]
