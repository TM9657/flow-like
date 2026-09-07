---
title: GCP deployment
description: Configure Cloud Run, Cloud SQL IAM authentication, storage isolation, secrets, and schema deployment.
sidebar:
  order: 10
---

The GCP API runs on Cloud Run and authenticates to Cloud SQL, Cloud Storage,
Firestore, Pub/Sub, and Secret Manager through its runtime service account.
Provision those resources, database grants, and secrets before routing traffic
to a revision. Deploy [execution and queue workers](/self-hosting/gcp/workers/)
and [scheduled jobs](/self-hosting/gcp/automation/) with separate identities.

## Build and runtime configuration

Build the shared API image from the repository root without cloud credentials
or installation configuration:

```sh
docker buildx build --platform linux/amd64 \
  --load --tag flow-like-gcp-api:local \
  -f apps/backend/gcp/api/Dockerfile .
```

Supply a complete configuration with `provider: "gcp"`, the installation's
OIDC/OAuth settings, and a `mail.smtp` block. Select exactly one nonempty source:

| Variable | Source |
| --- | --- |
| `FLOW_LIKE_CONFIG_JSON` | Complete JSON, optionally injected from a Cloud Run secret |
| `FLOW_LIKE_CONFIG_FILE` | Readable JSON file mounted read-only |
| `FLOW_LIKE_CONFIG_SECRET_REF` | Secret Manager key or qualified reference such as `secret://gcp-secret-manager/HUB_CONFIG` |

The document replaces the complete public fallback and is read once at
startup. Conflicting or invalid sources stop startup. Grant access to a
referenced configuration secret individually, and restart or deploy a new
revision after changes. OAuth client secrets must be separate secret-store
entries referenced by `client_secret_env`; literal client secrets are rejected.
The public embedded fallback does not configure a GCP installation.

