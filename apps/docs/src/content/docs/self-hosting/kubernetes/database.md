---
title: Database
description: Database configuration for the Kubernetes backend.
sidebar:
  order: 30
---

The Flow-Like API uses a relational database for platform data. The Prisma schema is in:

- `packages/api/prisma/schema/`

## CockroachDB (Default)

The Helm chart deploys a 3-node CockroachDB cluster by default. This provides:

- **Native schema compatibility** — Flow-Like's Prisma schema is CockroachDB-first
- **High availability** — Automatic failover with 3 nodes
- **Distributed SQL** — Horizontal scaling built-in
- **PostgreSQL compatible** — Standard drivers work out of the box

### Internal CockroachDB (default)

```yaml
database:
  type: internal
  internal:
    replicas: 3
    persistence:
      size: 10Gi
```

For local development, a single-node cluster is sufficient:

```yaml
database:
  type: internal
  internal:
    replicas: 1
    persistence:
      size: 1Gi
```

### External Database

Use an external PostgreSQL or CockroachDB instance:

```yaml
database:
  type: external
  external:
    connectionString: "postgresql://user:pass@host:5432/flowlike"
    # Or use an existing secret:
    existingSecret: "my-db-secret"
```

## Schema Migrations

### Helm Chart Migration Job

The chart includes a migration job that runs on install/upgrade:

```yaml
database:
  migration:
    enabled: true
```

### Manual Migration

```bash
cd apps/backend/kubernetes
./scripts/migrate-db.sh
```

Or via Docker:

```bash
./scripts/migrate-db.sh --docker
```

## Runtime Configuration

The API requires:

- `DATABASE_URL` — CockroachDB/PostgreSQL connection string

This is read by `flow_like_api::state::State`.
