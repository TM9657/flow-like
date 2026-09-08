#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
recipe="${1:?Pass a Dockerfile with build-deps and runtime-deps stages}"
platform="${2:-linux/amd64}"
check_recipe="$(mktemp)"
trap 'rm -f "$check_recipe"' EXIT

# Extend the actual recipe so this checks its packages and compatibility object.
# BuildKit skips the unrelated stages that compile the complete application.
cat "$repo_root/$recipe" > "$check_recipe"
cat >> "$check_recipe" <<'DOCKERFILE'

FROM build-deps AS native-deps-check-build
COPY .github/scripts/tests/container-native-deps.c /tmp/native-deps-check.c
RUN libraries="wayland-client wayland-server libpipewire-0.3 egl gbm xcb-randr xcb-render x11 xi xtst libinput libudev xkbcommon OpenCL" \
    && pkg-config --exists $libraries \
    && cc -O2 /tmp/native-deps-check.c /tmp/ort-glibc-compat.o -o /tmp/native-deps-check \
       $(pkg-config --cflags --libs $libraries) -ldl -Wl,--no-as-needed -lstdc++ \
    && clang_library=$(find /usr/lib -name libclang.so.1 -print -quit) \
    && test -n "$clang_library" \
    && /tmp/native-deps-check "$clang_library" \
    && protoc --version

FROM runtime-deps AS native-deps-check-runtime
COPY --from=native-deps-check-build /tmp/native-deps-check /tmp/native-deps-check
RUN /tmp/native-deps-check
DOCKERFILE

docker buildx build --platform "$platform" --target native-deps-check-runtime \
  --output type=cacheonly -f "$check_recipe" "$repo_root"
