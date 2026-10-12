FROM rust:1.94.0-slim-bookworm@sha256:a86cada82e36ebd7a9bffed7548792c55a952fdb20718eea9278a936bcb76e62 AS build
COPY rust-toolchain.toml /rust-toolchain.toml
RUN rustup toolchain install
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates cmake curl gcc g++ make libssl-dev openssl \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://github.com/eclipse-cyclonedds/cyclonedds/archive/refs/tags/11.0.1.tar.gz -o /tmp/cyclonedds.tar.gz \
    && echo 'c25d46075ad6b5cee564bda9e5f49509e9a6dbb1a8b858e708eb360d335cf973  /tmp/cyclonedds.tar.gz' | sha256sum -c - \
    && tar -xzf /tmp/cyclonedds.tar.gz -C /tmp \
    && cmake -S /tmp/cyclonedds-11.0.1 -B /tmp/cyclone-build \
       -DBUILD_EXAMPLES=OFF -DBUILD_TESTING=OFF -DENABLE_SECURITY=ON -DENABLE_SSL=ON \
       -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/opt/cyclone \
    && cmake --build /tmp/cyclone-build --parallel 4 \
    && cmake --install /tmp/cyclone-build
WORKDIR /work/.github/fixtures/industrial/dds
COPY .github/fixtures/industrial/dds/Cargo.toml .github/fixtures/industrial/dds/Cargo.lock .github/fixtures/industrial/dds/lib.rs ./
COPY .github/fixtures/industrial/dds/CMakeLists.txt .github/fixtures/industrial/dds/Telemetry.idl .github/fixtures/industrial/dds/reference.c .github/fixtures/industrial/dds/abi-checks.h ./
COPY packages/vendor/dust-dds /work/packages/vendor/dust-dds
COPY packages/industrial/src/dds.rs /work/packages/industrial/src/dds.rs
COPY packages/industrial/src/dds /work/packages/industrial/src/dds
COPY .github/fixtures/industrial/dds/generate-security.sh ./
COPY packages/industrial/tests/dds_e2e.rs /work/packages/industrial/tests/dds_e2e.rs
RUN sh generate-security.sh /opt/dds-security \
    && cmake -S . -B /tmp/reference-build -DCMAKE_PREFIX_PATH=/opt/cyclone -DCMAKE_BUILD_TYPE=Release \
    && cmake --build /tmp/reference-build --parallel 4 \
    && cargo test --locked --lib \
    && cargo test --locked --test dds_e2e --no-run \
    && find target/debug/deps -maxdepth 1 -type f -name 'dds_e2e-*' -executable -exec cp '{}' /dds-e2e ';'

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /opt/cyclone/lib/libddsc.so* /opt/cyclone/lib/libdds_security_*.so /usr/local/lib/
COPY --from=build /opt/dds-security /opt/dds-security
RUN ldconfig
COPY --from=build /tmp/reference-build/dds-reference /usr/local/bin/dds-reference
COPY --from=build /dds-e2e /usr/local/bin/dds-e2e
COPY --chmod=755 .github/fixtures/industrial/dds/run.sh /usr/local/bin/run-dds-e2e
ENV FLOW_LIKE_DDS_REFERENCE=/usr/local/bin/dds-reference
ENV FLOW_LIKE_DDS_SECURITY_DIR=/opt/dds-security
USER 65534:65534
ENTRYPOINT ["run-dds-e2e"]
