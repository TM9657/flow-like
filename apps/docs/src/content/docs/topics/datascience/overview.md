---
title: Data Science Overview
description: Explore, analyze, and model data with Flow-Like's data science toolkit
sidebar:
  order: 1
---

Flow-Like brings powerful data science capabilities to a visual, no-code environment. Whether you're exploring datasets, building ML models, or creating dashboards—you can do it all without writing code.

:::tip[For Data Scientists]
This section assumes familiarity with data science concepts like SQL, machine learning, and data visualization. We'll show you how to apply your knowledge in Flow-Like's visual environment.
:::

## What Can You Build?

| Application Type | Description |
|-----------------|-------------|
| **Data Pipelines** | Load, transform, and analyze data from multiple sources |
| **Interactive Dashboards** | Charts and visualizations with Nivo and Plotly |
| **ML Workflows** | Train and deploy classification, regression, and clustering models |
| **Federated Analytics** | Query across PostgreSQL, MySQL, Parquet, Delta Lake, and more |
| **AI-Powered Analysis** | Combine traditional ML with GenAI agents |

## Core Capabilities

### 1. Data Loading & Storage
Import data from CSVs, Excel files, databases, cloud storage, and APIs. Flow-Like's storage system keeps your data organized and accessible.

👉 [Learn about Data Loading & Storage](/topics/datascience/loading/)

### 2. DataFusion SQL Analytics
Use SQL to query data from any source—local files, databases, or cloud data lakes. DataFusion unifies your data under a single query interface.

👉 [Learn about DataFusion & SQL](/topics/datascience/datafusion/)

### 3. Machine Learning Models
Train and deploy ML models for classification, regression, clustering, and dimensionality reduction using the linfa ML library.

👉 [Learn about Machine Learning](/topics/datascience/ml/)

### 4. Data Visualization
Create beautiful charts and dashboards using Nivo (17 chart types) and Plotly (scientific visualizations) directly in your A2UI interfaces.

👉 [Learn about Data Visualization](/topics/datascience/visualization/)

### 5. GenAI for Data Science
Leverage AI agents for data analysis—natural language queries, automated insights, and intelligent data processing.

👉 [Learn about AI-Powered Analysis](/topics/datascience/ai-analysis/)

## The Data Science Workflow

A typical data science workflow in Flow-Like:

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│   1. LOAD DATA                                                   │
│      CSV, Excel, Parquet, APIs, Databases                        │
│                    │                                             │
│                    ▼                                             │
│   2. EXPLORE & TRANSFORM                                         │
│      DataFusion SQL, filtering, aggregation                      │
│                    │                                             │
│                    ▼                                             │
│   3. ANALYZE                                                     │
│      ├── Traditional ML (classification, clustering)            │
│      └── GenAI Agents (natural language analysis)               │
│                    │                                             │
│                    ▼                                             │
│   4. VISUALIZE                                                   │
│      Charts, dashboards, reports                                 │
│                    │                                             │
│                    ▼                                             │
│   5. DEPLOY                                                      │
│      Scheduled runs, APIs, Chat interfaces                       │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Quick Example: Sales Analysis

Here's what a sales analysis workflow might look like:

```
Read CSV ──▶ Mount to DataFusion ──▶ SQL Query ──▶ Bar Chart
    │              │                     │            │
    │              │                     │            │
    │         "sales_data"         "SELECT region,    │
    │                               SUM(revenue)      │
    │                               GROUP BY region"  │
    │                                                 │
    └─────────────────────────────────────────────────┘
```

1. **Read CSV** – Load your sales data file
2. **Mount to DataFusion** – Register as a SQL-queryable table
3. **SQL Query** – Aggregate by region
4. **Bar Chart** – Visualize results in A2UI

## Supported Data Sources

### Local Files
| Format | Support |
|--------|---------|
| CSV | ✅ Full (streaming, chunked reads) |
| Excel (.xlsx) | ✅ Full (sheets, cells, tables) |
| Parquet | ✅ Full (columnar, efficient) |
| JSON / NDJSON | ✅ Full (with schema) |

### Databases
| Database | Query | Write |
|----------|-------|-------|
| PostgreSQL | ✅ | ✅ |
| MySQL | ✅ | ✅ |
| SQLite | ✅ | ✅ |
| DuckDB | ✅ | ✅ |
| ClickHouse | ✅ | ✅ |
| Oracle | ✅ | ✅ |

### Data Lakes
| Format | Features |
|--------|----------|
| Delta Lake | Read, write, time travel |
| Apache Iceberg | Read, snapshots |
| Hive Partitioned | Parquet, JSON |

### Cloud Storage
| Provider | Support |
|----------|---------|
| AWS S3 | ✅ Full |
| Azure Blob | ✅ Full |
| Google Cloud Storage | ✅ Full |
| AWS Athena | ✅ Query |

## ML Algorithms Available

| Category | Algorithms |
|----------|------------|
| **Classification** | Decision Trees, Naive Bayes, SVM |
| **Regression** | Linear Regression |
| **Clustering** | K-Means, DBSCAN |
| **Dimensionality Reduction** | PCA |
| **Deep Learning** | ONNX Runtime (YOLO, TIMM, custom models) |

## Visualization Options

| Library | Chart Types |
|---------|-------------|
| **Nivo** | Bar, Line, Pie, Radar, Heatmap, Scatter, Funnel, Treemap, Sunburst, Calendar, Sankey, Stream, Waffle, Chord + more |
| **Plotly** | Bar, Line, Scatter, Pie, Area, Histogram, Heatmap, Box, Violin |

## Prerequisites

Before starting with data science in Flow-Like:

1. **Flow-Like Desktop** installed ([Download](/start/get/))
2. **Data files** or database connections ready
3. For ML: understanding of basic ML concepts
4. For AI analysis: API keys for LLM providers

## Next Steps

Choose your starting point:

- **Working with data?** Start with [Data Loading & Storage](/topics/datascience/loading/)
- **Need SQL analytics?** Jump to [DataFusion & SQL](/topics/datascience/datafusion/)
- **Building ML models?** See [Machine Learning](/topics/datascience/ml/)
- **Creating dashboards?** Head to [Data Visualization](/topics/datascience/visualization/)
- **Want AI-powered insights?** Explore [AI-Powered Analysis](/topics/datascience/ai-analysis/)
