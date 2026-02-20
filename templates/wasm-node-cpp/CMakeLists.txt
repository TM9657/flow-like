cmake_minimum_required(VERSION 3.14)
project(flow_like_wasm_node LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# --- SDK via FetchContent (always pulls latest from the dev branch) ---
include(FetchContent)
if(EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/../wasm-sdk-cpp/CMakeLists.txt)
    # Local SDK (for in-tree / monorepo builds)
    FetchContent_Declare(
        flow_like_wasm_sdk
        SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../wasm-sdk-cpp
    )
else()
    # Remote SDK (for standalone template usage)
    FetchContent_Declare(
        flow_like_wasm_sdk
        GIT_REPOSITORY https://github.com/TM9657/flow-like.git
        GIT_TAG        dev
        SOURCE_SUBDIR  libs/wasm-sdk/wasm-sdk-cpp
    )
endif()
FetchContent_MakeAvailable(flow_like_wasm_sdk)

# --- Node WASM module ---
add_executable(node src/node.cpp)
target_link_libraries(node PRIVATE flow_like_wasm_sdk)

if(EMSCRIPTEN)
    set_target_properties(node PROPERTIES
        OUTPUT_NAME "node"
        SUFFIX ".wasm"
    )
    target_link_options(node PRIVATE
        -sSTANDALONE_WASM
        -sEXPORTED_FUNCTIONS=_get_node,_run,_alloc,_dealloc,_get_abi_version
        --no-entry
        -sERROR_ON_UNDEFINED_SYMBOLS=0
        -sALLOW_MEMORY_GROWTH=1
        -O2
    )
    target_compile_options(node PRIVATE
        -fno-exceptions
        -fno-rtti
        -O2
    )
endif()
