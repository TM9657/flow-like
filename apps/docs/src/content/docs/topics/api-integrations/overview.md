---
title: API Integrations
description: Connect to REST APIs, webhooks, and 300+ built-in platform integrations
sidebar:
  order: 1
---

Flow-Like provides comprehensive API integration capabilities—from raw HTTP requests to 300+ pre-built platform connectors for GitHub, Slack, Notion, Microsoft 365, Google Workspace, and more.

## HTTP Requests

### Make Any API Call

```
Make Request
├── URL: "https://api.example.com/users"
├── Method: GET
├── Headers: {"Authorization": "Bearer {token}"}
└── Body: (optional)
    │
    ▼
API Call (Fetch)
    │
    ▼
Response
├── Status: 200
├── Headers: {...}
└── Body: [{user data}]
```

### Request Building

Build requests step by step:

```
Make Request
    │
    ▼
Set URL ("https://api.example.com/orders")
    │
    ▼
Set Method (POST)
    │
    ▼
Set Header ("Content-Type", "application/json")
    │
    ▼
Set Bearer Auth (api_key)
    │
    ▼
Set Struct Body ({ customer_id: 123, items: [...] })
    │
    ▼
API Call ──▶ Response
```

### Response Handling

Parse responses based on content type:

```
API Call Response
    │
    ├──▶ To Struct (JSON) ──▶ Typed data object
    ├──▶ To Text ──▶ Raw string
    ├──▶ To Bytes ──▶ Binary data
    │
    ├──▶ Get Status ──▶ 200
    ├──▶ Is Success ──▶ true
    └──▶ Get Header ("Content-Type") ──▶ "application/json"
```

### Streaming Responses

Handle large or streaming responses:

```
Streaming API Call
├── URL: "https://api.openai.com/v1/chat/completions"
├── stream: true
└── Handler: process each chunk
```

### File Downloads

Download files from URLs:

```
HTTP Download
├── URL: "https://example.com/report.pdf"
└── Path: /downloads/report.pdf
    │
    ▼
File saved to path
```

## Authentication Methods

### Bearer Token

```
Set Bearer Auth (token)
    │
    ▼
Header: "Authorization: Bearer {token}"
```

### API Key in Header

```
Set Header ("X-API-Key", api_key)
```

### Basic Auth

```
Set Header ("Authorization", "Basic " + base64(user:pass))
```

### OAuth 2.0

Pre-built OAuth flows for major platforms:

```
Google Provider (OAuth)
├── Scopes: ["drive.readonly", "gmail.send"]
└── Credentials from Secrets
    │
    ▼
Authenticated requests to Google APIs
```

## Built-in Integrations

### GitHub

Full GitHub API coverage with 35+ nodes:

| Category | Nodes |
|----------|-------|
| **Repos** | List, Get, Search, Clone, Fork |
| **Issues** | List, Get, Create, Update, Search, Comment |
| **Pull Requests** | List, Get, Create, Merge, Review, Files |
| **Files** | Get Contents, Create/Update, Delete, Download |
| **Branches** | List, Get, Create, Delete |
| **Commits** | List, Get, Compare |
| **Actions** | List Workflows, Trigger, Get Runs, Cancel |
| **Releases** | List, Get, Create |

**Example: Create issue on error**
```
Try
    │
    ▼
Your workflow
    │
    └── Catch
            │
            ▼
        GitHub Create Issue
        ├── repo: "myorg/myrepo"
        ├── title: "Workflow Error: {error_type}"
        └── body: "Error details: {error_message}"
```

### Microsoft 365

50+ nodes for Microsoft Graph API:

| Service | Capabilities |
|---------|--------------|
| **OneDrive** | List, Upload, Download, Share, Copy, Move |
| **SharePoint** | Sites, Libraries, Files, Folders |
| **Outlook** | Send, List, Read, Reply, Forward, Folders |
| **To Do** | Lists, Tasks, Complete |
| **Planner** | Plans, Buckets, Tasks |
| **Teams** | (via Graph API) |

**Example: Save report to SharePoint**
```
Generate Report (data)
    │
    ▼
Microsoft Provider (OAuth)
    │
    ▼
SharePoint Upload File
├── site_id: "contoso.sharepoint.com"
├── library: "Documents"
├── path: "/Reports/monthly.pdf"
└── content: report_pdf
```

### Google Workspace

30+ nodes for Google APIs:

