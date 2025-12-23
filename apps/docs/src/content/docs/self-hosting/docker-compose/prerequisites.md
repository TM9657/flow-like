---
title: Prerequisites
description: What you need before deploying Flow-Like with Docker Compose.
sidebar:
  order: 21
---

## What you are installing

The Flow-Like Docker Compose deployment consists of:

- **API service**: handles user requests, board management, and execution dispatch
- **Execution Runtime**: runs workflow executions in shared mode (multiple concurrent jobs)
- **PostgreSQL**: database for metadata storage
- **DB Init**: one-time job that runs database migrations

## Required tools

| Tool | Version | Purpose |
|------|---------|---------|
| Docker | 20.10+ | Container runtime |
| Docker Compose | 2.x | Service orchestration |
| OpenSSL | — | Generate JWT keypair |

## Required external services

### S3-compatible storage

Flow-Like requires external object storage. Supported providers:

- AWS S3
- Cloudflare R2
- Google Cloud Storage
- Azure Blob Storage
- MinIO (self-hosted)
- Any S3-compatible service

You will need:
- Endpoint URL
- Access credentials (key ID + secret)
- Three buckets: metadata, content, logs

## System requirements

### Minimum (development)

| Resource | Requirement |
|----------|-------------|
| CPU | 2 cores |
| RAM | 4 GB |
| Disk | 20 GB |

### Recommended (production)

| Resource | Requirement |
|----------|-------------|
| CPU | 4+ cores |
| RAM | 8+ GB |
| Disk | 50+ GB SSD |

The runtime container is configured with resource limits:
- CPU: 4 cores (limit), 1 core (reservation)
- Memory: 8 GB (limit), 2 GB (reservation)

## Network requirements

Outbound access from the host to:
- Your S3 endpoint (or cloud storage provider)
- Container registries (for pulling images)

Inbound access (if exposing externally):
- Port 8080 (API)
- Port 9090 (Metrics, optional)
