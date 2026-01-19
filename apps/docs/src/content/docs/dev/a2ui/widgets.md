---
title: Widgets
description: Building reusable UI components with A2UI
sidebar:
  order: 2
  badge:
    text: Coming Soon
    variant: caution
---

:::caution[Coming Soon]
Widget creation is currently under development. This documentation previews the planned features.
:::

## What are Widgets?

Widgets are **reusable, self-contained UI components** built with A2UI. Unlike pages (which are app-specific), widgets can be:

- Used across multiple pages in your project
- Shared with other projects
- Configured with different data sources per usage

```
┌───────────────────────────────────────────────────────────────┐
│                      Widget: KPI Card                         │
├───────────────────────────────────────────────────────────────┤
│                                                                │
│   ┌─────────┐                                                  │
│   │   📈    │   Revenue                                        │
│   └─────────┘   $124,500                                       │
│                 ▲ 12.5% from last month                        │
│                                                                │
└───────────────────────────────────────────────────────────────┘

Used in:
  ├── Dashboard Page (binds to /revenue)
  ├── Reports Page (binds to /monthly-summary)
  └── Other Project → Widget Library
```

## Widget vs Page

| Aspect | Widget | Page |
|--------|--------|------|
| **Scope** | Reusable anywhere | App-specific |
| **Purpose** | Self-contained component | Full-screen layout |
| **Data** | Configured per usage | Bound to app state |
| **Sharing** | Exportable/importable | Not shareable |

## Widget Structure

A widget defines:

1. **Inputs** - Configurable properties
2. **Components** - A2UI structure
3. **Bindings** - Data connections
4. **Styling** - Visual customization
5. **Actions** - User interaction handlers

### Example Widget Definition

```json
{
  "id": "kpi-card",
  "name": "KPI Card",
  "description": "Displays a key metric with trend indicator",
  "inputs": {
    "title": { "type": "string", "required": true },
    "value": { "type": "binding", "required": true },
    "trend": { "type": "binding" },
    "icon": { "type": "string", "default": "chart" }
  },
  "components": [
    {
      "id": "root",
      "component": {
        "Card": {
          "children": { "explicitList": ["header", "content"] }
        }
      }
    },
    {
      "id": "header",
      "component": {
        "Row": {
          "children": { "explicitList": ["icon", "title"] }
        }
      }
    },
    {
      "id": "icon",
      "component": {
        "Icon": { "name": { "path": "/inputs/icon" } }
      }
    },
    {
      "id": "title",
      "component": {
        "Text": {
          "text": { "path": "/inputs/title" },
          "usageHint": "caption"
        }
      }
    },
    {
      "id": "content",
      "component": {
        "Column": {
          "children": { "explicitList": ["value", "trend"] }
        }
      }
    },
    {
      "id": "value",
      "component": {
        "Text": {
          "text": { "path": "/inputs/value" },
          "usageHint": "h2"
        }
      }
    },
    {
      "id": "trend",
      "component": {
        "Text": {
          "text": { "path": "/inputs/trend" },
          "usageHint": "caption"
        }
      }
    }
  ]
}
```

## Creating Widgets

Flow-Like lets you create widgets with AI, manually, or both:

### Visual Builder

