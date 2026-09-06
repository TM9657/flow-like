# Building and publishing backend containers

Backend Dockerfiles belong to the `flow-like` repository. Build the AWS images
from this repository root, where `Cargo.toml`, `Cargo.lock`, `packages/`, and
`apps/` are available. The deployment repository supplies runtime configuration
and currently invokes these same Dockerfiles during Terraform apply.

The [Backend containers workflow](../../.github/workflows/containers.yml) builds,
checks, and pushes release images to GHCR. Deployment-side digest resolution and
promotion still need wiring before Terraform can skip its current local builds.

## AWS build targets

Paths in this table are relative to the `flow-like` root. The platform is part
of the build contract: the Rust recipes contain architecture-specific tools,
libraries, cache paths, or runtime images.

| Image | Dockerfile | Platform | Deployment-dependent build input |
| --- | --- | --- | --- |
| API | `apps/backend/aws/api/Dockerfile` | `linux/arm64` | Public config fallback; optional `CRDB_CA_URL` |
| Executor Lambda | `apps/backend/aws/executor/Dockerfile` | `linux/amd64` | None |
| Async executor | `apps/backend/aws/executor-ecs/Dockerfile` | `linux/amd64` | None |
| Compiler | `apps/backend/aws/compiler-ecs/Dockerfile` | `linux/arm64` | None |
| File tracker | `apps/backend/aws/file-tracker/Dockerfile` | `linux/arm64` | Optional `CRDB_CA_URL` |
| Media transformer | `apps/backend/aws/media-transformer/Dockerfile` | `linux/arm64` | None |
| Event bridge | `apps/backend/aws/event-bridge/Dockerfile` | `linux/arm64` | None |
| Maintenance | `apps/backend/aws/maintenance/Dockerfile` | `linux/arm64` | None |
| Signaling | `apps/backend/docker-compose/signaling/Dockerfile` | `linux/arm64` | None |
| Migration | `apps/backend/aws/migration/Dockerfile` | `linux/arm64` | None |

The DSQL migration job has its own
[`aws/migration/Dockerfile`](aws/migration/Dockerfile) and
[build/run instructions](aws/migration/README.md). It is a separate manual
database operation. Publishing its image does not run a migration.

The API's `Dockerfile-distroless` and `Dockerfile-scratch` alternatives have also
moved beside its default Dockerfile. Terraform uses only `api/Dockerfile`.

## Build locally

From the deployment checkout, first run `cd flow-like`. From a standalone
`flow-like` checkout, use its root directly. Docker must be running with Buildx;
use a native ARM64 builder for this API example:

```sh
docker buildx build \
  --platform linux/arm64 \
  --provenance=false \
  --load \
  --tag flow-like-aws-api:local \
  --file apps/backend/aws/api/Dockerfile \
  .
```

The final `.` is always the `flow-like` root for the AWS targets above. For a
different image, change the Dockerfile, platform, and tag using the table.
`--load` puts the single-platform result in the local Docker image store.

The API and file tracker use an empty `CRDB_CA_URL` for DSQL. For Aurora in
`eu-west-1`, add this argument to each build:

```sh
--build-arg CRDB_CA_URL=https://truststore.pki.rds.amazonaws.com/eu-west-1/eu-west-1-bundle.pem
```

An external database may need its own CA URL. Database passwords, signing keys,
bucket names, queue URLs, and secret-manager credentials are runtime inputs.
They are not required to compile these images. `CARGO_BUILD_JOBS` is optional
for the Rust AWS recipes; choose it for the builder's available memory and CPU.

