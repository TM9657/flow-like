---
title: Troubleshooting
description: Common issues and solutions for Docker Compose deployment.
sidebar:
  order: 26
---

## Service won't start

### Check service status

```bash
docker compose ps
docker compose logs <service-name>
```

### Database not ready

If `db-init` or `api` fail with database connection errors:

```bash
# Check if PostgreSQL is healthy
docker compose exec postgres pg_isready -U flowlike

# View PostgreSQL logs
docker compose logs postgres

# Restart the init job
docker compose up db-init
```

### Port already in use

```bash
# Find what's using the port
lsof -i :8080

# Or change the port in .env
API_PORT=8081
```

## API issues

### API can't connect to runtime

```bash
# Check runtime health
curl http://localhost:9000/health

# Verify EXECUTOR_POOL_URL in docker-compose.yml
docker compose exec api env | grep EXECUTOR
```

### Health check failing

```bash
# Direct health check
docker compose exec api curl -v http://localhost:8080/health

# Check for startup errors
docker compose logs api | grep -i error
```

## Database issues

### Migration failed

```bash
# View migration logs
docker compose logs db-init

# Re-run migrations
docker compose up db-init --force-recreate
```

### Reset database

```bash
# Remove volume and recreate
docker compose down
docker volume rm docker-compose_postgres_data
docker compose up -d
```

### Connect directly to database

```bash
docker compose exec postgres psql -U flowlike -d flowlike
```

## Storage issues

### S3 connection errors

```bash
# Test connectivity from API container
docker compose exec api curl -I "$S3_ENDPOINT"

# Check credentials are set
docker compose exec api env | grep S3
```

### Permission denied

Verify your bucket policy allows the credentials to read/write:

```bash
# AWS CLI test (if installed)
aws s3 ls s3://your-bucket --endpoint-url $S3_ENDPOINT
```

### Path-style URL issues

Some providers require path-style URLs:

```env
S3_USE_PATH_STYLE=true
```

## Runtime issues

### Execution timeout

Increase timeout in `.env`:

```env
EXECUTION_TIMEOUT_SECONDS=7200  # 2 hours
```

### Out of memory

Increase runtime memory limits:

```yaml
# In docker-compose.yml
runtime:
  deploy:
    resources:
      limits:
        memory: 16G
```

### Too many concurrent executions

Reduce the limit or scale horizontally:

```env
MAX_CONCURRENT_EXECUTIONS=5
```

```bash
docker compose up -d --scale runtime=2
```

## JWT / Authentication issues

### Invalid token errors

Regenerate the keypair and update both services:

```bash
./tools/gen-execution-keys.sh
# Update .env with new EXECUTION_KEY, EXECUTION_PUB
docker compose down
docker compose up -d
```

### Missing EXECUTION_KEY

```bash
# Check if keys are set
docker compose exec api env | grep EXECUTION
```

## Logs and debugging

### Enable debug logging

```env
RUST_LOG=debug,docker_compose_api=trace
```

### Follow all logs

```bash
docker compose logs -f
```

### Export logs to file

```bash
docker compose logs > flowlike-logs.txt
```

## Full reset

If all else fails, start fresh:

```bash
# Stop and remove everything
docker compose down -v

# Remove cached images
docker compose build --no-cache

# Start fresh
docker compose up -d
```

:::caution
The `-v` flag removes all volumes including the database. Back up your data first if needed.
:::
