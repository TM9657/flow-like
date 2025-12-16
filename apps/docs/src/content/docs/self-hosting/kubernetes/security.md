---
title: Security notes
description: Isolation, credentials, and network policy notes.
sidebar:
  order: 90
---

This folder is intended to support a security-focused execution model for workflows.

## Isolation (Kata Containers)

- The executor is designed to run as a Kubernetes Job.
- In hardened clusters, set a `RuntimeClass` (for example `kata`) so each workflow runs in a lightweight VM boundary.

Relevant Helm knobs:

- `executor.runtimeClass`
- `runtimeClass.*` (if the chart creates the runtime class)

## Credentials

Goals:

- Avoid distributing long-lived S3 credentials into untrusted execution environments.
- Prefer scoped, time-limited credentials (presigned URLs or restricted IAM roles).

Current state of this folder:

- The Kubernetes API creates/uses S3 credentials to access buckets.
- The executor consumes bucket config and accesses the stores.

If you deploy in production:

- Prefer workload identity (IRSA / GKE Workload Identity / AKS federated identity) over static keys.
- If you must use static keys, store them in Kubernetes Secrets, restrict access via RBAC, and rotate regularly.

## Network policy

The chart includes a `NetworkPolicy` concept for limiting executor egress to storage endpoints.

Validate network policies in your cluster:

- Some CNIs don’t enforce egress policies by default.
- If your storage is external, allow only required endpoints.

## Supply chain

- Pin and scan container images.
- Keep the executor image minimal.
- Consider signing images (cosign) and enforcing via admission policy.
