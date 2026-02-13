---
title: Building Chatbots
description: Create conversational AI assistants with memory, RAG, and multi-platform deployment
sidebar:
  order: 1
---

Flow-Like makes it easy to build sophisticated chatbots—from simple Q&A bots to AI assistants with memory, knowledge bases, and tool-calling capabilities.

## Chatbot Architecture

```
User Message
    │
    ▼
Chat Event (entry point)
    │
    ▼
┌─────────────────────────────────────┐
│  Conversation Context               │
│  ├── Chat history (memory)          │
│  ├── User profile                   │
│  └── Session state                  │
└─────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────┐
│  Processing Pipeline                │
│  ├── Intent detection (optional)    │
│  ├── RAG retrieval (optional)       │
│  ├── Tool execution (optional)      │
│  └── LLM response generation        │
└─────────────────────────────────────┘
    │
    ▼
Response to User
```

## Simple Q&A Bot

The most basic chatbot—direct LLM responses:

```
Chat Event (user_message)
    │
    ▼
Invoke LLM
├── model: GPT-4
├── system: "You are a helpful assistant for our product."
└── prompt: user_message
    │
    ▼
Response ──▶ Return to user
```

**Add personality:**
```
System Prompt:
"You are Max, a friendly customer support agent for TechCorp.
- Be concise and helpful
- Use casual, friendly language
- If unsure, offer to connect with a human agent
- Never discuss competitor products"
```

## Chat with Memory

Remember conversation history:

```
Board Variables:
├── chat_history: Array<Message> = []
└── user_name: String | null

Chat Event (user_message)
    │
    ▼
Get Variable: chat_history
    │
    ▼
Build Messages:
├── System: "You are a helpful assistant..."
├── History: [...chat_history]
└── User: user_message
    │
    ▼
Invoke LLM (messages)
    │
    ▼
Append to chat_history:
├── { role: "user", content: user_message }
└── { role: "assistant", content: response }
    │
    ▼
Set Variable: chat_history
    │
    ▼
Return response
```

### Memory Strategies

| Strategy | Best For | Implementation |
|----------|----------|----------------|
| **Full History** | Short conversations | Keep all messages |
| **Sliding Window** | Long conversations | Keep last N messages |
| **Summarized** | Very long sessions | Periodically summarize |
| **Semantic** | Topic-focused | Keep relevant messages |

**Sliding window example:**
```
Get Variable: chat_history
    │
    ▼
Slice (last 20 messages)
    │
    ▼
Build Messages ──▶ Invoke LLM
```

**Summarized memory:**
```
If chat_history.length > 50:
    │
    ▼
Summarize older messages
    │
    ▼
Replace with summary + recent 10 messages
```

## RAG-Powered Bot

Answer questions from your knowledge base:

```
Chat Event (user_message)
    │
    ▼
Embed Query (user_message)
    │
    ▼
Vector Search (knowledge_base, k=5)
    │
    ▼
Build Prompt:
"Answer based on this context:
{retrieved_documents}

User question: {user_message}

If the context doesn't contain the answer, say so."
    │
    ▼
Invoke LLM
    │
    ▼
Response (with source citations)
```

### Building the Knowledge Base

**Ingest pipeline:**
```
For Each document in /docs
    │
    ▼
Extract Text (AI Extract Document)
    │
    ▼
Chunk (500 tokens, 50 overlap)
    │
    ▼
Embed Chunks
    │
    ▼
Insert to LanceDB (with metadata)
```

### Hybrid Search

Combine semantic and keyword search:

```
Embed Query (user_message)
    │
    ▼
Hybrid Search
├── Vector search (semantic)
├── Full-text search (keywords)
└── Rerank results
    │
    ▼
Top 5 results ──▶ Build context
```

## Agent Bot

Create a bot that can take actions:

