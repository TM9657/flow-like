# ============================================================
# Stage 1: Chef - install cargo-chef for dependency caching
# ============================================================
FROM rust:1.87-bookworm AS chef

RUN cargo install cargo-chef
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/* \
    && rustup toolchain install nightly-2025-12-15 \
    && rustup default nightly-2025-12-15

WORKDIR /app

# ============================================================
# Stage 2: Planner - analyze dependencies and create recipe
# ============================================================
FROM chef AS planner

COPY . .

# Remove packages that require edition 2024
RUN sed -i '/apps\/desktop\/src-tauri/d' Cargo.toml && \
    sed -i '/apps\/backend\/local\/api/d' Cargo.toml && \
    sed -i '/apps\/backend\/kubernetes\/api/d' Cargo.toml && \
    sed -i '/apps\/backend\/kubernetes\/executor/d' Cargo.toml

RUN cargo chef prepare --recipe-path recipe.json

# ============================================================
# Stage 3: Builder - build dependencies (cached) then source
# ============================================================
FROM chef AS builder

# Copy recipe and build dependencies (this layer is cached if dependencies don't change)
COPY --from=planner /app/recipe.json recipe.json

# Limit build parallelism to reduce memory pressure during lance compilation
ENV CARGO_BUILD_JOBS=2
ENV RUSTFLAGS="--cfg tokio_unstable"

RUN cargo chef cook --release --recipe-path recipe.json

# Now copy source and build (only this rebuilds when source changes)
COPY . .

# Remove packages that require edition 2024
RUN sed -i '/apps\/desktop\/src-tauri/d' Cargo.toml && \
    sed -i '/apps\/backend\/local\/api/d' Cargo.toml && \
    sed -i '/apps\/backend\/kubernetes\/api/d' Cargo.toml && \
    sed -i '/apps\/backend\/kubernetes\/executor/d' Cargo.toml

# Build release binary
RUN cargo build --release --package docker-compose-api

# Runtime stage - matching GLIBC version with builder
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/docker-compose-api /app/api

RUN useradd -r -u 1000 flowlike && \
    chown -R flowlike:flowlike /app

USER flowlike

EXPOSE 8080 9090

ENV RUST_LOG=info

CMD ["/app/api"]
