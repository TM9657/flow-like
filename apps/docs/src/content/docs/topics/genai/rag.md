---
title: RAG & Knowledge Bases
description: Build AI systems that answer questions from your documents
sidebar:
  order: 4
---

**Retrieval-Augmented Generation (RAG)** allows your AI to answer questions using information from your own documents. Instead of relying only on what the model was trained on, RAG fetches relevant content and includes it in the AI's context.

:::tip[What is RAG?]
RAG = **Retrieve** relevant documents + **Augment** the prompt with them + **Generate** an informed response
:::

## Why Use RAG?

| Without RAG | With RAG |
|-------------|----------|
| AI only knows training data | AI accesses your documents |
| Can't answer company-specific questions | Answers from your knowledge base |
| May hallucinate facts | Cites actual sources |
| General, generic responses | Specific, relevant answers |

## RAG Architecture

A RAG system in Flow-Like has two phases:

### 1. Indexing Phase (One-Time Setup)
```
Documents ──▶ Chunk Text ──▶ Embed ──▶ Store in Database
```

### 2. Query Phase (Every Question)
```
User Question ──▶ Embed Query ──▶ Search Database ──▶ Add to Prompt ──▶ Generate Answer
```

## Building a RAG System

### Step 1: Prepare Your Documents

First, you need to get your documents into Flow-Like:

1. **Upload files** to your app's [Storage](/apps/storage/)
2. **Read file contents** using Storage nodes
3. **Split into chunks** for efficient retrieval

### Step 2: Create Embeddings

**Embeddings** are numerical representations that capture the meaning of text. Similar texts have similar embeddings, enabling semantic search.

#### Load an Embedding Model

Use the **Load Embedding Model** node:

```
Load Embedding Model
    │
    ├── Model: (select an embedding model)
    │
    └── Result ──▶ (embedding model reference)
```

**Recommended embedding models:**
- `text-embedding-3-small` (OpenAI) – Fast, affordable
- `text-embedding-3-large` (OpenAI) – Higher quality
- `nomic-embed-text` (Ollama) – Local, free
- `voyage-2` (VoyageAI) – High quality

#### Chunk Your Documents

Large documents need to be split into smaller pieces. Use **Chunk Text**:

```
Chunk Text
    │
    ├── Text: (your document)
    ├── Chunk Size: 500
    ├── Overlap: 50
    │
    └── Chunks ──▶ (array of text pieces)
```

| Parameter | Description | Recommendation |
|-----------|-------------|----------------|
| **Chunk Size** | Characters per chunk | 300-1000 |
| **Overlap** | Characters shared between chunks | 10-20% of chunk size |

#### Embed Your Documents

For each chunk, create an embedding using **Embed Document**:

```
For Each Chunk
    │
    ▼
Embed Document
    │
    ├── Document: (chunk text)
    ├── Model: (embedding model)
    │
    └── Vector ──▶ (embedding array)
```

### Step 3: Store in Database

Flow-Like provides a local vector database for storing and searching embeddings.

#### Open a Database

Use **Open Database** to create or connect to a database:

```
Open Database
    │
    ├── Name: "my_knowledge_base"
    │
    └── Database ──▶ (database connection)
```

#### Insert Documents

Use **Insert** or **Upsert** to store your chunks with their embeddings:

```
Insert
    │
    ├── Database: (connection)
    ├── Data: {
    │       "text": "chunk content...",
    │       "source": "document.pdf",
    │       "page": 5
    │   }
    ├── Vector: (embedding)
    │
    └── End
```

:::note[Vector Column]
The database automatically creates a vector column for similarity search. Your additional data (text, source, etc.) is stored alongside.
:::

### Step 4: Search at Query Time

When a user asks a question:

#### Embed the Query

```
Embed Query
    │
    ├── Query: "What is our return policy?"
    ├── Model: (same embedding model!)
    │
    └── Vector ──▶ (query embedding)
```

:::caution[Important]
Always use the **same embedding model** for indexing and querying. Different models produce incompatible embeddings!
:::

#### Search the Database

Use **Vector Search** to find similar documents:

```
Vector Search
    │
    ├── Database: (connection)
    ├── Vector: (query embedding)
    ├── Limit: 5
    │
    └── Results ──▶ (matching documents)
```

### Step 5: Generate the Answer

Now combine the retrieved documents with the user's question:

