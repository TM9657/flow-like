---
title: Local development
description: Run the Kubernetes backend components locally.
sidebar:
  order: 40
---

This folder provides a local dev stack (Postgres + Redis) for the Kubernetes backend components.

**Note:** S3-compatible storage must be provided externally (AWS S3, Cloudflare R2, Google Cloud Storage, etc.).

## Start dependencies

```bash
cd apps/backend/kubernetes
cp .env.example .env
# edit .env - configure S3 credentials

docker compose up -d
```

Services:
- `postgres`: PostgreSQL 16
- `db-migrate`: applies Prisma schema to Postgres
- `redis`: queue/cache

## Run the Kubernetes API locally

```bash
cd apps/backend/kubernetes/api
# make sure DATABASE_URL, REDIS_URL, and S3_* are set (from your .env)
cargo run
```

## Run the executor locally (for debugging)

The executor is normally run as a Kubernetes Job. You can run it locally by providing the expected job input env vars (see [Executor](/self-hosting/kubernetes/executor/)).

```bash
cd apps/backend/kubernetes/executor
cargo run
```

## Tear down

```bash
docker compose down -v
```
