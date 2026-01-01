# Flow-Like Docker Compose Backend

Single-machine deployment using Docker Compose with shared execution runtime.

## Quick Start

```bash
cd apps/backend/docker-compose
cp .env.example .env
# Configure storage credentials in .env

docker compose up -d
```

**With monitoring:**
```bash
docker compose --profile monitoring up -d
```

## Documentation

Full step-by-step documentation: **[docs.flow-like.com/self-hosting/docker-compose](https://docs.flow-like.com/self-hosting/docker-compose/)**

| Guide | Description |
|-------|-------------|
| [Overview](https://docs.flow-like.com/self-hosting/docker-compose/overview/) | Architecture and components |
| [Prerequisites](https://docs.flow-like.com/self-hosting/docker-compose/prerequisites/) | System requirements |
| [Installation](https://docs.flow-like.com/self-hosting/docker-compose/installation/) | Step-by-step setup |
| [Configuration](https://docs.flow-like.com/self-hosting/docker-compose/configuration/) | Environment variables |
| [Storage Providers](https://docs.flow-like.com/self-hosting/docker-compose/storage/) | AWS, Azure, GCP, R2 |
| [Monitoring](https://docs.flow-like.com/self-hosting/docker-compose/monitoring/) | Prometheus & Grafana |
| [Scaling](https://docs.flow-like.com/self-hosting/docker-compose/scaling/) | Multi-instance setup |
| [Troubleshooting](https://docs.flow-like.com/self-hosting/docker-compose/troubleshooting/) | Common issues |

## Services

| Service | Port | Description |
|---------|------|-------------|
| api | 8080 | Flow-Like API |
| runtime | 9000 | Execution runtime |
| postgres | 5432 | Database |
| redis | 6379 | Job queue |
| grafana | 3002 | Dashboards (monitoring profile) |
| prometheus | 9091 | Metrics (monitoring profile) |

## Build Caching

The Dockerfiles use BuildKit cache mounts to persist Cargo registry and build artifacts across rebuilds. This significantly speeds up subsequent builds by avoiding recompilation of unchanged dependencies.

**First build:** ~15-20 minutes (full compilation)
**Subsequent builds:** ~1-3 minutes (incremental)

To clear the build cache:
```bash
docker builder prune --filter type=exec.cachemount
```

## Common Commands

```bash
# View logs
docker compose logs -f api

# Check health
curl http://localhost:8080/health/live

# Stop services
docker compose down

# Remove all data
docker compose down -v
```
