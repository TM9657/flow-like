---
title: Local Development
description: Run the Kubernetes backend locally using k3d.
sidebar:
  order: 40
---

k3d creates a lightweight Kubernetes cluster inside Docker, giving you a production-like environment locally with full observability (Prometheus, Grafana, Tempo).

## Prerequisites

Install the required tools:

```bash
# macOS
brew install k3d kubectl helm docker

# Linux (k3d)
curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
# Also install kubectl and helm from their official sources
```

Make sure Docker is running with sufficient resources (8GB RAM recommended).

## Quick Start

```bash
cd apps/backend/kubernetes
./scripts/k3d-setup.sh
```

This creates a complete local Kubernetes environment in about 5 minutes.

## What Gets Deployed

| Component | Description |
|-----------|-------------|
| **k3d cluster** | 1 server + 2 agents |
| **Local registry** | `localhost:5111` (external) / `flow-like-registry:5000` (internal) |
| **CockroachDB** | 3-node distributed database |
| **Redis** | Job queue and execution state |
| **API** | Flow-Like API service (port 8080) |
| **Executor Pool** | Workflow execution workers |
| **Prometheus** | Metrics collection |
| **Grafana** | Dashboards and visualization |
| **Tempo** | Distributed tracing |

:::note[Storage]
Storage is external by default. The k3d setup uses placeholder credentials—configure your own S3-compatible storage (AWS S3, Cloudflare R2, MinIO, etc.) in `values.yaml` or via `--set` flags.
:::

## Accessing Services

### Port Forwarding

After deployment, access services via port-forwarding:

```bash
# API (main endpoint) - exposed via nodePort, no port-forward needed
# Access at http://localhost:8080

# Grafana (monitoring dashboards) - exposed via nodePort at 30002
# Access at http://localhost:30002

# Prometheus (raw metrics)
kubectl port-forward -n flow-like svc/flow-like-prometheus 9090:9090 &
```

### Service URLs

| Service | Access Method | URL |
|---------|--------------|-----|
| API | NodePort (automatic) | http://localhost:8080 |
| Grafana | NodePort (automatic) | http://localhost:30002 |
| Prometheus | `kubectl port-forward svc/flow-like-prometheus 9090:9090` | http://localhost:9090 |
| CockroachDB | `kubectl port-forward svc/flow-like-cockroachdb-public 26257:26257` | localhost:26257 |

### Grafana Access

Default credentials:
- **Username**: `admin`
- **Password**: Retrieved from secret:

```bash
kubectl get secret -n flow-like flow-like-grafana \
  -o jsonpath='{.data.admin-password}' | base64 -d && echo
```

## Monitoring Dashboards

Grafana comes pre-configured with these dashboards:

| Dashboard | Description |
|-----------|-------------|
| **System Overview** | CPU, memory, network across all pods |
| **API Service** | Request rates, latencies, error rates |
| **Executor Pool** | Job queue depth, execution times, worker status |
| **CockroachDB** | Query performance, replication lag, storage |
| **Redis** | Commands/sec, memory, connected clients |
| **Tracing** | Request traces via Tempo integration |

## Common Operations

### View Logs

```bash
# API logs
kubectl logs -f deployment/flow-like-api -n flow-like

# Executor logs
kubectl logs -f deployment/flow-like-executor-pool -n flow-like

# All pods
kubectl logs -f -l app.kubernetes.io/instance=flow-like -n flow-like
```

### Rebuild After Code Changes

```bash
./scripts/k3d-setup.sh rebuild
```

This rebuilds Docker images, pushes to the local registry, and triggers a rolling restart.

### Cluster Management

```bash
# Show status
./scripts/k3d-setup.sh status

# Delete cluster
./scripts/k3d-setup.sh delete

# Shell into API pod
kubectl exec -it deployment/flow-like-api -n flow-like -- /bin/sh
```

### Helm Operations

```bash
# Check current values
helm get values flow-like -n flow-like

# Upgrade with new values
helm upgrade flow-like ./helm -n flow-like --set api.replicas=2

# View release history
helm history flow-like -n flow-like
```

## Troubleshooting

### Pods Not Starting

```bash
# Check pod status
kubectl get pods -n flow-like

# Describe failing pod
kubectl describe pod <pod-name> -n flow-like

# Check events
kubectl get events -n flow-like --sort-by='.lastTimestamp'
```

### Database Connection Issues

```bash
# Check CockroachDB logs
kubectl logs -f statefulset/flow-like-cockroachdb -n flow-like

# Verify database is ready
kubectl exec -it flow-like-cockroachdb-0 -n flow-like -- cockroach sql --insecure \
  -e "SHOW DATABASES;"
```

### Image Pull Errors

```bash
# Verify local registry
curl http://localhost:5111/v2/_catalog

# Rebuild and push images
./scripts/k3d-setup.sh rebuild
```

### Network Policy Issues

If the API can't reach external services (like authentication providers), check the network policy:

```bash
# View network policies
kubectl get networkpolicy -n flow-like

# Test external connectivity from API pod
kubectl exec -it deployment/flow-like-api -n flow-like -- \
  wget -qO- --timeout=5 https://httpbin.org/ip || echo "Failed"
```

The network policy allows egress to external HTTPS (port 443) by default. If you need additional ports, update the `networkPolicy` section in your Helm values.

### Executor JWT Verification

If executions fail with authentication errors in the executor:

```bash
# Check executor logs
kubectl logs -f deployment/flow-like-executor-pool -n flow-like

# Verify BACKEND_PUB secret is set
kubectl get secret flow-like-api-keys -n flow-like -o jsonpath='{.data.BACKEND_PUB}' | base64 -d
```

The executor needs `BACKEND_PUB` and `BACKEND_KID` environment variables from the API keys secret to verify execution JWTs.

## Next Steps

- [Configuration Reference](/self-hosting/kubernetes/configuration/) — All Helm values
- [Production Deployment](/self-hosting/kubernetes/installation/) — Deploy to a real cluster
- [Storage Setup](/self-hosting/kubernetes/storage/) — Configure S3-compatible storage
