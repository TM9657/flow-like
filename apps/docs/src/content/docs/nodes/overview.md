---
title: Node Catalog
description: Explore all available nodes for building workflows
sidebar:
    order: 01
    badge:
      text: Reference
      variant: note
---

Nodes are the building blocks of every Flow-Like workflow. Each node performs a specific operation — from simple math to AI inference to HTTP requests.

![A screenshot showing how you can right-click into the node catalog, e.g. browsing available nodes for mail operations](../../../assets/NodeCatalog.webp)

## Finding Nodes

**In the Studio:**
- Right-click the canvas → Browse categories

<Aside type="tip">
Search by what you want to do, not the node name. "send email" finds mail nodes, "parse json" finds JSON utilities.
</Aside>

## Node Categories

<CardGrid>
  <LinkCard title="🤖 AI & LLM" href="/nodes/AI/Chat/" description="LLMs, embeddings, vector search, RAG pipelines" />
  <LinkCard title="🔀 Control Flow" href="/nodes/Control/Branch/" description="Conditions, loops, parallel execution" />
  <LinkCard title="🗄️ Database" href="/nodes/Database/" description="Query and manage data stores" />
  <LinkCard title="📡 Events" href="/nodes/Events/" description="Triggers, webhooks, schedulers" />
  <LinkCard title="🖼️ Image" href="/nodes/Image/" description="Image processing and manipulation" />
  <LinkCard title="📝 Logging" href="/nodes/Logging/" description="Debug output, tracing, metrics" />
  <LinkCard title="🔢 Math" href="/nodes/Math/" description="Arithmetic, statistics, transforms" />
  <LinkCard title="💾 Storage" href="/nodes/Storage/" description="Files, objects, persistence" />
  <LinkCard title="📦 Data Structures" href="/nodes/Structs/" description="Arrays, objects, parsing" />
  <LinkCard title="🛠️ Utilities" href="/nodes/Utils/" description="Text, dates, conversions" />
  <LinkCard title="📊 Variables" href="/nodes/Variable/" description="State management, getters/setters" />
  <LinkCard title="🌐 Web & HTTP" href="/nodes/Web/" description="API calls, webhooks, scraping" />
</CardGrid>

## Anatomy of a Node

Every node has:

| Component | Description |
|-----------|-------------|
| **Input Pins** | Data coming into the node (left side) |
| **Output Pins** | Data produced by the node (right side) |
| **Exec In** | When to start executing (top, white) |
| **Exec Out** | Signal completion / trigger next (bottom, white) |
| **Type** | Each pin has a specific data type |

## Type Safety

Pins are typed. You can only connect compatible types:

- ✅ `String` → `String`
- ✅ `Number` → `Number | Any`
- ❌ `String` → `Number` (blocked)

The editor shows compatible connections when dragging — incompatible pins are dimmed.

## Custom Nodes

Need something specific? Write your own nodes in Rust:

<LinkCard title="Writing Custom Nodes" href="/dev/writing-nodes/" description="Create nodes for your specific use case" />
