---
title: API service
description: The Kubernetes API service (k8s-api).
sidebar:
  order: 60
---

Source:
- `apps/backend/kubernetes/api/`

## Purpose

This service is the Kubernetes-hosted API entrypoint for Flow-Like. It:

- Bootstraps `flow_like_api::state::State` (DB + platform services)
- Exposes Flow-Like API routes via `flow_like_api::construct_router`
- Adds Kubernetes-specific endpoints (health, jobs)

## Entrypoint

- `apps/backend/kubernetes/api/src/main.rs`

Key initialization:
- Logging via `tracing_subscriber` + `RUST_LOG`
- Config loaded via `apps/backend/kubernetes/api/src/config.rs`
- S3 store created using `flow_like_storage::object_store::aws::AmazonS3Builder`
- Database connection is created inside `flow_like_api::state::State` and requires `DATABASE_URL`

## Configuration

See [Configuration](/self-hosting/kubernetes/configuration/).

At minimum for the API runtime:

- `DATABASE_URL`
- `S3_ENDPOINT`, `S3_REGION`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`
- Bucket names (see note in [Configuration](/self-hosting/kubernetes/configuration/))
- `REDIS_URL` (used by the Kubernetes job endpoints)

## HTTP

The service binds to:

- `0.0.0.0:${PORT}` (defaults to `8080`)

Routes in this implementation:

- `/health/*` (see `apps/backend/kubernetes/api/src/health.rs`)
- `/jobs/*` (see `apps/backend/kubernetes/api/src/jobs/*`)
- `/api/v1/*` (Flow-Like API router)

## Notes

- The Kubernetes `/jobs` endpoints are meant to dispatch workflow execution as Kubernetes Jobs (executor).
- Authentication/authorization is primarily implemented in `packages/api` and depends on your deployment’s auth setup.
