---
title: Executor
description: The Kubernetes Job executor (k8s-executor).
sidebar:
  order: 70
---

Source:
- `apps/backend/kubernetes/executor/`

## Purpose

The executor runs workflows in an isolated environment (typically as a Kubernetes Job, optionally using a Kata `RuntimeClass`).

High-level steps:

- Load the target board definition from the metadata store (S3)
- Construct a `FlowLikeState` with the content store
- Execute the board using `flow_like::flow::execution::InternalRun`

## Entrypoint

- `apps/backend/kubernetes/executor/src/main.rs`

## Execution logic

- `apps/backend/kubernetes/executor/src/execution.rs`

Notable behaviors:

- `InternalRun::execute` takes `Arc<FlowLikeState>` and returns `Option<LogMeta>`.
- The executor currently does not treat `None` vs `Some(_)` as a hard failure; adjust this if you want job failure semantics.

## Job input contract

The executor expects a JSON payload describing what to run and which stores to use.

The precise env var names come from `apps/backend/kubernetes/executor/src/main.rs` and the `JobInput` struct there. If you change the job spec, keep this doc updated.

Typical fields:

- `app_id`
- `board_id`
- `event_id` (optional)
- `payload` (JSON)
- storage credentials/config:
  - `endpoint`
  - `region`
  - `meta_bucket`
  - `content_bucket`

## Local debugging

You can run the executor locally by setting the same environment variables your Job spec sets, then running:

```bash
cd apps/backend/kubernetes/executor
cargo run
```
