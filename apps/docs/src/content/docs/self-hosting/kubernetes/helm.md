---
title: Helm chart
description: Helm chart structure and configuration.
sidebar:
  order: 50
---

Chart location:

- `apps/backend/kubernetes/helm/`

## What It Deploys

| Component | Description |
|-----------|-------------|
| **CockroachDB** | 3-node distributed SQL database (internal, default) |
| **Redis** | Job queue and execution state |
| **API Service** | Flow-Like API with autoscaling |
| **Executor Pool** | Reusable execution workers with autoscaling |
| **DB Migration Job** | Prisma migrations on install/upgrade |

## Key Values

```yaml
# API configuration
api:
  enabled: true
  replicaCount: 3
  autoscaling:
    enabled: true
    minReplicas: 3
    maxReplicas: 10

# Executor pool (reusable workers)
executorPool:
  enabled: true
  replicaCount: 2
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 10

# Database (internal CockroachDB by default)
database:
  type: internal  # or "external"
  internal:
    replicas: 3
    persistence:
      size: 10Gi

# S3-compatible storage (required)
storage:
  external:
    endpoint: ""
    region: "us-east-1"
    accessKeyId: ""
    secretAccessKey: ""
    metaBucket: "flow-like-meta"
    contentBucket: "flow-like-content"
```

## Install

```bash
cd apps/backend/kubernetes
helm install flow-like ./helm -n flow-like --create-namespace \
  --set storage.external.endpoint='https://s3.example.com' \
  --set storage.external.accessKeyId='YOUR_KEY' \
  --set storage.external.secretAccessKey='YOUR_SECRET'
```

## Upgrade

```bash
helm upgrade flow-like ./helm -n flow-like
```

## With External Database

```bash
helm install flow-like ./helm -n flow-like --create-namespace \
  --set database.type=external \
  --set database.external.connectionString='postgresql://user:pass@host:5432/flowlike' \
  --set storage.external.endpoint='https://s3.example.com' \
  --set storage.external.accessKeyId='YOUR_KEY' \
  --set storage.external.secretAccessKey='YOUR_SECRET'
```

## Using Existing Secrets

For production, use externally-managed secrets:

```bash
helm install flow-like ./helm -n flow-like --create-namespace \
  --set database.external.existingSecret='my-db-secret' \
  --set storage.external.existingSecret='my-s3-secret'
```
