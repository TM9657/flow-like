const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });
    const optimize = b.standardOptimizeOption(.{});

    const sdk_dep = b.dependency("flow_like_sdk", .{
        .target = target,
        .optimize = optimize,
    });

    const lib = b.addExecutable(.{
        .name = "node",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    lib.root_module.addImport("flow_like_sdk", sdk_dep.module("flow_like_sdk"));
    lib.entry = .disabled;
    lib.rdynamic = true;

    b.installArtifact(lib);
}
