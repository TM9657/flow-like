---
title: For LangChain Users
description: How LangChain concepts translate to Flow-Like's visual AI workflow
sidebar:
  order: 2
---

Coming from **LangChain** (Python)? This guide maps your AI orchestration knowledge to Flow-Like's visual approach. You'll find familiar patterns, but with a drag-and-drop interface instead of code.

## Quick Concept Mapping

| LangChain Concept | Flow-Like Equivalent |
|-------------------|---------------------|
| Chain | Flow (sequence of nodes) |
| Agent | Agent node + Tools |
| Tool | Quick Action / Callable flow |
| Memory | Variables + History arrays |
| Prompt Template | Prompt node |
| LLM | Model Provider + Invoke |
| Retriever | Vector Search nodes |
| VectorStore | LanceDB + Embeddings |
| Document Loader | Read nodes + Parse |
| Output Parser | Extract Knowledge |
| Runnable | Node or subflow |
| Callbacks | Console Log (debug mode) |

## Core Patterns Compared

### Chains → Flows

In LangChain, you build **Chains** by composing components:

**LangChain:**
```python
from langchain import PromptTemplate, LLMChain
from langchain.llms import OpenAI

prompt = PromptTemplate(
    input_variables=["product"],
    template="What is a good name for a company that makes {product}?"
)

chain = LLMChain(llm=OpenAI(), prompt=prompt)
result = chain.run("eco-friendly water bottles")
```

**Flow-Like:**
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Quick Action   │     │     Prompt      │     │   Invoke LLM    │
│   (product)     ├────▶│  "What is..."   ├────▶│    OpenAI       │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                         │
                                                         ▼
                                                    [company_name]
```

The visual flow is the chain—each node is a step in the pipeline.

### Agents → Agent Nodes

LangChain **Agents** make decisions about tool usage. Flow-Like has dedicated Agent nodes:

**LangChain:**
```python
from langchain.agents import initialize_agent, Tool

tools = [
    Tool(name="Calculator", func=calculator_func, description="..."),
    Tool(name="Search", func=search_func, description="...")
]

agent = initialize_agent(
    tools=tools,
    llm=llm,
    agent=AgentType.ZERO_SHOT_REACT_DESCRIPTION
)
result = agent.run("What is 25 * 48, then search for that number")
```

**Flow-Like:**
```
┌─────────────────────────────────────────────────────────┐
│ Board: CalculatorTool                                   │
│  Quick Action Event ──▶ Calculate ──▶ Return Result    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Board: SearchTool                                       │
│  Quick Action Event ──▶ Web Search ──▶ Return Result   │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Main Flow:                                              │
│                                                         │
│  Chat Event ──▶ Make Agent ──▶ Run Agent ──▶ Response  │
│                     │                                   │
│                     ├── Tool: CalculatorTool            │
│                     └── Tool: SearchTool                │
└─────────────────────────────────────────────────────────┘
```

Each **Tool** is a separate Board with a Quick Action Event—the agent can call it when needed.

### Prompts → Prompt Nodes

**LangChain PromptTemplate:**
```python
prompt = PromptTemplate(
    input_variables=["context", "question"],
    template="""Answer based on this context:
    {context}

    Question: {question}
    Answer:"""
)
```

**Flow-Like Prompt Node:**
```
┌─────────────────────────────────────────────────────────┐
│ Prompt Node                                             │
│                                                         │
│ Template:                                               │
│   "Answer based on this context:                        │
│    {context}                                            │
│                                                         │
│    Question: {question}                                 │
│    Answer:"                                             │
│                                                         │
│ Inputs:                                                 │
│   ◀── context                                           │
│   ◀── question                                          │
│                                                         │
│ Outputs:                                                │
│   ──▶ formatted_prompt                                  │
└─────────────────────────────────────────────────────────┘
```

Variables are auto-extracted from `{variable}` placeholders in your template.

### Memory → Variables + Arrays

**LangChain Memory:**
```python
from langchain.memory import ConversationBufferMemory

memory = ConversationBufferMemory()
chain = ConversationChain(llm=llm, memory=memory)
```

**Flow-Like:**
```
Variables Panel:
├── chat_history: Array<Message>
└── user_context: String

Chat Event
    │
    ▼
Get Variable: chat_history
    │
    ▼
Build Messages (system + history + new)
    │
    ▼
Invoke LLM
    │
    ▼
Append to Variable: chat_history
```

Memory persists in Board Variables. Use arrays for conversation history.

### RAG Retrievers → Vector Search

**LangChain RAG:**
```python
from langchain.vectorstores import Chroma
from langchain.embeddings import OpenAIEmbeddings
from langchain.chains import RetrievalQA

