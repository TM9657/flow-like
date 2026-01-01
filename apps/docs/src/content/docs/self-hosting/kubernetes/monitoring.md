---
title: Monitoring
description: Observability stack with Prometheus, Grafana, and Tempo for the Kubernetes deployment.
sidebar:
  order: 35
---

The Helm chart includes a complete observability stack: Prometheus for metrics, Grafana for visualization, and Tempo for distributed tracing.

## Enabling Monitoring

Monitoring is enabled by default. To disable it:

```bash
helm install flow-like ./helm -n flow-like \
  --set monitoring.enabled=false
```

## Components

### Prometheus

Collects metrics from all Flow-Like services and Kubernetes infrastructure.

**Scrape targets:**
- API Service (`/metrics` on port 9090)
- Executor Pool (`/metrics` on port 9090)
- CockroachDB (built-in metrics)
- Redis Exporter
- cAdvisor (container metrics from kubelet)

**Access Prometheus UI:**

```bash
kubectl port-forward -n flow-like svc/flow-like-prometheus 9090:9090
# Open http://localhost:9090
```

### Grafana

Pre-configured dashboards for all components.

**Access Grafana:**

```bash
kubectl port-forward -n flow-like svc/flow-like-grafana 3000:80
# Open http://localhost:3000
```

**Default credentials:**
- Username: `admin`
- Password: Retrieved from secret:

```bash
kubectl get secret -n flow-like flow-like-grafana \
  -o jsonpath='{.data.admin-password}' | base64 -d && echo
```

### Tempo

Receives OpenTelemetry traces from the API and Executor services.

**Configuration:**
- OTLP endpoint: `flow-like-tempo:4317` (gRPC)
- Retention: 72 hours (configurable)

## Pre-built Dashboards

### System Overview

Cluster-wide resource utilization:
- CPU usage per pod/container
- Memory usage and limits
- Network I/O
- Disk usage (if applicable)

### API Service

API-specific metrics:
- Request rate (requests/sec)
- Response latency percentiles (p50, p95, p99)
- Error rate by status code
- Active connections

### Executor Pool

Execution metrics:
- Job queue depth
- Jobs in progress
- Execution duration histogram
- Success/failure rates
- Worker pool utilization

### CockroachDB

Database performance:
- Query rate and latency
- Transaction throughput
- Replication lag
- Storage utilization
- Node health

### Redis

Cache and queue metrics:
- Commands per second
- Memory usage
- Connected clients
- Key eviction rate
- Queue lengths

### Tracing

Distributed traces via Tempo:
- Request traces across services
- Latency breakdown by service
- Error traces
- Service dependency map

## Custom Alerts

Add custom Prometheus alerting rules in `values.yaml`:

```yaml
monitoring:
  prometheus:
    alertRules:
      groups:
        - name: flow-like
          rules:
            - alert: HighErrorRate
              expr: |
                sum(rate(http_requests_total{status=~"5.."}[5m]))
                / sum(rate(http_requests_total[5m])) > 0.05
              for: 5m
              labels:
                severity: critical
              annotations:
                summary: High error rate detected
```

## Configuration Reference

```yaml
monitoring:
  enabled: true

  prometheus:
    image:
      repository: prom/prometheus
      tag: v2.48.0
    retention: 15d
    resources:
      requests:
        cpu: 100m
        memory: 256Mi
      limits:
        cpu: 500m
        memory: 512Mi

  grafana:
    image:
      repository: grafana/grafana
      tag: 10.2.2
    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 200m
        memory: 256Mi

  tempo:
    enabled: true
    image:
      repository: grafana/tempo
      tag: 2.3.1
    retention: 72h
    resources:
      requests:
        cpu: 100m
        memory: 256Mi
      limits:
        cpu: 500m
        memory: 512Mi
```

## Metrics Endpoint Security

The `/metrics` endpoints on API and Executor services are exposed on a separate internal port (9090) that is not exposed outside the cluster. Only Prometheus within the cluster can scrape these endpoints.

For production, ensure:
- Metrics port (9090) is not exposed via Ingress
- Network policies restrict access to monitoring namespace
- Grafana is behind authentication (SSO/OAuth recommended)

## External Monitoring Integration

### Datadog

Add Datadog annotations to enable auto-discovery:

```yaml
api:
  podAnnotations:
    ad.datadoghq.com/api.check_names: '["prometheus"]'
    ad.datadoghq.com/api.init_configs: '[{}]'
    ad.datadoghq.com/api.instances: '[{"prometheus_url": "http://%%host%%:9090/metrics"}]'
```

### New Relic

Export to New Relic via Prometheus remote write:

```yaml
monitoring:
  prometheus:
    remoteWrite:
      - url: https://metric-api.newrelic.com/prometheus/v1/write?prometheus_server=flow-like
        bearer_token: YOUR_LICENSE_KEY
```

## Troubleshooting

### No metrics appearing

1. Check if monitoring pods are running:
   ```bash
   kubectl get pods -n flow-like -l app.kubernetes.io/component=monitoring
   ```

2. Verify Prometheus targets:
   ```bash
   kubectl port-forward -n flow-like svc/flow-like-prometheus 9090:9090
   # Navigate to http://localhost:9090/targets
   ```

3. Check API metrics endpoint:
   ```bash
   kubectl exec -it deployment/flow-like-api -n flow-like -- \
     curl -s localhost:9090/metrics | head -20
   ```

### Grafana dashboard not loading

1. Check Grafana logs:
   ```bash
   kubectl logs -f deployment/flow-like-grafana -n flow-like
   ```

2. Verify datasources are configured:
   ```bash
   kubectl get configmap -n flow-like flow-like-grafana-datasources -o yaml
   ```

### Traces not appearing

1. Check Tempo is receiving data:
   ```bash
   kubectl logs -f deployment/flow-like-tempo -n flow-like
   ```

2. Verify OTLP endpoint is reachable from API:
   ```bash
   kubectl exec -it deployment/flow-like-api -n flow-like -- \
     nc -zv flow-like-tempo 4317
   ```
