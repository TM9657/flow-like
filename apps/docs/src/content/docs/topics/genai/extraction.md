---
title: Extraction & Structured Output
description: Use AI to extract structured data from unstructured text
sidebar:
  order: 6
---

**AI Extraction** lets you pull structured data from unstructured text. Instead of getting freeform responses, you can have the AI return data in a specific format—perfect for forms, document processing, and data pipelines.

:::tip[Why Extraction?]
Turn: *"John Smith, 35 years old, lives in New York and works as an engineer"*
Into: `{ "name": "John Smith", "age": 35, "city": "New York", "occupation": "engineer" }`
:::

## Use Cases

| Use Case | Input | Output |
|----------|-------|--------|
| **Form filling** | Customer email | Structured contact info |
| **Document processing** | PDF invoice | Line items, totals, dates |
| **Data entry** | Natural language | Database records |
| **Content classification** | Article text | Categories, tags, sentiment |
| **Entity recognition** | Any text | People, places, organizations |

## Extraction Nodes

Flow-Like provides two main extraction nodes:

### Extract Knowledge (from Prompt)

Use when you have a **direct text input**:

```
Extract Knowledge
    │
    ├── Model: (AI model)
    ├── Prompt: "Extract the person's info from: {text}"
    ├── Schema: (JSON schema)
    │
    ├── Done ──▶ (extraction complete)
    └── Result ──▶ (structured data)
```

### Extract Knowledge from History

Use when you want to extract from a **conversation**:

```
Extract Knowledge from History
    │
    ├── Model: (AI model)
    ├── History: (chat history)
    ├── Schema: (JSON schema)
    │
    ├── Done ──▶ (extraction complete)
    └── Result ──▶ (structured data)
```

## Defining Your Schema

The **schema** tells the AI exactly what structure you want. It uses JSON Schema format:

### Simple Example

Extract basic contact information:

```json
{
  "type": "object",
  "properties": {
    "name": {
      "type": "string",
      "description": "The person's full name"
    },
    "email": {
      "type": "string",
      "description": "Email address"
    },
    "phone": {
      "type": "string",
      "description": "Phone number"
    }
  },
  "required": ["name"]
}
```

### With Nested Objects

Extract a more complex structure:

```json
{
  "type": "object",
  "properties": {
    "order": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "date": { "type": "string", "description": "ISO format date" },
        "total": { "type": "number" }
      }
    },
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "product": { "type": "string" },
          "quantity": { "type": "integer" },
          "price": { "type": "number" }
        }
      }
    }
  }
}
```

### With Enums (Fixed Options)

Constrain values to specific options:

```json
{
  "type": "object",
  "properties": {
    "sentiment": {
      "type": "string",
      "enum": ["positive", "negative", "neutral"],
      "description": "Overall sentiment of the message"
    },
    "priority": {
      "type": "string",
      "enum": ["low", "medium", "high", "urgent"]
    },
    "category": {
      "type": "string",
      "enum": ["billing", "technical", "sales", "other"]
    }
  }
}
```

## Building an Extraction Flow

### Example: Customer Support Ticket Classifier

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  Chat Event                                             │
│      │                                                  │
│      ├──▶ history                                       │
│      │                                                  │
│      ▼                                                  │
│  Extract Knowledge from History                         │
│      │                                                  │
│      ├── Schema: {                                      │
│      │     "customer_name": "string",                   │
│      │     "issue_type": ["billing","tech","other"],    │
│      │     "priority": ["low","medium","high"],         │
│      │     "summary": "string"                          │
│      │   }                                              │
│      │                                                  │
│      ├── Done                                           │
│      │      │                                           │
│      │      ▼                                           │
│      │   Route by issue_type:                           │
│      │      ├── billing ──▶ Billing Team Flow           │
│      │      ├── tech ──▶ Tech Support Flow              │
│      │      └── other ──▶ General Support Flow          │
│      │                                                  │
│      └── Result ──▶ (structured ticket data)            │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Example: Invoice Data Extraction

```
Read PDF ──▶ Extract Knowledge ──▶ Insert to Database
                 │
                 ├── Schema: {
                 │     "vendor": "string",
                 │     "invoice_number": "string",
                 │     "date": "string",
                 │     "line_items": [...],
                 │     "subtotal": "number",
                 │     "tax": "number",
                 │     "total": "number"
                 │   }
                 │
                 └── Result ──▶ structured invoice
```

## Schema Tips

### 1. Use Descriptions
Add descriptions to help the AI understand what you want:

```json
{
  "properties": {
    "deadline": {
      "type": "string",
      "description": "The deadline mentioned, in YYYY-MM-DD format. If no deadline, use null."
    }
  }
}
```

### 2. Mark Required Fields
Specify which fields are mandatory:

```json
{
  "required": ["name", "email"],
  "properties": {
    "name": { "type": "string" },
    "email": { "type": "string" },
    "phone": { "type": "string" }  // optional
  }
}
```

