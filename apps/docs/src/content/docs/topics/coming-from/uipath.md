---
title: For UiPath Developers
description: How UiPath RPA concepts translate to Flow-Like
sidebar:
  order: 1
---

Coming from **UiPath**? This guide maps familiar RPA concepts to their Flow-Like equivalents. You'll find that many concepts transfer directly, while Flow-Like adds powerful AI and data capabilities on top.

## Quick Concept Mapping

| UiPath Concept | Flow-Like Equivalent |
|----------------|---------------------|
| Studio | Studio |
| Activity | Node |
| Sequence | Flow (linear execution) |
| Flowchart | Flow (with branches) |
| Arguments | Pins (inputs/outputs) |
| Variables | Variables |
| Orchestrator | Execution Backends |
| Process | App/Board |
| Package | Package |
| Queue | Events |
| Attended Robot | Desktop App |
| Unattended Robot | Backend Executor |

## Core Concepts Compared

### Activities → Nodes

In UiPath, **Activities** are the building blocks of automation. In Flow-Like, these are called **Nodes**.

**UiPath Activity:**
```
[Assign]
  To: customerName
  Value: "John Doe"
```

**Flow-Like Node:**
```
┌─────────────────┐
│   Set Variable  │
│   customerName  │◀── "John Doe"
└─────────────────┘
```

**Key differences:**
- Flow-Like nodes are more granular
- Nodes connect via typed **Pins** instead of arguments
- Data flows visually through wires

### Sequences → Flows

UiPath **Sequences** execute activities top-to-bottom. Flow-Like **Flows** work similarly but with visual connections.

**UiPath Sequence:**
```
Sequence
├── Read CSV
├── For Each Row
│   ├── Process Data
│   └── Log Message
└── Write CSV
```

**Flow-Like Flow:**
```
Read CSV ──▶ For Each ──▶ Process ──▶ Log
                │
                └──────────────────────▶ Write CSV
```

The execution wire (white) shows the order explicitly.

### Flowcharts → Flows with Branches

UiPath **Flowcharts** allow decision-based routing. In Flow-Like, use **Branch** nodes:

**UiPath Flowchart:**
```
[Start] → [Decision: amount > 1000?]
              │
        Yes ──┼── No
              │
[Approve]  [Auto-Process]
```

**Flow-Like:**
```
Start ──▶ Branch (amount > 1000)
              │
         True │ False
              ▼
        ┌─────┴─────┐
    Approve    Auto-Process
```

### Arguments → Pins

UiPath uses **In/Out/InOut Arguments** to pass data. Flow-Like uses **Pins**:

| UiPath Argument | Flow-Like Pin |
|-----------------|---------------|
| In | Input Pin (left side) |
| Out | Output Pin (right side) |
| InOut | Not directly—use variables |

**Connecting data:**
```
┌──────────────┐          ┌──────────────┐
│  Read CSV    │          │  Process     │
│              ├─ data ──▶├─ input       │
└──────────────┘          └──────────────┘
```

### Variables → Variables

Both platforms have variables, but Flow-Like's are typed and scoped to Boards:

| UiPath Variable | Flow-Like Variable |
|-----------------|-------------------|
| String | String |
| Int32 | Integer |
| DataTable | CSVTable / Database |
| Array | Array (typed) |
| Dictionary | Struct |
| GenericValue | Dynamic (avoid) |

**Creating variables:**
1. Open the **Variables** panel in your Board
2. Click **Add Variable**
3. Set name, type, and default value

### Orchestrator → Execution Backends

UiPath **Orchestrator** manages robot deployment. Flow-Like has **Execution Backends**:

| UiPath Feature | Flow-Like Equivalent |
|----------------|---------------------|
| Orchestrator | Kubernetes/Docker backends |
| Tenant | Organization |
| Folder | App |
| Process | Board |
| Job | Run |
| Queue | Event Queue |
| Asset | Secrets/Variables |
| Schedule | Scheduled Events |

**Deployment options:**
- **Desktop** – Like attended robots
- **Docker Compose** – Self-hosted backend
- **Kubernetes** – Scalable cloud deployment

### Attended vs Unattended

| UiPath | Flow-Like |
|--------|-----------|
| Attended Robot | Desktop App (local execution) |
| Unattended Robot | Backend Executor (remote) |

Flow-Like apps can run:
- **Locally** – On the desktop, user-triggered
- **Remotely** – On backend infrastructure
- **Hybrid** – Mix of both

## Common Activities Mapped

### Data Manipulation

| UiPath Activity | Flow-Like Node |
|-----------------|----------------|
| Assign | Set Variable |
| Build Data Table | Create Database + Insert |
| Add Data Row | Insert to Database |
| Filter Data Table | SQL Filter / DataFusion Query |
| For Each Row | For Each / Loop Rows |
| Read CSV | Buffered CSV Reader |
| Write CSV | Write String (formatted) |
| Read Range (Excel) | Get Row / Loop Rows |
| Write Range (Excel) | Write Cell / Insert Row |

### File Operations

