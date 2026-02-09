---
title: GenAI Overview
description: Build AI-powered applications with Flow-Like's generative AI capabilities
sidebar:
  order: 1
---

**Generative AI** is at the heart of modern intelligent applications. Flow-Like provides everything you need to build sophisticated AI-powered workflows—from simple chatbots to complex multi-agent systems with knowledge retrieval.

:::tip[No Code Required]
All GenAI features in Flow-Like are designed for visual builders. You don't need to write code to create powerful AI applications!
:::

## What Can You Build?

With Flow-Like's GenAI capabilities, you can create:

| Application Type | Description |
|-----------------|-------------|
| **Chatbots & Assistants** | Conversational AI with memory and context |
| **Knowledge Bases (RAG)** | AI that answers questions from your documents |
| **Data Extraction** | Automatically pull structured data from text |
| **AI Agents** | Autonomous assistants that use tools and make decisions |
| **Content Generators** | Create text, summaries, and creative content |

## Core Concepts

Before diving in, here are the key concepts you'll work with:

### 1. AI Models & Providers
AI models are the "brains" behind your GenAI applications. Flow-Like supports dozens of providers including OpenAI, Anthropic, Google, and local models via Ollama.

👉 [Learn about AI Models & Setup](/topics/genai/models/)

### 2. Chat & Conversations
Build interactive chat experiences with full conversation history, streaming responses, and the Chat UI.

👉 [Learn about Chat & Conversations](/topics/genai/chat/)

### 3. RAG & Knowledge Bases
Give your AI access to your own documents using Retrieval-Augmented Generation (RAG). Upload files, create embeddings, and search semantically.

👉 [Learn about RAG & Knowledge Bases](/topics/genai/rag/)

### 4. AI Agents
Create autonomous AI agents that can use tools, search databases, call APIs, and make multi-step decisions.

👉 [Learn about AI Agents](/topics/genai/agents/)

### 5. Extraction & Structured Output
Use AI to extract structured data from unstructured text—perfect for forms, document processing, and data pipelines.

👉 [Learn about Extraction & Structured Output](/topics/genai/extraction/)

## Quick Example: A Simple Chatbot

Here's a high-level view of what a basic chatbot looks like in Flow-Like:

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│   Chat Event    │────▶│  Invoke LLM  │────▶│  Push Response  │
│  (receives msg) │     │  (generate)  │     │  (send reply)   │
└─────────────────┘     └──────────────┘     └─────────────────┘
```

1. **Chat Event** – Receives the user's message and chat history
2. **Invoke LLM** – Sends the conversation to an AI model
3. **Push Response** – Streams the AI's response back to the user

You'll learn how to build this and much more in the following guides!

## Choosing the Right Approach

| If you want to... | Start here |
|------------------|-----------|
| Build a conversational assistant | [Chat & Conversations](/topics/genai/chat/) |
| Answer questions from your documents | [RAG & Knowledge Bases](/topics/genai/rag/) |
| Extract data from PDFs or emails | [Extraction & Structured Output](/topics/genai/extraction/) |
| Create an autonomous agent with tools | [AI Agents](/topics/genai/agents/) |
| Use a local/self-hosted model | [AI Models & Setup](/topics/genai/models/) |

## Prerequisites

Before building GenAI apps, make sure you have:

1. **Flow-Like Desktop** installed ([Download](/start/get/))
2. **API keys** for your chosen AI provider(s), or a local model running via Ollama
3. **Models configured** in Flow-Like ([AI Models Setup](/start/models/))

:::note[Model Availability]
Some features like vision (image understanding) or tool use require specific models. Check your provider's documentation for supported capabilities.
:::

## Next Steps

Ready to start building? Choose your path:

- **New to AI?** Start with [AI Models & Setup](/topics/genai/models/) to configure your first model
- **Building a chatbot?** Jump to [Chat & Conversations](/topics/genai/chat/)
- **Have documents to search?** Head to [RAG & Knowledge Bases](/topics/genai/rag/)
