# Build with --platform linux/amd64 to match the bundled tools and ONNX runtime.
FROM public.ecr.aws/amazonlinux/amazonlinux:2023 AS build

ARG ONNXRUNTIME_VERSION=1.24.2
ARG ONNXRUNTIME_SHA256=43725474ba5663642e17684717946693850e2005efbd724ac72da278fead25e6

WORKDIR /app

RUN yum update -y && \
    yum install -y \
        clang \
        git \
        pkgconf-pkg-config \
        openssl-devel \
        glib2-devel \
        gtk3-devel \
        tar \
        unzip \
        xz && \
    curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v25.3/protoc-25.3-linux-x86_64.zip && \
    unzip protoc-25.3-linux-x86_64.zip -d /usr/local && \
    rm protoc-25.3-linux-x86_64.zip && \
    curl -fsSL --retry 3 \
        "https://github.com/microsoft/onnxruntime/releases/download/v${ONNXRUNTIME_VERSION}/onnxruntime-linux-x64-${ONNXRUNTIME_VERSION}.tgz" \
        -o /tmp/onnxruntime.tgz && \
    echo "${ONNXRUNTIME_SHA256}  /tmp/onnxruntime.tgz" | sha256sum -c - && \
    mkdir -p /opt/onnxruntime && \
    tar -xzf /tmp/onnxruntime.tgz --strip-components=1 -C /opt/onnxruntime && \
    rm /tmp/onnxruntime.tgz && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.97.1 && \
    yum remove -y xz && \
    yum clean all

ENV PATH="/root/.cargo/bin:${PATH}"
ENV CC=clang
ENV ORT_LIB_PATH=/opt/onnxruntime/lib
ENV ORT_PREFER_DYNAMIC_LINK=1
ENV LD_LIBRARY_PATH=/opt/onnxruntime/lib
RUN ln -sf /usr/bin/clang /usr/local/bin/cc
ARG CARGO_BUILD_JOBS

COPY . .

RUN --mount=type=cache,id=flow-like-rust-1.97.1-amazonlinux2023-amd64-registry,target=/root/.cargo/registry,sharing=locked \
    --mount=type=cache,id=flow-like-rust-1.97.1-amazonlinux2023-amd64-git,target=/root/.cargo/git,sharing=locked \
    --mount=type=cache,id=flow-like-rust-1.97.1-amazonlinux2023-amd64-target,target=/app/target,sharing=locked \
    source "$HOME"/.cargo/env && \
    if [ -n "${CARGO_BUILD_JOBS:-}" ]; then export CARGO_BUILD_JOBS; fi && \
    cargo build --locked --release --manifest-path=apps/backend/aws/executor-ecs/Cargo.toml 2>&1

RUN --mount=type=cache,id=flow-like-rust-1.97.1-amazonlinux2023-amd64-target,target=/app/target/,sharing=locked \
    cp target/release/aws-executor-ecs /app/aws-executor-ecs

RUN strip --strip-all /app/aws-executor-ecs

FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal

COPY LICENSE /usr/share/licenses/flow-like/LICENSE

COPY --from=build /opt/onnxruntime/lib/ /usr/lib64/
COPY --from=build /opt/onnxruntime/LICENSE /usr/share/licenses/onnxruntime/LICENSE
COPY --from=build /opt/onnxruntime/ThirdPartyNotices.txt /usr/share/licenses/onnxruntime/ThirdPartyNotices.txt
COPY --from=build /app/aws-executor-ecs /usr/local/bin/executor-async

ENTRYPOINT ["/usr/local/bin/executor-async"]
