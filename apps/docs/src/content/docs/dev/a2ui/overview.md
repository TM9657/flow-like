---
title: A2UI Integration
description: Building agent-driven interfaces with Google's A2UI protocol
sidebar:
  order: 0
  badge:
    text: Coming Soon
    variant: caution
---

:::caution[Coming Soon]
A2UI integration is currently under development. This documentation previews how Flow-Like will leverage A2UI.
:::

## What is A2UI?

**A2UI** (Agent-to-UI) is an open protocol created by **Google** that enables AI agents to generate rich, interactive user interfaces. Instead of text-only responses or risky code execution, A2UI lets agents send declarative component descriptions that clients render using native widgets.

Flow-Like integrates A2UI to bring agent-driven interfaces directly into your workflow automation—creating **Pages** and **Widgets** that connect seamlessly to your flows.

## Human + AI: The Best of Both Worlds

Flow-Like doesn't just consume A2UI from agents—we provide a **visual drag-and-drop builder** that produces the same A2UI format. This means:

```
┌─────────────────────────────────────────────────────────────┐
│              Two Ways to Build UIs                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   🤖 AI-Generated              👤 Human-Created             │
│   ─────────────────           ─────────────────             │
│   Agent creates UI             Visual builder               │
│   from prompts                 drag-and-drop                │
│          │                            │                      │
│          └──────────┬─────────────────┘                     │
│                     ▼                                        │
│              ┌─────────────┐                                │
│              │    A2UI     │                                │
│              │   Format    │                                │
│              └──────┬──────┘                                │
│                     ▼                                        │
│              ┌─────────────┐                                │
│              │  Flow-Like  │                                │
│              │  Renderer   │                                │
│              └─────────────┘                                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

| Approach | Best For | Output |
|----------|----------|--------|
| **AI Agent** | Rapid prototyping, natural language descriptions | A2UI JSON |
| **Visual Builder** | Precise control, pixel-perfect design | A2UI JSON |
| **Combined** | AI generates, human refines | A2UI JSON |

### Why This Matters

- **Interoperable**: AI-generated and human-created UIs use the same format
- **Iterate Freely**: Start with AI, refine manually—or vice versa
- **No Lock-in**: Everything is standard A2UI, portable and future-proof
- **Collaborate**: Designers use the builder, developers use code, AI assists both

:::note[Learn More]
A2UI is Apache 2.0 licensed and developed openly on GitHub.
📚 **Official Docs**: [a2ui.org](https://a2ui.org/)
💻 **Source Code**: [github.com/google/A2UI](https://github.com/google/A2UI)
:::

## Pages and Widgets

```
┌─────────────────────────────────────────────────────────────┐
│                       Your App                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                    PAGES                             │   │
│   │           (App-specific layouts)                     │   │
│   │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│   │  │  Dashboard  │  │   Reports   │  │  Settings   │  │   │
│   │  └──────┬──────┘  └──────┬──────┘  └─────────────┘  │   │
│   └─────────┼────────────────┼──────────────────────────┘   │
│             │                │                               │
│             ▼                ▼                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                   WIDGETS                            │   │
│   │         (Reusable across projects)                   │   │
│   │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │   │
│   │  │KPI Card │ │ Chart   │ │ Table   │ │ Form    │    │   │
│   │  └─────────┘ └─────────┘ └─────────┘ └─────────┘    │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Pages vs Widgets

Flow-Like extends A2UI with two concepts:

| Concept | Scope | Description |
|---------|-------|-------------|
| **Pages** | App-specific | Full-screen layouts configured for your app. Define navigation, structure, and app-specific UI. |
| **Widgets** | Reusable | Self-contained UI components that can be used across your project or shared with other projects. |

## Why A2UI?

A2UI solves a fundamental problem: **how can AI agents safely send rich UIs across trust boundaries?**

### Key Benefits

| Feature | Description |
|---------|-------------|
| **Secure by Design** | Declarative data format, not executable code. Agents can only use pre-approved components. |
| **LLM-Friendly** | Flat, streaming JSON structure designed for easy generation by language models. |
| **Framework-Agnostic** | One agent response works everywhere—React, Angular, Flutter, native mobile. |
| **Progressive Rendering** | Stream UI updates as they're generated. Users see interfaces building in real-time. |

### How It Works

1. **User** sends a message to an AI agent
2. **Agent** generates A2UI messages describing the UI
3. **Messages** stream to the Flow-Like client
4. **Client** renders using native components
5. **User** interacts with the UI, sending actions back
6. **Agent** responds with updated A2UI messages

## A2UI Architecture

### Message Types

A2UI uses four message types:

