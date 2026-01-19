---
title: Pages
description: Creating app-specific page layouts with A2UI
sidebar:
  order: 1
  badge:
    text: Coming Soon
    variant: caution
---

:::caution[Coming Soon]
Page creation is currently under development. This documentation previews the planned features.
:::

## What are Pages?

Pages are **app-specific layouts** built with A2UI components. Unlike widgets (which are reusable across projects), pages are configured specifically for your application's needs.

```
┌─────────────────────────────────────────────────────────────┐
│                      Your App                               │
├─────────────────────────────────────────────────────────────┤
│  Navigation: [Dashboard] [Reports] [Settings]               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                    PAGE CONTENT                      │    │
│  │                                                      │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │    │
│  │  │  Widget  │  │  Widget  │  │  Widget  │          │    │
│  │  └──────────┘  └──────────┘  └──────────┘          │    │
│  │                                                      │    │
│  │  ┌──────────────────────────────────────────┐       │    │
│  │  │              Another Widget               │       │    │
│  │  └──────────────────────────────────────────┘       │    │
│  │                                                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Page vs Widget

| Aspect | Page | Widget |
|--------|------|--------|
| **Scope** | App-specific | Reusable anywhere |
| **Purpose** | Full-screen layout | Self-contained component |
| **Navigation** | Has route/URL | Embedded in pages |
| **Sharing** | Not shareable | Exportable/importable |

## Page Structure

Each page defines:

1. **Route** - The URL path for this page
2. **Layout** - How content is arranged
3. **Widgets** - The components displayed
4. **Data Bindings** - Connections to flows and variables
5. **Permissions** - Who can access this page

### Example Page Definition

```json
{
  "id": "dashboard-page",
  "route": "/dashboard",
  "title": "Dashboard",
  "layout": {
    "type": "Column",
    "children": ["header", "metrics-row", "main-content"]
  },
  "widgets": [
    {
      "id": "header",
      "widgetRef": "app-header",
      "props": { "title": "Dashboard" }
    },
    {
      "id": "metrics-row",
      "type": "Row",
      "children": ["metric-revenue", "metric-orders", "metric-customers"]
    },
    {
      "id": "metric-revenue",
      "widgetRef": "kpi-card",
      "bindings": { "value": "/metrics/revenue" }
    }
  ],
  "permissions": {
    "roles": ["admin", "analyst"]
  }
}
```

## Creating Pages

Flow-Like offers two approaches that produce the same A2UI output:

### 1. Visual Builder (Drag & Drop)

The visual builder lets you design pages interactively:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Page Builder                                         [Preview ▾]   │
├─────────────┬───────────────────────────────────┬───────────────────┤
│             │                                    │                   │
│  WIDGETS    │           CANVAS                   │   PROPERTIES      │
│             │                                    │                   │
│  ┌───────┐  │   ┌─────────────────────────┐     │   Widget: Card    │
│  │ Text  │  │   │  ┌─────┐ ┌─────┐       │     │   ───────────────  │
│  └───────┘  │   │  │ KPI │ │ KPI │       │     │   Title: Revenue   │
│  ┌───────┐  │   │  └─────┘ └─────┘       │     │   Binding:         │
│  │ Card  │  │   │  ┌─────────────────┐   │     │   [/metrics/rev ▾] │
│  └───────┘  │   │  │     Chart       │   │     │                    │
│  ┌───────┐  │   │  └─────────────────┘   │     │   FLOW ACTIONS     │
│  │ Chart │  │   │                         │     │   ───────────────  │
│  └───────┘  │   └─────────────────────────┘     │   On Click:        │
│  ┌───────┐  │                                    │   [Select flow ▾]  │
│  │ Table │  │                                    │                    │
│  └───────┘  │                                    │                    │
│             │                                    │                    │
├─────────────┴───────────────────────────────────┴───────────────────┤
│  Layers: [Page] > [Row] > [KPI Card]                   [Save] [Undo]│
└─────────────────────────────────────────────────────────────────────┘
```

1. **Drag** widgets from the palette
2. **Drop** onto the canvas
3. **Configure** properties and bindings
4. **Preview** in real-time
5. **Save** as A2UI format

### 2. AI Agent Generation

Describe what you want in natural language:

```
Create a dashboard page with:
- Header showing "Sales Dashboard"
- Three KPI cards: Revenue, Orders, Customers
- Line chart showing revenue over time
- Table of recent orders
```

The agent generates the A2UI structure—which you can then refine in the visual builder.

### Best of Both Worlds

The magic is that **both approaches produce the same format**:

1. **AI generates** → You **refine in builder**
2. **You build manually** → AI **suggests improvements**
3. **Start from template** → **Customize either way**

### 3. Code-First

Define pages programmatically:

```typescript
const dashboardPage = {
  route: "/dashboard",
  layout: Column({
    children: [
      Header({ title: "Dashboard" }),
      Row({
        children: [
          KPICard({ binding: "/revenue" }),
          KPICard({ binding: "/orders" }),
        ]
      }),
      Chart({ binding: "/revenue-history" })
    ]
  })
};
```

## Page Layouts

### Common Layout Patterns

**Single Column**
```
┌────────────────────────┐
│        Header          │
├────────────────────────┤
│                        │
│        Content         │
│                        │
├────────────────────────┤
│        Footer          │
└────────────────────────┘
```

**Sidebar Layout**
```
┌──────┬─────────────────┐
│      │     Header      │
│ Side ├─────────────────┤
│ bar  │                 │
│      │     Content     │
│      │                 │
└──────┴─────────────────┘
```

**Dashboard Grid**
```
┌────────┬────────┬────────┐
│  KPI   │  KPI   │  KPI   │
├────────┴────────┴────────┤
│          Chart           │
├─────────────┬────────────┤
│   Table     │   List     │
└─────────────┴────────────┘
```

## Page Navigation

Pages integrate with your app's navigation:

```json
{
  "navigation": {
    "items": [
      { "label": "Dashboard", "page": "dashboard", "icon": "home" },
      { "label": "Reports", "page": "reports", "icon": "chart" },
      { "label": "Settings", "page": "settings", "icon": "gear" }
    ]
  }
}
```

## Data Binding

Pages can bind to:

- **Flow outputs** - Results from flow executions
- **App variables** - Global state
- **User context** - Current user info
- **URL parameters** - Route params

```json
{
  "bindings": {
    "user": "/context/user",
    "orders": "/flows/get-orders/output",
    "selectedId": "/route/params/id"
  }
}
```

## Permissions

Control page access:

```json
{
  "permissions": {
    "public": false,
    "roles": ["admin", "manager"],
    "conditions": [
      { "variable": "/user/verified", "equals": true }
    ]
  }
}
```

## Responsive Behavior

Pages adapt to screen sizes:

| Breakpoint | Layout Adjustment |
|------------|-------------------|
| Mobile | Stack columns vertically |
| Tablet | Two-column grid |
| Desktop | Full layout |

:::tip[Get Early Access]
Want to try page creation early?
📧 **info@great-co.de**
:::