See [runtime API configuration](/self-hosting/containers/#runtime-api-configuration)
for source handling and optional custom compiled defaults. A default compiled
through a BuildKit secret is still embedded in the binary and must contain no
credentials.

## Required API environment

```text
GCP_PROJECT_ID=<app-project-id>
GCP_CONTENT_BUCKET=<content-bucket>
GCP_META_BUCKET=<metadata-bucket>
GCP_CDN_BUCKET=<cdn-bucket>
GCP_LOG_BUCKET=<logs-bucket>
SECRET_PREFIX=/flow-like/gcp-<environment>
MAIL_PROVIDER=smtp
CORS_ALLOWED_ORIGINS=https://<frontend-domain>

GCP_POSTGRES_AUTH_MODE=iam
GCP_POSTGRES_HOST=<private-IP-or-cloudsql-hostname>
GCP_POSTGRES_DATABASE=flow_like
GCP_POSTGRES_USER=<service-account-local-part>@<project>.iam
GCP_POSTGRES_SERVER_CA=<instance-server-CA-PEM>
```

Set `GCP_POSTGRES_HOST` to an RFC1918 private IPv4 address or the instance's
actual `*.cloudsql.goog` hostname, without a scheme, port, or path. The
database client fixes the port at 5432. The database name and IAM user must be
at most 63 UTF-8 bytes; the user omits `.gserviceaccount.com` from the service
account email. The CA setting accepts PEM with real or escaped newlines and is
read at startup. Rotate the revision when the instance CA changes.

Metadata and content must use separate buckets. Both use the `apps/<app_id>/`
prefix space; a downscoped content token could otherwise also read board and
event definitions. Startup rejects a shared bucket. If generic aliases are
also set, each pair must agree:

| GCP setting | Generic alias |
| --- | --- |
| `GCP_META_BUCKET` | `META_BUCKET` |
| `GCP_CONTENT_BUCKET` | `CONTENT_BUCKET` |
| `GCP_LOG_BUCKET` | `LOG_BUCKET` |
| `GCP_CDN_BUCKET` | `CDN_BUCKET_NAME` |

`STORAGE_PROVIDER`, `RUNTIME_CREDENTIALS_PROVIDER`, `META_BUCKET_PROVIDER`,
`CONTENT_BUCKET_PROVIDER`, and `LOGS_BUCKET_PROVIDER` must select `gcp` when
set; `gcs` and `google` are accepted aliases. Other cloud feature sets are
compiled out of this image.

Cloud Run supplies `PORT` (default `8080` in the image). Optional telemetry
settings are `OTEL_EXPORTER_OTLP_ENDPOINT` for OTLP/gRPC and
`GCP_REQUIRE_OTEL=true` to make a missing endpoint fatal. Use `RUST_LOG` for
log filtering.

## Identity and access

Use a dedicated API service account. Scope its permissions to the application
resources:

| Role | Scope and use |
| --- | --- |
| `roles/cloudsql.client` | Cloud SQL connect permission |
| `roles/cloudsql.instanceUser` | Cloud SQL IAM database login |
| `roles/secretmanager.secretAccessor` | Each allowed secret individually |
| `roles/storage.objectUser` | Each metadata, content, and CDN bucket |
| `roles/storage.objectCreator` | Logs bucket for append-only writes |
| `roles/iam.serviceAccountTokenCreator` | On the API account itself for `signBlob`, used to mint V4 signed URLs |
| `roles/datastore.user` | Firestore database/project for cache and execution state |
| `roles/pubsub.publisher` | Execution and compilation topics |
| `roles/run.invoker` | HTTP executor service |
| `roles/logging.logWriter` | Application project for container logs |

Apply Cloud SQL roles through IAM bindings covering the target instance. IAM
database authentication also requires an in-database login and grants; roles
alone do not grant table access. The API needs no Cloud Scheduler permissions
or access to a separate Terraform state project.

Secret access must be per secret. The provider tries a prefix-qualified name
and then an unprefixed fallback, so project-wide access can cross environment
boundaries even when `SECRET_PREFIX` differs. Before first startup, provision
versions of all required secrets under that prefix:

| Secret | Minimum bytes |
| --- | --- |
| `BACKEND_KEY` | `64` |
| `BACKEND_PUB` | `64` |
| `BACKEND_KID` | `8` |
| `SINK_SECRET` | `32` |
| `SINK_TOKEN_ENCRYPTION_KEY` | `32` |
| `MAINTENANCE_TOKEN` | `32` |

The API disables environment overrides for its secret store. Setting an
environment variable with one of those names does not satisfy the startup
check. Grant separate access to configuration and OAuth secrets as needed.

For synchronous execution, pair the executor invoker grant with
`EXECUTOR_AUTH=gcp_id_token`, `EXECUTION_BACKEND=http`, and `EXECUTOR_URL` on
the API. Dispatch obtains a Google ID token whose audience is the executor
URL's origin. The execution JWT inside the request remains the application's
run credential. A metadata token failure fails dispatch before an anonymous
request can reach the executor.

### Rejected credential sources

The API fails startup when static credentials, endpoint overrides, or proxy
settings are present, including empty values:

```text
GOOGLE_APPLICATION_CREDENTIALS, GOOGLE_APPLICATION_CREDENTIALS_JSON,
GOOGLE_CREDENTIALS, GOOGLE_OAUTH_ACCESS_TOKEN, CLOUDSDK_AUTH_ACCESS_TOKEN,
GCE_METADATA_HOST, GCE_METADATA_IP, GCE_METADATA_ROOT, METADATA_SERVER_DETECTION,
HTTP_PROXY, HTTPS_PROXY, ALL_PROXY (and lowercase forms),
GOOGLE_SERVICE_ACCOUNT, GOOGLE_SERVICE_ACCOUNT_PATH,
GOOGLE_SERVICE_ACCOUNT_KEY, SERVICE_ACCOUNT,
GOOGLE_SKIP_SIGNATURE, GOOGLE_ALLOW_HTTP, GOOGLE_ALLOW_INVALID_CERTIFICATES,
GOOGLE_PROXY_URL, GOOGLE_PROXY_CA_CERTIFICATE, GOOGLE_PROXY_EXCLUDES,
STORAGE_EMULATOR_HOST, PUBSUB_EMULATOR_HOST, FIRESTORE_EMULATOR_HOST,
DATASTORE_EMULATOR_HOST, SECRET_MANAGER_EMULATOR_HOST,
CLOUDSDK_API_ENDPOINT_OVERRIDES_*, GOOGLE_*_CUSTOM_ENDPOINT
```

The database client additionally rejects `DATABASE_URL`, database passwords,
the libpq connection settings (`PGPASSWORD`, `PGPASSFILE`, `PGSERVICE`,
`PGSERVICEFILE`, `PGHOST`, `PGHOSTADDR`, `PGPORT`, `PGUSER`, `PGDATABASE`,
`PGSSLMODE`, `PGSSLROOTCERT`, `PGSSLCERT`, `PGSSLKEY`, `PGOPTIONS`, `PGAPPNAME`),
`INSTANCE_CONNECTION_NAME`, `CLOUD_SQL_CONNECTION_NAME`, `CLOUD_SQL_PROXY_PATH`,
and supported `CSQL_PROXY_*` selectors. These controls keep identity acquisition
on the instance metadata path and database connections on the configured direct
TLS path. The exact lists are in the
[API guard](https://github.com/Rheosoph/flow-like/blob/main/apps/backend/gcp/api/src/config.rs)
and [PostgreSQL client](https://github.com/Rheosoph/flow-like/blob/main/packages/gcp-data/src/postgres.rs).

## Database bootstrap and token rotation

Create Cloud SQL IAM service-account users for the API and migration job. As
the schema-owning migration identity, connect to `flow_like` and grant runtime
DML rights. Replace the quoted roles with the actual database users:

```sql
grant connect on database flow_like to "<api-account>@<project>.iam";
grant usage on schema public to "<api-account>@<project>.iam";
grant select, insert, update, delete on all tables in schema public
  to "<api-account>@<project>.iam";
grant usage, select, update on all sequences in schema public
  to "<api-account>@<project>.iam";

alter default privileges for role "<migration-account>@<project>.iam"
  in schema public grant select, insert, update, delete on tables
  to "<api-account>@<project>.iam";
alter default privileges for role "<migration-account>@<project>.iam"
  in schema public grant usage, select, update on sequences
  to "<api-account>@<project>.iam";
```

Run existing-table grants after the initial schema push. Default privileges
cover future objects created by the migration role. Keep DDL, role
administration, and database ownership off the API identity. The file-tracking
worker needs its own IAM user with update rights on `App` and `User`.

The API and file tracker request a token scoped to
`https://www.googleapis.com/auth/sqlservice.login` and use SQL connection
options with TLS `verify-full` against the supplied instance CA. Their pools
cannot replace the token for new connections. The API closes readiness five
to eight minutes before expiry, waits for readiness propagation, drains, and
exits. Use at least two instances when availability must survive token rotation
and keep revisions restartable. Per-process jitter staggers the exits.

## Schema deployment and recovery

Build the migration image from the repository root and deploy it as a manually
triggered Cloud Run Job under the separate migration account:

```sh
docker build -f apps/backend/gcp/migration/Dockerfile -t flow-like-gcp-migration .
gcloud run jobs execute <migration-job> --region <region> --wait
```

Configure the same `GCP_POSTGRES_*` settings as the API, with the migration
account's database user. The migration runner additionally accepts an instance
DNS name ending in `.sql.goog`; the Rust runtime client currently accepts
`.cloudsql.goog` or a private IPv4 address. `GCP_PROJECT_ID` and `GCP_REGION`
are optional and unused by this runner.

The image builds and validates a PostgreSQL mirror of the tracked CockroachDB
Prisma schema. The runner obtains a SQL-login token from metadata, runs the
guarded pre-push column conversions, and then runs `prisma db push`. Each child
receives its connection URL through `DATABASE_URL`. The runner does not log
or write that URL to disk, and forwards SIGTERM to the active child.

The migration runner has a transport boundary that differs from the runtime
client. With a private-IP host it uses TLS without server certificate
verification and logs a warning: Prisma cannot verify the CA without also
matching the hostname. With an instance DNS name present in the server
certificate, it enables `sslaccept=strict`, writes the supplied CA to a
restricted temporary file, and removes the file at exit. The pre-push client
uses equivalent node-postgres TLS settings. Use a certificate-matching instance
DNS name when migration server verification is required. The CA is validated
even for private-IP connections.

The job omits `--accept-data-loss`. If Prisma refuses a destructive diff,
inspect the warnings and current schema, including any pre-push conversions
already applied. Perform an intended destructive step separately from a
management host as the migration identity with a short-lived token, then rerun
the job, or deploy an additive schema first. The job returns the failing
pre-push exit code or, after that succeeds, Prisma's exit code. Inspect job logs
before directing an API revision that needs the schema to this database.

Prisma passes the datasource URL to its schema-engine child as an argument,
so its SQL-login token is visible to processes sharing the container during
the push. Run only the migration processes in that container. This is a
current limitation of the
[migration entrypoint](https://github.com/Rheosoph/flow-like/blob/main/apps/backend/gcp/migration/migrate.ts).
For runner checks from `apps/backend/gcp/migration`, use `bun install`,
`bun test tests/`, and `bun run typecheck`.

## SMTP and readiness checks

The GCP image requires `MAIL_PROVIDER=smtp`. In the complete runtime
configuration, `mail.smtp` names the environment variables containing the relay
host, port, username, and password. Supply those values through Secret Manager
references. Scope the relay credential to sending and rotate it independently.
The public fallback has no SMTP block: mail initialization can log
`SMTP settings required for SMTP provider` and continue without a mail client.
Verify an outbound email before routing traffic.

`/health/live` checks the process. `/health/ready`, `/health/startup`, and the
`/health` compatibility alias require an accepting token lifecycle and a
successful Cloud SQL ping. During token rotation, readiness closes before the
listener drains. SIGTERM closes readiness and drains immediately so Cloud Run's
termination grace can be used for active requests.

Cloud Run publishes one container port. `/metrics` is on the serving router;
exclude it from the public load balancer URL map and restrict service ingress
to the load balancer. Before a first revision receives traffic, verify secret
versions/access, database grants, the current CA, distinct buckets with agreeing
aliases, the Firestore database, Pub/Sub topics, and the complete SMTP-enabled
runtime configuration. Missing topics can fail dispatch even when startup
checks pass.