| UiPath Activity | Flow-Like Node |
|-----------------|----------------|
| Read Text File | Read to String |
| Write Text File | Write String |
| Copy File | Copy |
| Move File | Copy + Delete |
| Delete | Delete |
| Path Exists | Exists |
| Get Files | List Paths |

### Control Flow

| UiPath Activity | Flow-Like Node |
|-----------------|----------------|
| If | Branch |
| Switch | Multiple Branches |
| While | While Loop |
| Do While | While Loop (check at end) |
| For Each | For Each |
| Break | Break |
| Continue | Continue |
| Try Catch | Try / Catch nodes |
| Throw | Error node |
| Retry Scope | Retry node |

### Web & API

| UiPath Activity | Flow-Like Node |
|-----------------|----------------|
| HTTP Request | HTTP Request |
| Deserialize JSON | Parse JSON |
| Serialize JSON | Stringify |
| SOAP Request | HTTP Request (raw) |

### Database

| UiPath Activity | Flow-Like Node |
|-----------------|----------------|
| Connect | Register PostgreSQL/MySQL |
| Execute Query | SQL Query |
| Execute Non Query | Execute SQL |
| Disconnect | (automatic) |

## What Flow-Like Adds

### Built-in AI Capabilities

Unlike UiPath, Flow-Like has native AI:

| Capability | Flow-Like Nodes |
|------------|-----------------|
| LLM Chat | Invoke LLM, Chat Event |
| Document AI | Extract Knowledge |
| Embeddings | Embed Document/Query |
| Vector Search | Vector Search, Hybrid Search |
| AI Agents | Make Agent, Agent Tools |
| ML Models | Decision Trees, KMeans, etc. |

### Visual Data Analytics

Query any data source with SQL:

```
Create DataFusion Session
    │
    ▼
Mount CSV + Register PostgreSQL
    │
    ▼
SQL Query: "SELECT * FROM local_csv
            JOIN remote_db ON ..."
```

### Modern Integrations

| Integration | Support |
|-------------|---------|
| S3, Azure, GCS | Native |
| Delta Lake, Iceberg | Native |
| GitHub, Notion | Native |
| REST APIs | Full HTTP client |

## Migration Tips

### 1. Start with Simple Processes
Begin by migrating straightforward sequences before tackling complex flowcharts.

### 2. Rethink Data Tables
Flow-Like uses typed databases instead of generic DataTables. Consider:
- **CSVTable** for tabular data
- **LanceDB** for persistent storage
- **DataFusion** for SQL queries

### 3. Embrace the Type System
Flow-Like is strongly typed. Plan your data structures:
```
Struct: Customer
├── id: String
├── name: String
├── orders: Array<Order>
└── created: DateTime
```

### 4. Use Events Instead of Queues
UiPath Queues become Flow-Like Events:
- **Quick Action** – Manual trigger
- **Chat Event** – Conversational trigger
- **Scheduled** – Time-based trigger

### 5. Leverage AI
Where you'd use Document Understanding or AI Center, use Flow-Like's native AI nodes—they're simpler and more integrated.

## Example Migration

### UiPath Process: Invoice Processing

**Original UiPath:**
```
Main.xaml
├── Read PDF
├── Extract Invoice Data (Document Understanding)
├── For Each Line Item
│   ├── Validate
│   └── Add to DataTable
├── Insert to Database
└── Send Email Confirmation
```

**Flow-Like Equivalent:**
```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  Quick Action Event (receives PDF)                      │
│       │                                                 │
│       ▼                                                 │
│  Read PDF to String                                     │
│       │                                                 │
│       ▼                                                 │
│  Extract Knowledge (LLM)                                │
│     Schema: {vendor, date, total, line_items: [...]}   │
│       │                                                 │
│       ▼                                                 │
│  For Each: line_items                                   │
│       │                                                 │
│       ├──▶ Validate Item                               │
│       │                                                 │
│       └──▶ Insert to Database                          │
│                 │                                       │
│                 ▼                                       │
│           Send Email                                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Key improvements:**
- AI extraction is built-in (no separate AI Center)
- Structured output with schema validation
- Native database integration
- Simpler deployment

## FAQ

### Can I use my UiPath packages?
No, but most common activities have Flow-Like equivalents. For specialized activities, you can create custom nodes via WASM.

### Is there an Orchestrator equivalent?
Yes—Flow-Like's backend infrastructure (Docker/Kubernetes) provides similar capabilities. See [Self-Hosting](/self-hosting/overview/).

### How do I handle Attended scenarios?
Use the Desktop App. Users can trigger flows via:
- Quick Actions (button clicks)
- Chat Events (conversational)
- Custom UI (A2UI pages)

### Can I schedule processes?
Yes, configure scheduled events for your flows. See [Events](/apps/events/).

### What about version control?
Flow-Like has built-in versioning. Every save creates a checkpoint you can restore.

## Next Steps

- **[Studio Overview](/studio/overview/)** – Learn the Flow-Like IDE
- **[Working with Nodes](/studio/nodes/)** – Deep dive into nodes
- **[Events](/apps/events/)** – Set up triggers
- **[GenAI](/topics/genai/overview/)** – Explore AI capabilities
