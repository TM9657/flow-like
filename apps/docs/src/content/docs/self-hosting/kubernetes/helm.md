---
title: Helm chart
description: Helm chart structure and install/upgrade commands.
sidebar:
  order: 50
---

Chart location:

- `apps/backend/kubernetes/helm/`

## What it deploys

- API Deployment/Service (the `k8s-api` binary)
- Executor settings for Kubernetes Jobs (the `k8s-executor` image)
- Optional internal dependencies (depending on values): Postgres, Redis

## Values

Main file:

- `apps/backend/kubernetes/helm/values.yaml`

Highlights:

- `api.*`: replicas, image, resources, autoscaling
- `executor.*`: image, runtimeClass, resources, retry/timeout/ttl
- `storage.external.*`: S3-compatible storage configuration (required)
- `database.*`: internal Postgres vs external connection string
- `redis.*`: Redis chart config

The chart always creates runtime configuration secrets:

- `{{release}}-db` (key: `DATABASE_URL`)
- `{{release}}-s3` (keys: `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, bucket names)

## Secrets strategy

Recommended patterns:

- Production: create secrets out-of-band (ExternalSecrets, SealedSecrets, Vault, etc.) and reference via chart values.
- Dev: use `apps/backend/kubernetes/scripts/setup-config.sh` to create secrets/configmaps from `.env`.

## DB migration hook

- `apps/backend/kubernetes/helm/templates/db-migration-job.yaml`

This job is disabled by default. If you enable it, ensure:

- `DATABASE_URL` is available to the job (the chart provides `{{release}}-db`)
- The job has access to Prisma schema + Prisma tooling (recommended: run migrations in CI before deploying the API)

## Install

```bash
cd apps/backend/kubernetes
helm install flow-like ./helm -n flow-like --create-namespace
```

## Upgrade

```bash
helm upgrade flow-like ./helm -n flow-like
```
