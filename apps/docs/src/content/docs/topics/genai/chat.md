---
title: Chat & Conversations
description: Build interactive chat experiences with conversational AI
sidebar:
  order: 3
---

Conversational AI is one of the most popular GenAI applications. Flow-Like makes it easy to build chat experiences with **full conversation history**, **streaming responses**, and a **ready-to-use Chat UI**.

## How Chat Works in Flow-Like

A chat application in Flow-Like consists of three main components:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Chat Event    │────▶│  Your Flow      │────▶│  Push Response  │
│  (user message) │     │  (AI logic)     │     │  (send reply)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

1. **Chat Event** – Receives the user's message along with the full conversation history
2. **Your Flow** – Processes the input and generates a response using AI
3. **Push Response** – Sends the AI's reply back to the Chat UI

## Building Your First Chatbot

### Step 1: Create a Chat Event

1. Open your app in Flow-Like
2. Go to **Events** in the sidebar
3. Click **New Event**
4. Select **Chat Event**
5. Choose which Board and Flow will handle the chat

### Step 2: Set Up the Flow

In your flow, you need:

1. **Chat Event Node** – This is your starting point. It outputs:
   - `history` – The full conversation history
   - `local_session` – Data specific to this chat session
   - `global_session` – Data shared across all sessions
   - `attachments` – Any files the user uploaded
   - `user` – Information about the user

2. **Invoke LLM Node** – Sends the conversation to an AI model

3. **Push Response Node** – Streams the response back to the user

### Step 3: Connect the Nodes

```
Chat Event                    Invoke LLM                   Push Response
    │                             │                             │
    ├──▶ history ─────────────▶ History                        │
    │                             │                             │
    │                      Model ◀── (your model)              │
    │                             │                             │
    │                      On Stream ─────────────▶ Chunk      │
    │                             │                             │
    └─────────────────────────────┴────────────────────────────┘
```

**Key connections:**
- Connect `history` from Chat Event to the `History` input on Invoke LLM
- Connect `On Stream` execution to trigger Push Chunk
- Connect `Chunk` output to the Push Chunk node

## Understanding Chat History

The **Chat History** object contains the entire conversation between the user and the AI:

| Component | Description |
|-----------|-------------|
| **Messages** | Array of all messages (user + assistant) |
| **System Prompt** | Instructions that guide the AI's behavior |
| **Parameters** | Settings like temperature, max tokens, etc. |

### Working with History

You can manipulate chat history using these nodes:

| Node | Purpose |
|------|---------|
| **Make History** | Create a new empty history |
| **Push Message** | Add a message to the history |
| **Pop Message** | Remove the last message |
| **Clear History** | Remove all messages |
| **Set System Message** | Set or update the system prompt |

### Setting a System Prompt

The **system prompt** defines your AI's personality and behavior. Use the **Set System Message** node:

```
Chat Event                Set System Message              Invoke LLM
    │                          │                              │
    ├──▶ history ─────────▶ History                          │
    │                          │                              │
    │                    System ◀── "You are a helpful..."   │
    │                          │                              │
    │                    Result ──────────────────────────▶ History
```

**Example system prompts:**

```
You are a helpful customer service agent for Acme Corp.
Be friendly, professional, and always offer to escalate to a human if needed.
```

```
You are a creative writing assistant.
Help users brainstorm ideas, improve their prose, and overcome writer's block.
Respond in an encouraging, supportive tone.
```

## Streaming Responses

For a better user experience, stream responses token-by-token instead of waiting for the full reply:

### Using Invoke LLM with Streaming

The **Invoke LLM** node has two execution outputs:

| Output | Description |
|--------|-------------|
| **On Stream** | Fires for each chunk of the response |
| **Done** | Fires when the complete response is ready |

For chat applications, use **On Stream** to show responses as they arrive:

```
        Invoke LLM
            │
   On Stream│                Done
       ▼    │                 │
  Push Chunk│          Push Response
            │           (optional)
```

### Push Chunk vs Push Response

| Node | When to Use |
|------|-------------|
| **Push Chunk** | During streaming—sends each piece as it arrives |
| **Push Response** | After completion—sends the full response at once |

For the best chat experience, use **Push Chunk** during streaming.

## Customizing the Chat UI

When you create a Chat Event, Flow-Like automatically generates a **Chat UI**. You can customize it:

### In the Event Configuration

- **Default Prompt** – Pre-fill the input field
- **Welcome Message** – Show a greeting when the chat opens
- **Suggested Actions** – Quick-reply buttons
- **File Attachments** – Enable/disable file uploads

### At Runtime via Nodes

Use these nodes to enhance the chat dynamically:

| Node | Purpose |
|------|---------|
| **Push Local Session** | Store data for this conversation |
| **Push Global Session** | Store data across all conversations |
| **Push Attachment** | Send files to the user |

## Advanced: Multi-Turn Context

For complex conversations, you might want to:

### Add Context to Conversation

Insert information into the conversation that the user didn't type:

```
Chat Event          Make Message           Push Message         Invoke LLM
    │                    │                      │                   │
    ├──▶ history ────────┴───────────────▶ History               │
    │                                           │                   │
    │              Content ◀── "Relevant info"  │                   │
    │              Role ◀── "user"              │                   │
    │                                           │                   │
    │                                    Result ─────────────────▶ History
```

### Implement Memory Across Sessions

Use **Local Session** and **Global Session** to persist information:

- **Local Session**: Remembers things for this specific conversation
- **Global Session**: Remembers things across all of this user's conversations

```
Chat Event              Push Local Session
    │                         │
    ├──▶ local_session ───────┤
    │                         │
    │   key: "user_preferences"
    │   value: {theme: "dark"}
```

## Handling Attachments

Users can send files through the Chat UI. Access them via the `attachments` output:

```
Chat Event
    │
    ├──▶ attachments ───▶ (array of files)
```

Each attachment contains:
- `name` – File name
- `type` – MIME type (e.g., "image/png")
- `data` – The file content

### Processing Images

For AI models that support vision (like GPT-4o or Claude 3.5):

1. Get attachments from the Chat Event
2. Use **Pull Attachments** to download image data
3. The history automatically includes the images for vision models

## Best Practices

### 1. Keep System Prompts Clear
Write specific, actionable instructions. Vague prompts lead to unpredictable behavior.

### 2. Handle Errors Gracefully
Add error handling for when the AI fails to respond:
```
Invoke LLM
    │
  Done│        Failed
    ▼           ▼
(success)   (show error message)
```

### 3. Set Reasonable Limits
Use **Set Max Tokens** to limit response length and control costs.

### 4. Test with Edge Cases
Try empty messages, very long inputs, and unexpected requests to ensure your chatbot handles them gracefully.

## Example: Support Chatbot

Here's a complete flow for a customer support chatbot:

```
Chat Event
    │
    ▼
Set System Message: "You are a support agent for..."
    │
    ▼
Invoke LLM ──▶ On Stream ──▶ Push Chunk
    │
  Done
    │
    ▼
Log Response (for analytics)
```

## Next Steps

Now that you've built a chat interface, explore:

- **[RAG & Knowledge Bases](/topics/genai/rag/)** – Give your chatbot access to your documents
- **[AI Agents](/topics/genai/agents/)** – Let your AI use tools and take actions
- **[Extraction](/topics/genai/extraction/)** – Pull structured data from conversations
