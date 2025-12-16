---
title: Scripts
description: Helper scripts for the Kubernetes backend.
sidebar:
  order: 80
---

Location:
- `apps/backend/kubernetes/scripts/`

## `setup-config.sh`

Creates/updates Kubernetes `Secret` and `ConfigMap` resources from your environment or `apps/backend/kubernetes/.env`.

```bash
cd apps/backend/kubernetes
./scripts/setup-config.sh
```

See [Configuration](/self-hosting/kubernetes/configuration/) for what is created.

## `migrate-db.sh`

Applies the Prisma schema to a PostgreSQL database.

- Host tooling mode:

```bash
cd apps/backend/kubernetes
./scripts/migrate-db.sh
```

- Docker mode (uses the `db-migrate` service in `apps/backend/kubernetes/docker-compose.yml`):

```bash
cd apps/backend/kubernetes
./scripts/migrate-db.sh --docker
```