| Service | Capabilities |
|---------|--------------|
| **Drive** | List, Upload, Download, Delete, Share |
| **Gmail** | Send, Draft, List Labels |
| **Sheets** | Read, Write, Append |
| **Slides** | Create, Add Slides, Export |
| **Meet** | Create, Get Meetings |
| **Calendar** | Events management |

**Example: Upload to Drive and share**
```
Google Provider (OAuth)
    │
    ▼
Drive Upload File
├── file: report.pdf
├── folder_id: "abc123"
└── name: "Q4 Report.pdf"
    │
    ▼
Get file_id
    │
    ▼
Drive Share
├── file_id: file_id
├── email: "team@company.com"
└── role: "reader"
```

### Notion

Full Notion API integration:

| Feature | Nodes |
|---------|-------|
| **Databases** | List, Get, Query |
| **Pages** | Get, Create, Update |
| **Search** | Search across workspace |

**Example: Sync data to Notion**
```
SQL Query (get_customers)
    │
    ▼
For Each customer
    │
    ▼
Notion Create Page
├── database_id: "customers_db"
└── properties:
    ├── Name: customer.name
    ├── Email: customer.email
    └── Status: customer.status
```

### Atlassian (Jira & Confluence)

60+ nodes for Atlassian products:

**Jira:**
| Category | Nodes |
|----------|-------|
| **Issues** | Create, Get, Update, Delete, Search, Batch |
| **Comments** | Add, List |
| **Transitions** | Get, Execute |
| **Sprints** | List, Create, Start, Complete |
| **Boards** | List, Get, Config |
| **Attachments** | Get, Download, Delete |

**Confluence:**
| Category | Nodes |
|----------|-------|
| **Spaces** | List |
| **Pages** | Get, Create, Update, Delete |
| **Search** | Search content |
| **Comments** | Get, Add |
| **Labels** | Get, Add, Remove |

**Example: Auto-create Jira issues from form**
```
HTTP Event (POST /submit-bug)
    │
    ▼
Jira Create Issue
├── project: "BUGS"
├── type: "Bug"
├── summary: request.title
├── description: request.description
└── priority: request.priority
    │
    ▼
Return { issue_key: "BUGS-123" }
```

### Databricks

20+ nodes for data platform integration:

| Category | Nodes |
|----------|-------|
| **Clusters** | List, Get, Start, Stop |
| **Jobs** | List, Get, Run, Status |
| **SQL** | Execute queries |
| **DBFS** | List, Read, Write |
| **Unity Catalog** | Catalogs, Schemas, Tables |

### Discord & Telegram

Messaging platform integrations:

**Discord (25+ nodes):**
- Send/edit/delete messages
- Channel management
- DMs, reactions, polls
- Interactive components (buttons, menus)

**Telegram (60+ nodes):**
- Messages, media, files
- Chats, forums, topics
- Payments, games
- Inline mode, commands

**Example: Discord notification bot**
```
Scheduled Event (every hour)
    │
    ▼
Check system metrics
    │
    ├── Alert condition? ──▶ Discord Send Message
    │                        ├── channel_id: "alerts"
    │                        └── content: "⚠️ High CPU: {value}%"
    │
    └── Normal ──▶ Done
```

### LinkedIn

Post updates and share content:

```
LinkedIn Provider (OAuth)
    │
    ▼
Share Text Post
├── text: "Excited to announce our Q4 results!"
└── visibility: "PUBLIC"
```

## Webhooks (HTTP Events)

Receive incoming webhooks:

```
HTTP Event
├── Method: POST
├── Path: /webhooks/stripe
└── Outputs: body, headers
    │
    ▼
Verify Signature (headers.stripe-signature)
    │
    ▼
Branch: event.type
├── "payment_succeeded" ──▶ Process payment
├── "subscription_created" ──▶ Provision access
└── "invoice.paid" ──▶ Send receipt
```

## Email (SMTP/IMAP)

### Send Emails

```
SMTP Connection
├── host: "smtp.example.com"
├── port: 587
├── username: {from secrets}
└── password: {from secrets}
    │
    ▼
Send Mail
├── to: "customer@example.com"
├── subject: "Your order has shipped"
├── body: email_content
└── attachments: [tracking_pdf]
```

### Read Emails

```
IMAP Connection
├── host: "imap.example.com"
└── credentials: {from secrets}
    │
    ▼
List Messages (folder: "INBOX", unread: true)
    │
    ▼
For Each message
    │
    ├── Extract data
    └── Process
```