### 3. Allow Null for Missing Data
When data might not be present:

```json
{
  "properties": {
    "phone": {
      "type": ["string", "null"],
      "description": "Phone number if mentioned, null otherwise"
    }
  }
}
```

### 4. Use Arrays for Multiple Items
When there might be multiple values:

```json
{
  "properties": {
    "mentioned_people": {
      "type": "array",
      "items": { "type": "string" },
      "description": "All people mentioned in the text"
    }
  }
}
```

## Working with Results

The extraction result is a structured object. Use it directly in your flows:

### Access Fields

```
Extract Knowledge
    │
    └── Result ──▶ Get Property "customer_name" ──▶ (string value)
```

### Conditional Logic

```
Extract Knowledge
    │
    └── Result ──▶ Get Property "priority"
                       │
                       ▼
                   Branch:
                       ├── "high" ──▶ Alert Team
                       └── other ──▶ Queue Normally
```

### Store in Database

```
Extract Knowledge
    │
    └── Result ──▶ Insert to Database (directly as record)
```

## Common Extraction Patterns

### Contact Information

```json
{
  "type": "object",
  "properties": {
    "first_name": { "type": "string" },
    "last_name": { "type": "string" },
    "email": { "type": "string" },
    "phone": { "type": ["string", "null"] },
    "company": { "type": ["string", "null"] }
  },
  "required": ["first_name", "last_name"]
}
```

### Sentiment Analysis

```json
{
  "type": "object",
  "properties": {
    "sentiment": { "type": "string", "enum": ["positive", "negative", "neutral"] },
    "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
    "key_phrases": { "type": "array", "items": { "type": "string" } },
    "summary": { "type": "string" }
  }
}
```

### Action Items

```json
{
  "type": "object",
  "properties": {
    "action_items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "task": { "type": "string", "description": "What needs to be done" },
          "assignee": { "type": ["string", "null"], "description": "Who should do it" },
          "deadline": { "type": ["string", "null"], "description": "When it's due (ISO date)" },
          "priority": { "type": "string", "enum": ["low", "medium", "high"] }
        },
        "required": ["task"]
      }
    }
  }
}
```

### Meeting Notes

```json
{
  "type": "object",
  "properties": {
    "title": { "type": "string" },
    "date": { "type": "string" },
    "attendees": { "type": "array", "items": { "type": "string" } },
    "topics_discussed": { "type": "array", "items": { "type": "string" } },
    "decisions": { "type": "array", "items": { "type": "string" } },
    "action_items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "task": { "type": "string" },
          "owner": { "type": "string" }
        }
      }
    },
    "next_meeting": { "type": ["string", "null"] }
  }
}
```

## Best Practices

### 1. Be Specific in Descriptions
Vague: `"date": { "type": "string" }`
Better: `"date": { "type": "string", "description": "Date in YYYY-MM-DD format" }`

### 2. Start Simple, Then Expand
Begin with the most important fields, test, then add more.

### 3. Handle Missing Data
Always consider what happens when information isn't available:
- Use `["type", "null"]` for optional fields
- Set sensible defaults in your flow logic

### 4. Validate Results
Even with schemas, verify critical extractions:

```
Extract Knowledge
    │
    └── Result
           │
           ▼
       Validate Email Format
           │
           ├── Valid ──▶ Continue
           └── Invalid ──▶ Flag for Review
```

### 5. Test with Edge Cases
Try extraction with:
- Minimal information
- Extra irrelevant information
- Ambiguous phrasing
- Multiple possible values

## Combining with Other Features

### Extraction + RAG

Search documents, then extract structured data:

```
Vector Search ──▶ Extract Knowledge from Results ──▶ Structured Output
```

### Extraction + Agents

Let agents extract data as part of their workflow:

```
Agent (with extraction tool) ──▶ Autonomous data collection
```

### Extraction + Chat

Extract info during conversation for personalization:

```
Chat Event ──▶ Extract Preferences ──▶ Customize Response
```

## Troubleshooting

### "Extraction returns wrong format"
- Check your JSON schema syntax is valid
- Verify property types match expected data
- Add clearer descriptions

### "Missing fields in output"
- Mark fields as required only if truly necessary
- Allow null for optional fields
- Check the source text actually contains the information

### "Inconsistent results"
- Use lower temperature (more deterministic)
- Add examples in descriptions
- Use enums for constrained values

### "AI makes up data"
- Add "null if not mentioned" to descriptions
- Use `["type", "null"]` for optional fields
- Validate critical fields after extraction

## Next Steps

Now that you can extract structured data:

- **[Chat & Conversations](/topics/genai/chat/)** – Extract data from chat interactions
- **[RAG & Knowledge Bases](/topics/genai/rag/)** – Process extracted data into your knowledge base
- **[AI Agents](/topics/genai/agents/)** – Build agents that use extraction as a tool
