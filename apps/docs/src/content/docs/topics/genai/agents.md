---
title: AI Agents
description: Create autonomous AI agents that use tools and make decisions
sidebar:
  order: 5
---

**AI Agents** are autonomous assistants that can use tools, search databases, call APIs, and make multi-step decisions to accomplish tasks. Unlike simple chatbots that just respond to messages, agents can **take actions**.

:::tip[Agents vs Chatbots]
**Chatbot**: "The weather in Paris is sunny" (just provides information)
**Agent**: *Checks weather API* → "It's currently 22°C and sunny in Paris"
:::

## What Can Agents Do?

Flow-Like agents can:

| Capability | Example |
|------------|---------|
| **Use tools** | Search databases, call APIs, run calculations |
| **Make decisions** | Choose which tool to use based on the question |
| **Multi-step reasoning** | Break down complex tasks into steps |
| **Query data** | Run SQL queries on your databases |
| **Call your flows** | Execute other Flow-Like flows as tools |
| **Use MCP servers** | Connect to Model Context Protocol tools |

## Agent Architecture

An agent works in a loop:

```
┌────────────────────────────────────────────────────────┐
│                                                        │
│    User Input                                          │
│        │                                               │
│        ▼                                               │
│    Agent Thinks: "What do I need to do?"               │
│        │                                               │
│        ▼                                               │
│    ┌───────────────────────────────────┐               │
│    │  Tool Use?                         │              │
│    │  ├── Yes ──▶ Call Tool ──▶ Loop   │◀─────┐       │
│    │  └── No ──▶ Final Response        │      │       │
│    └───────────────────────────────────┘      │       │
│                     │                         │       │
│                     └─────────────────────────┘       │
│                                                        │
└────────────────────────────────────────────────────────┘
```

## Building Your First Agent

### Step 1: Create an Agent

Use the **Make Agent** node:

```
Make Agent
    │
    ├── Model: (your AI model)
    │
    └── Agent ──▶ (agent object)
```

### Step 2: Set the Agent's Instructions

Use **Set Agent System Prompt** to define behavior:

```
Set Agent System Prompt
    │
    ├── Agent: (from Make Agent)
    ├── System Prompt: "You are a helpful assistant..."
    │
    └── Agent ──▶ (configured agent)
```

**Example system prompt for an agent:**

```
You are a data analyst assistant. You have access to tools for:
- Searching the company database
- Running SQL queries
- Generating charts

Always explain your reasoning before using a tool.
Summarize your findings after completing the task.
```

### Step 3: Add Tools

Tools give your agent capabilities. Flow-Like supports several types:

#### Flow Tools (Recommended)

Turn any Flow-Like flow into a tool using **Add Flow Tools**:

```
Add Flow Tools
    │
    ├── Agent: (your agent)
    ├── Flows: (select flows to expose as tools)
    │
    └── Agent ──▶ (agent with tools)
```

**Creating a flow as a tool:**
1. Create a flow that performs a specific task
2. Use clear, descriptive names for the flow
3. Document inputs/outputs clearly
4. Add it to your agent with "Add Flow Tools"

#### MCP Tools (Model Context Protocol)

Connect to external tool servers using **Add MCP Tools**:

```
Add MCP Tools
    │
    ├── Agent: (your agent)
    ├── MCP Server: (server configuration)
    ├── Mode: Automatic or Manual
    │
    └── Agent ──▶ (agent with MCP tools)
```

**MCP Mode:**
- **Automatic**: Agent uses tools freely
- **Manual**: Tools only run with user approval

#### SQL Tools

Let your agent query databases with **Add SQL Session**:

```
Add SQL Session
    │
    ├── Agent: (your agent)
    ├── Session: (DataFusion SQL session)
    │
    └── Agent ──▶ (agent with data access)
```

#### Thinking Tool

Enable step-by-step reasoning with **Add Thinking Tool**:

```
Add Thinking Tool
    │
    ├── Agent: (your agent)
    │
    └── Agent ──▶ (agent with reasoning)
```

### Step 4: Run the Agent

Use **Invoke Agent** or **Invoke Agent Streaming**:

#### Non-Streaming

```
Invoke Agent
    │
    ├── Agent: (configured agent)
    ├── History: (chat history)
    │
    ├── End ──▶ (execution complete)
    └── Response ──▶ (final result)
```

#### Streaming (Better UX)

```
Invoke Agent Streaming
    │
    ├── Agent: (configured agent)
    ├── History: (chat history)
    │
    ├── On Chunk ──▶ (triggers for each piece)
    ├── Chunk ──▶ (current text)
    │
    ├── Done ──▶ (execution complete)
    └── Response ──▶ (final result)
```

## Agent Loop: Continuous Execution