All API images accept runtime configuration, so installation-specific hub/OIDC
settings no longer require a build. GCP and Azure also retain an optional legacy
custom compiled default: the `flow_like_config` BuildKit secret plus its
`FLOW_LIKE_CONFIG_SHA256` build argument. Those contents are embedded in the
binary and must contain no client secrets. Their [GCP](gcp/api/README.md#secure-image-build)
and [Azure](azure/api/README.md#secure-image-build) instructions describe both paths.

## Runtime API configuration

Set exactly one nonempty input on the API container:

| Input | Value |
| --- | --- |
| `FLOW_LIKE_CONFIG_JSON` | A complete JSON config, including when injected from a deployment secret |
| `FLOW_LIKE_CONFIG_FILE` | A path to a readable mounted JSON file, such as `/run/secrets/flow-like.config.json` |
| `FLOW_LIKE_CONFIG_SECRET_REF` | A key or `secret://...` reference resolved by that API's configured secret store |

With no override, the API uses the compiled default. An override replaces the
whole document; it is not merged with hosted defaults. Conflicting inputs,
unreadable files, unresolved secrets, and invalid configurations stop startup.
Empty environment values count as unset so deployment templates can omit an
input. Whitespace-only values are errors. Configuration is loaded once per
process; roll out/restart the API after changing it. Documents must be UTF-8 and
at most 4 MiB. The deployment platform or secret provider may impose a smaller
payload limit; a mounted file avoids environment-variable size limits.

For example, start an image with your normal database/storage/key environment
and a read-only mounted config:

```sh
docker run --env-file ./api.env \
  --mount type=bind,src=/absolute/path/flow-like.config.json,dst=/run/secrets/flow-like.config.json,readonly \
  --env FLOW_LIKE_CONFIG_FILE=/run/secrets/flow-like.config.json \
  ghcr.io/rheosoph/flow-like-docker-compose-api@sha256:<selected-platform-digest>
```

The same selected document configures hub metadata, OpenID issuer/audience/client
validation, and third-party OAuth provider endpoints. OAuth client secrets remain
separate secret-store values referenced by `client_secret_env`; literal
`oauth_providers.*.client_secret` strings are rejected. Loading JSON from a secret
does not make all Hub fields private: clients still receive the public Hub
metadata. The API does not log the override's contents or secret reference.

Secret references use existing provider configuration and permissions: AWS API
uses Parameter Store, GCP API uses Secret Manager, Azure API uses Key Vault,
and self-hosted APIs can use mounted files or secret-injected environment values.
No new cloud permissions are granted by this change. Runtime configuration also
does not change compiled storage/provider features or the image's CPU architecture.

## Registry choice

Use GitHub Actions to build and GitHub Container Registry (GHCR) to publish the
shared release. The workflow publishes names such as
`ghcr.io/rheosoph/flow-like-aws-api`,
`ghcr.io/rheosoph/flow-like-aws-executor`, and equivalent workload names for
Azure and GCP. The namespace is the lowercased GitHub repository owner, so forks
publish to their own namespace. No packages are created until the workflow runs.

Deployments copy the selected image into their cloud registry:

| Runtime | Deployment registry |
| --- | --- |
| AWS Lambda and ECS | Private ECR in the deployment region |
| GCP Cloud Run | Artifact Registry |
| Azure Container Apps | The installation's ACR |

Lambda requires ECR in the function's region and a single-architecture image.
Keep `--provenance=false` for its deployable image. Store release evidence
separately if adding provenance would wrap it in an image index.
See [AWS container image requirements](https://docs.aws.amazon.com/lambda/latest/dg/images-create.html)
and [AWS's Buildx example](https://docs.aws.amazon.com/lambda/latest/dg/java-image.html).

GHCR lets the upstream build run without access to deployment AWS accounts. An
AWS-only installation can instead publish directly to a central private ECR
repository and replicate to its deployment accounts and regions. For the three
cloud targets here, GHCR provides a common distribution point.

## Publish from GitHub Actions

The build job needs `contents: read` and `packages: write`. Authenticate to GHCR
with `GITHUB_TOKEN`; explicitly choose package visibility and give the deployment
repository read access if packages remain private. New packages default to
private. See [GitHub's publishing instructions](https://docs.github.com/en/actions/tutorials/publish-packages/publish-docker-images)
and [GHCR access settings](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).

Commit and push these files in the `flow-like` repository first. When working in
the deployment checkout, the submodule's changes need their own upstream commit
before the parent repository can point to it. Then:

1. Permit GitHub Actions to create packages in the organization. Protect `dev`,
   `main`, `alpha`, and `v*` tags: pushes to them publish images. Existing packages
   must grant this source repository Actions write access.
2. Open **Actions > Backend containers > Run workflow**, select an approved
   branch, and choose `aws`, `gcp`, `azure`, `docker-compose`, `kubernetes`,
   `self-hosted`, or `all`. No custom registry PAT or
   AWS/GCP/Azure credentials are needed. The Git repositories currently pinned in
   `Cargo.lock` are publicly readable.
3. After the selected image set succeeds, download the
   `container-images-<commit>-<run>-<attempt>` artifact. Its `container-images.json`
   maps workloads to `ghcr.io/<owner>/flow-like-<cloud>-<workload>@sha256:...`.
4. Review package visibility explicitly. Keep packages private unless the
   intended audience should receive the binaries and their embedded metadata.
   Private consumers need package read access. The workflow never changes visibility.

The default matrix contains 10 AWS, 7 GCP, and 8 Azure builds, plus 9 Compose and
6 Kubernetes images built on both native AMD64 and ARM64 runners: 55 build jobs
for 40 image repositories. Shared Kubernetes helpers reuse Compose image entries
instead of being rebuilt under a second name. The exact recipes
and native platforms are in
[`container_images.py`](../../.github/scripts/container_images.py).
Cloud APIs use the committed `flow-like.config.json` as their fallback; the AWS
API and file tracker use an empty `CRDB_CA_URL` (DSQL). Records identify these
build inputs and the `full-document-v1` runtime config contract. GCP/Azure APIs
are now included without installation config build secrets. Aurora/custom-CA
variants still require their matching CA build input; runtime Hub configuration
does not replace baked CA certificates.

## Kubernetes and Docker Compose

Use `self-hosted` to publish all 15 self-hosted image repositories for both
architectures. `docker-compose` selects its nine images. `kubernetes` selects
its six images plus the shared Compose runtime, compiler, signaling, and
object-store initializer. PostgreSQL, Redis, RustFS, and monitoring tools remain
their upstream images; this workflow does not rebuild or rebrand them.

| Image set | Published workloads |
| --- | --- |
| `flow-like-docker-compose-*` | `api`, `runtime`, `execution-manager`, `compiler`, `sink-services`, `signaling`, `db-init`, `object-store-init`, `web` |
| `flow-like-kubernetes-*` | `api`, `executor`, `execution-manager`, `sink-trigger`, `migration`, `web` |

Each platform has its own digest record, such as `docker-compose-runtime-arm64`.
Select the architecture of the deployment host/nodes. These releases publish
separate single-platform images, not a multi-architecture index under one tag.
For a mixed-architecture Kubernetes cluster, use a qualified multi-architecture
index or constrain the applicable workload to the selected architecture.

The Compose and Kubernetes API records are marked
`runtime-config-self-hosted-default`. Their reviewed `flow-like.config.example.json`
remains the fallback, including placeholder OIDC settings. Supply your complete
installation config at runtime; the same prebuilt API can then serve different
installations. Records still hash the compiled fallback for release evidence.

Compose already accepts `API_IMAGE`, `RUNTIME_IMAGE`, `EXECUTION_MANAGER_IMAGE`,
`COMPILER_IMAGE`, `SINK_SERVICES_IMAGE`, `SIGNALING_IMAGE`, `DB_INIT_IMAGE`,
`OBJECT_STORE_INIT_IMAGE`, and `WEB_IMAGE`. Set these to selected manifest digest
references. With a validated deployment `.env`, `docker compose up --no-build`
can use those images without compiling. The existing `prepare-images.py` helper
still builds the local runtime/manager to pin sandbox image IDs; adapting that
installer and generating Helm values directly from release manifests are
separate follow-up changes. Keep the current preflight and isolation requirements.

Helm already pulls images from registries. Override its local-development
repository/tag defaults with the matching published image references. The
execution manager and sandbox have explicit digest inputs. Other chart images
still use repository/tag fields; those need digest-aware values generation before
the whole installation has a uniform immutable promotion path. Configure the API's
runtime config source before replacing a custom compiled image with a shared one.

### Reuse the web image across installations

Both web images read only these public settings at container startup:

| Runtime variable | Meaning |
| --- | --- |
| `FLOW_LIKE_WEB_API_URL` | Required browser-reachable HTTP(S) API base URL |
| `FLOW_LIKE_WEB_REDIRECT_URL` | Optional login callback URL; defaults to the browser origin plus `/callback` |
| `FLOW_LIKE_WEB_LOGOUT_URL` | Optional logout return URL; defaults to the browser origin plus `/` |

Compose maps its existing `NEXT_PUBLIC_*` deployment values into these runtime
variables. Helm uses `web.runtimeConfig.apiUrl`, falling back to `api.publicUrl`,
and optional `web.runtimeConfig.redirectUrl` / `logoutUrl`. These are runtime
values; changing them requires recreating/restarting the web container, not
rebuilding its image. Restart or roll out Pods after changing Helm values.

The entrypoint validates URLs and writes only this allowlist to
`/tmp/flow-like-web/runtime-config.js`. Nginx serves it with `no-store`; the web
app loads it before hydration. A missing API URL stops startup. Credentials,
query strings, and fragments in these URLs are rejected. Never supply tokens
or server secrets as public settings. The image runs without root and supports
a read-only root filesystem with writable `/tmp`.

Non-container web/desktop builds retain their build-time configuration behavior.
The runtime mode is enabled only by these web Dockerfiles.
Static SEO/social metadata still uses the web build's existing site defaults;
this runtime contract changes API routing and app sign-in/sign-out, not every
generated metadata URL. Third-party integration OAuth is a separate, pre-existing
limitation: it still uses the hosted `flow-like.com/thirdparty/callback` relay
and can select the stored profile's hub for its API. Self-hosting that integration
flow needs a separate callback/provider-registration change. These runtime
variables do not configure that relay.

The web builders use Bun for frozen dependency installation and pinned Node.js 24
for Next.js compilation and export. Bun's Node fallback crashed Next's export
workers during local validation. Neither runtime is shipped in the final web
image; it contains static assets, Nginx, and the public-config generator.

## Release checks and identity

PRs run helper and context tests without building or publishing images. Approved
pushes build the whole set, including on source changes, so every release manifest
corresponds to one commit. Manual runs can select one complete cloud set. Jobs
use up to 16 parallel jobs and native runners; ARM64 runner access must be available
to the repository. Rust builds default to two Cargo compilation jobs per runner.

Each job builds once, loads the result locally, checks platform/license/credential
paths, scans image files and image configuration with pinned Trivy, and scans
printable text from ELF binaries. Only then does it authenticate and push that
same image. Raw reports and extracted binary text are not uploaded or printed.
These are credential/publication gates, not a vulnerability or complete license
compliance scan. Read the [publication audit](CONTAINER-PUBLISHING-AUDIT.md)
for what is intentionally visible and what the checks cannot prove.

Tags include the full source SHA, architecture, run ID, and run attempt. There is
no mutable `latest` tag. Digest records also bind the Dockerfile and applicable
configuration inputs. The final job rejects missing, duplicate, wrong-platform,
or mixed-build records. A failed set can leave individually checked images in
GHCR, but it produces no complete manifest. Use **Re-run all jobs**, not only
failed jobs, because a manifest must contain records from one run attempt.
Tags are descriptive; consumers must pin the digest. The manifest artifact is
retained for 90 days. Save it with deployment release evidence before it expires.

## Parallel builds and promotion

AWS executor jobs need AMD64; most other AWS jobs need ARM64. Measure CPU, peak
memory, disk, and cache size before raising the workflow's `max-parallel` limit.
GitHub offers native ARM64 runners, and Docker notes that emulation is
particularly slow for compilation. See the [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
and [native build guidance](https://docs.docker.com/build/building/multi-platform/).

The workflow uses a separate GitHub Actions layer-cache scope per image. It does
not publish builder layers to a public GHCR `buildcache` package: Rust builders
copy the source checkout, so that would distribute intermediate source files.
GitHub Actions caches are also readable by eligible PR workflows, including fork
PRs. Never put credentials or private Git source in them.

The Dockerfiles put downloaded crates and compiled targets in
`RUN --mount=type=cache`; ordinary layer caching does not export those mounts.
Set repository variable `CONTAINER_CARGO_CACHE=true` to enable the pinned
cache-mount restore/extraction steps. This is off by default because the
[existing cache capacity](../../.github/README.md#cache-capacity) is already
constrained, and per-image Rust targets can consume many gigabytes. Budget storage
and check dependency visibility before enabling it. Without mount persistence,
unchanged layers can be reused, but a source change can trigger a cold Rust compile.
Web `.next` cache mounts likewise persist only on a retained builder; the optional
Cargo cache steps do not export them from ephemeral GitHub runners.
See [Docker's Actions cache guidance](https://docs.docker.com/build/ci/github-actions/cache/).

For sustained release volume, use isolated persistent native BuildKit workers
with enough SSD and memory for Cargo targets. Their retained mounts avoid
uploading huge cache archives on every run. That infrastructure is a follow-up;
this workflow currently uses ephemeral GitHub-hosted builders.

Several AWS recipes share a Cargo target cache with `sharing=locked`. On one
BuildKit worker those compilation steps can serialize. Give concurrent jobs
separate workers or target-cache namespaces, or deliberately compile compatible
binaries together. Keep trusted release caches separate from untrusted jobs.

Publish a complete release manifest after the required matrix jobs succeed.
The provided manifest records the source commit, workload, platform, recipe hash,
applicable default config/CA identity, and image digest. Promotion copies these exact images without
recompiling. The deployment job authenticates to its cloud through
[OIDC](https://docs.github.com/en/actions/how-tos/secure-your-work/security-harden-deployments/oidc-in-cloud-providers),
copies the image, verifies its destination digest, and supplies the references
to Terraform before generating the saved plan. Reuse each unchanged image digest
from development through production. Supply the destination's API config at
runtime and retain its version with deployment release evidence.

The current Azure registry is private and disables the trusted-service bypass
required by `az acr import`. Use a network-connected promotion worker to copy
images into that ACR. See [ACR import restrictions](https://learn.microsoft.com/en-us/azure/container-registry/container-registry-import-images).

GCP and Azure deployment roots already accept prebuilt digest references. AWS
still needs a prebuilt-image input and conditional build resources; this
relocation prepares that change. A release resolver should either find the
complete matching manifest or explicitly build/wait for it before planning.
API hub/OIDC configuration can now vary at runtime without changing that image.
