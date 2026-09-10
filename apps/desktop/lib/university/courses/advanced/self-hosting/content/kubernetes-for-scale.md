Staging convinced everyone. Now the platform team wants the real thing: multi-host scheduling, autoscaling, network policies — a production cluster inside the VPC. You point Helm at the repository chart with your generated values, run `helm upgrade --install`, and Helm refuses before it creates a single object: `executionManager.image.digest must pin the manager and gateway image`.

> **Predict first:** the chart already defaults to public images your cluster can pull. Why does it still refuse to render, and what is the one-command fix?

## 1 · What the chart gives you

The Helm chart at `apps/backend/kubernetes/helm/` deploys, by default: the web app, the API, an execution manager that runs each execution in its own gVisor sandbox pod, a queue bridge that feeds it from Redis, an internal CockroachDB, chart-managed Redis with authentication, bundled RustFS object storage with a bootstrap Job, a database-migration Job, and a Prometheus–Grafana–Tempo monitoring stack. Ingress, the WASM compiler, and sink services exist in the values but start disabled. Autoscaling is configurable and off by default.

@KubernetesArchitecture

Read the diagram's badges — they are the course in miniature: clients pass through an optional Ingress to the web pods and the API Service (HPA configurable); the state row shows CockroachDB marked "single-node default", Redis, a dashed object-storage card, and an optional WASM compiler; the execution row shows the executor path, a Job dispatcher marked "incomplete — creates Job; runner pending", optional sink services, and a network card that says "check selectors". The observability strip is enabled by chart defaults.

## 2 · Defaults are published images — sandboxes want digests

Here's the hook's answer. Every first-party image defaults to a published repository — `ghcr.io/rheosoph/flow-like-kubernetes-<workload>` for the API, web, executor, execution manager, migration, and sink trigger, and the shared `ghcr.io/rheosoph/flow-like-docker-compose-<workload>` images for the queue bridge, compiler, signaling, and RustFS bootstrap — at the mutable `dev` tag with `pullPolicy: Always`. The packages are public and every tag is a multi-architecture index, so AMD64 and ARM64 nodes can share one cluster and nothing needs a pull Secret.

The default `execution.isolationMode: per_run` is where the refusal comes from. The manager starts a fresh sandbox pod per execution from `executionManager.sandbox.image` with `imagePullPolicy: IfNotPresent` — so the chart requires `executionManager.image.digest` and a `repository@sha256:…` sandbox reference that names exactly one build, and fails the render otherwise. A moving `dev` tag is not an acceptable identity for code that runs untrusted flows. Resolve one release to digests, without Docker on the workstation:

```bash
./scripts/resolve-images.py --tag 1.4.0
```

The resolver queries the registry API and writes `.generated/values-images.yaml`: a `digest` for every first-party image, `executionManager.image.digest`, the full `executionManager.sandbox.image` reference, and `pullPolicy: IfNotPresent`. `--tag` also accepts `dev`, `beta`, `latest`, or an immutable `sha-<commit>-run-<id>-<attempt>` build tag. Forks and private mirrors add `--registry`, `--pull-secret NAME` (which lands in `global.imagePullSecrets`), and a `GHCR_TOKEN` in the environment for the registry read; `--arch amd64|arm64` pins node selectors when a mixed cluster must avoid one pool.

Local development is the other way round: `./scripts/dev.sh setup` creates a k3d cluster and `scripts/build-images.sh` builds the same repository names at a local tag straight into it — single-architecture, and only for `trusted_shared` evaluation.

A production install therefore always overrides three things: digest-pinned images (plus pull Secrets for a private registry), the generated Secrets that `scripts/setup-config.sh` writes, and the storage decision. Keep credentials out of values files with the `existingSecret` pattern:

```bash
kubectl -n flow-like create secret generic flow-like-backend-jwt \
  --from-env-file=flow-like-backend-jwt.env \
  --dry-run=client -o yaml | kubectl apply -f -
```

The JWT Secret carries the same `BACKEND_KEY`, `BACKEND_PUB`, and `BACKEND_KID` trio you generated on staging. Storage follows the same rules as Compose: the bundled RustFS store speaks S3 through the API's AWS feature, and external Azure, GCP, or S3 buckets are a values change plus a Secret.

## 3 · The database decision

`database.type: internal` runs one CockroachDB pod with `start-single-node --insecure`. That's an evaluation database, full stop. And don't try to fix it by raising `database.internal.replicas` — independent single-node pods never form a cluster; they just corrupt your assumptions.

Production means `database.type: external` plus a Secret containing `DATABASE_URL` (PostgreSQL-compatible, `sslmode=require`). One more policy call: the migration Job runs on every install and upgrade and currently executes `prisma db push --accept-data-loss`. Read that flag again, then decide whether it runs automatically or whether you disable `database.migration` and run an approved migration process instead.

## 4 · Render, install, verify

Render before you install — it's how you catch a wrong Secret name while it's still cheap. `deploy.sh` does lint, template, and upgrade in one go and appends `.generated/values-images.yaml` after your own files, so resolved digests always win:

```bash
./scripts/deploy.sh \
  -f helm/values-production.yaml \
  -f values-operator.yaml
```

Then verify the rollout and open the app:

```bash
kubectl rollout status deployment/flow-like-api -n flow-like
kubectl rollout status deployment/flow-like-execution-manager -n flow-like
kubectl port-forward -n flow-like service/flow-like-web 3001:3001
```

**Watch out:** the two isolation modes want different dispatch lanes, and the chart enforces it at render time. `per_run` requires `execution.backend: http` with `asyncBackend: redis` — the queue bridge is the Redis consumer. `trusted_shared` requires `http` on both lanes, because the shared executor pool has no Redis consumer. The full dispatch story is next lesson.

**Recap**

- Chart defaults are the public `ghcr.io/rheosoph` images at `dev`; per-run isolation additionally demands digests, and `resolve-images.py` supplies them without a build machine.
- Internal CockroachDB is single-node and insecure — use an external `DATABASE_URL` for production and review the migration Job's `--accept-data-loss`.
- `deploy.sh` lints and renders before `helm upgrade --install`, with the resolved image file applied last; verify with rollout status.
