// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "FlowLikeWasmNode",
    dependencies: [
        .package(path: "../wasm-sdk-swift"),
    ],
    targets: [
        .executableTarget(
            name: "Node",
            dependencies: [
                .product(name: "FlowLikeSDK", package: "wasm-sdk-swift"),
            ],
            path: "Sources/Node",
            linkerSettings: [
                .unsafeFlags([
                    "-Xlinker", "--export=get_node",
                    "-Xlinker", "--export=get_nodes",
                    "-Xlinker", "--export=run",
                    "-Xlinker", "--export=alloc",
                    "-Xlinker", "--export=dealloc",
                    "-Xlinker", "--export=get_abi_version",
                ]),
            ]
        ),
    ]
)
