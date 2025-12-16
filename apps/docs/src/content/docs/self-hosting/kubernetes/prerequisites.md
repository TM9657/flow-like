---
title: Prerequisites
description: What you need before installing Flow-Like on Kubernetes.
sidebar:
  order: 11
---

This section is written for readers who are new to Kubernetes.

## What you are installing

The Flow-Like Kubernetes backend consists of:

- **API service**: a long-running Kubernetes `Deployment`.
- **Executor**: short-lived Kubernetes `Job`s created by the API to run workflow executions.
- **Optional dependencies** ("auto mode"): internal PostgreSQL if you don't provide external credentials.
- **Required external service**: S3-compatible storage (AWS S3, Cloudflare R2, Google Cloud Storage, etc.)

## Kubernetes basics (quick glossary)

- **Cluster**: the Kubernetes environment you deploy into.
- **Namespace**: a logical partition inside a cluster. We recommend a dedicated namespace like `flow-like`.
- **Pod**: the unit that runs containers.
- **Deployment**: keeps a set of pods running (used for the API).
- **Job**: runs pods until completion (used for workflow executions).
- **Service**: stable network endpoint inside the cluster.
- **Secret**: stores sensitive values like database credentials.

## Required tools (on your machine)

- `kubectl` (to talk to your cluster)
- `helm` (to install the chart)

## Required cluster capabilities

- A working Kubernetes cluster (managed Kubernetes, k3s, MicroK8s, etc.)
- A default `StorageClass` (recommended)
  - Needed if you want persistent volumes for internal Postgres/Redis.
- Outbound network access from the API/executor pods to:
  - your database (if external)
  - your S3 endpoint (if external)

## Optional (but common) add-ons

- **Ingress controller** (NGINX, Traefik, etc.)
  - Needed if you want to expose the API outside the cluster.
- **DNS + TLS** (cert-manager)
  - Needed if you want HTTPS with a real domain.

## Optional security/runtime features

### Kata containers

The executor jobs can use a `RuntimeClass` (for example Kata) for stronger isolation.

- If your cluster does not support Kata, you can still run the executor jobs without it.
- If you do use Kata, your cluster must already have the runtime installed and configured.

## Credentials you may need

You can run everything “internally” (auto mode), or you can bring your own external services.

### External PostgreSQL

If you use an external database, you’ll typically provide:

- a PostgreSQL connection string (`DATABASE_URL`) via a Kubernetes secret

### S3-compatible storage (required)

Flow-Like requires external S3-compatible storage. You'll need to provide:

- endpoint
- region
- access key id / secret access key
- bucket names (metadata + content)
