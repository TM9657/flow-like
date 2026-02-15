---
title: Building Internal Tools
description: Create dashboards, forms, and admin panels with Flow-Like's A2UI system
sidebar:
  order: 1
---

Flow-Like's **A2UI (Agent-to-UI)** system lets you build rich internal tools—dashboards, admin panels, forms, and data viewers—without writing frontend code. Design visually, connect to your workflows, and deploy instantly.

## What You Can Build

| Tool Type | Use Cases |
|-----------|-----------|
| **Dashboards** | KPI displays, real-time metrics, system status |
| **Admin Panels** | User management, content moderation, settings |
| **Data Viewers** | Search interfaces, record browsers, log viewers |
| **Forms** | Data entry, surveys, approval workflows |
| **Reports** | Scheduled reports, export tools, analytics |
| **Control Centers** | Trigger workflows, manage processes, monitor jobs |

## Core Concepts

### Pages & Routing

Every app can have multiple **Pages**, each with a unique route:

```
App: Customer Portal
├── /dashboard      → Overview Page
├── /customers      → Customer List
├── /customers/:id  → Customer Detail
├── /reports        → Reports Page
└── /settings       → Settings Page
```

Navigate between pages programmatically or via Link components.

### Components

A2UI provides 50+ components for building interfaces:

#### Layout Components
| Component | Purpose |
|-----------|---------|
| **Row** | Horizontal flex container |
| **Column** | Vertical flex container |
| **Grid** | CSS Grid layout |
| **Card** | Content container with borders |
| **Tabs** | Tabbed navigation |
| **Accordion** | Collapsible sections |
| **Modal** | Popup dialogs |
| **Drawer** | Side panels |

#### Data Display
| Component | Purpose |
|-----------|---------|
| **Table** | Full-featured data tables with sorting, filtering, pagination |
| **NivoChart** | 25+ chart types (bar, line, pie, heatmap, etc.) |
| **PlotlyChart** | Advanced scientific charts |
| **Text** | Typography (headings, body, labels) |
| **Badge** | Status indicators |
| **Progress** | Progress bars |
| **Avatar** | User images |
| **Markdown** | Rich text display |

#### Form Inputs
| Component | Purpose |
|-----------|---------|
| **TextField** | Text input (text, email, password, number) |
| **Select** | Dropdown selection |
| **Checkbox** | Boolean toggle |
| **Switch** | Toggle switch |
| **RadioGroup** | Single selection from options |
| **Slider** | Range selection |
| **DateTimeInput** | Date/time picker |
| **FileInput** | File upload |

#### Interactive
| Component | Purpose |
|-----------|---------|
| **Button** | Clickable actions |
| **Link** | Navigation links |
| **Tooltip** | Hover information |
| **Popover** | Click-triggered info |

### Data Binding

Components connect to data through **bindings**:

```
Table Component
├── data ◀── Variable: customers
├── columns ◀── [name, email, status, actions]
└── onRowClick ──▶ Navigate to /customers/{id}
```

**Binding Types:**
- **Literal** – Static values: `"Hello World"`
- **Variable** – Dynamic: `$customers`
- **Path** – Nested access: `$customer.orders[0].total`
- **Template** – Interpolation: `"Welcome, {$user.name}!"`

### Actions

Components trigger workflows through **Actions**:

| Action Type | Purpose |
|-------------|---------|
| `invoke` | Run a workflow (Quick Action) |
| `navigate` | Go to another page |
| `updateData` | Update a variable |
| `openModal` | Show a dialog |
| `closeModal` | Hide a dialog |

```
Button: "Submit Order"
├── onClick: invoke → ProcessOrder workflow
│   └── payload: { customer_id, items, total }
└── Loading state while workflow runs
```

## Building a Dashboard

### Step 1: Create the Page

1. Open your App in the Studio
2. Navigate to **Pages**
3. Click **Add Page**
4. Set route: `/dashboard`
5. Choose layout: **Grid** (2 columns)

