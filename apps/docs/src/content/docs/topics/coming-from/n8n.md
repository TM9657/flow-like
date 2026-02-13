---
title: For n8n Users
description: How n8n workflow concepts translate to Flow-Like
sidebar:
  order: 4
---

Coming from **n8n**? You'll feel right at home—Flow-Like shares the visual workflow paradigm. This guide highlights the key differences and shows you how to leverage Flow-Like's additional capabilities.

## Quick Concept Mapping

| n8n Concept | Flow-Like Equivalent |
|-------------|---------------------|
| Workflow | Board (App) |
| Node | Node |
| Connection | Wire |
| Trigger | Event |
| Webhook | HTTP Event |
| Cron | Scheduled Event |
| Manual Trigger | Quick Action Event |
| Execution | Run |
| Expression | Inline expressions / Get Field |
| Variables | Variables (Board-scoped) |
| Credentials | Secrets |
| Sub-workflow | Call Board |

## Workflows → Boards

Both platforms use visual node-based workflows. The core concepts are nearly identical:

**n8n Workflow:**
```
[Webhook] → [HTTP Request] → [IF] → [Send Email]
                                 ↘ [Slack Message]
```

**Flow-Like Board:**
```
[HTTP Event] ──▶ [HTTP Request] ──▶ [Branch] ──▶ [Send Email]
                                        └──▶ [Slack Message]
```

### Key Differences

| Aspect | n8n | Flow-Like |
|--------|-----|-----------|
| Data format | JSON items | Typed structs |
| Type system | Dynamic | Strongly typed |
| Expressions | `{{ }}` syntax | Inline pins |
| Execution | Web-based | Desktop + Cloud |
| AI | Add-on nodes | Native integration |

## Triggers → Events

| n8n Trigger | Flow-Like Event |
|-------------|-----------------|
| Manual Trigger | Quick Action Event |
| Webhook | HTTP Event |
| Cron/Schedule | Scheduled Event |
| On App Event | Chat Event |
| When Called by Another Workflow | Quick Action (called from other Board) |

### Webhook Example

**n8n:**
```
[Webhook]
├── HTTP Method: POST
├── Path: /process-order
└── Response Mode: Response Node
```

**Flow-Like:**
```
[HTTP Event]
├── Method: POST
├── Path: /process-order
└── Outputs: request_body, headers
        │
        ▼
[Process] ──▶ [HTTP Response]
```

### Scheduled Execution

**n8n:**
```
[Schedule Trigger]
├── Cron Expression: 0 9 * * *
└── Timezone: UTC
```

**Flow-Like:**
```
[Scheduled Event]
├── Every: Day
├── At: 09:00
└── Timezone: UTC
```

## Data & Expressions

### n8n Expressions

n8n uses `{{ }}` expressions:
```javascript
{{ $json.customer.name }}
{{ $json.items[0].price * $json.items[0].quantity }}
{{ $now.toFormat('yyyy-MM-dd') }}
```

### Flow-Like Equivalents

Data access is done via nodes:
```
Get Field (data, "customer.name") ──▶ name

Get Field (item, "price") ──┐
                            ├──▶ Multiply ──▶ line_total
Get Field (item, "quantity") ─┘

Get Current Time ──▶ Format Date ──▶ formatted_date
```

Or inline for simple cases:
```
┌─────────────────┐
│ Template String │
│ "Hello {name}"  ├◀── name
│                 │
└────────┬────────┘
         │
         ▼
    "Hello Alice"
```

### Item Lists

n8n processes items in arrays automatically. Flow-Like uses explicit loops:

**n8n:**
```
[Webhook] → [HTTP Request] → [Send Email]
            (returns 5 items)  (sends 5 emails)
```

**Flow-Like:**
```
[HTTP Event] ──▶ [HTTP Request] ──▶ [For Each] ──▶ [Send Email]
                  (returns array)     │
                                      └──▶ (done)
```

This gives you more control over how items are processed.

## Common Nodes Mapped

### Data Transformation

