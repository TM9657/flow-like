---
title: Widgets
description: Reusable UI components for your Flow-Like apps
sidebar:
  order: 43
---

Widgets are reusable UI building blocks that you can use across pages and apps. Build them once, use them everywhere.

## What are Widgets?

A widget is a self-contained UI component with:

- **Visual design** - Layout and appearance
- **Customization options** - Properties you can configure per-use
- **Data bindings** - Connections to your flow data

```
┌─────────────────────────────────────────────────────────────┐
│              Widget: Metric Card                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────┐                                               │
│   │   📈    │   Revenue                                     │
│   └─────────┘   $124,500                                    │
│                 ▲ 12.5% from last month                     │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  Customization Options:                                     │
│  • Title → "Revenue"                                        │
│  • Icon → "chart"                                           │
│  • Value binding → /sales/total                             │
│  • Trend binding → /sales/trend                             │
└─────────────────────────────────────────────────────────────┘
```

## Widgets vs Components

| Concept | Description | Scope |
|---------|-------------|-------|
| **Components** | Basic building blocks (Text, Button) | Built-in |
| **Widgets** | Composed from components | Your organization |

Think of components as LEGO bricks and widgets as the custom structures you build with them.

## Using Widgets

### In the Page Builder

1. Open a page in the Page Builder
2. Look in the **Components** panel
3. Find **Widgets** section (or search)
4. Drag the widget onto your canvas
5. Configure its properties

### Customizing Per-Instance

Each widget instance can have different values:

```
Page: Dashboard
├── Metric Card Widget
│   └── Title: "Revenue" / Value: /sales/revenue
├── Metric Card Widget
│   └── Title: "Orders" / Value: /orders/count
└── Metric Card Widget
    └── Title: "Users" / Value: /users/active
```

Same widget, three different configurations.

## Creating Widgets

### The Widget Builder

Flow-Like provides a visual Widget Builder for creating your own widgets.

```
┌─────────────────────────────────────────────────────────────────────┐
│  Widget Builder: KPI Card                             [Test] [Save] │
├─────────────┬───────────────────────────────────┬───────────────────┤
│             │                                    │                   │
│  COMPONENTS │           CANVAS                   │   CUSTOMIZATIONS  │
│             │                                    │                   │
│  ┌───────┐  │   ┌─────────────────────────┐     │   Title           │
│  │ Text  │  │   │  ┌──────┐               │     │   ☑ Customizable  │
│  └───────┘  │   │  │ Icon │  Title        │     │   Default: "KPI"  │
│  ┌───────┐  │   │  └──────┘               │     │                   │
│  │ Icon  │  │   │  ┌──────────────────┐   │     │   Icon            │
│  └───────┘  │   │  │    Value         │   │     │   ☑ Customizable  │
│  ┌───────┐  │   │  │    $124,500      │   │     │   Default: "chart"│
│  │ Card  │  │   │  └──────────────────┘   │     │                   │
│  └───────┘  │   │  Trend: ▲ 12.5%        │     │   Value           │
│             │   └─────────────────────────┘     │   ☑ Customizable  │
│             │                                    │   Binding path... │
├─────────────┴───────────────────────────────────┴───────────────────┤
│  Test Data: { title: "Revenue", value: 124500, trend: 12.5 }        │
└─────────────────────────────────────────────────────────────────────┘
```

### Steps to Create a Widget

1. **Open Widget Builder** from Studio > Widgets
2. **Add components** to build your layout
3. **Mark customization options** for configurable properties
4. **Define data bindings** for dynamic content
5. **Test with sample data** to verify behavior
6. **Save and publish** your widget

### Customization Options

When building a widget, you can mark certain properties as customizable:

| Option Type | Description | Example |
|-------------|-------------|---------|
| **Text** | Editable text | Titles, labels |
| **Number** | Numeric value | Sizes, limits |
| **Color** | Color picker | Accent colors |
| **Boolean** | On/off toggle | Show/hide elements |
| **Binding** | Data path | Dynamic content |
| **Select** | Dropdown options | Variants, sizes |

### Exposed Properties

For advanced widgets, you can expose component properties to allow deeper customization:

```json
{
  "widget": "metric-card",
  "customizations": {
    "title": "Revenue",
    "showTrend": true
  },
  "exposedProps": {
    "card.borderRadius": "8px",
    "icon.color": "#3b82f6"
  }
}
```

## Widget Library

### Built-in Widgets

Flow-Like includes common widgets out of the box:

| Widget | Description |
|--------|-------------|
| **Metric Card** | Display a single metric with trend |
| **Data Table** | Sortable, filterable table |
| **Line Chart** | Time series visualization |
| **Bar Chart** | Categorical comparisons |
| **Form Card** | Input form in a card |
| **Navigation Bar** | App navigation menu |
| **Footer** | Page footer with links |

### Organization Widgets

Create widgets for your organization:

- **Branded components** - Company colors and styles
- **Common patterns** - Frequently used layouts
- **Team templates** - Shared starting points

## Widget Versioning

Widgets support versioning for safe updates:

| Version | Status | Description |
|---------|--------|-------------|
| v1.0.0 | Stable | Original release |
| v1.1.0 | Stable | Added trend indicator |
| v2.0.0 | Latest | New layout, breaking changes |

Pages can pin to a specific version or follow the latest.

### Version Selection

When using a widget:

1. Choose the widget
2. Select version:
   - **Latest** - Always use newest
   - **Stable** - Use latest stable
   - **Specific** - Pin to exact version

## Best Practices

### Design for Reuse

:::tip[Single Purpose]
Each widget should do one thing well. A "Metric Card" displays a metric—don't add navigation to it.
:::

### Sensible Defaults

Provide good default values so widgets look good immediately:

- Default title: "Metric" (not empty)
- Default color: Theme primary (not hard-coded)
- Default size: Medium (not tiny or huge)

### Document Your Widgets

Add descriptions to help others understand:

- **Widget description** - What it does
- **Option descriptions** - What each customization controls
- **Usage examples** - Common configurations

### Test Edge Cases

Test your widgets with:

- Empty data
- Very long text
- Very short text
- Many items
- No items
- Different screen sizes

## What's Next?

- **[Pages](/apps/pages/)** - Use widgets in pages
- **[Custom UI](/apps/a2ui/)** - Learn about the underlying format
- **[Widget Builder Guide](/reference/widget-builder/)** - Detailed builder reference
