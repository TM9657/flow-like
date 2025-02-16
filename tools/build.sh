#!/usr/bin/env bash

# Make sure the git submodules are up to date
git submodule update --init --recursive     
git submodule update --remote

# Build llama
cd llama
mkdir build
cd build
cmake .. -DCMAKE_ARGS="-DLLAMA_METAL_EMBED_LIBRARY=ON -DLLAMA_METAL=on"
cmake --build . --config Release
cd ..
mkdir ../src-tauri/lib
mkdir ../src-tauri/bin

# https://stackoverflow.com/questions/3466166/how-to-check-if-running-in-cygwin-mac-or-linux
# Copy the binaries to the tauri project
if [ "$(uname)" == "Darwin" ]; then
    # Do something under Mac OS X platform        
    cp ggml-metal.metal ../src-tauri/lib/ggml-metal.metal-aarch64-apple-darwin
    cp ./build/libggml_static.a ../src-tauri/lib/libggml_static.a-aarch64-apple-darwin
    cp ./build/libllama.a ../src-tauri/lib/libllama.a-aarch64-apple-darwin
    cp ./build/examples/llava/libllava_static.a ../src-tauri/lib/libllava_static.a-aarch64-apple-darwin
    cp ./build/bin/server ../src-tauri/bin/llm-server-aarch64-apple-darwin
    cp ./build/bin/ggml-common.h ../src-tauri/lib/ggml-common.h-aarch64-apple-darwin
elif [ "$(expr substr $(uname -s) 1 5)" == "Linux" ]; then
    # Do something under GNU/Linux platform
    cp ggml-metal.metal ../src-tauri/lib/ggml-metal.metal-unknown-linux-gnu
elif [ "$(expr substr $(uname -s) 1 10)" == "MINGW32_NT" ]; then
    # Do something under 32 bits Windows NT platform
    cp ggml-metal.metal ../src-tauri/lib/ggml-metal.metal-x86_64-pc-windows-msvc
elif [ "$(expr substr $(uname -s) 1 10)" == "MINGW64_NT" ]; then
    # Do something under 64 bits Windows NT platform
    cp ggml-metal.metal ../src-tauri/lib/ggml-metal.metal-x86_64-pc-windows-msvc
fi