| n8n Node | Flow-Like Node |
|----------|----------------|
| Set | Set Variable / Create Struct |
| Function | Expression nodes / Custom WASM |
| Function Item | For Each + Transform |
| Merge | Merge Arrays / Join |
| Split In Batches | Chunk Array |
| Remove Duplicates | Deduplicate |
| Sort | Sort Array |
| Limit | Take / Skip |
| Aggregate | Reduce / SQL Aggregate |
| Filter | Filter Array / Branch in loop |
| Item Lists | For Each |

### Conditionals

| n8n Node | Flow-Like Node |
|----------|----------------|
| IF | Branch |
| Switch | Multiple Branches |
| Compare Datasets | Compare / SQL Join |

### HTTP & API

| n8n Node | Flow-Like Node |
|----------|----------------|
| HTTP Request | HTTP Request |
| Webhook | HTTP Event |
| Respond to Webhook | HTTP Response |
| GraphQL | HTTP Request (POST with query) |

### Files

| n8n Node | Flow-Like Node |
|----------|----------------|
| Read Binary File | Read to Binary |
| Write Binary File | Write Binary |
| Read/Write Files | Read to String / Write String |
| Spreadsheet File | Buffered CSV Reader |
| PDF | Read to String (PDF parse) |
| Extract from File | Various Read nodes |

### Database

| n8n Node | Flow-Like Nodes |
|----------|-----------------|
| Postgres | Register PostgreSQL + SQL Query |
| MySQL | Register MySQL + SQL Query |
| MongoDB | (via HTTP API) |
| Redis | (via HTTP API) |
| SQL Node | DataFusion SQL Query |

### Communication

| n8n Node | Flow-Like Node |
|----------|----------------|
| Send Email | SMTP Email node |
| Slack | HTTP Request (Slack API) |
| Discord | HTTP Request (Discord API) |
| Telegram | HTTP Request (Telegram API) |

### AI & LLM

| n8n Node | Flow-Like Node |
|----------|----------------|
| OpenAI | Invoke LLM (OpenAI provider) |
| Anthropic | Invoke LLM (Anthropic provider) |
| AI Agent | Make Agent + Run Agent |
| AI Memory | Variables (chat_history array) |
| Vector Store | LanceDB + Vector Search |
| Embeddings | Embed Document/Query |

## Credentials → Secrets

**n8n:** Store credentials in the UI, reference by name.

**Flow-Like:** Use Secrets management or environment variables:
```
Get Secret ("OPENAI_API_KEY") ──▶ api_key
```

## Sub-workflows → Board Calls

**n8n:**
```
[Execute Workflow]
├── Workflow: "Process Order"
├── Mode: Run once for each item
└── Wait for sub-workflow to finish: true
```

**Flow-Like:**
```
Board: ProcessOrder
└── Quick Action Event (order)
        │
        ▼
    [Process logic]
        │
        ▼
    [Return result]

Board: Main
└── [For Each order] ──▶ [Call Board: ProcessOrder] ──▶ [Collect]
```

## Error Handling

**n8n:**
```
[Try/Catch]
├── Try: [Risky Node]
└── Catch: [Error Handler]
```

**Flow-Like:**
```
[Try] ──▶ [Risky Node] ──▶ [Continue]
  │
  └──▶ [Catch] ──▶ [Error Handler]
```

### Retry Logic

**n8n:**
```
Settings → Retry On Fail → Max Tries: 3
```

**Flow-Like:**
```
[Retry]
├── Max Attempts: 3
├── Delay: 1000ms
└── Backoff: Exponential
    │
    ▼
[HTTP Request] ──▶ [Continue]
```

## Variables & State

### Workflow Variables (n8n)

n8n has environment variables and static data.

### Board Variables (Flow-Like)

Flow-Like has typed, scoped variables:
```
Variables Panel:
├── counter: Integer = 0
├── results: Array<Result> = []
├── config: Config = { timeout: 30 }
└── is_processing: Boolean = false
```

Use `Get Variable` and `Set Variable` nodes to read/write.

## Looping

### For Each (n8n)

Items flow through automatically in n8n.

### For Each (Flow-Like)