### Step 2: Add KPI Cards

Drag **Card** components for each metric:

```
┌─────────────────┐  ┌─────────────────┐
│  Total Revenue  │  │  Active Users   │
│     $45,230     │  │      1,234      │
│   ↑ 12% MTD     │  │   ↑ 5% today    │
└─────────────────┘  └─────────────────┘
```

**Card Configuration:**
```
Card: Revenue
├── Text (h2): "Total Revenue"
├── Text (h1): {$metrics.revenue}
├── Text (caption): "↑ {$metrics.revenue_change}% MTD"
└── Style: bg-green-50
```

### Step 3: Add Charts

Add a **NivoChart** for trends:

```
NivoChart
├── type: "line"
├── data: {$salesTrend}
├── colors: ["#3b82f6", "#10b981"]
├── enableGridX: false
└── legends: bottom
```

### Step 4: Add Data Table

Add a **Table** for recent activity:

```
Table: Recent Orders
├── data: {$recentOrders}
├── columns:
│   ├── id (sortable)
│   ├── customer
│   ├── amount (format: currency)
│   ├── status (badge)
│   └── actions (buttons)
├── pagination: true
├── pageSize: 10
└── onRowClick: navigate → /orders/{id}
```

### Step 5: Connect Data

Create a workflow to fetch dashboard data:

```
Board: DashboardData
└── Init Event (runs on page load)
        │
        ├──▶ SQL Query: Get Metrics
        │       │
        │       ▼
        │   Set Variable: metrics
        │
        ├──▶ SQL Query: Get Sales Trend
        │       │
        │       ▼
        │   Set Variable: salesTrend
        │
        └──▶ SQL Query: Get Recent Orders
                │
                ▼
            Set Variable: recentOrders
```

## Building a Form

### Step 1: Create Form Layout

```
Column (gap: 16px)
├── Text (h2): "Create Customer"
├── TextField: name (required)
├── TextField: email (type: email, required)
├── Select: tier (options: Free, Pro, Enterprise)
├── Checkbox: newsletter
├── Row
│   ├── Button: "Cancel" (variant: outline)
│   └── Button: "Create" (variant: default)
└── Text: {$error} (color: red, hidden if empty)
```

### Step 2: Handle Submission

**Button: Create**
```
onClick: invoke → CreateCustomer
├── payload:
│   ├── name: {$form.name}
│   ├── email: {$form.email}
│   ├── tier: {$form.tier}
│   └── newsletter: {$form.newsletter}
└── onSuccess: navigate → /customers/{result.id}
```

**Workflow: CreateCustomer**
```
Quick Action Event (name, email, tier, newsletter)
    │
    ▼
Validate Email Format
    │
    ├── Invalid ──▶ Set error variable ──▶ Return
    │
    ▼
SQL Insert: customers table
    │
    ▼
Return: { id, success: true }
```

### Step 3: Add Validation

Client-side validation via component properties:

```
TextField: email
├── type: "email"
├── required: true
├── placeholder: "user@example.com"
├── error: {$emailError}
└── onChange: validate email format
```

Server-side validation in the workflow before database insert.

## Table Features

The Table component is powerful for data-heavy tools:

### Sorting
```
Table
├── columns:
│   ├── name (sortable: true)
│   ├── created (sortable: true, default: desc)
│   └── status
└── onSort: refetch with new order
```

### Filtering
```
Row (above table)
├── TextField: search (onDebounce: filter)
├── Select: status filter
└── DateTimeInput: date range

Table
├── data: {$filteredData}
└── columns: ...
```

### Actions Column
```
Column: Actions
└── Row
    ├── Button (icon: edit) → openModal: EditDialog
    ├── Button (icon: trash) → invoke: DeleteRecord
    └── Button (icon: eye) → navigate: /records/{id}
```

### Export
```
Button: "Export CSV"
├── onClick: invoke → ExportData
└── Downloads CSV file
```

## Chart Types

