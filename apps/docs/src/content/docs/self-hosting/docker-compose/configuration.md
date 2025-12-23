---
title: Configuration
description: Environment variables and configuration options for Docker Compose deployment.
sidebar:
  order: 23
---

Configuration is managed via the `.env` file in `apps/backend/docker-compose/`.

## Database

| Variable | Default | Description |
|----------|---------|-------------|
| `POSTGRES_USER` | `flowlike` | Database username |
| `POSTGRES_PASSWORD` | `flowlike_dev` | Database password |
| `POSTGRES_DB` | `flowlike` | Database name |
| `POSTGRES_PORT` | `5432` | Host port mapping |

:::caution
Change `POSTGRES_PASSWORD` from the default for production deployments.
:::

## API Service

| Variable | Default | Description |
|----------|---------|-------------|
| `API_PORT` | `8080` | API HTTP port |
| `METRICS_PORT` | `9090` | Prometheus metrics port |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

## Execution Runtime

| Variable | Default | Description |
|----------|---------|-------------|
| `RUNTIME_PORT` | `9000` | Runtime HTTP port |
| `MAX_CONCURRENT_EXECUTIONS` | `10` | Max parallel executions |
| `EXECUTION_TIMEOUT_SECONDS` | `3600` | Timeout per execution (1 hour) |

## JWT Execution Keys

These keys enable stateless trust between API and execution runtimes:

| Variable | Required | Description |
|----------|----------|-------------|
| `EXECUTION_KEY` | Yes | Base64-encoded ES256 private key |
| `EXECUTION_PUB` | Yes | Base64-encoded ES256 public key |
| `EXECUTION_KID` | No | Key identifier (default: `execution-es256-v1`) |

Generate these using:

```bash
./tools/gen-execution-keys.sh
```

## Storage Provider

| Variable | Default | Description |
|----------|---------|-------------|
| `STORAGE_PROVIDER` | `aws` | Provider: `aws`, `azure`, `gcp` |
| `META_BUCKET` | `flow-like-meta` | Metadata bucket name |
| `CONTENT_BUCKET` | `flow-like-content` | Content bucket name |
| `LOGS_BUCKET` | `flow-like-logs` | Logs bucket name |

See [Storage Providers](/self-hosting/docker-compose/storage/) for provider-specific configuration.

## LLM Provider Configuration

The API can proxy LLM requests to various providers:

| Variable | Description |
|----------|-------------|
| `OPENROUTER_API_KEY` | OpenRouter API key (default provider) |
| `HOSTED_OPENAI_API_KEY` | OpenAI API key |
| `HOSTED_ANTHROPIC_API_KEY` | Anthropic API key |
| `HOSTED_AZURE_API_KEY` | Azure OpenAI API key |
| `HOSTED_AZURE_ENDPOINT` | Azure OpenAI endpoint URL |

## Monitoring

| Variable | Default | Description |
|----------|---------|-------------|
| `METRICS_ENABLED` | `true` | Enable Prometheus metrics |
| `METRICS_PORT` | `9090` | Metrics endpoint port |
| `SENTRY_DSN` | — | Sentry error tracking DSN |

## Example minimal configuration

```env
# Database
POSTGRES_PASSWORD=your-secure-password

# Storage (AWS S3 example)
STORAGE_PROVIDER=aws
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=AKIA...
S3_SECRET_ACCESS_KEY=...
META_BUCKET=my-flow-like-meta
CONTENT_BUCKET=my-flow-like-content
LOGS_BUCKET=my-flow-like-logs

# JWT Keys (generated with gen-execution-keys.sh)
EXECUTION_KEY=...
EXECUTION_PUB=...
```