```
Set System Message
    │
    ├── System: "Answer using ONLY the provided context..."
    │
    ▼
Push Message (add context)
    │
    ├── Content: "Context:\n{retrieved documents}"
    ├── Role: "user"
    │
    ▼
Push Message (add question)
    │
    ├── Content: "Question: {user question}"
    ├── Role: "user"
    │
    ▼
Invoke LLM ──▶ Answer
```

## Search Methods

Flow-Like supports multiple search strategies:

### Vector Search
Finds documents by **semantic similarity**—great for conceptual questions.

```
"What's our vacation policy?" → finds "PTO guidelines" document
```

### Full-Text Search
Finds documents by **exact keywords**—great for specific terms.

```
"policy number 12345" → finds documents containing "12345"
```

### Hybrid Search
Combines **vector + full-text** for the best of both worlds:

```
Hybrid Search
    │
    ├── Vector: (query embedding)
    ├── Search Term: "vacation policy"
    ├── Re-Rank: true
    │
    └── Results ──▶ (best matches)
```

The **Re-Rank** option reorders results for better relevance.

## Complete RAG Flow Example

Here's a full RAG chatbot flow:

```
Chat Event
    │
    ├──▶ history
    │
    ▼
Get Last Message (extract user question)
    │
    ▼
Embed Query
    │
    ▼
Hybrid Search (find relevant docs)
    │
    ▼
Format Context (combine retrieved docs)
    │
    ▼
Set System Message: "Answer based on context..."
    │
    ▼
Push Message: (context + question)
    │
    ▼
Invoke LLM
    │
    ▼
Push Response ──▶ (stream answer to user)
```

## Best Practices

### 1. Chunk Strategically
- Use smaller chunks (300-500 chars) for precise answers
- Use larger chunks (800-1000 chars) for more context
- Consider semantic chunking (by paragraph/section)

### 2. Include Metadata
Store useful metadata with each chunk:
```json
{
  "text": "chunk content",
  "source": "employee_handbook.pdf",
  "page": 12,
  "section": "Benefits",
  "updated": "2025-01-15"
}
```

### 3. Craft Good System Prompts
Tell the AI to use only the provided context:

```
Answer the user's question using ONLY the information provided in the context.
If the context doesn't contain the answer, say "I don't have information about that."
Always cite your sources.
```

### 4. Handle "No Results" Gracefully
When the search returns no relevant documents, acknowledge it:

```
If (results.length == 0)
    └── Respond: "I couldn't find relevant information..."
```

### 5. Use SQL Filters for Precision
Narrow down results using metadata filters:

```
Vector Search
    │
    ├── SQL Filter: "source = 'hr_policies.pdf'"
    │
    └── Results (only from HR policies)
```

## Updating Your Knowledge Base

### Adding New Documents
Run your indexing flow whenever you have new documents.

### Updating Existing Documents
Use **Upsert** instead of **Insert**—it updates existing records or creates new ones based on a unique ID.

### Removing Documents
Use **Delete** with filters to remove outdated content:

```
Delete
    │
    ├── Database: (connection)
    ├── SQL Filter: "source = 'old_document.pdf'"
    │
    └── End
```

## Performance Tips

### 1. Batch Embeddings
Instead of embedding one document at a time, use **Embed Documents** (plural) for batch processing.

### 2. Limit Results
Don't retrieve too many documents—5-10 is usually enough. More can overwhelm the AI's context window.

### 3. Use Hybrid Search
For production systems, hybrid search usually outperforms pure vector search.

### 4. Optimize Chunk Overlap
10-20% overlap ensures important information at chunk boundaries isn't lost.

## Common Issues

### "AI ignores my documents"
- Check that your system prompt instructs the AI to use the context
- Verify documents are being retrieved (log the search results)
- Ensure the retrieved text is actually being added to the prompt

### "Search returns irrelevant results"
- Try different chunk sizes
- Use hybrid search with re-ranking
- Check you're using the same embedding model for indexing and queries

### "Database is empty"
- Verify your indexing flow ran successfully
- Check the database name matches between indexing and querying
- Look for errors in the indexing flow logs

## Next Steps

With RAG set up, explore:

- **[AI Agents](/topics/genai/agents/)** – Let your AI search the knowledge base autonomously
- **[Extraction](/topics/genai/extraction/)** – Pull structured data from retrieved documents
- **[Chat & Conversations](/topics/genai/chat/)** – Build a conversational RAG interface
