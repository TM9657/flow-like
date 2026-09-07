# Container publication audit

The September 6, 2026 source audit identified no live credentials in the inspected
backend image recipes, shipped application inputs, or high-confidence token and
private-key pattern checks of 11,036 tracked text files. Matches examined were
test/example fixtures. This is a source-level finding, not proof that every built
image is secret-free. The publication workflow scans each actual image before push.

## Information image consumers can read

- API binaries embed a complete default `flow-like.config.json` through
  `packages/api/src/runtime_config.rs`. Runtime config can replace that document;
  Hub, OpenID validation, and OAuth provider handlers use the same selected
  configuration. The tracked default contains public client IDs, domains/endpoints,
  Stripe product IDs, and environment-variable/secret-reference names. It does
  not contain the referenced secret values. Review whether those public
  identifiers are appropriate for the package's audience.
- Optional GCP/Azure API BuildKit secret mounts prevent a loose custom-default
  config file from being saved in a build layer. They do not conceal the config
  embedded in the executable. Automatic builds supply no custom config and
  include these APIs with the reviewed public fallback.
- Signaling images ship their selected TypeScript sources. Migration images ship
  schemas, migration/pre-push code, and runtime dependencies. Inspected credentials
  are runtime environment lookups. GCP's loopback `build:build` database URL is a
  dummy Prisma build input, not a deployed database credential.
- Rust final stages copy selected binaries, public CA certificates, and optional
  ONNX libraries. They do not copy the full source checkout. Image layers and
  executable text remain accessible to anyone allowed to pull the image.

## Changes made

| Finding | Publication control |
| --- | --- |
| Developer credentials could enter broad builder `COPY . .` | Extended root `.dockerignore` for nested Git metadata, Git/npm/Cargo credentials, cloud CLI and SSH profiles, key stores, and Terraform state; tested with synthetic paths |
| A public `mode=max` registry cache would expose intermediate source | Removed that publishing example; workflow uses repository-scoped Actions caches, never a GHCR build-cache package |
| Final images omitted the root license | Inspected final stages now include `/usr/share/licenses/flow-like/LICENSE`; workflow labels use `BUSL-1.1` |
| AWS executor images omitted ONNX archive notices | Both executor variants now include the archive's `LICENSE` and `ThirdPartyNotices.txt` |
| Cold builders invoked an uninstalled tool | Added `unzip` to four AWS build recipes that already invoked it |
| Compiled secrets can evade ordinary file scanning | Workflow scans image files/config plus printable strings extracted from every ELF file before pushing |

The root license has custom Business Source License parameters; the included
full text governs use. Adding these notices is distribution hygiene, not a full
dependency-license assessment or legal clearance.

## Build inputs and cache boundary

The standard matrix needs no deployment credentials, database password, client
secret, or signing key at build time. All 18 distinct Git repositories referenced
by the audited `Cargo.lock` were anonymously readable, including the pinned
`any-speech-to-text` revision. No private-repository PAT is needed today.

Keep deployment secrets at runtime. If a future dependency becomes private,
review its redistribution terms and cache exposure before adding authentication.
Use ephemeral BuildKit secret mounts; do not persist tokenized URLs, credential
helpers, or cloud profiles in layers or Cargo caches. GitHub Actions caches are
not secret storage and can be restored by eligible fork PR workflows.

## Verification limits

The publication matrix covers nine Compose and six Kubernetes images, each on
AMD64 and ARM64, plus the cloud targets. Their APIs embed the matching public
self-hosting example config, including placeholder OIDC settings. The release
records label this `runtime-config-self-hosted-default`, include its fallback
hash, and identify the `full-document-v1` runtime contract. Supply the complete
installation config at startup; the default alone is not production configuration.

Runtime documents are not Docker build inputs and do not enter image layers or
build caches. The loader rejects literal OAuth client secrets; use existing
secret-store references for those values. Loading the document from a secret
does not hide public Hub fields from clients. Source-selection, file, secret,
and parse failures are sanitized and never silently select the hosted fallback.

Self-hosted backend final stages now include the root license. The object-store
initializer uses a repository-root context so it can include that license while
still copying only its selected Python sources and requirements. The Kubernetes
migration image uses the reviewed Compose Prisma lockfile instead of floating
installs. The Kubernetes sink trigger uses a matching glibc runtime instead of a
static-only runtime lacking its loader. Neither build starts a migration or
contacts an installation's database.

