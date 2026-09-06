#!/usr/bin/env bash
set -euo pipefail

: "${RUNNER_TEMP:?RUNNER_TEMP must be set}"
: "${GITHUB_ENV:?GITHUB_ENV must be set}"
: "${GITHUB_PATH:?GITHUB_PATH must be set}"

# Ubuntu 22.04 has no glslc package. This SDK supports its glibc version and
# supplies the newer Vulkan and SPIR-V headers required by ggml.
sdk_version="1.4.313.0"
# Published at https://vulkan.lunarg.com/sdk/files.json.
sdk_sha256="4e957b66ade85eeaee95932aa7e3b45aea64db373c58a5eaefc8228cc71445c2"
sdk_root="$RUNNER_TEMP/flow-like-vulkan-sdk"
sdk_path="$sdk_root/$sdk_version/x86_64"
archive="$sdk_root/vulkansdk-linux-x86_64-$sdk_version.tar.xz"
mkdir -p "$sdk_root"
trap 'rm -f "$archive"' EXIT

curl --fail --location --retry 3 \
  --output "$archive" \
  "https://sdk.lunarg.com/sdk/download/$sdk_version/linux/vulkansdk-linux-x86_64-$sdk_version.tar.xz"
printf '%s  %s\n' "$sdk_sha256" "$archive" | sha256sum --check --strict

# Keep linking against Ubuntu's libvulkan. SDK loader libraries must not enter
# the app bundle or raise its minimum glibc version.
tar -xJf "$archive" -C "$sdk_root" \
  "$sdk_version/x86_64/bin/glslc" \
  "$sdk_version/x86_64/include" \
  "$sdk_version/x86_64/share/cmake/SPIRV-Headers"
"$sdk_path/bin/glslc" --version
printf 'VULKAN_SDK=%s\n' "$sdk_path" >> "$GITHUB_ENV"
printf '%s/bin\n' "$sdk_path" >> "$GITHUB_PATH"
