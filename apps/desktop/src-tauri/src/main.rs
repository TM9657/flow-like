// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Compile the shared application implementation into the desktop executable
// directly. The package library is reserved for Tauri's mobile entry point, so
// its required `staticlib`/`cdylib` outputs remain tiny on host builds.
include!("application.rs");

#[cfg(not(any(all(target_os = "macos", target_arch = "aarch64"), target_os = "ios")))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.iter().any(|arg| {
        arg == "--flowpilot-workflow-benchmark"
            || arg
                .to_str()
                .is_some_and(|arg| arg.starts_with("--flowpilot-workflow-benchmark="))
    }) {
        #[cfg(all(debug_assertions, desktop))]
        std::process::exit(functions::ai::copilot::run_workflow_benchmark_cli(args));
        #[cfg(not(all(debug_assertions, desktop)))]
        {
            eprintln!("Workflow benchmark CLI requires a development build.");
            std::process::exit(2);
        }
    }
    run()
}
