# Flow-Like Docker Compose Backend

This folder contains the Docker Compose deployment for Flow-Like with a shared execution runtime.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Docker Compose Network                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐     ┌─────────────────────────────────────────────────┐   │
│  │   API       │─────│           Redis Job Queue                        │   │
│  │  Container  │     │   exec:jobs (LPUSH ←─── BRPOP)                  │   │
│  │  :8080      │     └─────────────────────────────────────────────────┘   │
│  └─────────────┘                          │                                │
│        │                                  ▼                                │
│        │             ┌─────────────────────────────────────────────────┐   │
│        │             │           Execution Runtime                      │   │
│        │◀───callback─│   Queue Worker + HTTP Server                     │   │
│        │             │   :9000                                          │   │
│        │             └─────────────────────────────────────────────────┘   │
│        │                                                                    │
│        ▼                                                                    │
│  ┌─────────────┐     ┌─────────────┐                                        │
│  │ PostgreSQL  │     │    Redis    │                                        │
│  │    :5432    │     │    :6379    │                                        │
│  └─────────────┘     └─────────────┘                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────┐
                        │  External Storage   │
                        │  (S3/Azure/GCP)     │
                        └─────────────────────┘
```

## Execution Flow

The Docker Compose backend uses two separate backends:

### 1. Streaming (`EXECUTION_BACKEND`) - `/invoke` endpoints
- Default: `http` - Direct HTTP SSE to runtime
- Returns Server-Sent Events stream with realtime progress
- Best for interactive/UI workflows needing realtime feedback

### 2. Async (`ASYNC_EXECUTION_BACKEND`) - `/invoke/async` endpoints
- Default: `redis` - Push to Redis queue, worker polls with BRPOP
- Options: `http`, `redis`, `sqs`, `kafka`
- Best for background/batch jobs

| Env Var | Endpoint | Default |
|---------|----------|--------|
| `EXECUTION_BACKEND` | `/invoke` | `http` |
| `ASYNC_EXECUTION_BACKEND` | `/invoke/async` | `redis` |

Configure in `.env`:
```bash
# Streaming: Direct HTTP with SSE (default)
EXECUTION_BACKEND=http

# Async: Redis queue (default)
ASYNC_EXECUTION_BACKEND=redis
```

## Services

1. **API** (`api`): The main Flow-Like API service
   - Handles user requests, board management, execution dispatch
   - Issues JWTs for execution runtime authentication
   - Dispatches streaming jobs via HTTP, async jobs via configured backend
   - Communicates with external storage (S3/Azure/GCP)

2. **Execution Runtime** (`runtime`): Shared execution environment
   - Queue worker polls Redis for async jobs
   - HTTP server for streaming execution
   - Authenticates with API using JWT tokens
   - Pushes execution updates back to API

3. **PostgreSQL** (`postgres`): Database for metadata
   - Stores app configurations, board definitions, profiles

4. **Redis** (`redis`): Execution state and job queue
   - Stores execution progress events with native TTL
   - Fast polling for real-time execution status
   - Job queue for async execution dispatch

5. **DB Init** (`db-init`): One-time initialization job
   - Runs Prisma migrations to set up schema

## Prerequisites

- Docker and Docker Compose v2.x
- External S3-compatible storage (AWS S3, Azure Blob, GCP Storage, MinIO)

## Quick Start

1. **Copy and configure environment:**
   ```bash
   cp .env.example .env
   # Edit .env with your storage credentials
   ```

2. **Generate JWT keypair** (required for execution trust):
   ```bash
   ./scripts/gen-execution-keys.sh
   # Copy the output to your .env file
   ```

3. **Start services:**
   ```bash
   docker-compose up -d
   ```

4. **Check status:**
   ```bash
   docker-compose ps
   docker-compose logs -f api
   ```

## Configuration

### Storage Options

The API supports three storage backends. Configure one:

**AWS S3:**
```env
STORAGE_PROVIDER=aws
AWS_ENDPOINT=https://s3.us-east-1.amazonaws.com
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=your-key
AWS_SECRET_ACCESS_KEY=your-secret
META_BUCKET=flow-like-meta
CONTENT_BUCKET=flow-like-content
LOG_BUCKET=flow-like-logs
```

**Azure Blob Storage:**
```env
STORAGE_PROVIDER=azure
AZURE_STORAGE_ACCOUNT_NAME=your-account
AZURE_STORAGE_ACCOUNT_KEY=your-key
AZURE_STORAGE_ACCOUNT_KEY=your-key
AZURE_META_CONTAINER=flow-like-meta
AZURE_CONTENT_CONTAINER=flow-like-content
AZURE_LOG_CONTAINER=flow-like-logs
```

**Google Cloud Storage:**
```env
STORAGE_PROVIDER=gcp
GCS_PROJECT_ID=your-project
GOOGLE_APPLICATION_CREDENTIALS_JSON={"type":"service_account",...}
GCP_META_BUCKET=flow-like-meta
GCP_CONTENT_BUCKET=flow-like-content
GCP_LOG_BUCKET=flow-like-logs
```

### JWT Execution Keys

The execution JWT system enables stateless trust between services:
- API signs JWTs for execution runtimes
- Runtime authenticates callbacks using these JWTs
- Works across Kubernetes, Docker, Lambda, and other environments

Generate keys:
```bash
./scripts/gen-execution-keys.sh
```

## Isolated vs Shared Execution

This Docker Compose setup uses **shared execution** where a single runtime
container handles multiple jobs concurrently. For isolated execution per-job,
consider using:

- **Kubernetes** deployment (see `../kubernetes/`) - Kata containers
- **AWS Lambda** (see `../aws/`) - Per-invocation isolation
- **Docker-in-Docker** - Can be added with security considerations

The shared runtime is suitable for:
- Development and testing
- Trusted workloads
- High-throughput scenarios with controlled input

## Scaling

To scale the runtime horizontally:
```bash
docker-compose up -d --scale runtime=3
```

The API will round-robin requests across runtime instances.

## Troubleshooting

**API can't connect to runtime:**
```bash
docker-compose logs runtime
# Check EXECUTOR_URL is set correctly for HTTP mode
```

**Queue worker not processing jobs:**
```bash
# Check queue worker is enabled
docker-compose exec runtime env | grep QUEUE_WORKER
# Check Redis queue length
docker-compose exec redis redis-cli LLEN exec:jobs
# Check runtime logs for BRPOP activity
docker-compose logs -f runtime
```

**Database connection issues:**
```bash
docker-compose exec postgres pg_isready -U flowlike
```

**Storage errors:**
```bash
# Test S3 connectivity from API container
docker-compose exec api curl -I $S3_ENDPOINT
```

## Related Documentation

- Main docs: `apps/docs/src/content/docs/self-hosting/docker-compose/`
- Kubernetes: `../kubernetes/README.md`
- AWS Lambda: `../aws/README.md`