A2UI includes 25+ chart types via Nivo:

| Chart | Best For |
|-------|----------|
| `bar` | Categorical comparisons |
| `line` | Trends over time |
| `pie` | Part-to-whole |
| `radar` | Multi-variable comparison |
| `heatmap` | 2D data density |
| `scatter` | Correlation |
| `funnel` | Conversion flows |
| `treemap` | Hierarchical data |
| `sankey` | Flow diagrams |
| `calendar` | Date-based heatmaps |
| `bullet` | Progress vs targets |
| `radialBar` | Circular progress |

### Example: Sales by Region

```
NivoChart
├── type: "bar"
├── data: [
│   { region: "North", sales: 45000 },
│   { region: "South", sales: 32000 },
│   { region: "East", sales: 28000 },
│   { region: "West", sales: 51000 }
│ ]
├── keys: ["sales"]
├── indexBy: "region"
├── colors: { scheme: "blues" }
└── legends: [{ position: "bottom" }]
```

## State Management

### Page State
Local to the current page, resets on navigation:
```
Set Page State (key: "filterValue", value: "active")
Get Page State (key: "filterValue") ──▶ "active"
```

### Global State
Persists across pages (stored in IndexedDB):
```
Set Global State (key: "user", value: { id, name, role })
Get Global State (key: "user") ──▶ { id, name, role }
```

### Variables
Board-level state for workflow data:
```
Variables:
├── customers: Array<Customer>
├── selectedCustomer: Customer | null
├── isLoading: Boolean
└── error: String | null
```

## Responsive Design

A2UI uses Tailwind CSS classes for responsive layouts:

```
Grid
├── columns: 1 (mobile)
├── md:columns: 2 (tablet)
├── lg:columns: 3 (desktop)
└── gap: 16px
```

**Breakpoints:**
- `sm`: 640px
- `md`: 768px
- `lg`: 1024px
- `xl`: 1280px
- `2xl`: 1536px

## Reusable Widgets

Create reusable UI components as **Widgets**:

```
Widget: CustomerCard
├── Props:
│   ├── customer: Customer
│   └── onEdit: Action
├── Content:
│   └── Card with customer info
└── Actions:
    └── Edit button triggers onEdit
```

Use in pages:
```
WidgetInstance
├── widgetId: "CustomerCard"
├── props: { customer: {$selectedCustomer} }
└── onEdit: openModal → EditCustomerModal
```

## Example: Admin Panel

Complete admin panel structure:

```
App: Admin Panel
├── /
│   └── Dashboard (metrics, charts, recent activity)
├── /users
│   ├── Table of users
│   ├── Search/filter bar
│   └── Actions: Edit, Suspend, Delete
├── /users/:id
│   ├── User details
│   ├── Activity log
│   └── Edit form
├── /content
│   ├── Content list
│   └── Moderation queue
├── /settings
│   ├── App settings form
│   └── API keys management
└── Layout:
    ├── Sidebar (navigation)
    ├── Header (user menu, notifications)
    └── Main content area
```

## Best Practices

### 1. Loading States
Always show loading indicators:
```
{$isLoading ? Spinner : Table}
```

### 2. Error Handling
Display errors clearly:
```
{$error && Alert (variant: destructive): $error}
```

### 3. Empty States
Handle empty data gracefully:
```
{$data.length === 0 ? EmptyState : Table}
```

### 4. Confirmation Dialogs
Confirm destructive actions:
```
Button: Delete
├── onClick: openModal → ConfirmDelete
└── Modal confirms then invokes DeleteRecord
```

### 5. Keyboard Navigation
Use proper tab order and focus management for accessibility.

## Next Steps

- **[Data Visualization](/topics/datascience/visualization/)** – Deep dive into charts
- **[Events](/apps/events/)** – Trigger workflows from UI
- **[DataFusion](/topics/datascience/datafusion/)** – Query data for dashboards
- **[API Integrations](/topics/api-integrations/overview/)** – Connect external data