```
Board: SearchTool
└── Quick Action Event (query)
        │
        ▼
    Web Search ──▶ Return results

Board: CalculatorTool
└── Quick Action Event (expression)
        │
        ▼
    Evaluate ──▶ Return result

Board: CreateTicketTool
└── Quick Action Event (title, description)
        │
        ▼
    Jira Create Issue ──▶ Return ticket_id

Board: MainChatbot
├── Variables:
│   └── chat_history: Array<Message>
│
└── Chat Event (user_message)
        │
        ▼
    Make Agent
    ├── model: GPT-4
    ├── tools:
    │   ├── SearchTool: "Search the web for information"
    │   ├── CalculatorTool: "Perform calculations"
    │   └── CreateTicketTool: "Create a support ticket"
    └── system: "You are a support assistant..."
        │
        ▼
    Run Agent (user_message, chat_history)
        │
        ▼
    Response ──▶ Update history ──▶ Return
```

The agent decides when to use tools based on the conversation.

## Multi-Turn Conversations

Handle complex dialogues with context:

```
Chat Event
    │
    ▼
Detect Intent:
├── "greeting" ──▶ Greeting flow
├── "product_question" ──▶ RAG lookup
├── "support_request" ──▶ Ticket creation flow
├── "order_status" ──▶ Order lookup
└── "other" ──▶ General LLM
```

### Slot Filling

Collect required information across turns:

```
Variables:
├── slots:
│   ├── name: null
│   ├── email: null
│   └── issue: null
└── current_slot: "name"

Chat Event
    │
    ▼
Extract slot value from message
    │
    ▼
Update slots
    │
    ▼
All slots filled?
├── Yes ──▶ Process request
└── No ──▶ Ask for next slot
```

**Conversation flow:**
```
Bot: "What's your name?"
User: "I'm Alice"
Bot: "Hi Alice! What's your email?"
User: "alice@example.com"
Bot: "Got it. What can I help you with today?"
User: "I can't log into my account"
Bot: "I've created a support ticket for your login issue.
      Reference: #12345"
```

## Platform Integrations

### Discord Bot

```
Discord Session (bot_token)
    │
    ▼
On Message Event
    │
    ▼
Process with chatbot logic
    │
    ▼
Discord Send Message (channel_id, response)
```

**With commands:**
```
Discord Command: /ask {question}
    │
    ▼
Process question ──▶ Reply
```

### Telegram Bot

```
Telegram Session (bot_token)
    │
    ▼
On Message Event
    │
    ▼
Process with chatbot logic
    │
    ▼
Telegram Send Message (chat_id, response)
```

**With keyboards:**
```
Telegram Send Message
├── text: "How can I help?"
└── keyboard: [
    ["Product Info", "Support"],
    ["Order Status", "Talk to Human"]
]
```

### Slack Bot

```
HTTP Event (Slack webhook)
    │
    ▼
Verify Slack signature
    │
    ▼
Process message
    │
    ▼
HTTP Request (Slack response URL)
```

### Web Chat (A2UI)

Build a chat interface in your app:

```
Page: /chat
├── Column:
│   ├── ScrollArea (chat messages)
│   │   └── For Each message in chat_history:
│   │       └── MessageBubble (message)
│   │
│   └── Row:
│       ├── TextField (user input)
│       └── Button: Send
│           └── onClick: invoke ChatHandler
```

## Advanced Patterns

### Conversation Branching

```
Chat Event
    │
    ▼
Classify Intent (LLM)
    │
    ▼
Switch intent:
    │
    ├── "sales" ──▶ Sales Bot (different personality)
    │
    ├── "support" ──▶ Support Bot (access to tickets)
    │
    └── "general" ──▶ General Assistant
```

### Human Handoff

```
Chat Event
    │
    ▼
Process with AI
    │
    ▼
Confidence below threshold?
├── No ──▶ Return AI response
│
└── Yes ──▶ Check agent availability
                │
                ├── Available ──▶ Transfer to human
                │                   │
                │                   ▼
                │               Notify agent
                │                   │
                │                   ▼
                │               "Connecting you with a human agent..."
                │
                └── Not available ──▶ "I'm not sure about that.
                                        Would you like me to create
                                        a support ticket?"
```

### Proactive Messages

```
Scheduled Event (check_for_updates)
    │
    ▼
Query: Users with pending actions
    │
    ▼
For Each user
    │
    ▼
Send reminder message
├── "Hi! Your order shipped yesterday. Track it here: {link}"
```

### Multi-Language Support

```
Chat Event (user_message)
    │
    ▼
Detect Language (LLM or library)
    │
    ▼
Set response language
    │
    ▼
Process with language-specific system prompt
    │
    ▼
Respond in detected language
```