embeddings = OpenAIEmbeddings()
vectorstore = Chroma(embedding_function=embeddings)

qa_chain = RetrievalQA.from_chain_type(
    llm=llm,
    chain_type="stuff",
    retriever=vectorstore.as_retriever(search_kwargs={"k": 5})
)
```

**Flow-Like RAG:**
```
Ingest Pipeline:
┌─────────────────────────────────────────────────────────┐
│ Read Documents ──▶ Chunk ──▶ Embed ──▶ Insert to LanceDB│
└─────────────────────────────────────────────────────────┘

Query Pipeline:
┌─────────────────────────────────────────────────────────┐
│ Chat Event                                              │
│     │                                                   │
│     ▼                                                   │
│ Embed Query                                             │
│     │                                                   │
│     ▼                                                   │
│ Vector Search (LanceDB, k=5)                           │
│     │                                                   │
│     ▼                                                   │
│ Build Context Prompt                                    │
│     │                                                   │
│     ▼                                                   │
│ Invoke LLM ──▶ Response                                │
└─────────────────────────────────────────────────────────┘
```

### Document Loaders → Read + Parse Nodes

| LangChain Loader | Flow-Like Nodes |
|------------------|-----------------|
| `TextLoader` | Read to String |
| `PyPDFLoader` | Read to String (PDF) |
| `CSVLoader` | Buffered CSV Reader |
| `JSONLoader` | Read to String + Parse JSON |
| `DirectoryLoader` | List Paths + For Each + Read |
| `WebBaseLoader` | HTTP Request |
| `UnstructuredLoader` | Read + Chunk |

**Example PDF Loading:**
```
List Paths (*.pdf)
    │
    ▼
For Each path
    │
    ▼
Read to String (path)
    │
    ▼
Chunk Document
    │
    ▼
Embed Document ──▶ Insert to LanceDB
```

### Output Parsers → Extract Knowledge

**LangChain Structured Output:**
```python
from langchain.output_parsers import PydanticOutputParser
from pydantic import BaseModel

class Person(BaseModel):
    name: str
    age: int
    occupation: str

parser = PydanticOutputParser(pydantic_object=Person)
prompt = PromptTemplate(
    template="Extract person info:\n{text}\n{format_instructions}",
    input_variables=["text"],
    partial_variables={"format_instructions": parser.get_format_instructions()}
)
```

**Flow-Like Extract Knowledge:**
```
┌─────────────────────────────────────────────────────────┐
│ Extract Knowledge Node                                  │
│                                                         │
│ Schema:                                                 │
│   {                                                     │
│     "name": "string",                                   │
│     "age": "number",                                    │
│     "occupation": "string"                              │
│   }                                                     │
│                                                         │
│ Input: ◀── document_text                                │
│ Output: ──▶ Person (typed struct)                       │
└─────────────────────────────────────────────────────────┘
```

The node handles prompting, parsing, and validation automatically.

## LCEL → Visual Pipelines

LangChain Expression Language (LCEL) chains look like:

```python
chain = prompt | llm | parser
result = chain.invoke({"topic": "AI"})
```

**Flow-Like:**
```
Input ──▶ Prompt ──▶ LLM ──▶ Parser ──▶ Output
```

The pipe (`|`) becomes a visual wire. Parallel execution uses multiple branches:

**LCEL Parallel:**
```python
chain = RunnableParallel(summary=summarize_chain, translation=translate_chain)
```

**Flow-Like Parallel:**
```
              ┌──▶ Summarize ──┐
Input ──▶ Split                 ├──▶ Merge ──▶ Output
              └──▶ Translate ──┘
```

## Common Patterns

### Conversational RAG

**LangChain:**
```python
memory = ConversationBufferMemory()
qa_chain = ConversationalRetrievalChain.from_llm(
    llm=llm,
    retriever=retriever,
    memory=memory
)
```

**Flow-Like:**
```
Variables:
├── chat_history: Array<Message>
└── active_context: String

Chat Event (user_message)
    │
    ├──▶ Embed Query ──▶ Vector Search
    │                         │
    │                         ▼
    │                   Retrieve Context
    │                         │
    └─────────────────────────┤
                              ▼
                    Build Messages:
                    [system + context + history + query]
                              │
                              ▼
                         Invoke LLM
                              │
                              ├──▶ Append to history
                              │
                              └──▶ Response
