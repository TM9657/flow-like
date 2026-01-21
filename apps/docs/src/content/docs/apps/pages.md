---
title: Pages
description: Create visual interfaces for your apps with Pages
sidebar:
  order: 41
---

Pages are full-screen visual interfaces you can create for your Flow-Like apps. Unlike the chat interface, pages give you complete control over layout and design.

## What are Pages?

A page is a custom screen that displays widgets, text, images, charts, and other content. Each page is connected to a **board** (flow) that provides its data and logic.

```
┌─────────────────────────────────────────────────────────────┐
│                      Your App                               │
├─────────────────────────────────────────────────────────────┤
│  Routes:  / → Dashboard  |  /reports → Reports              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 PAGE: Dashboard                      │   │
│  │                                                      │   │
│  │  ┌────────┐  ┌────────┐  ┌────────┐                │   │
│  │  │ Sales  │  │ Orders │  │ Users  │                │   │
│  │  │ $124k  │  │  847   │  │  234   │                │   │
│  │  └────────┘  └────────┘  └────────┘                │   │
│  │                                                      │   │
│  │  ┌────────────────────────────────────┐            │   │
│  │  │          Sales Chart               │            │   │
│  │  │        📈 Revenue over time        │            │   │
│  │  └────────────────────────────────────┘            │   │
│  │                                                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Pages vs Chat UI

| Feature | Pages | Chat UI |
|---------|-------|---------|
| **Design control** | Full visual customization | Conversational format |
| **Layout** | Grid, columns, freeform | Sequential messages |
| **Best for** | Dashboards, reports, forms | Q&A, conversations |
| **User input** | Forms, buttons, interactions | Text messages |
| **Data display** | Charts, tables, cards | Markdown responses |

## Creating a Page

Pages are created through the **Page Builder**, which you access from your app's settings.

### From the App Settings

1. Open your app in the desktop application
2. Click the **gear icon** (⚙️) to open settings
3. Navigate to **Pages & Routes**
4. Click **Create Page** or open the Page Builder

### From a Flow

You can also create a page directly connected to a specific flow:

1. Open the flow in the Studio
2. Click the **page icon** in the flow settings
3. The new page will be automatically linked to that flow's data

## Page Builder Overview

The Page Builder is a visual drag-and-drop editor:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Page Builder                                         [Preview] 💾  │
├─────────────┬───────────────────────────────────┬───────────────────┤
│             │                                    │                   │
│  COMPONENTS │           CANVAS                   │   PROPERTIES      │
│             │                                    │                   │
│  Layout     │  ┌─────────────────────────┐      │  Selected: Card   │
│  ┌───────┐  │  │  Drop widgets here      │      │                   │
│  │ Row   │  │  │                         │      │  Title:           │
│  └───────┘  │  │   ┌───────┐ ┌───────┐  │      │  [Revenue      ]  │
│  ┌───────┐  │  │   │ Card  │ │ Card  │  │      │                   │
│  │Column │  │  │   └───────┘ └───────┘  │      │  Background:      │
│  └───────┘  │  │                         │      │  [● Default   ▾]  │
│             │  │   ┌───────────────────┐ │      │                   │
│  Display    │  │   │     Chart         │ │      │  Data Binding:    │
│  ┌───────┐  │  │   └───────────────────┘ │      │  [/sales/total ▾] │
│  │ Text  │  │  └─────────────────────────┘      │                   │
│  └───────┘  │                                    │                   │
│  ┌───────┐  │                                    │                   │
│  │ Image │  │                                    │                   │
│  └───────┘  │                                    │                   │
├─────────────┴───────────────────────────────────┴───────────────────┤
│  Layers: [Page] > [Row] > [Card]                       [Undo] [Redo]│
└─────────────────────────────────────────────────────────────────────┘
```

### Main Areas

| Area | Description |
|------|-------------|
| **Components** | Drag these onto your canvas to build the page |
| **Canvas** | Visual preview of your page layout |
| **Properties** | Configure the selected component |
| **Layers** | View and navigate the page structure |

## Adding Content

### Drag and Drop

1. Find a component in the left panel
2. Drag it onto the canvas
3. Drop it where you want it to appear
4. Configure its properties in the right panel

### Available Components

| Category | Components |
|----------|------------|
| **Layout** | Row, Column, Grid, Container |
| **Display** | Text, Image, Icon, Divider |
| **Data** | Table, Chart, Metric Card |
| **Input** | Button, Form, Dropdown, Switch |
| **Advanced** | Code Block, Video, Custom HTML |

## Connecting to Data

Pages become powerful when connected to your flows. Use **data bindings** to display live information.

### Binding to Flow Data

1. Select a component (like a Text or Chart)
2. In the Properties panel, find **Data Binding**
3. Choose a path from your flow's data (e.g., `/sales/total`)
4. The component will display live data when the page runs

### Example Bindings

| Component | Binding | Shows |
|-----------|---------|-------|
| Text | `/user/name` | "John Doe" |
| Metric | `/sales/revenue` | "$124,500" |
| Chart | `/sales/monthly` | Bar chart |
| Table | `/orders/recent` | List of orders |

## Layout Types

Pages support different layout systems:

| Layout | Description | Best For |
|--------|-------------|----------|
| **Freeform** | Position elements anywhere | Landing pages |
| **Stack** | Vertical flow, top to bottom | Articles, forms |
| **Grid** | Rows and columns | Dashboards |
| **Sidebar** | Main area with side panel | Apps with navigation |

## Page Settings

### Appearance

- **Background color** - Set the page background
- **Theme** - Light or dark mode
- **Max width** - Constrain content width
- **Padding** - Space around content

### Metadata

- **Title** - Browser tab title
- **Description** - SEO description
- **Open Graph image** - Social media preview

## Previewing Your Page

Click **Preview** to see how your page will look:

- **Desktop** - Full-width view
- **Tablet** - Medium-width view
- **Mobile** - Narrow mobile view

Responsive components adapt automatically to different screen sizes.

## Tips for Great Pages

:::tip[Start with Layout]
Build your page structure first (rows, columns), then add content. This makes reorganization easier.
:::

:::tip[Use Consistent Spacing]
Keep padding and gaps uniform throughout your page for a professional look.
:::

:::tip[Test with Real Data]
Connect to your actual flows early to see how the page handles real content.
:::

## What's Next?

- **[Routes](/apps/routes/)** - Connect pages to URL paths
- **[Events](/apps/events/)** - Make your pages interactive
- **[Custom UI](/apps/a2ui/)** - Learn about the underlying A2UI format
