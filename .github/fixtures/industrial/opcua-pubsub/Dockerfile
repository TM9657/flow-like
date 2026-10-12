FROM rust:1.94.0-slim-bookworm@sha256:a86cada82e36ebd7a9bffed7548792c55a952fdb20718eea9278a936bcb76e62 AS rust-build
COPY rust-toolchain.toml /rust-toolchain.toml
RUN rustup toolchain install
WORKDIR /work/.github/fixtures/industrial/opcua-pubsub
COPY .github/fixtures/industrial/opcua-pubsub/Cargo.toml .github/fixtures/industrial/opcua-pubsub/Cargo.lock .github/fixtures/industrial/opcua-pubsub/lib.rs ./
COPY packages/industrial/src/opcua_pubsub.rs /work/packages/industrial/src/opcua_pubsub.rs
COPY packages/industrial/tests/opcua_pubsub_e2e.rs /work/packages/industrial/tests/opcua_pubsub_e2e.rs
RUN CARGO_BUILD_JOBS=2 cargo test --locked --lib \
    && CARGO_BUILD_JOBS=2 cargo test --locked --test opcua_pubsub_e2e --no-run \
    && find target/debug/deps -maxdepth 1 -type f -name 'opcua_pubsub_e2e-*' -executable -exec cp '{}' /opcua-pubsub-e2e ';'

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS peer-build
RUN apt-get update -qq && apt-get install -y --no-install-recommends gcc libc6-dev libmbedtls-dev cmake make python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ADD --checksum=sha256:c56d5c034e13cfd0fc642e3ac9a85a6aad8f4b1612040c8d4e87f9b9a2df12a5 https://codeload.github.com/open62541/open62541/tar.gz/refs/tags/v1.4.21 /tmp/open62541.tar.gz
RUN mkdir /opt/open62541 && tar -xzf /tmp/open62541.tar.gz --strip-components=1 -C /opt/open62541
COPY .github/fixtures/industrial/opcua-pubsub/CMakeLists.txt .github/fixtures/industrial/opcua-pubsub/peer.c /fixture/
RUN cmake -S /fixture -B /tmp/peer -DOPEN62541_SOURCE=/opt/open62541 -DCMAKE_BUILD_TYPE=Release \
    && cmake --build /tmp/peer --target opcua-pubsub-peer -j2

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
RUN apt-get update -qq && apt-get install -y --no-install-recommends libmbedtls14 libmbedx509-1 libmbedcrypto7 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=peer-build /tmp/peer/opcua-pubsub-peer /usr/local/bin/opcua-pubsub-peer
COPY --from=peer-build /opt/open62541/LICENSE /usr/share/licenses/flow-opcua-pubsub/open62541.MPL-2.0
COPY --from=rust-build /opcua-pubsub-e2e /usr/local/bin/opcua-pubsub-e2e
COPY .github/fixtures/industrial/opcua-pubsub/run.sh /fixture/run.sh
ENV FLOW_LIKE_OPCUA_PUBSUB_PEER=/usr/local/bin/opcua-pubsub-peer
ENTRYPOINT ["sh", "/fixture/run.sh"]