For complex tasks, use **Agent Loop** to let the AI control its own execution:

```
Agent Loop
    │
    ├── Agent: (your agent)
    ├── History: (conversation)
    ├── Flow References: (available flows)
    │
    ├── On Iteration ──▶ (fires each step)
    ├── Done ──▶ (agent decides it's finished)
    │
    └── Response ──▶ (final result)
```

The agent keeps running until it decides it has completed the task or needs human input.

## Complete Agent Example

Here's a full agent flow for a research assistant:

```
Chat Event
    │
    ├──▶ history
    │
    ▼
Make Agent (with Claude 3.5)
    │
    ▼
Set Agent System Prompt:
    "You are a research assistant. Use your tools to find
     information and summarize it for the user."
    │
    ▼
Add Flow Tools:
    - search_knowledge_base
    - search_web
    - create_summary
    │
    ▼
Add Thinking Tool
    │
    ▼
Invoke Agent Streaming
    │
    ├── On Chunk ──▶ Push Chunk
    │
    └── Done ──▶ Log completion
```

## Creating Effective Tools

### Tool Design Principles

| Do | Don't |
|----|-------|
| Give tools clear, descriptive names | Use vague names like "tool1" |
| Document what the tool does | Leave tools undocumented |
| Make tools do one thing well | Create mega-tools that do everything |
| Handle errors gracefully | Let tools crash silently |
| Return structured data | Return unformatted text dumps |

### Example Tool Flow

A tool for searching a knowledge base:

```
┌────────────────────────────────────────────────┐
│  Flow: search_knowledge_base                   │
│                                                │
│  Inputs:                                       │
│    - query (string): What to search for        │
│    - limit (number): Max results (default: 5)  │
│                                                │
│  Flow:                                         │
│    Embed Query ──▶ Vector Search ──▶ Format    │
│                                                │
│  Output:                                       │
│    - results (array): Matching documents       │
│                                                │
└────────────────────────────────────────────────┘
```

## Best Practices

### 1. Start Simple
Begin with 1-2 tools, then add more as needed. Too many tools can confuse the agent.

### 2. Write Clear System Prompts
Tell the agent:
- What its role is
- What tools are available
- When to use each tool
- How to handle errors

### 3. Use Thinking for Complex Tasks
Enable the thinking tool for tasks that require reasoning:
- Multi-step research
- Decision making
- Problem solving

### 4. Handle Tool Failures
Your tool flows should return helpful error messages:

```
If (search fails)
    └── Return: {"error": "Search unavailable", "suggestion": "Try a different query"}
```

### 5. Limit Agent Iterations
For safety, set a maximum number of tool calls:

```
Agent Loop
    │
    ├── Max Iterations: 10
    │
    └── ...
```

### 6. Log Agent Actions
Track what the agent does for debugging:

```
Invoke Agent
    │
    ├── On Chunk ──▶ Log (for debugging)
    │
    └── Done ──▶ Log final response
```

## Advanced: Multi-Agent Systems

For complex applications, you can create multiple specialized agents:

```
┌─────────────────────────────────────────────────────┐
│                                                     │
│  Router Agent: "What type of task is this?"         │
│       │                                             │
│       ├── Research ──▶ Research Agent               │
│       ├── Code ──▶ Coding Agent                     │
│       └── Data ──▶ Data Analysis Agent              │
│                                                     │
└─────────────────────────────────────────────────────┘
```

Each agent has specialized tools and system prompts for their domain.

## Troubleshooting

### "Agent doesn't use tools"
- Check the model supports tool use (GPT-4, Claude 3.5+, etc.)
- Verify tools are properly connected
- Make the system prompt explicitly mention available tools

### "Agent uses wrong tool"
- Improve tool names and descriptions
- Add examples to the system prompt
- Consider using fewer, more distinct tools

### "Agent loops forever"
- Set a maximum iteration limit
- Check tool error handling
- Verify the agent has a clear "done" condition

### "Responses are slow"
- Use streaming to show progress
- Simplify tools where possible
- Consider using faster models for simple decisions

## Security Considerations

:::caution[Agent Safety]
Agents can take actions. Always consider:
- What data can the agent access?
- What actions can the agent perform?
- Can users manipulate the agent via prompts?
:::

**Recommendations:**
- Limit tool permissions to what's necessary
- Use "Manual" mode for sensitive operations
- Log all agent actions for audit
- Test with adversarial inputs

## Next Steps

Now that you understand agents, explore:

- **[Extraction](/topics/genai/extraction/)** – Use agents to extract structured data
- **[RAG & Knowledge Bases](/topics/genai/rag/)** – Give agents access to your documents
- **[Chat & Conversations](/topics/genai/chat/)** – Build agent-powered chat interfaces
