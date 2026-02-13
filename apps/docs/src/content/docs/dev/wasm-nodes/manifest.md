---
title: Package Manifest
description: Complete reference for WASM package manifest files
sidebar:
  order: 2
---

Every WASM package requires a `manifest.toml` file that declares its metadata, permissions, and nodes. This document provides a complete reference.

## Minimal Example

```toml
manifest_version = 1
id = "com.example.hello"
name = "Hello World"
version = "1.0.0"
description = "A simple hello world node"

[[nodes]]
id = "hello"
name = "Say Hello"
description = "Outputs a greeting"
category = "Custom/Examples"
```

## Full Example

```toml
manifest_version = 1
id = "com.example.google-drive"
name = "Google Drive Integration"
version = "2.1.0"
description = "Read and write files to Google Drive"
license = "MIT"
repository = "https://github.com/example/flow-like-gdrive"
homepage = "https://example.com/flow-like-gdrive"
keywords = ["google", "drive", "cloud", "storage"]
min_flow_like_version = "0.5.0"

[[authors]]
name = "Jane Developer"
email = "jane@example.com"
url = "https://jane.dev"

[permissions]
memory = "standard"
timeout = "extended"
variables = true
cache = true
streaming = true
models = false
a2ui = false

[permissions.network]
http_enabled = true
allowed_hosts = ["*.googleapis.com", "accounts.google.com"]
websocket_enabled = false

[permissions.filesystem]
node_storage = true
user_storage = false
upload_dir = true
cache_dir = true

[[permissions.oauth_scopes]]
provider = "google"
scopes = [
  "https://www.googleapis.com/auth/drive.readonly",
  "https://www.googleapis.com/auth/drive.file"
]
reason = "Read and write files to Google Drive"
required = true

[[nodes]]
id = "list_files"
name = "List Drive Files"
description = "List files in a Google Drive folder"
category = "Cloud/Google Drive"
icon = "data:image/svg+xml;base64,..."
oauth_providers = ["google"]

[nodes.metadata]
docs_url = "https://example.com/docs/list-files"

[[nodes]]
id = "download_file"
name = "Download File"
description = "Download a file from Google Drive"
category = "Cloud/Google Drive"
oauth_providers = ["google"]

[[nodes]]
id = "upload_file"
name = "Upload File"
description = "Upload a file to Google Drive"
category = "Cloud/Google Drive"
oauth_providers = ["google"]
```

## Root Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `manifest_version` | integer | ✅ | Always `1` for current version |
| `id` | string | ✅ | Unique package ID (reverse domain style) |
| `name` | string | ✅ | Human-readable package name |
| `version` | string | ✅ | Semantic version (e.g., "1.2.3") |
| `description` | string | ✅ | Brief description of the package |
| `license` | string | | SPDX license identifier |
| `repository` | string | | Source code repository URL |
| `homepage` | string | | Package homepage URL |
| `keywords` | string[] | | Search keywords |
| `min_flow_like_version` | string | | Minimum required Flow-Like version |
| `wasm_path` | string | | Path to WASM file (for local dev) |
| `wasm_hash` | string | | SHA-256 hash for integrity |

## Authors

```toml
[[authors]]
name = "Your Name"
email = "you@example.com"  # optional
url = "https://your.site"   # optional
```

## Permissions

### Resource Tiers

```toml
[permissions]
memory = "standard"   # minimal, light, standard, heavy, intensive
timeout = "standard"  # quick, standard, extended, long_running
```

**Memory Tiers:**

| Tier | Memory | Description |
|------|--------|-------------|
| `minimal` | 16 MB | Simple operations |
| `light` | 32 MB | Basic processing |
| `standard` | 64 MB | Most nodes (default) |
| `heavy` | 128 MB | Data processing |
| `intensive` | 256 MB | ML, large datasets |

**Timeout Tiers:**

| Tier | Duration | Description |
|------|----------|-------------|
| `quick` | 5s | Fast operations |
| `standard` | 30s | Most nodes (default) |
| `extended` | 60s | API calls |
| `long_running` | 5min | ML inference |

### Capability Flags

