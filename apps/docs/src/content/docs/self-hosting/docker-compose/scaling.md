---
title: Scaling
description: Scale the Flow-Like Docker Compose deployment for higher throughput.
sidebar:
  order: 25
---

## Horizontal scaling

Scale the runtime container to handle more concurrent executions:

```bash
docker compose up -d --scale runtime=3
```

The API round-robins requests across runtime instances.

### Considerations

- Each runtime instance respects `MAX_CONCURRENT_EXECUTIONS`
- With 3 instances × 10 concurrent = 30 total concurrent executions
- All instances share the same storage backend
- No shared state between runtime instances (stateless)

## Vertical scaling

Adjust resource limits in `docker-compose.yml`:

```yaml
runtime:
  deploy:
    resources:
      limits:
        cpus: '8'
        memory: 16G
      reservations:
        cpus: '2'
        memory: 4G
```

Also increase concurrent executions:

```env
MAX_CONCURRENT_EXECUTIONS=20
```

## Database scaling

For higher database throughput:

### Option 1: Tune PostgreSQL

Add these settings to your PostgreSQL service:

```yaml
postgres:
  command:
    - "postgres"
    - "-c"
    - "max_connections=200"
    - "-c"
    - "shared_buffers=256MB"
```

### Option 2: External database

Use a managed PostgreSQL service (AWS RDS, Azure Database, Cloud SQL):

```env
# Remove the postgres service from docker-compose.yml
# Update DATABASE_URL to point to external database
DATABASE_URL=postgresql://user:pass@external-host:5432/flowlike
```

## Load balancing

For multiple API instances, use an external load balancer:

```bash
docker compose up -d --scale api=2
```

Then configure nginx or similar:

```nginx
upstream flowlike-api {
    server localhost:8080;
}

server {
    listen 80;
    location / {
        proxy_pass http://flowlike-api;
    }
}
```

:::note
When scaling API instances, ensure they share the same `EXECUTION_KEY` and `EXECUTION_PUB` for JWT verification.
:::

## Production recommendations

| Workload | API replicas | Runtime replicas | Runtime resources |
|----------|--------------|------------------|-------------------|
| Development | 1 | 1 | 2 CPU, 4GB |
| Small team | 1 | 2 | 4 CPU, 8GB |
| Medium | 2 | 4 | 4 CPU, 8GB |
| Large | 3+ | 6+ | 8 CPU, 16GB |

For large production workloads, consider [Kubernetes deployment](/self-hosting/kubernetes/overview/) for better orchestration, auto-scaling, and isolation.
