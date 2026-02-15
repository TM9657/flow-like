---
title: AI Models & Setup
description: Configure AI models and providers to power your GenAI applications
sidebar:
  order: 2
---

Before building AI-powered flows, you need to **configure at least one AI model**. Flow-Like supports a wide range of providers—from cloud APIs like OpenAI and Anthropic to local models via Ollama.

## Supported Providers

Flow-Like supports these AI providers out of the box:

| Provider | Type | Best For |
|----------|------|----------|
| **OpenAI** | Cloud | GPT-4, GPT-4o, o1, o3 models |
| **Azure OpenAI** | Cloud | Enterprise deployments |
| **Anthropic** | Cloud | Claude 3.5, Claude 4 models |
| **Google** | Cloud | Gemini models |
| **Ollama** | Local | Self-hosted, privacy-first |
| **Groq** | Cloud | Ultra-fast inference |
| **DeepSeek** | Cloud | Reasoning models |
| **Mistral** | Cloud | European AI provider |
| **Together AI** | Cloud | Open-source models |
| **OpenRouter** | Cloud | Model aggregator |
| **Perplexity** | Cloud | Search-enhanced AI |
| **Cohere** | Cloud | Enterprise NLP |
| **HuggingFace** | Cloud | Open-source models |
| **xAI (Grok)** | Cloud | X's AI models |
| **VoyageAI** | Cloud | Embedding models |
| **Hyperbolic** | Cloud | High-performance inference |
| **Moonshot** | Cloud | Multilingual models |

## Adding a Model to Flow-Like

### Step 1: Open Profiles

Go to **Settings → Profiles** in Flow-Like Desktop:

1. Click your profile avatar in the sidebar
2. Select **Profiles** from the menu
3. Click on your active profile

### Step 2: Add a Provider

In the **Models** section of your profile:

1. Click **Add Provider**
2. Select your provider from the list
3. Enter your **API key** (or connection details for local models)
4. Click **Save**

:::tip[Getting API Keys]
Each provider has their own signup process:
- **OpenAI**: [platform.openai.com](https://platform.openai.com)
- **Anthropic**: [console.anthropic.com](https://console.anthropic.com)
- **Google AI**: [aistudio.google.com](https://aistudio.google.com)
- **Ollama**: No key needed—just run Ollama locally!
:::

### Step 3: Select Models

After adding a provider, Flow-Like will fetch the available models. Choose which ones you want to use:

1. Browse the available models
2. Toggle on the models you want active
3. Optionally, set a **default model** for quick access

## Using Models in Your Flows

Once configured, you can use AI models in your flows with these nodes:

### Find Model Node

The **Find Model** node automatically selects the best available model based on your preferences:

```
┌─────────────────┐     ┌─────────────────┐
│  Make Model     │────▶│   Invoke LLM    │
│  Preferences    │     │                 │
└─────────────────┘     └─────────────────┘
```

**How it works:**
1. Add a **Make Preferences** node
2. Set preferences like speed, cost, or capability requirements
3. Connect to your LLM node
4. Flow-Like automatically picks the best matching model

### Direct Model Selection

Alternatively, use a specific model by its identifier:

1. Add your provider's **Prepare Model** node (e.g., "Prepare OpenAI")
2. Select the specific model from the dropdown
3. Connect to your LLM invocation node

## Model Preferences

The **Model Preferences** system helps you choose the right model dynamically:

| Preference | Description |
|------------|-------------|
| **Speed** | Prioritize fast response times |
| **Cost** | Prefer cheaper models |
| **Quality** | Prefer more capable models |
| **Context Size** | Require specific context window |
| **Capabilities** | Require features like vision or tools |

This is especially useful when you want your flows to:
- Automatically fall back to alternatives if a model is unavailable
- Balance cost vs. quality based on the task
- Use different models in development vs. production

## Local Models with Ollama

For privacy-sensitive applications or offline use, you can run models locally with **Ollama**:

### Setting Up Ollama

1. Download Ollama from [ollama.ai](https://ollama.ai)
2. Install and run Ollama on your machine
3. Pull a model: `ollama pull llama3.2` or `ollama pull mistral`
4. In Flow-Like, add Ollama as a provider (usually auto-detected)

### Recommended Local Models

| Model | Size | Good For |
|-------|------|----------|
| **llama3.2** | 3B/8B | General purpose, fast |
| **mistral** | 7B | Coding, reasoning |
| **phi-4** | 14B | High quality, balanced |
| **deepseek-r1** | 7B/32B | Complex reasoning |
| **nomic-embed-text** | - | Text embeddings |

:::note[Hardware Requirements]
Local models require a capable machine. For 7B models, you'll want at least 8GB RAM. For larger models (32B+), 32GB+ RAM or a GPU is recommended.
:::

## Model Configuration Tips

### For Chat Applications
- Use models with large context windows (32K+ tokens)
- Enable streaming for better user experience
- Consider cost for high-volume applications

### For RAG & Knowledge Retrieval
- Pair a generative model with an embedding model
- Use the same embedding model for indexing and querying
- Consider specialized models like `nomic-embed-text`

### For Agents & Tools
- Use models explicitly designed for tool use (GPT-4, Claude 3.5+)
- Larger models generally perform better with complex tools
- Test with simpler tasks first

## Troubleshooting

### "No models available"
- Check your API key is valid and has credits
- Verify your internet connection (for cloud providers)
- For Ollama, ensure it's running (`ollama serve`)

### "Model not responding"
- Check rate limits on your API key
- Try a different model from the same provider
- For local models, check system resources

### "Unexpected responses"
- Verify the model supports your use case (e.g., vision, tools)
- Check your system prompt and temperature settings
- Try a more capable model

## Next Steps

With your models configured, you're ready to build:

- **Simple chat**: Continue to [Chat & Conversations](/topics/genai/chat/)
- **Knowledge retrieval**: Jump to [RAG & Knowledge Bases](/topics/genai/rag/)
- **Autonomous agents**: See [AI Agents](/topics/genai/agents/)
