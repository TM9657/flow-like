---
title: Flow-Like Architecture
description: Deep Dive into Flow-Like's Architecture
sidebar:
  order: 11
---
Flow-Like is a modular, type-safe workflow automation platform built primarily in Rust with a TypeScript/React frontend. This document covers the high-level architecture and how the different components fit together.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Desktop App (Tauri)                         │
│                    apps/desktop (React + Vite)                       │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │     Tauri Commands      │
                    │   (src-tauri/src/*.rs)  │
                    └────────────┬────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                        Core Rust Packages                            │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐   │
│  │  flow-like   │  │ flow-like-   │  │   flow-like-catalog      │   │
│  │   (core)     │──│   storage    │──│   (node implementations) │   │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘   │
│         │                │                        │                  │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌─────────────┴───────────┐     │
│  │ flow-like-  │  │ flow-like-  │  │   Sub-catalogs:         │     │
│  │   types     │  │   bits      │  │   core, std, data, web, │     │
│  └─────────────┘  └─────────────┘  │   media, ml, onnx, llm, │     │
│                                    │   processing            │     │
│                                    └─────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────────┐
│                         Backend Services                             │
│                                                                      │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐   │
│  │   flow-like-api  │  │ flow-like-       │  │   Deployment     │   │
│  │   (REST API)     │  │ executor         │  │   Backends       │   │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘   │
│                                                                      │
│  Deployable via: Docker Compose, Kubernetes, AWS Lambda             │
└─────────────────────────────────────────────────────────────────────┘
```

## Package Structure

Flow-Like uses a monorepo structure with Cargo workspaces for Rust and Bun workspaces for TypeScript.

### Core Rust Packages (`packages/`)

| Package | Description |
|---------|-------------|
| `flow-like` (core) | Core library for workflow execution, board management, credentials, and state |
| `flow-like-types` | Shared type definitions, protobuf schemas, and utility types |
| `flow-like-storage` | Storage abstraction layer supporting S3, Azure Blob, GCS, and LanceDB for vectors |
| `flow-like-bits` | Reusable workflow components ("bits") |
| `flow-like-model-provider` | AI/ML model integrations (embeddings, LLMs, local inference) |
| `flow-like-api` | REST API with authentication, multi-tenancy, and execution backends |
| `flow-like-executor` | Environment-agnostic workflow execution runtime |
| `flow-like-catalog` | Node implementations for the visual workflow editor |
| `flow-like-catalog-macros` | Procedural macros for node registration |

### Node Catalog Sub-packages (`packages/catalog/`)

The catalog is split into domain-specific sub-crates:

| Sub-package | Description |
|-------------|-------------|
| `core` | Core execution types and traits |
| `std` | Standard nodes: control flow, math, variables, logging |
| `data` | Data manipulation, events, transformations |
| `web` | HTTP requests, webhooks, email |
| `media` | Image processing, bits for media files |
| `ml` | Machine learning: classification, clustering, regression |
| `onnx` | ONNX model inference |
| `llm` | LLM integrations: generative AI, embeddings, agents |
| `processing` | Document processing, text extraction |

### Applications (`apps/`)

| Application | Description |
|-------------|-------------|
| `desktop` | Tauri desktop app with React/Vite frontend |
| `backend/docker-compose` | Docker Compose deployment for self-hosting |
| `backend/kubernetes` | Kubernetes deployment with Helm charts |
| `backend/aws` | AWS Lambda deployment |
| `backend/local` | Local development API server |
| `docs` | Documentation website (Astro/Starlight) |
| `website` | Marketing website (Astro) |
| `embedded` | Embeddable widget (Next.js) |
| `web-app` | Web-based workflow editor (Next.js) |
| `schema-gen` | JSON schema generation utility |

## Data Flow

### Workflow Execution

```
1. User creates workflow in visual editor (Desktop/Web App)
                    │
                    ▼
2. Workflow saved as "Board" (JSON) → Storage (S3/Local)
                    │
                    ▼
3. Execution triggered via API or Desktop
                    │
                    ▼
4. Executor loads Board + Catalog nodes
                    │
                    ▼
5. ExecutionContext manages state per node
                    │
                    ▼
6. Results stored back to Storage
```

### Type System

Flow-Like workflows are **fully typed**. Every pin on every node has a defined type:

- **Execution pins**: Control flow triggers
- **Data pins**: Boolean, Integer, Float, String, Struct, Array, Generic
- **Struct pins**: Can enforce JSON Schema validation

This enables compile-time-like safety in a visual editor.

## Storage Architecture

Flow-Like uses a two-bucket model:

| Bucket | Purpose |
|--------|---------|
| Meta | Small, frequent reads (board definitions, user configs) — S3 Express One Zone recommended |
| Content | Large objects (bits, media, ML models) — Standard S3 |

Supported backends:
- AWS S3 (with STS temporary credentials)
- Azure Blob Storage (ADLS Gen2 with Directory SAS)
- Google Cloud Storage
- Cloudflare R2
- MinIO (S3-compatible)

## Execution Backends

Flow-Like supports multiple execution backends:

| Backend | Isolation | Use Case |
|---------|-----------|----------|
| Local (Desktop) | Process | Single-user, offline |
| HTTP Warm Pool | Container/Pod | Trusted workloads |
| Lambda | MicroVM | Multi-tenant SaaS |
| Kubernetes Job | Pod (Kata optional) | Untrusted code |

→ See [Execution Backends](/self-hosting/execution-backends/) for details.

## Authentication & Multi-Tenancy

The API package (`flow-like-api`) supports:

- **Cognito** (AWS)
- **JWT validation**
- **Scoped credentials** (per-user storage paths)
- **Stripe integration** for billing

## Frontend Architecture

The desktop app uses:

- **Tauri**: Rust backend with webview frontend
- **React**: UI components
- **Vite**: Build tooling
- **Tailwind CSS**: Styling
- **shadcn/ui**: Component library
- **Zustand/React Query**: State management

The visual workflow editor is built with custom canvas rendering and node connection logic.

## Feature Flags

Many packages support feature flags for conditional compilation:

```toml
# flow-like (core)
[features]
tauri = ["flow-like-storage/tauri"]
local-ml = ["flow-like-model-provider/local-ml"]
flow = ["flow-runtime"]
hub = []
bit = ["hub"]
model = ["bit"]
app = ["bit", "model", "hub"]

# flow-like-api
[features]
aws = ["aws-config", "aws-sdk-sts"]
azure = ["hmac", "sha2", "base64", "urlencoding"]
gcp = ["sha2", "base64", "rsa", "urlencoding"]
kubernetes = ["kube", "k8s-openapi"]
lambda = ["aws-config", "aws-sdk-lambda"]
cognito = ["aws-sdk-cognitoidentityprovider"]
```

## Next Steps

- [Building from Source](/dev/build/) — Get the code running locally
- [Writing Nodes](/dev/writing-nodes/) — Extend the node catalog
- [Self-Hosting](/self-hosting/overview/) — Deploy on your infrastructure