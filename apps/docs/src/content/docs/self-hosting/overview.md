---
title: Self Hosting
description: Run Flow-Like on your own infrastructure.
sidebar:
  order: 60
---

Flow-Like can be deployed on your own infrastructure.

## Deployment Options

| Option | Best for | Isolation | Complexity |
|--------|----------|-----------|------------|
| [Docker Compose](/self-hosting/docker-compose/overview/) | Single machine, development | Container | Low |
| [Kubernetes](/self-hosting/kubernetes/overview/) | Production, multi-node | Pod / Kata | Medium |
| AWS Lambda | Serverless, multi-tenant | MicroVM | Low |

## Execution Backends

Flow-Like supports multiple execution backends with different isolation and performance characteristics:

| Backend | Isolation | Latency | Best For |
|---------|-----------|---------|----------|
| HTTP → Warm Pool | Process | Low | Trusted workloads |
| HTTP → Lambda URL | MicroVM | Medium | Multi-tenant SaaS |
| Kubernetes Job | Pod | High | Untrusted code |

→ [Learn more about execution backends](/self-hosting/execution-backends/)

## Connecting the Desktop App

After deploying your backend, configure the desktop app to connect to it by setting the `FLOW_LIKE_API_URL` environment variable:

```bash
export FLOW_LIKE_API_URL=https://your-api.example.com
./flow-like
```

→ [Desktop client configuration](/self-hosting/desktop-client/)

## Quick Links

- [Execution Backends](/self-hosting/execution-backends/) - Understanding job isolation and choosing the right backend
- [Docker Compose](/self-hosting/docker-compose/overview/) - Simple deployment for development and small teams
- [Kubernetes](/self-hosting/kubernetes/overview/) - Production-grade deployment with auto-scaling