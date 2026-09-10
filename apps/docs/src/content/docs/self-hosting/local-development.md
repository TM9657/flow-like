---
title: Native local backend
description: Run the API and runtime natively with local PostgreSQL and Redis
---

Use the native local backend when iterating on Rust API and runtime code. Docker
runs PostgreSQL, Redis and Adminer; the API and runtime run on your workstation.
This development setup uses shared processes, fixed database credentials and
published infrastructure ports. Use the
[Compose deployment](/self-hosting/docker-compose/installation/) for isolated
execution on a Linux host, or the
[k3d workflow](/self-hosting/kubernetes/local-development/) for the Kubernetes
chart. Both can run the public images published to `ghcr.io/rheosoph` without a
local build; the native backend below compiles from source.

## Prepare configuration and infrastructure

Install Docker Compose and the repository's Rust development toolchain. Prepare
private `.env` files in `apps/backend/local/api/` and
`apps/backend/local/runtime/`; both processes load their directory's environment.
The infrastructure file supplies SQL and Redis only. Configure object storage,
Hub/OIDC settings and API signing material for your development installation as
described in [Storage Providers](/dev/storage-providers/) and
[runtime API configuration](/self-hosting/containers/#runtime-api-configuration).
The local API also requires its CDN bucket configuration.

From the repository root:

```sh
cd apps/backend/local
docker compose up -d
docker compose ps --all
```

PostgreSQL listens on port 5432 with database/user `flowlike` and password
`flowlike_dev`. Redis listens on port 6379. The `db-init` service applies the
development schema before the API is started. It mounts `packages/api` into its
container, installs Prisma tooling there, and removes generated tooling files
afterward; use a development checkout with no needed local changes to those
generated files.

Adminer is available at `http://localhost:8082`. Select PostgreSQL, server
`postgres`, username/database `flowlike`, and password `flowlike_dev`.

## Start the API and runtime

Start each command from a separate terminal at the repository root:

```sh
cd apps/backend/local/api
cargo run
```

```sh
cd apps/backend/local/runtime
cargo run
```

The API listens on `http://localhost:8080`; the runtime defaults to
`http://localhost:9000`. The API passes its callback address with each run.
Hosted completion and remote embedding requests go through the authenticated API
proxy. Upstream provider credentials belong in the API environment.

| Setting | Process | Purpose |
| --- | --- | --- |
| `API_PORT` | API | Listener port, default 8080 |
| `DATABASE_URL` | API | Local PostgreSQL connection string |
| `REDIS_URL` | API and runtime | Local Redis connection string |
| `EXECUTOR_URL` | API | HTTP runtime URL |
| `EXECUTION_BACKEND=http` | API | Streaming HTTP dispatch |
| `ASYNC_EXECUTION_BACKEND` | API | HTTP dispatch or a Redis queue |
| `RUNTIME_PORT` | Runtime | Listener port, default 9000 |
| `QUEUE_WORKER_ENABLED=true` | Runtime | Consume Redis jobs when Redis asynchronous dispatch is selected |
| `REDIS_EXECUTION_QUEUE=exec:jobs:v3` | API and runtime | Explicit matching queue name when using Redis |
| `API_BASE_URL` | API and runtime | Local model proxy URL, default `http://localhost:8080` |

The runtime's `API_BASE_URL` is a fallback for executions without the API-supplied
callback address. `API_URL` remains a compatibility alias; use `API_BASE_URL` for
new configuration. See [Execution Backends](/self-hosting/execution-backends/)
for the dispatch model. Set the same queue protocol/name on both processes and
reconcile existing jobs before changing it.

## Stop the development services

Stop the native API and runtime processes, then run from
`apps/backend/local/`:

```sh
docker compose down
```

Named volumes retain PostgreSQL and Redis data. Use `docker compose down -v`
only when intentionally deleting that local data.
