---
title: Getting Started
description: Thank you very much for considering Contributions to our Project.
sidebar:
  order: 0
---

Thank you for considering contributing to Flow-Like! This document outlines the guidelines and best practices for contributing to this mostly Tauri-based project. Our application leverages Rust for the backend (via Tauri) and [Next.js/Astro] for the frontend. To ensure consistency and quality, please follow the practices below when contributing.

## Getting Started
1. Prerequisites: 
    1. Install [Rust](https://www.rust-lang.org/tools/install)
    2. Install [Bun](https://bun.sh/docs/installation)
    3. [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)
    4. Install Additional Dependencies (e.g Protobuf)
        - Linux: `protobuf-compiler, clang`
        - MacOS: `brew install protobuf`
2. Fork the Repository: Fork the repository to your GitHub account and clone it locally.
```bash
git clone https://github.com/your-username/flow-like.git
cd flow-like
```
3. Install Dependencies: `bun install`
4. Run the Project:
    - Desktop App: `bun run dev:desktop`
    - Docs: `bun run dev:docs`

## Code of Conduct

We aim to maintain a welcoming community. Please adhere to our [Code of Conduct](/contributing/code-of-conduct) in all interactions.

## How to Contribute

1. **Check Issues**: Look for open issues labeled `good first issue` or `help wanted` to get started.
2. **Create an Issue**: If you have a feature idea or a bug report, open an issue to discuss it first.
3. **Branching**: Create a new branch for your work:
```bash
git checkout -b feature/your-feature-name
```
4. **Commit Messages**: Write clear, concise commit messages (e.g "Add user authentication to backend" or "Fix button alignment in UI")

## Rust Guidelines

The core library of this project is written in Rust. Follow these best practices to ensure maintainability and performance.

### General Practices
- **Modularity**: Organize code into modules (e.g., lib.rs, main.rs, or subdirectories like src/commands/).
- **Error Handling**: Error Handling: Use `Result` and `Option` appropriately, avoiding `unwrap()` in production code. Prefer explicit error types with anyhow.
- **Safety**: NO unsafe code or dependencies using unsafe code.
- **Dependencies**: Keep external crates minimal and well-vetted. Update `Cargo.toml` accordingly.

### Formatting and Linting
- **cargo fmt**: Run `cargo fmt` before committing to enforce consistent Rust formatting.
- **Clippy**: Use `cargo clippy` to catch common mistakes and improve code quality. Resolve all warnings unless explicitly justified.
```bash title="Helpful command to auto resolve some warning"
# Please check the changes before committing
cargo clippy --fix
```

