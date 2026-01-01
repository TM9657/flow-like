---
title: Custom UI
description: Build Rich User Interfaces with AI or by Hand
sidebar:
  order: 46
---

Flow-Like supports [A2UI (Agent-to-User Interface)](https://a2ui.org/), an open protocol created by Google that enables rich, structured user interfaces to be generated dynamically.

## What is A2UI?

A2UI is a JSON-based format for describing user interfaces. It defines **pages** (full-screen layouts) and **widgets** (reusable components) that can be rendered into beautiful, interactive UIs.

The key insight: the same JSON format that AI agents produce can also be created by humans using a visual builder.

## Two Ways to Build

### 1. Visual Drag-and-Drop Builder

Flow-Like provides a visual interface builder where you can:

- **Drag and drop** widgets onto a canvas
- **Configure** each widget's properties visually
- **Preview** your interface in real-time
- **Connect** to your Flows for dynamic data

No coding required—perfect for designers and non-technical users who want full control over their interfaces.

### 2. AI-Generated Interfaces

Connect an AI agent to your Flow, and it can generate A2UI interfaces on-the-fly:

- **Dynamic layouts** that adapt to context
- **Personalized content** based on user data
- **Conversational UI** that evolves with the interaction

Both approaches output the same A2UI JSON format, so you can mix and match—start with a template you designed, let AI customize it, then refine it visually.

## Supported Widgets

Flow-Like supports the core A2UI widgets:

| Widget | Description |
|--------|-------------|
| **Text** | Formatted text with markdown support |
| **Image** | Images with alt text and captions |
| **Button** | Interactive buttons with actions |
| **Card** | Content containers with headers |
| **Form** | Input fields for user data |
| **Table** | Structured data display |
| **Chart** | Data visualizations |
| **List** | Ordered and unordered lists |

## When to Use A2UI

| Use Case | Recommendation |
|----------|----------------|
| Simple chat responses | Use the [Chat UI](/apps/chat-ui/) |
| Rich dashboards | Use A2UI with the visual builder |
| AI-generated reports | Use A2UI with AI generation |
| Interactive forms | Use A2UI with form widgets |
| Static landing pages | Use A2UI with the visual builder |

## Getting Started

1. Create a new app or open an existing one
2. Add an **A2UI Event** to expose your interface
3. Choose your approach:
   - **Visual**: Open the A2UI builder and start designing
   - **AI**: Connect a Flow that outputs A2UI JSON
4. Preview and test your interface
5. Share with your team or publish

:::tip[For Developers]
Want to generate A2UI programmatically in your Flows? Check out the [A2UI Developer Guide](/dev/a2ui/overview/) for the full specification and examples.
:::

## Human + AI Collaboration

The real power of A2UI in Flow-Like is combining human creativity with AI capabilities:

1. **Design a template** using the visual builder
2. **Mark sections as dynamic** where AI can fill in content
3. **Connect to an AI Flow** that provides personalized data
4. **Users see** a polished interface with relevant content

This hybrid approach gives you the best of both worlds: consistent branding and layout you control, with dynamic content that adapts to each user.