```toml
[permissions]
variables = true    # Access execution variables
cache = true        # Access execution cache
streaming = true    # Stream output to UI
a2ui = true         # Adaptive UI rendering
models = true       # Access LLM/model providers
```

### Network Permissions

```toml
[permissions.network]
http_enabled = true
allowed_hosts = ["api.example.com", "*.googleapis.com"]
websocket_enabled = false
```

- `allowed_hosts` supports wildcards (`*`)
- Empty `allowed_hosts` with `http_enabled = true` allows all hosts

### Filesystem Permissions

```toml
[permissions.filesystem]
node_storage = true   # Per-node persistent storage
user_storage = false  # Per-user storage
upload_dir = true     # Access uploaded files
cache_dir = true      # Temporary cache storage
```

### OAuth Scopes

```toml
[[permissions.oauth_scopes]]
provider = "google"
scopes = ["https://www.googleapis.com/auth/drive.readonly"]
reason = "Read files from your Google Drive"
required = true
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `provider` | string | ✅ | OAuth provider ID |
| `scopes` | string[] | ✅ | Required OAuth scopes |
| `reason` | string | ✅ | User-facing explanation |
| `required` | boolean | | If false, node works without OAuth |

**Supported Providers:**

- `google` - Google OAuth 2.0
- `github` - GitHub OAuth
- `microsoft` - Microsoft/Azure AD
- `slack` - Slack OAuth
- `discord` - Discord OAuth
- Custom providers via Flow-Like configuration

## Nodes

Each node in the package is declared with a `[[nodes]]` section:

```toml
[[nodes]]
id = "my_node"
name = "My Node"
description = "Does something useful"
category = "Custom/MyCategory"
icon = "data:image/svg+xml;base64,..."  # optional
oauth_providers = ["google"]             # optional

[nodes.metadata]
docs_url = "https://example.com/docs"
custom_key = "custom_value"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | ✅ | Unique identifier within package |
| `name` | string | ✅ | Display name |
| `description` | string | ✅ | Brief description |
| `category` | string | ✅ | Category path (e.g., "Cloud/Storage") |
| `icon` | string | | Base64 data URI or URL |
| `oauth_providers` | string[] | | Which OAuth providers this node uses |
| `metadata` | table | | Additional key-value metadata |

### OAuth Provider References

Nodes can only reference OAuth providers declared at the package level:

```toml
# Package-level declaration
[[permissions.oauth_scopes]]
provider = "google"
scopes = ["..."]
reason = "..."

[[permissions.oauth_scopes]]
provider = "github"
scopes = ["..."]
reason = "..."

# Node references
[[nodes]]
id = "google_only"
oauth_providers = ["google"]  # ✅ Valid

[[nodes]]
id = "both"
oauth_providers = ["google", "github"]  # ✅ Valid

[[nodes]]
id = "invalid"
oauth_providers = ["slack"]  # ❌ Error: not declared at package level
```

## Validation

The manifest is validated when:

1. **Loading** — Package won't load if invalid
2. **Publishing** — Registry rejects invalid manifests

Common validation errors:

| Error | Cause |
|-------|-------|
| `Package ID is required` | Missing `id` field |
| `Package must contain at least one node` | Empty `nodes` array |
| `Node references unknown OAuth provider` | `oauth_providers` not in package `oauth_scopes` |
| `Invalid memory tier` | Unknown value for `memory` |

## Best Practices

### Package IDs

Use reverse domain notation:

```toml
# Good
id = "com.yourcompany.package-name"
id = "io.github.username.package-name"

# Avoid
id = "my-package"
id = "package_v2"
```

### Minimal Permissions

Only request what you need:

```toml
# Good - specific hosts
[permissions.network]
http_enabled = true
allowed_hosts = ["api.openai.com"]

# Avoid - all hosts when not needed
[permissions.network]
http_enabled = true
allowed_hosts = []
```

### Clear OAuth Reasons

Help users understand why you need access:

```toml
# Good
reason = "Read your calendar events to schedule workflows"

# Avoid
reason = "Google access"
```

### Semantic Versioning

Follow semver for predictable updates:

- `1.0.0` → `1.0.1` — Bug fixes
- `1.0.0` → `1.1.0` — New features, backward compatible
- `1.0.0` → `2.0.0` — Breaking changes
