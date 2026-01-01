---
title: Kubernetes
description: Deploy the Flow-Like backend on Kubernetes.
sidebar:
  order: 10
---

Production-ready Kubernetes deployment using Helm with built-in observability.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Kubernetes Cluster                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│  Core Services (deployed by Helm):                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐                  │
│  │ CockroachDB │  │    Redis    │  │    API Service      │                  │
│  │  (3 nodes)  │  │ (job queue) │  │   (autoscaling)     │                  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘                  │
│                                     ┌─────────────────────┐                  │
│                                     │   Executor Pool     │                  │
│                                     │   (autoscaling)     │                  │
│                                     └─────────────────────┘                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  Observability Stack (optional):                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐                  │
│  │ Prometheus  │  │   Grafana   │  │       Tempo         │                  │
│  │  (metrics)  │  │ (dashboards)│  │    (tracing)        │                  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  External (user provides):                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    S3-compatible Storage                             │    │
│  │           (AWS S3, Cloudflare R2, GCS, MinIO, etc.)                 │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Local Development (k3d)

```bash
cd apps/backend/kubernetes
./scripts/k3d-setup.sh
```

Creates a complete local environment with monitoring in ~5 minutes.

→ [Local Development Guide](/self-hosting/kubernetes/local-development/)

### Production

```bash
helm install flow-like ./helm -n flow-like --create-namespace \
  --set storage.external.endpoint='https://s3.example.com' \
  --set storage.external.accessKeyId='YOUR_KEY' \
  --set storage.external.secretAccessKey='YOUR_SECRET'
```

→ [Production Installation Guide](/self-hosting/kubernetes/installation/)

## What's Included

### Core Services

| Component | Description |
|-----------|-------------|
| **CockroachDB** | 3-node distributed SQL database |
| **Redis** | Job queue and execution state |
| **API Service** | Flow-Like API with autoscaling |
| **Executor Pool** | Reusable execution workers with autoscaling |
| **DB Migration Job** | Prisma migrations on install/upgrade |

### Observability Stack

| Component | Description |
|-----------|-------------|
| **Prometheus** | Metrics collection from all services |
| **Grafana** | Pre-configured dashboards |
| **Tempo** | Distributed tracing with OpenTelemetry |

Enable with `--set monitoring.enabled=true` (enabled by default).

## Grafana Dashboards

Six pre-built dashboards for full visibility:

- **System Overview** — Cluster-wide resource usage
- **API Service** — Request rates, latencies, errors
- **Executor Pool** — Job queue depth, execution metrics
- **CockroachDB** — Query performance, replication
- **Redis** — Commands/sec, memory usage
- **Tracing** — Request traces via Tempo

## Documentation

- [Prerequisites](/self-hosting/kubernetes/prerequisites/)
- [Installation](/self-hosting/kubernetes/installation/)
- [Configuration](/self-hosting/kubernetes/configuration/)
- [Database](/self-hosting/kubernetes/database/)
- [Local Development](/self-hosting/kubernetes/local-development/)
- [Helm Chart](/self-hosting/kubernetes/helm/)
- [API Service](/self-hosting/kubernetes/api/)
- [Executor](/self-hosting/kubernetes/executor/)
- [Storage](/self-hosting/kubernetes/storage/)
- [Scripts](/self-hosting/kubernetes/scripts/)
- [Security](/self-hosting/kubernetes/security/)
