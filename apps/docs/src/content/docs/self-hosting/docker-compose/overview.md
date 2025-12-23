---
title: Docker Compose
description: Deploy the Flow-Like backend using Docker Compose.
sidebar:
  order: 20
---

This deployment lives in `apps/backend/docker-compose/` and provides a simple way to run Flow-Like on a single machine.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Docker Compose Network                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐     ┌─────────────────────────────────────────────────┐   │
│  │   API       │────▶│           Execution Runtime                      │   │
│  │  Container  │     │   (Server Mode - handles multiple jobs)          │   │
│  │  :8080      │◀────│   :9000                                          │   │
│  └─────────────┘     └─────────────────────────────────────────────────┘   │
│        │                                                                    │
│        ▼                                                                    │
│  ┌─────────────┐                                                            │
│  │ PostgreSQL  │                                                            │
│  │    :5432    │                                                            │
│  └─────────────┘                                                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────┐
                        │  External Storage   │
                        │  (S3/Azure/GCP)     │
                        └─────────────────────┘
```

## Services

| Service | Description | Port |
|---------|-------------|------|
| `api` | Main Flow-Like API service | 8080 |
| `runtime` | Shared execution environment | 9000 |
| `postgres` | PostgreSQL database | 5432 |
| `db-init` | One-time migration job | — |

## Quickstart

```bash
cd apps/backend/docker-compose
cp .env.example .env
# Edit .env with your storage credentials

# Generate JWT keypair for execution trust
../../tools/gen-execution-keys.sh

docker compose up -d
```

## Execution Model

This Docker Compose setup uses **shared execution** where a single runtime container handles multiple jobs concurrently. This is suitable for:

- Development and testing
- Trusted workloads
- High-throughput scenarios with controlled input

For stronger isolation (one container per execution), consider:
- [Kubernetes deployment](/self-hosting/kubernetes/overview/) with Kata containers
- AWS Lambda (per-invocation isolation)

## Documentation

- [Prerequisites](/self-hosting/docker-compose/prerequisites/)
- [Installation](/self-hosting/docker-compose/installation/)
- [Configuration](/self-hosting/docker-compose/configuration/)
- [Storage Providers](/self-hosting/docker-compose/storage/)
- [Scaling](/self-hosting/docker-compose/scaling/)
- [Troubleshooting](/self-hosting/docker-compose/troubleshooting/)
