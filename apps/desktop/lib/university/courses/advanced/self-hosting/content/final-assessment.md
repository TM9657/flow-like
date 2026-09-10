Thursday, 14:00. The go-live review. Priya sits across the table with the security checklist; the platform lead has the cluster dashboard open; you have the artifacts below. Six calls to make — everything you need is in the five lessons behind you.

## Artifact A — `staging-01`, the Compose box

A colleague "upgraded" the box by hand. `python3 scripts/up.py` now stops with `Startup refused: RUNTIME_IMAGE must equal the SANDBOX_IMAGE digest pin; run scripts/pull-images.py or scripts/prepare-images.py to align them`. The relevant `.env` lines:

```dotenv
FLOW_LIKE_IMAGE_TAG=1.4.0
RUNTIME_IMAGE=ghcr.io/rheosoph/flow-like-docker-compose-runtime:1.4.0
EXECUTION_MANAGER_IMAGE=ghcr.io/rheosoph/flow-like-docker-compose-execution-manager@sha256:9f2c…
SANDBOX_IMAGE=ghcr.io/rheosoph/flow-like-docker-compose-runtime@sha256:41ab…
SANDBOX_GATEWAY_IMAGE=ghcr.io/rheosoph/flow-like-docker-compose-execution-manager@sha256:9f2c…
```

## Artifact B — production values excerpt

`flow-like-values.yaml` for the cluster release, as currently committed:

```yaml
executionManager:
  image:
    repository: ghcr.io/rheosoph/flow-like-kubernetes-execution-manager
    tag: dev
    digest: ""
database:
  type: internal
execution:
  isolationMode: per_run
  backend: http
  asyncBackend: redis
networkPolicy:
  enabled: false
monitoring:
  enabled: true
```

## Artifact C — a teammate's proposal

> "Before go-live, switch `execution.backend` to `kubernetes_job` so every run gets its own pod. That gives us per-tenant isolation for free."

## Artifact D — a bug report from the pilot team

> "I configured `CRM_API_TOKEN` as a **Secret** runtime variable on my machine. Local runs work. Every remote run on the new cluster fails to authenticate against the CRM."

## Artifact E — the desktop rollout script

The IT team's laptop provisioning script contains this line — the templating step that should fill in the URL silently produced nothing:

```bash
export FLOW_LIKE_API_URL=""
```

## Artifact F — Priya's checklist

1. Show me the signing-key rotation plan.
2. Show me where last night's backups live.
3. Show me who gets paged when an alert fires.

Take them one at a time. The room is listening.
