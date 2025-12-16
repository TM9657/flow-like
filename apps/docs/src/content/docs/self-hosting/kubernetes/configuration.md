---
title: Configuration
description: Runtime configuration for the Kubernetes backend deployment.
sidebar:
  order: 20
---

This deployment is configured via environment variables (local dev) and Kubernetes `Secret`/`ConfigMap` (cluster).

## Source of truth

- Local: `apps/backend/kubernetes/.env` (copy from `.env.example`)
- Kubernetes: created via `apps/backend/kubernetes/scripts/setup-config.sh`
- API process: additionally requires `DATABASE_URL` because `flow_like_api::state::State` reads it directly.

## Required environment variables

### Database

- `DATABASE_URL` (required by the API runtime)
  - Example (local): `postgresql://flowlike:flowlike_dev@localhost:5432/flowlike`
  - Example (cluster): `postgresql://flowlike:flowlike_dev@<postgres-service>:5432/flowlike`

### Storage (S3-compatible, required)

- `S3_ENDPOINT`
- `S3_REGION`
- `S3_ACCESS_KEY_ID`
- `S3_SECRET_ACCESS_KEY`
- `META_BUCKET` / `META_BUCKET_NAME` (see note below)
- `CONTENT_BUCKET` / `CONTENT_BUCKET_NAME` (see note below)

Note: `k8s-api` currently reads `META_BUCKET_NAME` and `CONTENT_BUCKET_NAME` in `apps/backend/kubernetes/api/src/config.rs`. The local `.env.example` uses `META_BUCKET`/`CONTENT_BUCKET`. If you use the scripts, set both (or adjust `api/src/config.rs` to accept both names).

### Redis

- `REDIS_URL` (defaults to `redis://redis:6379` inside the cluster)

### Kubernetes job dispatching

- `KUBERNETES_NAMESPACE` (defaults to `flow-like`)
- `EXECUTOR_IMAGE` (image used for created Jobs)
- `EXECUTOR_RUNTIME_CLASS` (defaults to `kata`)
- `JOB_TIMEOUT_SECONDS` (defaults to `3600`)
- `JOB_MAX_RETRIES` (defaults to `3`)

### Logging

- `RUST_LOG` (recommended)

## Kubernetes secrets/configmaps

The helper script `apps/backend/kubernetes/scripts/setup-config.sh` creates:

- Secret `flow-like-db`
  - `DATABASE_URL`
- Secret `flow-like-s3`
  - `S3_ENDPOINT`, `S3_REGION`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`
  - `META_BUCKET_NAME`, `CONTENT_BUCKET_NAME` (and also `META_BUCKET`/`CONTENT_BUCKET` for compatibility)
- Secret `flow-like-redis-secret`
  - `REDIS_URL`
- ConfigMap `flow-like-api-config`
  - `API_HOST`, `API_PORT`, `RUST_LOG`, `METRICS_ENABLED`
- ConfigMap `flow-like-executor-config`
  - `K8S_NAMESPACE`, `EXECUTOR_IMAGE`, `USE_KATA_CONTAINERS`, `JOB_TIMEOUT`, `MAX_CONCURRENT_JOBS`, `RUST_LOG`

Optional (only if provided):
- Secret `flow-like-openai-secret`
- Secret `flow-like-sentry-secret`

## Local development quickstart

```bash
cd apps/backend/kubernetes
cp .env.example .env
# edit .env

docker compose up -d
```

If you want to mirror secrets/configmaps into a real cluster:

```bash
cd apps/backend/kubernetes
./scripts/setup-config.sh
```