Explicit loop control:
```
[Array Input] ──▶ [For Each] ──▶ [Process Item]
                      │              │
                      │          [Continue]
                      │
                      └──(done)──▶ [Next Step]
```

**Breaking early:**
```
[For Each] ──▶ [Branch: item.valid?]
                    │
               True │ False
                    ▼    │
              [Process] [Break]
```

## What Flow-Like Adds

### Desktop App

Run automations locally with a native UI:
- Quick Actions as buttons
- Chat interfaces
- Custom A2UI pages

### AI-Native

Built-in AI without external services:
- Local models (Ollama)
- RAG with vector search
- AI agents with tools
- Structured extraction

### Data Science

Beyond basic transformations:
- SQL across any data source (DataFusion)
- ML models (clustering, classification)
- Rich visualizations (charts, tables)
- Statistical analysis

### Strong Typing

Catch errors before runtime:
- Typed connections
- Schema validation
- Compile-time checks

### Versioning

Built-in version control:
- Every save is a checkpoint
- Restore any previous version
- Compare changes

## Migration Examples

### Example 1: API Polling

**n8n:**
```
[Cron] → [HTTP Request] → [IF status changed] → [Slack]
                              ↓
                         [NoOp]
```

**Flow-Like:**
```
[Scheduled Event: every 5 min]
    │
    ▼
[HTTP Request: GET /api/status]
    │
    ▼
[Get Variable: last_status]
    │
    ▼
[Branch: status ≠ last_status]
    │
   True ──▶ [Set Variable: last_status]
    │               │
    │               ▼
    │        [HTTP Request: Slack webhook]
    │
   False ──▶ (done)
```

### Example 2: Form Processing

**n8n:**
```
[Webhook] → [Airtable: Create] → [Send Email]
               ↓
          [Slack: Post]
```

**Flow-Like:**
```
[HTTP Event: POST /submit]
    │
    ├──▶ [HTTP Request: Airtable API]
    │
    └──▶ [HTTP Request: Slack API]
              │
              ▼
         [SMTP Email]
```

### Example 3: AI Chatbot

**n8n:**
```
[Webhook] → [OpenAI] → [Respond to Webhook]
               ↓
    [Get from Memory] [Store in Memory]
```

**Flow-Like:**
```
[Chat Event: user_message]
    │
    ├──▶ [Get Variable: chat_history]
    │
    ▼
[Build Messages: system + history + user_message]
    │
    ▼
[Invoke LLM: GPT-4]
    │
    ├──▶ [Append to Variable: chat_history]
    │
    └──▶ [Response to user]
```

## FAQ

### Can I import n8n workflows?
Not directly, but the node patterns are similar. Rebuild visually—it usually goes fast.

### Is there a cloud option?
Yes—deploy to Docker/Kubernetes backends, or run on desktop.

### What about n8n's community nodes?
Flow-Like has built-in integrations. For missing ones, use HTTP Request or create WASM nodes.

### Is it free?
Flow-Like is open source. Check the licensing for commercial use.

### Can I self-host?
Yes—full self-hosting support with Docker and Kubernetes.

## Feature Comparison

| Feature | n8n | Flow-Like |
|---------|-----|-----------|
| Visual workflow | ✅ | ✅ |
| Web UI | ✅ | ✅ (embedded) |
| Desktop app | ❌ | ✅ |
| Cloud execution | ✅ | ✅ |
| Self-hosted | ✅ | ✅ |
| AI/LLM | Via nodes | Native |
| Vector search | Via Pinecone, etc. | Built-in (LanceDB) |
| ML models | ❌ | ✅ |
| SQL engine | Basic nodes | Full DataFusion |
| Charts | ❌ | ✅ |
| Custom UI | ❌ | ✅ (A2UI) |
| Type safety | Loose | Strong |
| Open source | ✅ | ✅ |

## Next Steps

- **[Studio Overview](/studio/overview/)** – Learn the Flow-Like IDE
- **[Events](/apps/events/)** – Setting up triggers
- **[GenAI](/topics/genai/overview/)** – AI capabilities
- **[Data Science](/topics/datascience/overview/)** – Analytics features
