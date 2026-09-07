# syntax=docker/dockerfile:1.4
# ============================================================
# Stage 1: Builder
# ============================================================
FROM rust:1.97.1-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

# Limit build parallelism to reduce memory pressure during lance compilation
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}

# Use BuildKit cache mounts for faster rebuilds
RUN --mount=type=cache,target=/cargo-home,id=cargo-home,sharing=shared \
    --mount=type=cache,target=/app/target,id=cargo-target-runtime,sharing=locked \
    CARGO_HOME=/cargo-home cargo build --locked --release --package docker-compose-runtime && \
    cp /app/target/release/docker-compose-runtime /tmp/docker-compose-runtime

# ============================================================
# Stage 2: Runtime
# ============================================================
FROM debian:bookworm-slim

COPY LICENSE /usr/share/licenses/flow-like/LICENSE

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /tmp/docker-compose-runtime /app/runtime

RUN useradd -r -u 1000 flowlike && \
    chown -R flowlike:flowlike /app

USER flowlike

EXPOSE 9000

ENV RUST_LOG=info

CMD ["/app/runtime"]