Design widgets interactively with drag-and-drop:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Widget Builder: KPI Card                             [Test Data ▾] │
├─────────────┬───────────────────────────────────┬───────────────────┤
│             │                                    │                   │
│  COMPONENTS │           CANVAS                   │   WIDGET INPUTS   │
│             │                                    │                   │
│  ┌───────┐  │   ┌─────────────────────────┐     │   Define what can │
│  │ Text  │  │   │  ┌────┐                 │     │   be configured:  │
│  └───────┘  │   │  │ 📈 │  Revenue        │     │                   │
│  ┌───────┐  │   │  └────┘  $124,500       │     │   title: string   │
│  │ Icon  │  │   │         ▲ 12.5%         │     │   value: binding  │
│  └───────┘  │   │                         │     │   trend: binding  │
│  ┌───────┐  │   └─────────────────────────┘     │   icon: string    │
│  │ Card  │  │                                    │                   │
│  └───────┘  │                                    │   [+ Add Input]   │
│             │                                    │                   │
├─────────────┴───────────────────────────────────┴───────────────────┤
│  [Cancel]                              [Save Widget] [Export .widget]│
└─────────────────────────────────────────────────────────────────────┘
```

### AI-Generated

Describe the widget to an AI agent:

```
Create a KPI card widget that displays:
- An icon on the left
- A title and large value
- A trend indicator showing percentage change
- Make the trend green if positive, red if negative
```

The agent creates the A2UI structure—open it in the visual builder to refine.

### Hybrid Workflow

The power is in combining both:

| Workflow | Description |
|----------|-------------|
| **AI → Refine** | Agent creates widget, you polish in builder |
| **Build → Enhance** | Start manually, ask AI to add features |
| **Template → Customize** | Pick a starter widget, make it yours |

Everything produces standard A2UI format—fully interchangeable.

### A2UI Standard Components

Build with A2UI's standard catalog:

| Category | Components |
|----------|------------|
| **Layout** | Row, Column, List |
| **Display** | Text, Image, Icon, Video, Divider |
| **Interactive** | Button, TextField, CheckBox, DateTimeInput, Slider |
| **Container** | Card, Tabs, Modal |

## Widget Inputs

Inputs make widgets configurable:

```json
{
  "inputs": {
    "title": {
      "type": "string",
      "required": true,
      "description": "Card title"
    },
    "data": {
      "type": "binding",
      "required": true,
      "description": "Data source path"
    },
    "showTrend": {
      "type": "boolean",
      "default": true
    },
    "variant": {
      "type": "enum",
      "options": ["default", "compact", "large"],
      "default": "default"
    }
  }
}
```

### Input Types

| Type | Description | Example |
|------|-------------|---------|
| `string` | Text value | `"Revenue"` |
| `number` | Numeric value | `42` |
| `boolean` | True/false | `true` |
| `enum` | One of options | `"compact"` |
| `binding` | Data path | `"/metrics/revenue"` |
| `action` | Flow to trigger | `"submit-form"` |

## Using Widgets

### In Pages

Reference widgets by ID:

```json
{
  "id": "revenue-card",
  "widgetRef": "kpi-card",
  "inputs": {
    "title": "Revenue",
    "value": "/metrics/revenue",
    "trend": "/metrics/revenueTrend",
    "icon": "dollar"
  }
}
```

### Multiple Instances

Use the same widget with different data:

```json
{
  "components": [
    {
      "id": "revenue",
      "widgetRef": "kpi-card",
      "inputs": { "title": "Revenue", "value": "/revenue" }
    },
    {
      "id": "orders",
      "widgetRef": "kpi-card",
      "inputs": { "title": "Orders", "value": "/orders" }
    },
    {
      "id": "customers",
      "widgetRef": "kpi-card",
      "inputs": { "title": "Customers", "value": "/customers" }
    }
  ]
}
```

## Sharing Widgets

### Export

Package widgets for sharing:

```
┌─────────────────────────────────────────┐
│  Export Widget                          │
├─────────────────────────────────────────┤
│  Widget: KPI Card                       │
│  Version: 1.0.0                         │
│                                         │
│  ☑ Include styling                      │
│  ☑ Include example data                 │
│  ☐ Include flow dependencies            │
│                                         │
│  [Cancel]              [Export .widget] │
└─────────────────────────────────────────┘
```

### Import

Add widgets from other projects:

```
┌─────────────────────────────────────────┐
│  Import Widget                          │
├─────────────────────────────────────────┤
│  📁 Drop .widget file here              │
│                                         │
│  Or browse from:                        │
│  • Local files                          │
│  • Widget marketplace                   │
│  • Team shared library                  │
└─────────────────────────────────────────┘
```

## Widget Library

Your project maintains a widget library:

```
project/
├── pages/
│   ├── dashboard.json
│   └── reports.json
├── widgets/
│   ├── kpi-card.widget.json
│   ├── data-table.widget.json
│   ├── chart-line.widget.json
│   └── user-avatar.widget.json
└── shared/
    └── imported-widgets/
```

## Common Widget Patterns

### Data Display

```
┌──────────────────────────┐
│  📊 Chart Widget         │
│  ────────────────────    │
│     ╱╲    ╱╲             │
│    ╱  ╲  ╱  ╲  ╱         │
│   ╱    ╲╱    ╲╱          │
└──────────────────────────┘
```

### Form Input

```
┌──────────────────────────┐
│  📝 Contact Form Widget  │
│  ────────────────────    │
│  Name: [____________]    │
│  Email: [___________]    │
│  Message:                │
│  [___________________]   │
│  [Submit]                │
└──────────────────────────┘
```

### List Item

```
┌──────────────────────────┐
│  👤  John Doe            │
│      john@example.com    │
│      [View] [Edit]       │
└──────────────────────────┘
```

## Flow Integration

Widgets can connect to flows:

```json
{
  "actions": {
    "onSubmit": {
      "flow": "process-form",
      "inputs": {
        "data": "/form/values"
      }
    },
    "onRefresh": {
      "flow": "fetch-data",
      "outputs": {
        "result": "/widget/data"
      }
    }
  }
}
```

## Theming

Widgets inherit app theming but can be customized:

```json
{
  "styling": {
    "variants": {
      "default": {
        "background": "var(--card-bg)",
        "borderRadius": "var(--radius-md)"
      },
      "highlighted": {
        "background": "var(--accent-bg)",
        "border": "2px solid var(--accent)"
      }
    }
  }
}
```

:::tip[Get Early Access]
Want to start building widgets early?
📧 **info@great-co.de**
:::