| Message | Purpose |
|---------|---------|
| \`surfaceUpdate\` | Define or update UI components |
| \`dataModelUpdate\` | Update application state |
| \`beginRendering\` | Signal the client to render |
| \`deleteSurface\` | Remove a UI surface |

### The Adjacency List Model

Unlike nested JSON trees, A2UI uses a **flat adjacency list** where components reference children by ID:

\`\`\`json
{
  "surfaceUpdate": {
    "components": [
      {"id": "root", "component": {"Column": {"children": {"explicitList": ["greeting", "buttons"]}}}},
      {"id": "greeting", "component": {"Text": {"text": {"literalString": "Hello!"}}}},
      {"id": "buttons", "component": {"Row": {"children": {"explicitList": ["cancel", "ok"]}}}},
      {"id": "cancel", "component": {"Button": {"child": "cancel-text", "action": {"name": "cancel"}}}},
      {"id": "cancel-text", "component": {"Text": {"text": {"literalString": "Cancel"}}}},
      {"id": "ok", "component": {"Button": {"child": "ok-text", "action": {"name": "ok"}}}},
      {"id": "ok-text", "component": {"Text": {"text": {"literalString": "OK"}}}}
    ]
  }
}
\`\`\`

**Why flat lists?**
- ✅ Easy for LLMs to generate (no perfect nesting required)
- ✅ Send components incrementally as they're ready
- ✅ Update any component by ID
- ✅ Clear separation of structure and data

### Standard Component Catalog

A2UI defines a standard catalog organized by purpose:

| Category | Components |
|----------|------------|
| **Layout** | Row, Column, List |
| **Display** | Text, Image, Icon, Video, Divider |
| **Interactive** | Button, TextField, CheckBox, DateTimeInput, Slider |
| **Container** | Card, Tabs, Modal |

## Flow-Like Integration

### Connecting to Flows

A2UI components integrate with Flow-Like's execution engine:

\`\`\`
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   A2UI       │ ──▶ │  Flow-Like   │ ──▶ │   A2UI      │
│   Input      │     │   Board      │     │   Output    │
│   Widget     │     │              │     │   Widget    │
└──────────────┘     └──────────────┘     └──────────────┘
\`\`\`

- **Trigger Flows**: Button actions start flow executions
- **Display Results**: Flow outputs bind to component data
- **Bidirectional Binding**: Form inputs sync with flow variables
- **Real-time Updates**: Stream results as flows execute

### Custom Components

Beyond the standard catalog, Flow-Like provides custom components for:

- **Charts & Visualizations**: Data-bound charts connected to flows
- **Flow Controls**: Start, stop, monitor flow executions
- **Data Tables**: Display flow outputs with sorting/filtering
- **File Handling**: Upload/download integrated with storage

## Pages in Your App

Pages define the full-screen layouts for your application:

\`\`\`
┌─────────────────────────────────────────────┐
│  App Navigation                              │
├─────────────────────────────────────────────┤
│                                              │
│  ┌─────────────────────────────────────┐    │
│  │           Page: Dashboard            │    │
│  │  ┌─────────┐  ┌─────────┐           │    │
│  │  │ Widget  │  │ Widget  │           │    │
│  │  │ (KPIs)  │  │ (Chart) │           │    │
│  │  └─────────┘  └─────────┘           │    │
│  │  ┌─────────────────────────────┐    │    │
│  │  │      Widget (Data Table)     │    │    │
│  │  └─────────────────────────────┘    │    │
│  └─────────────────────────────────────┘    │
│                                              │
└─────────────────────────────────────────────┘
\`\`\`

- Configure navigation structure
- Define page layouts
- Set access permissions
- Bind to app-specific data

## Reusable Widgets

Widgets are self-contained components you can share:

\`\`\`
┌──────────────────────────────────────────────────────┐
│  Widget: Customer Card                               │
│  ─────────────────────────────────────────────────── │
│  ┌──────────┐                                        │
│  │  Avatar  │  John Doe                              │
│  │   [👤]   │  john@example.com                      │
│  └──────────┘  Customer since 2023                   │
│                                                       │
│  [View Profile]  [Send Message]                      │
└──────────────────────────────────────────────────────┘
\`\`\`

- **Create Once**: Build the widget in your project
- **Reuse Anywhere**: Use it in any page of your app
- **Share Across Projects**: Export and import widgets
- **Bind to Any Data**: Configure data sources per usage

## Roadmap

| Phase | Features | Status |
|-------|----------|--------|
| **Phase 1** | A2UI renderer, basic components | 🔧 In Development |
| **Phase 2** | Pages & widgets, flow bindings | 📋 Planned |
| **Phase 3** | Widget sharing, custom components | 📋 Planned |
| **Phase 4** | Visual builder, advanced theming | �� Planned |

## Resources

### Official A2UI Documentation

- [A2UI Quickstart](https://a2ui.org/quickstart/) - Get started in 5 minutes
- [Core Concepts](https://a2ui.org/concepts/overview/) - Understand the architecture
- [Component Reference](https://a2ui.org/reference/components/) - Full component catalog
- [A2UI Specification](https://a2ui.org/specification/v0.8-a2ui/) - Protocol details

### Flow-Like Integration

- [Pages Guide](/dev/a2ui/pages/) - Creating app pages *(coming soon)*
- [Widgets Guide](/dev/a2ui/widgets/) - Building reusable widgets *(coming soon)*
- [Flow Bindings](/dev/a2ui/bindings/) - Connecting to workflows *(coming soon)*

:::tip[Get Early Access]
Want to try A2UI integration early?
📧 **info@great-co.de**
:::
