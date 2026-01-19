---
title: Monitoring
description: Enable monitoring for Docker Compose deployment with Prometheus and Grafana.
sidebar:
  order: 25
---

The Docker Compose deployment includes optional monitoring with Prometheus, Grafana, and pre-built dashboards.

## Enabling Monitoring

Start the monitoring stack alongside your services:

```bash
docker compose --profile monitoring up -d
```

Or run just monitoring services:

```bash
docker compose --profile monitoring up -d prometheus grafana
```

## Accessing Services

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | http://localhost:3002 | admin / admin |
| Prometheus | http://localhost:9091 | — |

:::tip
Change the default Grafana password on first login for production use.
:::

## Pre-built Dashboards

Grafana comes pre-configured with dashboards for all services:

### System Overview

Overall system health:
- Container CPU and memory usage
- Network I/O across services
- Container restarts and uptime

### API Service

API performance metrics:
- Request rate (requests/second)
- Response latency (p50, p95, p99)
- Error rate by status code
- Active connections

### Execution Runtime

Runtime metrics:
- Jobs in progress
- Execution duration histogram
- Queue depth
- Success/failure rates

### PostgreSQL

Database performance:
- Active connections
- Query rate
- Transaction throughput
- Cache hit ratio

## Prometheus Targets

Prometheus scrapes metrics from:

| Target | Endpoint | Metrics |
|--------|----------|---------|
| API | `api:9090/metrics` | Request counts, latencies, errors |
| Runtime | `runtime:9090/metrics` | Execution metrics |
| PostgreSQL Exporter | `postgres-exporter:9187/metrics` | Database metrics |
| cAdvisor | `cadvisor:8080/metrics` | Container metrics |

## Configuration

### Custom Prometheus Config

Edit `monitoring/prometheus/prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'api'
    static_configs:
      - targets: ['api:9090']

  - job_name: 'runtime'
    static_configs:
      - targets: ['runtime:9090']

  # Add custom targets here
  - job_name: 'custom-service'
    static_configs:
      - targets: ['my-service:9090']
```

### Custom Grafana Dashboards

Add dashboard JSON files to `monitoring/grafana/provisioning/dashboards/`:

```bash
# Download a dashboard from Grafana.com
curl -o monitoring/grafana/provisioning/dashboards/my-dashboard.json \
  'https://grafana.com/api/dashboards/1860/revisions/latest/download'

# Restart Grafana to pick up changes
docker compose restart grafana
```

### Alert Rules

Add Prometheus alerting rules in `monitoring/prometheus/alerts/`:

```yaml
# monitoring/prometheus/alerts/api.yml
groups:
  - name: api
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(http_requests_total{status=~"5.."}[5m]))
          / sum(rate(http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate (> 5%)"
```

## Environment Variables

Configure monitoring via `.env`:

```env
# Prometheus
PROMETHEUS_RETENTION=15d
PROMETHEUS_PORT=9091

# Grafana
GRAFANA_PORT=3001
GRAFANA_ADMIN_PASSWORD=your-secure-password

# Metrics export
METRICS_ENABLED=true
METRICS_PORT=9090
```

## Metrics Endpoints

The API and Runtime services expose Prometheus metrics on a separate port (9090):

```bash
# Check API metrics
curl http://localhost:9090/metrics

# Check Runtime metrics (requires port mapping)
docker compose exec runtime curl localhost:9090/metrics
```

## Resource Usage

The monitoring stack adds minimal overhead:

| Service | CPU | Memory |
|---------|-----|--------|
| Prometheus | ~100m | ~256MB |
| Grafana | ~50m | ~128MB |
| cAdvisor | ~50m | ~128MB |

## Production Considerations

### Persistence

Enable persistent storage for Prometheus data:

```yaml
# docker-compose.override.yml
services:
  prometheus:
    volumes:
      - prometheus-data:/prometheus

volumes:
  prometheus-data:
```

### External Access

For production, place Grafana behind a reverse proxy with TLS:

```nginx
server {
    listen 443 ssl;
    server_name grafana.example.com;

    location / {
        proxy_pass http://localhost:3001;
        proxy_set_header Host $host;
    }
}
```

### Alertmanager

Add Alertmanager for alert routing:

```yaml
# docker-compose.override.yml
services:
  alertmanager:
    image: prom/alertmanager:v0.26.0
    ports:
      - "9093:9093"
    volumes:
      - ./monitoring/alertmanager:/etc/alertmanager
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
```

## Troubleshooting

### No metrics in Grafana

1. Check Prometheus is scraping targets:
   ```bash
   curl http://localhost:9091/api/v1/targets
   ```

2. Verify services expose metrics:
   ```bash
   docker compose exec api curl -s localhost:9090/metrics | head -20
   ```

3. Check Grafana datasource:
   - Go to Settings → Data Sources
   - Verify Prometheus URL is `http://prometheus:9090`

### High memory usage

Reduce Prometheus retention:

```yaml
# docker-compose.override.yml
services:
  prometheus:
    command:
      - '--storage.tsdb.retention.time=7d'
```
