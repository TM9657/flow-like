---
title: API Reference
description: Complete API endpoint reference for Flow-Like on Kubernetes.
sidebar:
  order: 61
---

This page documents all API endpoints available when running Flow-Like on Kubernetes.

## Quick Start

```bash
# 1. Forward the API port to your local machine
kubectl port-forward svc/flow-like-api 8083:8080 -n flow-like &

# 2. Test the health endpoint
curl http://localhost:8083/api/v1/health
# Response: {"status":"ok"}

# 3. Test the Kubernetes health probe
curl http://localhost:8083/health/live
# Response: {"status":"healthy","version":"0.1.0"}
```

## Understanding the Port

**Why is the port 8080 inside the container but I use 8083 locally?**

```
Your Machine                 Kubernetes Cluster
┌────────────┐              ┌──────────────────────────────┐
│            │  port-forward │  ┌──────────────────────┐   │
│ localhost  │──────────────▶│  │   flow-like-api      │   │
│   :8083    │              │  │   Port 8080          │   │
│            │              │  └──────────────────────┘   │
└────────────┘              └──────────────────────────────┘
```

- **8080** is the port the API listens on *inside* the container
- **8083** (or any port you choose) is the port on *your machine*
- `kubectl port-forward` creates a tunnel between them

You can use any local port:
```bash
kubectl port-forward svc/flow-like-api 3000:8080 -n flow-like
# Now access at http://localhost:3000
```

---

## Health Endpoints

These endpoints are used by Kubernetes to check if the service is running correctly.

### Kubernetes Health Probes

| Endpoint | Purpose | Used By |
|----------|---------|---------|
| `GET /health/live` | Liveness probe | Kubernetes restarts the pod if this fails |
| `GET /health/ready` | Readiness probe | Kubernetes stops sending traffic if this fails |
| `GET /health/startup` | Startup probe | Kubernetes waits for this before other probes |

**Example Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

### Application Health

| Endpoint | Purpose |
|----------|---------|
| `GET /api/v1/health` | Basic health check |
| `GET /api/v1/health/db` | Database connectivity check |

**Example:**
```bash
# Basic health
curl http://localhost:8083/api/v1/health
# {"status":"ok"}

# Database health (returns round-trip time)
curl http://localhost:8083/api/v1/health/db
# {"rtt":5}
```

---

## Job Management Endpoints

These Kubernetes-specific endpoints manage workflow execution jobs.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/jobs/submit` | Submit a new workflow job |
| `GET` | `/jobs/{job_id}` | Get job status |
| `POST` | `/jobs/{job_id}/cancel` | Cancel a running job |

### Submit a Job

```bash
curl -X POST http://localhost:8083/jobs/submit \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "your-app-id",
    "board_id": "your-board-id",
    "version": "latest",
    "payload": {"key": "value"},
    "mode": "async"
  }'
```

**Response:**
```json
{
  "job_id": "job-abc123",
  "status": "queued"
}
```

### Check Job Status

```bash
curl http://localhost:8083/jobs/job-abc123
```

**Response:**
```json
{
  "job_id": "job-abc123",
  "status": "running",
  "started_at": "2025-01-01T12:00:00Z",
  "completed_at": null,
  "error": null
}
```

### Cancel a Job

```bash
curl -X POST http://localhost:8083/jobs/job-abc123/cancel
# Returns: 204 No Content
```

---

## Flow-Like API Endpoints

All standard Flow-Like API endpoints are available under `/api/v1/`.

### Public Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/` | Hub information (platform config) |
| `GET /api/v1/version` | API version |
| `GET /api/v1/catalog` | Available node catalog |

### Information Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/info/legal` | Legal notice |
| `GET /api/v1/info/privacy` | Privacy policy |
| `GET /api/v1/info/terms` | Terms of service |
| `GET /api/v1/info/contact` | Contact information |
| `GET /api/v1/info/features` | Enabled features |
| `GET /api/v1/info/profiles` | Profile templates |

### Authentication Endpoints

| Endpoint | Description |
|----------|-------------|
| `POST /api/v1/auth/login` | User login |
| `POST /api/v1/auth/register` | User registration |
| `POST /api/v1/auth/refresh` | Refresh access token |
| `GET /api/v1/oauth/{provider}` | OAuth login redirect |

### User Endpoints (Authenticated)

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/user` | Get current user |
| `PUT /api/v1/user` | Update current user |
| `GET /api/v1/profile/{id}` | Get user profile |

### Application Endpoints (Authenticated)

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/apps` | List user's applications |
| `POST /api/v1/apps` | Create new application |
| `GET /api/v1/apps/{id}` | Get application details |
| `PUT /api/v1/apps/{id}` | Update application |
| `DELETE /api/v1/apps/{id}` | Delete application |

### Execution Endpoints (Authenticated)

| Endpoint | Description |
|----------|-------------|
| `POST /api/v1/execution/run` | Execute a workflow |
| `GET /api/v1/execution/{id}` | Get execution status |

### Store Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/store` | Browse marketplace |
| `GET /api/v1/store/{id}` | Get store item details |

---

## Testing the API

### Quick Health Check

```bash
# From your terminal
kubectl port-forward svc/flow-like-api 8083:8080 -n flow-like &
curl http://localhost:8083/api/v1/health
```

### Using HTTPie (alternative to curl)

```bash
# Install: brew install httpie

# Health check
http GET localhost:8083/api/v1/health

# Submit job
http POST localhost:8083/jobs/submit \
  app_id=myapp \
  board_id=myboard
```

### From Inside the Cluster

```bash
# Create a debug pod
kubectl run curl --image=curlimages/curl -it --rm -- sh

# Inside the pod
curl http://flow-like-api:8080/api/v1/health
```

---

## Response Codes

| Code | Meaning |
|------|---------|
| `200` | Success |
| `201` | Created |
| `204` | No Content (success with no body) |
| `400` | Bad Request (invalid input) |
| `401` | Unauthorized (missing/invalid token) |
| `403` | Forbidden (insufficient permissions) |
| `404` | Not Found |
| `500` | Internal Server Error |

---

## Next Steps

- [Configuration](/self-hosting/kubernetes/configuration/) — Environment variables and secrets
- [Security](/self-hosting/kubernetes/security/) — Authentication setup
- [Executor](/self-hosting/kubernetes/executor/) — How jobs are executed
