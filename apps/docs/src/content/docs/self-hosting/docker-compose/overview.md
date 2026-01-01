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
│  Core Services:                                                             │
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
├─────────────────────────────────────────────────────────────────────────────┤
│  Monitoring (optional):                                                     │
│  ┌─────────────┐  ┌─────────────┐                                          │
│  │ Prometheus  │  │   Grafana   │                                          │
│  │   :9091     │  │    :3002    │                                          │
│  └─────────────┘  └─────────────┘                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────┐
                        │  External Storage   │
                        │  (S3/Azure/GCP/R2)  │
                        └─────────────────────┘
```

## Services

| Service | Description | Port |
|---------|-------------|------|
| `api` | Main Flow-Like API service | 8080 |
| `runtime` | Shared execution environment | 9000 |
| `postgres` | PostgreSQL database | 5432 |
| `db-init` | One-time migration job | — |
| `prometheus` | Metrics collection (optional) | 9091 |
| `grafana` | Dashboards (optional) | 3002 |

## Quick Start

```bash
cd apps/backend/docker-compose
cp .env.example .env
# Edit .env with your storage credentials

# Generate JWT keypair for execution trust
../../tools/gen-execution-keys.sh

# Start core services
docker compose up -d

# Or include monitoring
docker compose --profile monitoring up -d
```

## Monitoring

Enable optional Prometheus + Grafana monitoring:

```bash
docker compose --profile monitoring up -d
```

Access Grafana at http://localhost:3002 (default: admin/admin).

→ [Monitoring Guide](/self-hosting/docker-compose/monitoring/)

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
- [Monitoring](/self-hosting/docker-compose/monitoring/)
- [Scaling](/self-hosting/docker-compose/scaling/)
- [Troubleshooting](/self-hosting/docker-compose/troubleshooting/)