## AI Provider APIs

Pre-built integrations for 18+ AI providers:

| Provider | Capabilities |
|----------|--------------|
| OpenAI | GPT-4, Embeddings, Vision |
| Anthropic | Claude models |
| Google | Gemini models |
| Groq | Fast inference |
| Ollama | Local models |
| Cohere | Embeddings, Rerank |
| Mistral | Open models |
| Together AI | Open models |
| Perplexity | Search + AI |
| HuggingFace | Model hub |
| VoyageAI | Embeddings |
| xAI | Grok models |

## MCP (Model Context Protocol)

Connect to MCP servers for AI tools:

```
MCP Local Server
├── command: "npx"
├── args: ["-y", "@modelcontextprotocol/server-filesystem"]
└── env: { "ROOT_PATH": "/data" }
```

```
MCP HTTP Server
├── url: "https://mcp.example.com"
└── auth: Bearer token
```

## DataFusion External Sources

Query external databases via SQL:

```
Create DataFusion Session
    │
    ▼
Register PostgreSQL ("prod_db", connection_string)
    │
    ▼
Register MySQL ("legacy_db", connection_string)
    │
    ▼
SQL Query:
"SELECT p.*, l.customer_name
 FROM prod_db.orders p
 JOIN legacy_db.customers l ON p.customer_id = l.id"
```

**Supported sources:**
- PostgreSQL, MySQL, SQLite
- ClickHouse, DuckDB, Oracle
- Delta Lake, Iceberg
- S3, Azure Blob, GCS

## Patterns & Best Practices

### 1. Centralize Authentication

Create a dedicated board for each service:

```
Board: GitHubService
├── Variables:
│   └── provider: GitHubProvider (initialized once)
│
└── Quick Actions:
    ├── CreateIssue (title, body)
    ├── GetPullRequests (repo)
    └── TriggerWorkflow (workflow_id)
```

### 2. Handle Rate Limits

```
Retry
├── max_attempts: 3
├── delay: 1000ms
├── backoff: exponential
└── retry_on: [429, 503]
    │
    ▼
API Call
```

### 3. Validate Responses

```
API Call
    │
    ▼
Is Success?
├── Yes ──▶ To Struct ──▶ Process
└── No ──▶ Get Status ──▶ Handle error
```

### 4. Use Webhooks Over Polling

Instead of:
```
❌ Every 5 minutes: Check for new orders
```

Use:
```
✅ HTTP Event: Receive order webhook
```

### 5. Store Credentials Securely

```
Get Secret ("GITHUB_PAT")
    │
    ▼
GitHub Provider (token)
```

Never hardcode tokens in workflows.

### 6. Log API Calls

```
Console Log: "Calling {api_name} with {params}"
    │
    ▼
API Call
    │
    ▼
Console Log: "Response: {status} in {duration}ms"
```

## Example: Multi-Platform Sync

Sync data across platforms:

```
HTTP Event (POST /customer-created)
    │
    ├──▶ Notion Create Page (customers database)
    │
    ├──▶ Jira Create Issue (onboarding task)
    │
    ├──▶ Slack Send Message (sales channel)
    │
    └──▶ HubSpot Create Contact (CRM)
```

## Example: Automated Reporting

Generate and distribute reports:

```
Scheduled Event (Monday 9am)
    │
    ▼
DataFusion Query (weekly metrics)
    │
    ▼
Generate Charts (Nivo)
    │
    ▼
Render Template (report.md)
    │
    ▼
Convert to PDF
    │
    ├──▶ SharePoint Upload
    │
    ├──▶ Slack Post (with PDF)
    │
    └──▶ Email to stakeholders
```

## FAQ

### How do I add a new API?
Use the HTTP Request nodes for any REST API. Build request, set auth, call, parse response.

### Can I use GraphQL?
Yes—use HTTP Request with POST method, set body to your GraphQL query.

### What about SOAP APIs?
Use HTTP Request with appropriate XML body and headers.

### How do I handle pagination?
Loop with While node, updating page/cursor until no more results.

### Can I cache API responses?
Store results in Variables or Database, check before calling API.

## Next Steps

- **[Building Chatbots](/topics/chatbots/overview/)** – Chat integrations
- **[Data Pipelines](/topics/data-pipelines/overview/)** – ETL with APIs
- **[Building Internal Tools](/topics/internal-tools/overview/)** – UIs for API data
- **[GenAI](/topics/genai/overview/)** – AI API integrations
