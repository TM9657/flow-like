---
title: Database
description: Schema and database setup for the Kubernetes backend.
sidebar:
  order: 30
---

The Flow-Like API uses a relational database for platform data. In this repo, the schema is expressed as Prisma schema files under:

- `packages/api/prisma/schema/`

## CockroachDB vs PostgreSQL

The Prisma schemas declare:

- `provider = "cockroachdb"`

For PostgreSQL environments (local dev, many Kubernetes clusters), the repo includes a helper script:

- `packages/api/scripts/make-postgres-prisma-mirror.sh`

It copies the schema tree to `packages/api/prisma-postgres-mirror/schema` and rewrites `provider` to `postgresql`.

## Applying the schema

### Local development (recommended)

The Kubernetes backend ships a `docker-compose.yml` that starts Postgres/Redis and applies the schema.

```bash
cd apps/backend/kubernetes
cp .env.example .env
# edit .env

docker compose up -d
```

The `db-migrate` service uses the tooling in `packages/api` to run:

- mirror schema to PostgreSQL
- `prisma db push` against `DATABASE_URL`

### Manual migration (host tools)

```bash
cd apps/backend/kubernetes
./scripts/migrate-db.sh
```

### Manual migration (Docker)

```bash
cd apps/backend/kubernetes
./scripts/migrate-db.sh --docker
```

### Kubernetes / Helm

The Helm chart contains a migration Job template:

- `apps/backend/kubernetes/helm/templates/db-migration-job.yaml`

Important:
- Today it is a *hook job* intended to run pre-install / pre-upgrade.
- You still need to ensure the job image has access to the Prisma schema and tooling. If you don’t ship a purpose-built migration image, you can run migrations out-of-band (CI/CD step or an admin job).

## Runtime configuration

The API runtime expects:

- `DATABASE_URL` (required)

This is read by `flow_like_api::state::State`.
