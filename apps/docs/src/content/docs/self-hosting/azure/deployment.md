---
title: Azure deployment
description: Configure the Azure API, PostgreSQL identities, schema job, email, and telemetry collector.
sidebar:
  order: 10
---

The Azure backend runs in Container Apps and uses user-assigned managed
identities for PostgreSQL and Azure services. Prepare the database roles,
runtime configuration, and secret access before directing traffic to a revision.
Use separate identities for the API, migrations, and each
[worker workload](/self-hosting/azure/workers/).

## Build and configure the API

Build from the repository root. The shared image requires no installation
configuration or cloud credential at build time:

```sh
docker buildx build \
  --platform linux/amd64 \
  --load --tag flow-like-azure-api:local \
  -f apps/backend/azure/api/Dockerfile .
```

Supply a complete installation configuration with `provider: "azure"`, your
Entra/OAuth settings, and the organization's feature and tier configuration.
Select exactly one nonempty runtime source:

| Variable | Source |
| --- | --- |
| `FLOW_LIKE_CONFIG_JSON` | Complete JSON, optionally injected from a Container Apps secret |
| `FLOW_LIKE_CONFIG_FILE` | Readable JSON file mounted read-only |
| `FLOW_LIKE_CONFIG_SECRET_REF` | Key Vault key or qualified reference such as `secret://azure-key-vault/HUB-CONFIG` |

The selected document replaces the embedded public fallback and is read once
at startup. Invalid or conflicting sources stop startup; a change requires a
restart or new revision. The managed identity must already be able to read a
referenced secret. OAuth client secrets belong in separate secret-store
entries referenced by `client_secret_env`; literal client secrets are rejected.
The public fallback does not configure an Azure installation.