```

### Function Calling

**LangChain Tools with OpenAI:**
```python
tools = [get_weather_tool, search_tool]
llm_with_tools = llm.bind_tools(tools)
result = llm_with_tools.invoke("What's the weather in Paris?")
```

**Flow-Like:**
```
Make Agent
    │
    ├── Tool: GetWeather (Board with Quick Action)
    ├── Tool: Search (Board with Quick Action)
    │
    ▼
Run Agent (handles tool calling loop)
    │
    ▼
Final Response
```

### Map-Reduce

**LangChain:**
```python
from langchain.chains import MapReduceDocumentsChain

map_reduce_chain = MapReduceDocumentsChain(
    llm_chain=map_chain,
    reduce_documents_chain=reduce_chain
)
```

**Flow-Like:**
```
Split Documents
    │
    ▼
For Each document
    │
    ▼
Map: Summarize ──▶ Collect Summaries
                        │
                        ▼
                   Reduce: Final Summary
```

## Feature Comparison

| Feature | LangChain | Flow-Like |
|---------|-----------|-----------|
| **Interface** | Python code | Visual drag-and-drop |
| **Learning curve** | Python required | Lower barrier |
| **Flexibility** | Very flexible | Visual constraints |
| **Debugging** | Print statements | Visual execution trace |
| **Versioning** | Git | Built-in + Git |
| **Deployment** | Custom infrastructure | Desktop/Cloud included |
| **RAG** | Many vector store options | LanceDB native |
| **Agents** | Multiple implementations | Unified Agent nodes |
| **Streaming** | Callback-based | Native streaming |

## What Flow-Like Adds

### Visual Debugging
- Watch data flow in real-time
- Inspect any wire's value
- Step through execution

### Data Processing
- Native DataFusion SQL engine
- Chart visualizations
- ML models (no Python needed)

### Full Application Stack
- UI pages (A2UI)
- Event-driven architecture
- Built-in deployment

### Type Safety
- Strongly typed pins
- Compile-time validation
- Schema enforcement

## Migration Tips

### 1. Think in Nodes, Not Functions
Each LangChain function call becomes a node. Chain composition becomes wiring.

### 2. Use Extract Knowledge Instead of Parsers
The Extract Knowledge node is your Pydantic output parser—just define the schema.

### 3. Boards Are Your Modules
Each Python module can become a Board. Import/export via Quick Actions.

### 4. Variables Replace State
Where you'd use class attributes or memory, use Board Variables.

### 5. Embrace Visual Loops
For Each nodes with visual branches often work better than Python list comprehensions.

## Example Migration

### LangChain: Q&A Bot

**Original Python:**
```python
from langchain.chains import RetrievalQA
from langchain.embeddings import OpenAIEmbeddings
from langchain.vectorstores import Chroma
from langchain.chat_models import ChatOpenAI

embeddings = OpenAIEmbeddings()
vectorstore = Chroma(
    persist_directory="./db",
    embedding_function=embeddings
)

qa = RetrievalQA.from_chain_type(
    llm=ChatOpenAI(model="gpt-4"),
    chain_type="stuff",
    retriever=vectorstore.as_retriever(k=5),
    return_source_documents=True
)

def answer(question: str):
    result = qa({"query": question})
    return result["result"], result["source_documents"]
```

**Flow-Like Equivalent:**
```
Board: QABot
├── Variables:
│   └── db: LanceDB connection
│
└── Events:
    └── Chat Event (question)
            │
            ▼
        Embed Query (OpenAI)
            │
            ▼
        Vector Search (db, k=5)
            │
            ├──▶ sources: Get Metadata
            │
            ▼
        Build Context Prompt
            │
            ▼
        Invoke LLM (GPT-4)
            │
            ▼
        Return: {answer, sources}
```

**Deployment:**
- LangChain: Set up FastAPI, Docker, hosting
- Flow-Like: Click "Publish" → Done

## FAQ

### Can I import my existing chains?
Not directly. You'll rebuild them visually, which often simplifies the logic.

### What about custom LLM providers?
Flow-Like supports OpenAI, Anthropic, Google, Ollama, and any OpenAI-compatible API.

### Is performance comparable?
Yes—Flow-Like's runtime is Rust-based and often faster than Python.

### Can I use my existing vector database?
Flow-Like uses LanceDB natively. You can re-embed your documents or connect external databases via SQL.

### What about LangSmith?
Flow-Like has built-in execution tracing. View logs, timing, and data at each node.

## Next Steps

- **[GenAI Overview](/topics/genai/overview/)** – Full AI capabilities guide
- **[RAG Setup](/topics/genai/rag/)** – Vector search and retrieval
- **[Agents](/topics/genai/agents/)** – Building AI agents
- **[Extraction](/topics/genai/extraction/)** – Structured data extraction
