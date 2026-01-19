---
title: Scripts
description: Helper scripts for the Kubernetes backend.
sidebar:
  order: 80
---

Location:
- `apps/backend/kubernetes/scripts/`

## `k3d-setup.sh`

Sets up a complete local Kubernetes environment for development using k3d.

```bash
cd apps/backend/kubernetes

# Full setup (create cluster, build images, deploy everything)
./scripts/k3d-setup.sh

# Rebuild images and restart deployments
./scripts/k3d-setup.sh rebuild

# Show current status
./scripts/k3d-setup.sh status

# Delete the cluster
./scripts/k3d-setup.sh delete
```

What it does:
- Creates a k3d cluster with a local container registry (`localhost:5050`)
- Deploys MinIO for S3-compatible storage (no external S3 needed)
- Builds and pushes API and Executor images
- Generates a `values-local.yaml` with dev settings
- Installs the Helm chart

See [Local development](/self-hosting/kubernetes/local-development/) for details.

## `setup-config.sh`

Creates/updates Kubernetes `Secret` and `ConfigMap` resources from your environment or `apps/backend/kubernetes/.env`.

```bash
cd apps/backend/kubernetes
./scripts/setup-config.sh
```

See [Configuration](/self-hosting/kubernetes/configuration/) for what is created.

## `migrate-db.sh`

Applies the Prisma schema to the database.

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
