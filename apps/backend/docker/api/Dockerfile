FROM rust:1.81.0-bookworm AS build
ARG upx_version=4.2.4

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libgtk-3-dev \
    libclang-dev \
    libjavascriptcoregtk-4.1-dev \
    libsoup-3.0-dev \
    pkg-config \
    protobuf-compiler \
    webkit2gtk-4.1 \
    xz-utils && \
    curl -Ls https://github.com/upx/upx/releases/download/v${upx_version}/upx-${upx_version}-amd64_linux.tar.xz -o - | tar xvJf - -C /tmp && \
    cp /tmp/upx-${upx_version}-amd64_linux/upx /usr/local/bin/ && \
    chmod +x /usr/local/bin/upx && \
    apt-get remove -y xz-utils && \
    rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release --manifest-path=apps/backend/Cargo.toml
RUN upx --ultra-brute -qq target/release/backend && upx -t target/release/backend

FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# Copy only the compiled binary
COPY --from=build /app/target/release/backend /app/backend

# Set a non-root user
USER 1000:1000

EXPOSE 3000

CMD ["./backend"]