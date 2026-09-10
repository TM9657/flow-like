Friday afternoon on `staging-01`, the Linux VM the infrastructure team lent you. You clone the repo, `cd apps/backend/docker-compose`, run `python3 scripts/setup-env.py`, and then `python3 scripts/up.py`. No Rust compiles, no image downloads — the script refuses within a second: `Startup refused: SANDBOX_IMAGE must be an immutable image ID or repository@sha256 digest`. You never touched that variable; the template ships it empty on purpose.

> **Predict first:** every other `*_IMAGE` line is empty too and nobody complains about those. Why does the stack insist on this one, and which script did you skip?

## 1 · Know your services

The Compose file runs the complete stack, not just an API:

| Service | Published port | Job |
| --- | --- | --- |
| `api-gateway` | `8080`, `3001`, `4444` | Nginx edge proxy in front of the API replicas, web app, and signaling |
| `web` / `api` | internal | Flow-Like web application; API replicas for auth, app state, dispatch |
| `execution-manager` | internal | Prepares and destroys one gVisor sandbox per execution |
| `queue-bridge` | internal | Redis queue consumers that hand background runs to the managers |
| `compiler` | internal | WASM compilation for custom nodes, behind `compiler-gateway` |
| `signaling` | internal | Realtime collaboration |
| `sink-services` | — | Cron and configured bot/event adapters |
| `postgres` / `redis` | internal | Metadata / run state and queues |
| `object-store` | `9000` via `object-gateway` | Bundled RustFS with prefix-scoped STS credentials |
| `db-init` / `object-store-init` | — | One-time database initialization and bucket bootstrap jobs |

@DockerComposeArchitecture

The map above shows how they connect: browser and desktop clients enter through the web app on 3001 and the Nginx API gateway on 8080; the API replicas sit behind the gateway; below them PostgreSQL, Redis, and object storage; along the bottom, the execution workers, the WASM compiler, sink services, and signaling, with an optional monitoring strip — Prometheus, Tempo, exporters, Grafana — enabled by the `monitoring` profile.

One card on that diagram is older than the stack: object storage is drawn as an external provider, but the current template bundles RustFS and its bootstrap job creates the metadata, content, and log buckets. External S3, Azure, or GCP storage is a choice, not a prerequisite.

## 2 · The four decisions the template forces

**Images — the hook's answer.** Every `*_IMAGE` line is empty, which selects the published image `ghcr.io/rheosoph/flow-like-docker-compose-<workload>` at the tag in `FLOW_LIKE_IMAGE_TAG` (default `dev`). Those packages are public, so nothing needs `docker login`. But the default `EXECUTION_ISOLATION_MODE=per_run` makes the manager start a fresh runner and gateway container for every execution from `SANDBOX_IMAGE` and `SANDBOX_GATEWAY_IMAGE` — and the manager never pulls. A moving tag like `dev` could change what runs inside a sandbox between two executions, so preflight only accepts an immutable digest or image ID there. The script you skipped pins everything:

```bash
python3 scripts/pull-images.py            # or --tag 1.4.0 for a release
```

It pulls the nine first-party images, writes `<WORKLOAD>_IMAGE=repository@sha256:…` for each, copies the runtime and execution-manager pins into the two sandbox variables, and leaves every secret line untouched. The digest names a multi-architecture index, so the same `.env` works on AMD64 and ARM64 daemons. Running code that isn't published yet? `python3 scripts/prepare-images.py` builds the runner and manager images locally and pins their image IDs; `python3 scripts/up.py --build` builds the rest — and refuses to run while any `*_IMAGE` is digest-pinned, because a digest cannot be built.

**Storage.** The template selects the bundled RustFS store through `STORAGE_PROVIDER=aws` and `RUNTIME_CREDENTIALS_PROVIDER=aws`, and the published API image includes that AWS runtime-credential feature. Empty AWS bucket overrides fall back to the generic bucket names. Switch to an external provider only when your data has to live somewhere specific — the [Storage](https://docs.flow-like.com/self-hosting/docker-compose/storage/) guide covers Azure, GCP, R2, and real S3.

**Trust keys.** Flow-Like signs backend JWTs with one ES256 keypair. `setup-env.py` generated it together with the service credentials, so `BACKEND_KEY`, `BACKEND_PUB`, and `BACKEND_KID` are already in `.env`. The private key stays with the API; runners and the compiler receive only the public key. For rotation later, `../../../tools/gen-execution-keys.sh --export` mints a new trio.

**Identity.** The hub configuration uses OpenID Connect placeholders. Copy `flow-like.config.example.json` to `flow-like.config.json`, wire in your provider's authority, client ID, and callback URLs, point `FLOW_LIKE_RUNTIME_CONFIG_FILE` at the copy — and never expose a deployment that still points at `https://your-auth-provider.com`.

Server-side schedules and bots run in `sink-services` behind the generated `SINK_SECRET` and a scoped JWT; the Events course covers what those events do.

## 3 · Verify like an operator

```bash
python3 scripts/preflight.py
python3 scripts/up.py
docker compose ps --all
```

Preflight renders the Compose graph, checks the image pins, the connection budgets, and the gVisor runtime on the daemon. `up.py` repeats it and then runs `docker compose up -d --no-build`: missing images are pulled, and a failed pull fails loudly instead of silently compiling for twenty minutes. `db-init` and `object-store-init` should exit successfully — they're one-time jobs, not crashes. Then hit the health endpoints with `curl --fail` on 8080 and 3001, and ask the manager directly whether it has a warm sandbox ready:

```bash
docker compose exec execution-manager /app/execution-manager healthcheck
```

Open `http://localhost:3001`, log in through your OIDC provider, and run a flow. That's the entire Flow-Like backend on one VM — Priya's customer records now sleep in your buckets.

**Watch out:** `docker compose config` without `--quiet` prints interpolated secrets. Preflight hides that output for exactly this reason. Never paste it into a ticket or public log.

**Recap**

- One host, published images: empty `*_IMAGE` values mean `ghcr.io/rheosoph/…:dev`, and the public packages need no login.
- Per-run isolation refuses moving tags for sandboxes — `pull-images.py` pins digests, `prepare-images.py` pins local builds, and `up.py --build` assigns local tags to empty `*_IMAGE` values before building.
- `setup-env.py` already made the keys and credentials; you supply the OIDC configuration and verify with `ps --all`, health checks, and the manager's healthcheck.
