---
title: Platform administration
description: Operate the audit trail and manage starter profiles and published home layouts
---

Platform administrators manage shared profile defaults and use audit records
to investigate changes. These features require an online backend, including
when their administration screens are opened in the desktop app.

## Audit trail

Operators can use the audit trail to identify who requested a change, inspect its
result, and check whether the stored records still match their hashes and server
signatures. Platform administrators read the root chain. App owners can read their
app's branch through `/api/v1/audit/entries?chain_id=<app-id>`.

### Coverage and failure behavior

When `audit.enabled` is true, authenticated POST, PUT, PATCH and DELETE requests
record `api.request.attempt` before dispatch and `api.request.finish` when the
handler produces response headers. Both entries share a request ID. The finish
entry records the HTTP status and the number of failed domain audit writes.
App-scoped routes put these records on the app chain. Other routes use the root
chain. Telemetry ingestion is excluded.

The middleware records the matched route template, method and actor. It does not
read request bodies, query strings, credential headers or concrete paths.
Action-specific hooks add resource IDs and bounded metadata for app, permission,
board, file, widget, database, graph and other mutations. A file access grant
records the authorization to upload or read. Provider access or event logs are
needed to establish what happened after a client received a signed URL or scoped
storage credentials.

An attempt write failure returns HTTP 503 before the handler runs. An outcome or
domain audit write failure after dispatch is traced and adds
`x-flow-like-audit-status: incomplete` to the original response. The response
retains the handler's status because a mutation may already have committed.
Operators should investigate attempts without a finish and finishes with a
nonzero `domain_audit_failures`. Domain changes and their audit entries are not
one atomic transaction. A crash can leave an attempt with an unknown outcome.
Execution state updates have the same boundary. Callback retries can repair a
missing terminal entry, but background crashes have no durable outbox recovery.

With `audit.log_executions` enabled, persisted run transitions record starts,
completion, failure, cancellation and timeout on the app chain. Repeated terminal
callbacks reuse the first record for that run and action. Streaming and background
work record their execution outcomes separately from HTTP response status. The
checked-in platform configuration enables execution logging; the configuration
type's default remains false for installations that have not opted in.

Anonymous requests and authentication failures are outside the mutation middleware's
coverage. Inbound and sink execution paths use explicit lifecycle hooks. This
trail does not capture offline desktop edits or every direct storage operation.

### Integrity and compatibility

New entry hashes start with `v2:`. They use a canonical JSON encoding that preserves
field boundaries and covers the record ID, sequence, timestamp, actor identity and
type, optional IP, action, resource, chain scope, summary, details, previous hash,
previous signature and signing key ID. P-256 ECDSA signs the resulting BLAKE3 hash.
Timestamps are normalized to the database's millisecond precision before hashing.

Writers serialize through a retained `MutationLock` row before reading a chain's
tail. This also coordinates the first append and the root chain, whose nullable
`chainId` cannot provide uniqueness by itself. Transaction retries retain the same
record ID so an acknowledged-late commit does not create a duplicate. Upgrade all
API writers together: older writers do not participate in this coordination and
cannot verify the v2 format. No existing audit entries are rewritten.

`GET /api/v1/audit/verify` checks the root chain. Supply `chain_id` to select a
branch and optional positive, inclusive `from` and `to` sequence bounds. The
verifier reads a database snapshot, checks sequence continuity, resolves branch
anchors from the root chain, reconstructs hashes and verifies signatures. The
`entries_checked` and assurance counters include an immediate predecessor or root
anchor when the selected range depends on it.
The dashboard automatically verifies root chains with at most 1,000 entries;
larger chains show "Not checked" and require an operator to request full
verification in the chain explorer.

`valid` means the requested verification checks succeeded. `fully_authenticated`
additionally requires signed v2 entries with available verification keys. Legacy
entries retain their original hash algorithm; its omitted metadata and ambiguous
field boundaries cannot be repaired retroactively. They are counted as legacy
and do not qualify as fully authenticated. Unsigned entries are counted separately.
An unavailable historical public key causes verification to fail with an
`unverifiable_signatures` count. It does not by itself prove that a record changed.

Verification cannot establish events that were never recorded. Detecting deletion
of a whole chain or its final records requires a previously retained checkpoint
outside this database. Retain signed chain heads independently when that evidence
is required. A valid subrange also does not certify all earlier history.

### Signing keys and IP addresses

`BACKEND_KEY` supplies the base64-encoded P-256 PKCS#8 PEM signing key and
`BACKEND_KID` identifies it. With `audit.require_signing` true, API startup refuses
to proceed without a usable signing key. The checked-in configuration requires it.

Before rotating the signing key, retain its public key. Supply historical keys in
the `AUDIT_VERIFYING_KEYS` secret as a JSON object mapping key IDs to P-256 SPKI
PEM public keys. The value uses JSON newlines inside each PEM string:

```json
{
  "previous-key-id": "-----BEGIN PUBLIC KEY-----\n<base64 public key>\n-----END PUBLIC KEY-----\n"
}
```

All API replicas must have the same signing key and ID, and the retained public
keys needed to verify their history. The registry accepts public keys only and
rejects a historical key that conflicts with the active signing key's ID.

