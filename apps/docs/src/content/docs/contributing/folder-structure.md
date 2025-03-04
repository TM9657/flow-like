---
title: Project Structure
description: Keep it organized
sidebar:
  order: 1
---

## Flow-Like Project Structure
This document provides an overview of the Flow-Like project's architecture and folder organization to help new contributors understand how the codebase is organized.

## Root Structure
The Flow-Like project follows a monorepo structure with dedicated directories for applications, shared packages, and project configuration:

```
flow-like/
├── apps/          # Standalone applications
├── packages/      # Shared libraries and components
├── .github/       # GitHub workflows and configuration
├── assets/        # Project assets (mostly for github)
├── tests/         # Project-wide tests & test data
├── tools/         # Development and build tools
└── README.md      # Project documentation
```

## Applications (`apps`)
The apps directory contains standalone applications that are part of the Flow-Like ecosystem:

```
apps/
├── desktop/       # Main desktop application (Tauri)
├── backend/       # Backend services
├── docs/          # Documentation site
├── schema-gen/    # Schema generation tools
├── web-app/       # Web application frontend
└── website/       # Marketing website
```

### Desktop Application (`desktop`)
The desktop application is built with Tauri (Rust) and modern web technologies:

```
apps/desktop/
├── src-tauri/         # Rust code for the Tauri desktop application
│   ├── assets/        # Application assets
│   └── src/           # Rust source code
├── components/        # React components specific to the desktop app
├── app/               # Next.js application structure
├── public/            # Public static assets
└── scripts/           # Build and utility scripts
```

## Shared Libraries (`packages`)
The packages directory contains shared code that's reused across applications:

```
packages/
├── core/              # Core Rust logic library
├── schema/            # Data schema definitions
└── ui/                # Shared UI components
```

### Core Logic (`core`)
The core logic package contains the Rust implementation of Flow-Like's core functionality:

```
packages/core/
├── src/
│   ├── bit.rs         # Bit implementation
│   ├── flow/          # Flow execution engine
│   │   ├── catalog/   # Node catalog
│   │   ├── execution/ # Execution engine
│   │   ├── node.rs    # Node implementation
│   │   └── ...
│   ├── models/        # Data models
│   ├── state/         # Application state management
│   ├── utils/         # Utility functions
│   └── lib.rs         # Library entry point
├── Cargo.toml         # Rust dependencies and configuration
└── tests/             # Core library tests
```

### UI Components (`ui`)
The UI package contains shared React components used across applications:

```
packages/ui/
├── components/        # React component library
│   ├── flow/          # Flow-specific components
│   │   ├── node/      # Node rendering components
│   │   ├── variables/ # Variable handling components
│   │   └── flow-preview.tsx # Flow preview component
│   └── ui/            # General UI components
├── types/             # TypeScript type definitions
└── package.json       # Package dependencies and configuration
```

## Key Files and Directories
- `src-tauri`: Contains the Rust code for the desktop application, built with Tauri
- flow: Houses the flow execution engine and node catalog
- flows: Contains reusable UI components for the flow editor interface
- `.github/workflows/tests.yml`: CI configuration for automated testing
- `Cargo.toml` files: Rust package manifests with dependency configurations
- `package.json`: Node.js package configuration for JavaScript/TypeScript components

## Development Workflow
The project uses a monorepo structure to encourage code sharing between applications while maintaining separation of concerns:

1. The core logic in Rust (core) provides the fundamental functionality
2. UI components (ui) provide reusable interface elements
3. Applications (apps/*) integrate these packages to create complete user experiences

When making changes, consider where your code belongs in this structure to maintain proper separation of concerns.

## Building and Running
Each application has its own build process, but you can typically:

- Run the desktop app: bun run dev:desktop
- Launch the documentation site: bun run dev:docs
- Build the entire project: See detailed instructions in the project README

## Conclusion
Understanding the Flow-Like project structure is essential for effective contribution. The separation between core logic (Rust), UI components (React), and applications helps maintain a clean and maintainable codebase while enabling powerful cross-platform capabilities.