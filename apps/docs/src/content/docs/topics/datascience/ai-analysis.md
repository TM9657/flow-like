---
title: AI-Powered Analysis
description: Combine GenAI agents with data science for intelligent insights
sidebar:
  order: 6
---

The most powerful data science workflows combine traditional analytics with AI. Flow-Like lets you build **AI agents that can query databases, analyze data, and generate insights**—all through natural language.

## Why AI + Data Science?

| Traditional Approach | AI-Powered Approach |
|---------------------|---------------------|
| Write SQL queries manually | Ask questions in plain English |
| Build fixed dashboards | Generate dynamic insights |
| Code data transformations | Describe what you need |
| Static reports | Conversational exploration |

:::tip[Best of Both Worlds]
AI agents use the same DataFusion, ML, and visualization capabilities—they just let you access them through conversation.
:::

## The Data Science Agent

A data science agent has access to your data and can:

1. **Query databases** using SQL
2. **Analyze results** and identify patterns
3. **Create visualizations** as charts
4. **Train ML models** and make predictions
5. **Explain findings** in plain language

## Building a Data Analysis Agent

### Step 1: Create the Agent

```
Make Agent
    │
    ├── Model: (a capable model like GPT-4 or Claude)
    │
    └── Agent ──▶ (agent object)
```

### Step 2: Set System Prompt

```
Set Agent System Prompt
    │
    ├── Agent: (from step 1)
    ├── System Prompt:
    │     "You are a data analyst assistant. You have access to:
    │      - SQL tools to query the data warehouse
    │      - Visualization tools to create charts
    │
    │      When analyzing data:
    │      1. First understand what tables are available
    │      2. Write SQL to answer the user's question
    │      3. Summarize findings in plain language
    │      4. Create visualizations when helpful
    │
    │      Always explain your reasoning."
    │
    └── Agent ──▶ (configured agent)
```

### Step 3: Add SQL Tools

Give the agent access to your DataFusion session:

```
Add SQL Session
    │
    ├── Agent: (configured agent)
    ├── Session: (DataFusion session with tables)
    │
    └── Agent ──▶ (agent with SQL access)
```

### Step 4: Add Custom Tools (Optional)

Add your own Flow-Like flows as tools:

```
Add Flow Tools
    │
    ├── Agent: (agent)
    ├── Flows: [
    │     "create_chart",
    │     "train_model",
    │     "export_report"
    │   ]
    │
    └── Agent ──▶ (agent with custom tools)
```

### Step 5: Run the Agent

```
Invoke Agent Streaming
    │
    ├── Agent: (fully configured)
    ├── History: "What were our top 5 products last quarter?"
    │
    ├── On Chunk ──▶ Push Chunk (stream response)
    │
    └── Done ──▶ Final response
```

## Example Conversation

**User:** "What were our sales trends last month?"

**Agent thinks:** *I need to query the sales table, aggregate by day, and analyze the trend.*

**Agent executes:**
```sql
SELECT DATE_TRUNC('day', order_date) as day,
       SUM(amount) as daily_sales
FROM orders
WHERE order_date >= '2025-01-01'
  AND order_date < '2025-02-01'
GROUP BY 1
ORDER BY 1
```

**Agent responds:**
> "Sales in January showed an upward trend, starting at $45,000 on January 1st and reaching $78,000 by January 31st—a 73% increase. The strongest days were Fridays, averaging $65,000 compared to $42,000 on Mondays.
>
> Here's the daily trend:
>
> ```nivo
> type: line
> title: January 2025 Daily Sales
> ---
> day,sales
> 2025-01-01,45000
> 2025-01-02,48000
> ...
> ```"

## SQL Tools in Detail

When you add a SQL session to an agent, it gains these capabilities:

### Execute SQL

The agent can run any SQL query:

```sql
SELECT customer_segment,
       COUNT(*) as customers,
       AVG(lifetime_value) as avg_ltv
FROM customers
GROUP BY customer_segment
ORDER BY avg_ltv DESC
```

### Explore Schema

The agent can discover what tables and columns exist:

```sql
-- What tables are available?
SHOW TABLES

-- What columns are in this table?
DESCRIBE sales
```

### Complex Analytics

Window functions, CTEs, joins—the agent can write sophisticated queries:

```sql
WITH monthly_sales AS (
    SELECT DATE_TRUNC('month', date) as month,
           product_category,
           SUM(revenue) as revenue
    FROM sales
    GROUP BY 1, 2
)
SELECT month, product_category, revenue,
       revenue - LAG(revenue) OVER (
           PARTITION BY product_category
           ORDER BY month
       ) as month_over_month_change
FROM monthly_sales
```

## Creating Tool Flows for Agents

Build custom capabilities as Flow-Like flows:

### Chart Generation Tool

```
┌────────────────────────────────────────────────────────────┐
│  Flow: create_chart                                        │
│                                                            │
│  Inputs:                                                   │
│    - data (string): CSV data                              │
│    - chart_type (string): bar, line, pie, etc.           │
│    - title (string): Chart title                          │
│                                                            │
│  Flow:                                                     │
│    Format Markdown ──▶ Return chart block                 │
│                                                            │
│  Output:                                                   │
│    - chart (string): Markdown with nivo/plotly block      │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### ML Prediction Tool

```
┌────────────────────────────────────────────────────────────┐
│  Flow: predict_churn                                       │
│                                                            │
│  Inputs:                                                   │
│    - customer_id (string): Customer to predict for        │
│                                                            │
│  Flow:                                                     │
│    Lookup Customer ──▶ Load Model ──▶ Predict             │
│                                                            │
│  Output:                                                   │
│    - prediction (object): {churn_risk: 0.75, factors: []} │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Report Export Tool

