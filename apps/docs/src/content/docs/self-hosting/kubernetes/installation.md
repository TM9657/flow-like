---
title: Installation
description: Install Flow-Like backend on Kubernetes using Helm.
sidebar:
  order: 12
---

This guide assumes you have `kubectl` and `helm` installed and a cluster you can deploy to.

## 1) Pick a namespace

```bash
kubectl create namespace flow-like
```

## 2) Configure S3 storage (required)

Flow-Like requires S3-compatible storage. Create a secret with your credentials:

```bash
kubectl -n flow-like create secret generic flow-like-external-s3 \
  --from-literal=S3_ACCESS_KEY_ID='YOUR_ACCESS_KEY' \
  --from-literal=S3_SECRET_ACCESS_KEY='YOUR_SECRET_KEY'
```

## 3) Decide: auto mode vs external database

The chart supports an "auto mode" for the database:

- If you don't provide an external database connection, it deploys internal Postgres and generates credentials.
- If you do provide external credentials, it skips the internal database.

You control this with values under:

- `database.type: auto|internal|external`

## 4) Install with auto mode database

```bash
cd apps/backend/kubernetes
helm install flow-like ./helm -n flow-like \
  --set storage.external.existingSecret=flow-like-external-s3 \
  --set storage.external.endpoint='https://s3.example.com' \
  --set storage.external.region='us-east-1' \
  --set storage.external.metaBucket='flow-like-meta' \
  --set storage.external.contentBucket='flow-like-content'
```

## 5) Install with an external database

Create a secret that contains `DATABASE_URL`:

```bash
kubectl -n flow-like create secret generic flow-like-external-db \
  --from-literal=DATABASE_URL='postgresql://USER:PASSWORD@HOST:5432/DBNAME'
```

Install the chart pointing to that secret:

```bash
cd apps/backend/kubernetes
helm install flow-like ./helm -n flow-like \
  --set database.type=external \
  --set database.external.existingSecret=flow-like-external-db \
  --set storage.external.existingSecret=flow-like-external-s3 \
  --set storage.external.endpoint='https://s3.example.com' \
  --set storage.external.region='us-east-1' \
  --set storage.external.metaBucket='flow-like-meta' \
  --set storage.external.contentBucket='flow-like-content'
```

## 6) Path-style S3 URLs (optional)

If your S3 provider needs path-style URLs (common for some self-hosted S3-compatible storage), set:

```bash
helm upgrade --install flow-like ./helm -n flow-like \
  --set storage.external.usePathStyle=true
```

## 7) Verify the install

```bash
kubectl -n flow-like get pods
kubectl -n flow-like get svc
```

If the API is running, you should see a deployment like `flow-like-flow-like-api` and a service like `flow-like-flow-like-api`.

## 8) Expose the API (optional)

If you want access from outside the cluster:

- enable `ingress.enabled=true`
- set `ingress.className` and `ingress.hosts`

The exact ingress setup depends on which ingress controller you use.