## Bot Personality & Safety

### System Prompt Engineering

```
System Prompt:
"""
You are Luna, an AI assistant for SpaceTravel Inc.

PERSONALITY:
- Enthusiastic about space exploration
- Uses space-related metaphors occasionally
- Professional but warm

BOUNDARIES:
- Never make up flight information
- Don't discuss competitors
- For medical questions, recommend consulting a doctor
- For pricing, always refer to official sources

RESPONSE STYLE:
- Keep responses under 150 words
- Use bullet points for lists
- Include relevant emojis sparingly 🚀
"""
```

### Guardrails

```
Chat Event (user_message)
    │
    ▼
Content Moderation Check
├── Inappropriate ──▶ Polite refusal
│
▼
Process normally
    │
    ▼
Response Validation
├── Contains PII ──▶ Redact
├── Off-topic ──▶ Redirect
│
▼
Return safe response
```

### Handling Edge Cases

```
Empty message ──▶ "I didn't catch that. Could you try again?"

Very long message ──▶ Truncate + "I noticed a long message.
                       Let me address the main points..."

Repeated questions ──▶ "I mentioned earlier that...
                        Would you like more details?"

Frustration detected ──▶ "I understand this is frustrating.
                          Let me connect you with someone
                          who can help directly."
```

## Metrics & Improvement

### Track Conversations

```
After each response:
    │
    ▼
Log to database:
├── conversation_id
├── user_message
├── bot_response
├── intent_detected
├── tools_used
├── response_time
└── timestamp
```

### Feedback Loop

```
After response:
    │
    ▼
Show: "Was this helpful? 👍 👎"
    │
    ▼
On feedback:
├── 👍 ──▶ Log positive
└── 👎 ──▶ Log negative + offer escalation
```

### Analyze & Improve

```
Weekly Analysis:
├── Most common questions ──▶ Add to FAQ/knowledge base
├── Low-rated responses ──▶ Review and improve prompts
├── Unanswered questions ──▶ Expand capabilities
└── Tool usage patterns ──▶ Optimize tool descriptions
```

## Example: Full Customer Support Bot

```
Board: SupportBot
├── Variables:
│   ├── chat_history: Array<Message>
│   ├── customer: Customer | null
│   ├── current_ticket: Ticket | null
│   └── awaiting_rating: Boolean
│
├── Tools:
│   ├── LookupOrder (order_id)
│   ├── CreateTicket (title, description)
│   ├── SearchKnowledge (query)
│   └── TransferToHuman ()
│
└── Chat Event (user_message, customer_id)
        │
        ▼
    Load customer profile (if customer_id)
        │
        ▼
    awaiting_rating?
    ├── Yes ──▶ Process rating ──▶ Thank user
    │
    ▼
    Make Agent
    ├── model: GPT-4
    ├── tools: [LookupOrder, CreateTicket, SearchKnowledge, TransferToHuman]
    └── system: "You are a support agent for TechCorp.
                 Customer: {customer.name}
                 Previous purchases: {customer.orders}
                 Be helpful and empathetic."
        │
        ▼
    Run Agent (user_message, chat_history)
        │
        ▼
    Update chat_history
        │
        ▼
    Ticket created?
    ├── Yes ──▶ Set awaiting_rating = true after resolution
        │
        ▼
    Return response
```

## Best Practices

### 1. Start Simple
Begin with a basic Q&A bot, then add features iteratively.

### 2. Test Extensively
Create test conversations covering edge cases before deployment.

### 3. Monitor in Production
Log all conversations, watch for failures, iterate on prompts.

### 4. Set Expectations
Be clear about what the bot can and cannot do.

### 5. Provide Escape Hatches
Always offer a way to reach a human.

### 6. Respect Privacy
Don't store sensitive information longer than needed.

## Next Steps

- **[GenAI Agents](/topics/genai/agents/)** – Deep dive into agent capabilities
- **[RAG](/topics/genai/rag/)** – Build knowledge bases
- **[API Integrations](/topics/api-integrations/overview/)** – Platform connections
- **[Building Internal Tools](/topics/internal-tools/overview/)** – Chat UIs
