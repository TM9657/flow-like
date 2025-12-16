---
title: Kubernetes
description: Deploy the Flow-Like backend on Kubernetes.
sidebar:
  order: 10
---

This deployment lives in `apps/backend/kubernetes/` and ships:

- `k8s-api`: Kubernetes-hosted API service
- `k8s-executor`: Kubernetes Job that runs workflow executions (optionally with Kata via `RuntimeClass`)
- Helm chart to deploy the above plus optional internal dependencies (Postgres, Redis)

## Quickstart

```bash
cd apps/backend/kubernetes
helm install flow-like ./helm -n flow-like --create-namespace \
  --set storage.external.endpoint='https://your-s3-endpoint' \
  --set storage.external.accessKeyId='YOUR_ACCESS_KEY' \
  --set storage.external.secretAccessKey='YOUR_SECRET_KEY'
```

The chart can run in "auto mode" for the database:

- If you don't provide an external database connection, it deploys internal Postgres and generates credentials.
- If you do provide external credentials, it skips the internal database.

**Note:** S3-compatible storage is always required (AWS S3, Cloudflare R2, Google Cloud Storage, etc.).

## Docs

- [Prerequisites](/self-hosting/kubernetes/prerequisites/)
- [Installation](/self-hosting/kubernetes/installation/)
- [Configuration](/self-hosting/kubernetes/configuration/)
- [Database](/self-hosting/kubernetes/database/)
- [Local development](/self-hosting/kubernetes/local-development/)
- [Helm chart](/self-hosting/kubernetes/helm/)
- [API service](/self-hosting/kubernetes/api/)
- [Executor](/self-hosting/kubernetes/executor/)
- [Scripts](/self-hosting/kubernetes/scripts/)
- [Security notes](/self-hosting/kubernetes/security/)