The [Entra External ID fragment](https://github.com/Rheosoph/flow-like/blob/main/apps/backend/azure/api/entra-external-id.fragment.example.json)
shows Azure-specific settings. Replace its example tenant, application, and
URLs, check discovery, issuer, audience, and JWKS, then merge it into the complete
configuration. The fragment alone fails schema validation. For shared source
handling and optional custom compiled defaults, see
[runtime API configuration](/self-hosting/containers/#runtime-api-configuration).
A compiled default remains recoverable from the binary even when supplied as
a BuildKit secret; it must contain no credentials.

## PostgreSQL identity and lifecycle

Configure the API with:

```text
AZURE_CLIENT_ID=<API managed identity client UUID>
AZURE_POSTGRES_AUTH_MODE=managed_identity
AZURE_POSTGRES_HOST=<server>.postgres.database.azure.com
AZURE_POSTGRES_DATABASE=flow_like
AZURE_POSTGRES_USER=<API managed identity name>
```

The database user is the identity's display/resource name, such as
`flowlike-dev-api-identity`. It is distinct from both the client UUID and the
principal/object UUID. Container Apps supplies `IDENTITY_ENDPOINT` and
`IDENTITY_HEADER`; the endpoint must be HTTP on loopback. The process rejects
alternate identity endpoints, authority overrides, and proxy variables to keep
that header on the local identity path.

The API obtains an Entra token for
`https://ossrdbms-aad.database.windows.net/.default` and uses it in SQL connection
options with TLS `verify-full`. The pool cannot replace the token for newly
opened connections, so readiness closes five to eight minutes before expiry
and the process exits after draining. Keep the app restartable and use at least
two replicas when availability must survive token rotation. Per-process jitter
staggers rotation across replicas.

### Bootstrap database roles

Azure RBAC grants do not create PostgreSQL roles. As the configured Flexible
Server Entra administrator, connect to `postgres` and create the API principal
by object ID. This avoids ambiguous display names:

```sql
select * from pg_catalog.pgaadauth_create_principal_with_oid(
  '<API managed identity name>',
  '<API managed identity principal UUID>',
  'service',
  false,
  false
);
```

Create the separate migration principal the same way, using its own name and
principal UUID, and grant it the schema ownership/DDL rights required to apply
the schema. Then connect to `flow_like` and grant runtime DML rights to the API.
Replace both quoted role names below with the deployment's actual names:

```sql
grant connect on database flow_like to "<API managed identity name>";
grant usage on schema public to "<API managed identity name>";
grant select, insert, update, delete on all tables in schema public
  to "<API managed identity name>";
grant usage, select, update on all sequences in schema public
  to "<API managed identity name>";

alter default privileges for role "<migration managed identity name>"
  in schema public grant select, insert, update, delete on tables
  to "<API managed identity name>";
alter default privileges for role "<migration managed identity name>"
  in schema public grant usage, select, update on sequences
  to "<API managed identity name>";
```

Run the existing-table grants after initial schema creation. Default privileges
cover future objects created by the named migration identity. Keep DDL, role
administration, and database ownership off the API identity.

### Rejected database configuration

The API's database client and migration job reject the following settings when
present, including empty values:

```text
DATABASE_URL, PGPASSWORD, POSTGRES_PASSWORD, AZURE_POSTGRES_PASSWORD,
AZURE_POSTGRES_CONNECTION_STRING, AZURE_POSTGRESQL_CONNECTIONSTRING,
PGHOST, PGHOSTADDR, PGPORT, PGUSER, PGDATABASE, PGSSLMODE, PGSSLROOTCERT,
PGSSLCERT, PGSSLKEY, PGPASSFILE, PGSERVICE, PGSERVICEFILE, PGOPTIONS, PGAPPNAME,
MSI_ENDPOINT, MSI_SECRET, IMDS_ENDPOINT, IDENTITY_SERVER_THUMBPRINT,
AZURE_AUTHORITY_HOST, HTTP_PROXY, HTTPS_PROXY, ALL_PROXY
```

Lowercase proxy names are also rejected. Unset an unwanted setting rather than
assigning an empty value. The API additionally rejects storage keys, SAS tokens,
client secrets, ACS connection strings, emulator switches, signature-skip flags,
and custom storage endpoints. The exact names are maintained in the
[API configuration guard](https://github.com/Rheosoph/flow-like/blob/main/apps/backend/azure/api/src/config.rs)
and [PostgreSQL client](https://github.com/Rheosoph/flow-like/blob/main/packages/azure-data/src/postgres.rs).

## Apply schema changes

Build the migration image from the repository root:

```sh
docker build -f apps/backend/azure/migration/Dockerfile -t flow-like-azure-migration .
```

Run it as a manually triggered Container Apps Job with the same PostgreSQL
settings as the API, replacing `AZURE_CLIENT_ID` and `AZURE_POSTGRES_USER` with
the migration identity. After the role bootstrap, run this job before an API
revision that needs the changed schema:

```sh
az containerapp job start --name <migration-job> --resource-group <resource-group>
```

The image builds and validates a PostgreSQL mirror of the tracked CockroachDB
Prisma schema. At runtime it acquires an Entra login token, runs the guarded
pre-push column conversions, and then runs `prisma db push`. Connection URLs
exist in the child processes' environment. Prisma receives
`sslmode=require&sslaccept=strict`; the pre-push node-postgres client receives
`uselibpqcompat=true&sslmode=verify-full`. Both verify the server certificate.

The job omits `--accept-data-loss`. If Prisma refuses a destructive change,
inspect its warnings and the current schema, including any conversions already
applied by the pre-push step. Apply an intended destructive step separately as
the migration identity with a short-lived token, then rerun the job; an additive
schema change can also avoid the destructive step.

Exit `2` indicates rejected configuration, and exit `1` can indicate token
failure or a signaled child. Otherwise the job returns the failing pre-push
exit code, or Prisma's exit code after pre-push succeeds. Cancellation forwards
SIGTERM to the child. The
[migration entrypoint](https://github.com/Rheosoph/flow-like/blob/main/apps/backend/azure/migration/migrate.ts)
defines this sequence. For local runner checks, use `bun install`, `bun test`,
and `bunx tsc --noEmit` from `apps/backend/azure/migration`.

## Email through Azure Communication Services

Set the following on the API and grant its identity the scoped communications
sender role:

```text
MAIL_PROVIDER=azure_communication_services
ACS_EMAIL_ENDPOINT=https://<resource>.communication.azure.com
ACS_EMAIL_SENDER=DoNotReply@<azure-managed-domain>.azurecomm.net
```

Use the actual verified sender exposed by your communications resource. The
endpoint and sender are identifiers. Authentication uses the API's managed
identity; access keys, connection strings, and `AZURE_CLIENT_SECRET` are
rejected. The sender role needs
`Microsoft.Communication/CommunicationServices/Read` and
`Microsoft.Communication/CommunicationServices/Write`; keep local authentication
disabled on the ACS resource. The client disables engagement tracking and waits
for the send operation to succeed before reporting success.

## Health and telemetry

`/health/live` checks the API process. `/health/ready` and `/health/startup`
require an accepting token lifecycle and a successful PostgreSQL ping.
`/health` aliases readiness for deployments using the compatibility probe.

The [native OTLP collector](https://github.com/Rheosoph/flow-like/blob/main/apps/backend/azure/otel-collector/collector.yaml)
receives OTLP/gRPC on port 4317 and exports logs, traces, and metrics to Azure
Monitor. Set `AZURE_CLIENT_ID`, `DEPLOYMENT_ENVIRONMENT`,
`AZURE_MONITOR_TRACES_ENDPOINT`, `AZURE_MONITOR_METRICS_ENDPOINT`, and
`AZURE_MONITOR_LOGS_ENDPOINT` on the collector. Configure the endpoints for the
deployment's private Data Collection Endpoint and grant the collector identity
`Monitoring Metrics Publisher` scoped to its Data Collection Rule.

Expose 4317 only as internal TCP ingress. Port 13133 is the health extension
for Container Apps probes and needs no ingress. Collector logs go to stdout;
retain them in the environment's Log Analytics workspace to diagnose exporter
and identity failures. The queue is memory-only, so a process loss can discard
queued telemetry. Verify ingestion, retries, alerts, and replica failover before
depending on this pipeline for operations.

Build the collector from the repository root so the image includes its license:

```sh
docker buildx build --platform linux/amd64 \
  --tag "$ACR_LOGIN_SERVER/flowlike/otel-collector:$VERSION" \
  --file apps/backend/azure/otel-collector/Dockerfile --push .
```

Promote a scanned and signed immutable registry digest. When updating the
collector base, update its pinned digest and version comment together.