`audit.log_ip` is false by default. When enabled, request audit records and
synchronous domain hooks can include a syntactically valid client IP from the
authentication middleware. Forwarded headers must be controlled by the deployment's
trusted proxy; parsing an address does not establish its provenance. Signed IP
fields are immutable. `ip_retention_days` does not currently erase them, so keep
IP recording disabled if automatic IP expiry is required.

### Audit regression checks

Unit tests cover framing, metadata tampering, signature failures, missing sequence
boundaries, legacy compatibility and middleware failure behavior. The PostgreSQL
integration tests in `packages/api/tests/audit_integrity.rs` use a disposable
database to check concurrent appends, retries, signed branches and persisted values.
They must not be pointed at a production database.

## Starter profiles

Administrators can curate the profiles offered to new users at `/admin/profiles` in the web and desktop apps. The old `/admin/user` and `/admin/user/edit` routes open the same manager.

The manager supports searching, sorting, creating, editing, duplicating, and deleting templates. The editor has a live profile preview and sections for the name, description, icon, cover, tags, interests, bits, apps, hubs, and flow connection style. Included apps retain their favorite and pin settings. Editing a template does not rewrite existing user profiles. Unsaved drafts survive navigation within the running app, scoped to the backend, account, and template. Saving or explicitly discarding clears the draft; reloading the app clears this memory cache.

Each saved template links to `/admin/home?default=<template-id>`. Its home follows the main default until an administrator publishes a template default. Duplicating a template copies its presentation and starting configuration, while its new home initially follows the main default. Personal layouts, shortcuts, and private bits are not copied. A published template home must be reset to follow the main default before deleting its template.

Both apps use the shared editor. Desktop supplies a native HTTP upload function; the browser uses Fetch. Image uploads accept PNG, JPEG, and WebP up to 10 MiB and 4096 pixels per edge. Images are converted to WebP when the browser can encode it, with a PNG fallback for other engines, keeping their proportions, with a longest edge of 512 pixels for icons or 1600 pixels for covers. Administrators can also supply an HTTP(S) image URL. Failed uploads keep the previous image.

`WriteProfile` controls template edits and media uploads. `ReadProfile` allows browsing, and `WriteLandingPage` separately controls default homes. These administration operations require a reachable backend and its permissions, including in the desktop app. Desktop users can still use their local profiles without signing in.

Image upload URLs accept `?format=webp`, `png`, or `jpeg` and use the corresponding extension; requests without a format keep the existing WebP behavior. Template writes use the requested ID and return the saved profile. Validation bounds names to 120 characters, descriptions to 10,000 characters, bits and apps to 500 entries each, and tags, interests, and additional hubs to 50 entries each. Stored apps, settings, and theme values survive template round trips. Apply the home-layout migration below before using published defaults.

### Profile editor checks

Run the focused frontend checks with `bun test packages/ui/components/profile-templates`. The responsive browser fixture and verification script live in `tests/home-browser`.

The browser check uses the real list and editor with a local fixture backend. It verifies saved fields, uploads, search, failure recovery, draft restoration, duplication, deletion, and layouts at 390, 768, and 1480 pixels. Run it with `node tests/home-browser/profile-verify.mjs` while the fixture server is running. PNG encoding fallback is simulated in Chrome; this check does not upload to configured storage or run inside the native WebKit view.

## Deploy editable home layouts

Apply the home layout schema before starting an API build that reads the new profile fields. The migration adds nullable `homeLayout` and `homeDefaultId` columns to `Profile`, plus a `HomeDefault` table for published layouts. Existing profile layouts remain unset and follow the published default.

The Aurora DSQL deployment migration job applies `migrations-dsql/20260905103000_profile_home_layouts/migration.sql` through its migration ledger.

For an existing PostgreSQL installation that has not applied the schema change, run this command from `packages/api` with `DATABASE_URL` pointing to that installation:

```sh
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f prisma/migrations/20260905103000_profile_home_layouts/migration.sql
```

Apply the migration once. Fresh PostgreSQL installations can use the existing `db:push` workflow, which reads the updated Prisma schema. If an existing installation uses `db:push` instead of the SQL migration, run the migration's final `UPDATE` separately to associate existing profiles with matching template IDs.

The backfill associates a profile with a template only when their IDs match. Profiles created by copying another profile may have a different ID and no recoverable template reference. New template installations carry an explicit reference, and new copies retain it.

After deployment, an administrator with `WriteLandingPage` or `Admin` permission can publish the main default and optional template defaults. Publishing checks the revision loaded when editing began. A conflicting publication returns HTTP 409 so the editor can retain the draft and ask the administrator to reload.

Resetting a user's home clears only `homeLayout`; `homeDefaultId` remains intact.
See [the home layout guide](/start/home/) for the default selection order and
user controls. Removing a template default restores the main default; removing
the main default restores the bundled layout.

Desktop custom layouts are saved locally and synchronized with the profile. Older clients that omit the home fields in partial API writes leave them unchanged. Default configuration contains widget settings, while each viewer loads app data with their own access permissions.