```
┌────────────────────────────────────────────────────────────┐
│  Flow: export_report                                       │
│                                                            │
│  Inputs:                                                   │
│    - title (string): Report title                         │
│    - content (string): Report markdown                    │
│    - format (string): pdf, csv, html                      │
│                                                            │
│  Flow:                                                     │
│    Generate Report ──▶ Save to Storage ──▶ Return URL     │
│                                                            │
│  Output:                                                   │
│    - download_url (string): Link to report                │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Complete Example: Analytics Assistant

Here's a complete flow for a data analytics chat assistant:

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  App Setup (runs once):                                     │
│                                                             │
│  Create DataFusion Session                                  │
│       │                                                     │
│       ▼                                                     │
│  Register PostgreSQL (production database)                  │
│       │                                                     │
│       ▼                                                     │
│  Mount CSV (reference data)                                 │
│       │                                                     │
│       ▼                                                     │
│  Store Session in Variable                                  │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Chat Event Handler:                                        │
│                                                             │
│  Chat Event                                                 │
│       │                                                     │
│       ├──▶ history                                          │
│       │                                                     │
│       ▼                                                     │
│  Make Agent (Claude 3.5 Sonnet)                            │
│       │                                                     │
│       ▼                                                     │
│  Set System Prompt: "You are a data analyst..."            │
│       │                                                     │
│       ▼                                                     │
│  Add SQL Session (from variable)                           │
│       │                                                     │
│       ▼                                                     │
│  Add Flow Tools: [create_chart, export_csv]                │
│       │                                                     │
│       ▼                                                     │
│  Add Thinking Tool                                          │
│       │                                                     │
│       ▼                                                     │
│  Invoke Agent Streaming                                     │
│       │                                                     │
│       ├── On Chunk ──▶ Push Chunk                          │
│       │                                                     │
│       └── Done ──▶ Log completion                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Use Cases

### 1. Ad-Hoc Data Exploration

**User prompts:**
- "Show me sales by region for last quarter"
- "Which products have declining sales?"
- "Compare this year to last year"

### 2. Automated Reporting

**User prompts:**
- "Generate a weekly sales report"
- "Create an executive summary of Q4 performance"
- "Export the top 100 customers to CSV"

### 3. Predictive Insights

**User prompts:**
- "Which customers are at risk of churning?"
- "Predict next month's revenue"
- "What factors drive customer lifetime value?"

### 4. Data Quality Checks

**User prompts:**
- "Are there any anomalies in yesterday's data?"
- "Check for duplicate records"
- "Find missing values in the customer table"

## Best Practices

### 1. Provide Good Context

Include table descriptions in your system prompt:

```
You have access to these tables:
- orders: Order transactions (id, customer_id, amount, date)
- customers: Customer info (id, name, segment, join_date)
- products: Product catalog (id, name, category, price)
```

### 2. Guide the Analysis Process

```
When analyzing data:
1. First understand the question
2. Check what data is available
3. Write and execute SQL
4. Summarize key findings
5. Suggest visualizations or next steps
```

### 3. Handle Large Results

```
For queries that might return many rows:
- Always use LIMIT unless explicitly asked for all data
- Summarize results instead of showing raw data
- Offer to export large datasets to files
```

### 4. Enable Reasoning

Add the **Thinking Tool** for complex analysis:

```
Add Thinking Tool
    │
    ├── Agent: (your agent)
    │
    └── Agent ──▶ (agent with step-by-step reasoning)
```

### 5. Secure Your Data

- Use read-only database connections when possible
- Limit which tables the agent can access
- Log all queries for audit purposes

## Combining with ML

Agents can leverage ML models you've trained:

### Option 1: Pre-trained Model Tool

Create a flow that loads and runs a saved model:

```
Flow: predict_with_model
    │
    ├── Input: features (array)
    │
    ├── Load ML Model (saved model)
    ├── Predict
    │
    └── Output: prediction
```

### Option 2: On-Demand Training

Let the agent trigger model training:

```
Flow: train_classifier
    │
    ├── Input: table_name, target_column
    │
    ├── Query Data
    ├── Split Dataset
    ├── Fit Decision Tree
    ├── Evaluate
    ├── Save Model
    │
    └── Output: accuracy, model_path
```

## Troubleshooting

### "Agent writes invalid SQL"
- Include table schemas in the system prompt
- Add examples of correct queries
- Use models known for good SQL (GPT-4, Claude)

### "Agent doesn't use tools"
- Verify tools are properly connected
- Mention available tools in the system prompt
- Try more explicit user prompts

### "Responses are slow"
- Use streaming to show progress
- Set query timeouts
- Consider caching frequent queries

### "Agent hallucinates data"
- Require the agent to always query before stating facts
- Include verification steps in the system prompt
- Log and validate SQL before execution

## Next Steps

Combine AI-powered analysis with:

- **[DataFusion & SQL](/topics/datascience/datafusion/)** – Understand the SQL capabilities
- **[Machine Learning](/topics/datascience/ml/)** – Build models the agent can use
- **[Data Visualization](/topics/datascience/visualization/)** – Create charts from agent output
- **[AI Agents](/topics/genai/agents/)** – Deep dive into agent capabilities