The Kubernetes migration entrypoint still contains its pre-existing
`--accept-data-loss` flag. Publishing that image does not authorize executing a
destructive schema update. Review the migration operation and recovery plan
before using it; this publishing change does not alter that policy.

Pinned Trivy secret scans are gates for recognized patterns, not a guarantee
against every secret. The filesystem helper rejects common credential paths and
checks the platform and license without starting the container. The extra binary
scan covers printable ELF strings because the pinned scanner skips ELF files;
compressed, encoded, encrypted, and unrecognized credentials can still evade it.
The default-config check also rejects common literal credential field names.

The Compose object-store initializer and Kubernetes migration images exposed a
scanner false positive in Debian's GnuTLS libraries: embedded self-test private
keys. All 22 findings across those two images matched complete public upstream
fixtures byte-for-byte. The supplemental scan now replaces only full PEM blocks
whose exact SHA-256 and byte length match the 12 reviewed fingerprints in
[`public_secret_fixtures.json`](../../.github/scripts/public_secret_fixtures.json).
That file cites immutable [GnuTLS self-test source](https://github.com/gnutls/gnutls/blob/477a733247460b94cd2b37a10579c27ca6fc196f/lib/crypto-selftests-pk.c).
Unknown, modified, truncated, or differently encoded keys remain in scan input.
No library, image path, or secret detection rule is excluded, and image contents
are never changed by this scanner-input treatment.

The AWS API binary also places public literals from the Copilot secret-redaction
code next to each other. Trivy interprets the Slack prefix followed by credential
field names as a token. The supplemental scan inserts a line break at that source
literal boundary only when the complete reviewed sequence matches, including its
surrounding private-key marker and field names. All original bytes remain in the
scan input. Changed or partial sequences receive no normalization. The source is
[`stream.rs`](../../packages/core/editor/src/flow/copilot/stream.rs), in
`redact_private_key_blocks`, `redact_known_secret_tokens`, and
`redact_inline_secret_values`.

Trivy JSON reports and extracted strings may themselves contain matched secrets.
They stay in temporary runner storage and are not uploaded as artifacts. On a
failure, reproduce the scan in a trusted environment and handle reports as
sensitive. Do not paste raw findings into public build logs or issues.

No complete Rust image set or remote publishing run was executed during this
audit. Local native ARM64 builds and the filesystem/platform/license plus pinned
Trivy image/config and binary-text gates passed for signaling, the Compose
object-store initializer, Kubernetes migration, and both web images. The
object-store/migration scans passed after applying the exact public-fixture
checks above; the web scans needed no fixture substitutions.

Each web image exported all 97 static pages, then passed startup checks as UID
1000 with a read-only root filesystem, writable `/tmp`, no network, and dropped
capabilities. Two synthetic URL configurations worked with the same image.
The generated script contained only allowlisted public fields, used `no-store`,
and appeared in the exported HTML. Missing or credential-bearing API URLs stopped
startup without echoing the value. Each image's 4,872 regular files and text from
210 ELF files yielded zero secret findings. These checks do not exercise a real
identity provider or the separate hosted third-party integration OAuth relay.

The local validation also passed all 49 CI helper tests, six generator tests,
six browser-config tests, 19 Compose deployment tests, 37 Kubernetes tests,
build-context tests, workflow lint, and Dockerfile static checks for the initial
53 matrix targets. The two newly included GCP/Azure API recipes also pass static
checks, bringing the matrix to 55 targets. These samples do not substitute for
building the remaining images or running the workflow on GitHub.

The runtime API change passes production and test-code compilation with
`cargo check -p flow-like-api --no-default-features --features local,redis --locked --offline`
and the corresponding `--tests` check. Six executable tests use the exact
production source-loader module and real file-backed SecretStore. They cover
source selection, bounded file reads, missing/invalid inputs, failure without
fallback, and suppression of nested provider logs containing secret references.
Full Hub-schema, OAuth resolver, and OpenID validation tests were added and
compile-checked; the full API test binary was not locally linked or executed.

CI must finish its image-specific checks before any image is
eligible for publication. CVE policy, SBOM review, signing, and cloud-registry
promotion are separate follow-up controls.
