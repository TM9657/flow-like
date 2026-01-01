---
title: Prerequisites
description: What you need before installing Flow-Like on Kubernetes.
sidebar:
  order: 11
---

## What You're Installing

| Component | Description |
|-----------|-------------|
| **API Service** | Long-running Kubernetes Deployment |
| **Executor Pool** | Autoscaling workers for workflow execution |
| **CockroachDB** | Internal distributed database (optional, can use external) |
| **Redis** | Job queue and cache |

## Required External Service

- **S3-compatible storage** — AWS S3, Cloudflare R2, Google Cloud Storage, MinIO, etc.

## Required Tools

### For Local Development (k3d)

```bash
# macOS
brew install k3d kubectl helm docker

# Linux
curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
# Install kubectl and helm separately
```

### For Production

- `kubectl` — Kubernetes CLI
- `helm` — Package manager for Kubernetes

## Cluster Requirements

### Local Development

k3d creates everything you need automatically:

```bash
cd apps/backend/kubernetes
./scripts/k3d-setup.sh
```

### Production Cluster

- Any Kubernetes distribution (EKS, GKE, AKS, k3s, etc.)
- Default `StorageClass` (for persistent volumes)
- Network access to your S3 endpoint

## Optional: Kata Containers

For stronger isolation, the executor can use Kata containers via `RuntimeClass`. Your cluster must have Kata installed and configured.

## Credentials You Need

### S3 Storage (required)

- Endpoint URL
- Region
- Access key ID
- Secret access key
- Bucket names (meta + content)

### External Database (optional)

If not using internal CockroachDB:

- PostgreSQL/CockroachDB connection string
