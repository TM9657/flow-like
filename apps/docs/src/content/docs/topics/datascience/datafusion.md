---
title: DataFusion & SQL Analytics
description: Query any data source with SQL using Apache DataFusion
sidebar:
  order: 3
---

**DataFusion** is Flow-Like's SQL analytics engine. It lets you query data from multiple sources—CSVs, Parquet files, databases, cloud storage—using standard SQL, all unified under a single query interface.

:::tip[Why DataFusion?]
DataFusion is Apache Arrow-based, meaning it's fast, memory-efficient, and supports complex analytical queries. Think of it as having a powerful SQL database that can query anything.
:::

## How It Works

DataFusion creates a virtual SQL layer over your data:

```
┌─────────────────────────────────────────────────────────────┐
│                    DataFusion Session                       │
│                                                             │
│   ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────────┐ │
│   │  CSV    │  │ Parquet │  │ Postgres │  │  Delta Lake  │ │
│   │  Files  │  │  Files  │  │   Table  │  │    Table     │ │
│   └────┬────┘  └────┬────┘  └────┬─────┘  └──────┬───────┘ │
│        │            │            │               │          │
│        └────────────┴─────┬──────┴───────────────┘          │
│                           │                                  │
│                    ┌──────▼──────┐                          │
│                    │  SQL Query  │                          │
│                    │   Engine    │                          │
│                    └──────┬──────┘                          │
│                           │                                  │
│                    ┌──────▼──────┐                          │
│                    │   Results   │                          │
│                    │  (CSVTable) │                          │
│                    └─────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

## Creating a Session

Start by creating a DataFusion session:

```
Create DataFusion Session
    │
    ├── Memory Limit: (optional, e.g., "4GB")
    ├── Batch Size: 8192
    ├── Enable Object Store: true
    │
    └── Session ──▶ (DataFusion session reference)
```

**Configuration options:**

| Option | Description | Default |
|--------|-------------|---------|
| Memory Limit | Maximum memory for queries | Unlimited |
| Batch Size | Rows processed at once | 8192 |
| Enable Object Store | Allow cloud storage access | true |
| Parallelize | Number of parallel workers | Auto |

## Mounting Data Sources

### Mount CSV Files

```
Mount CSV
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to CSV)
    ├── Table Name: "sales"
    ├── Has Header: true
    ├── Delimiter: ","
    │
    └── Session ──▶ (session with table registered)
```

Now query: `SELECT * FROM sales`

### Mount Parquet Files

```
Mount Parquet
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to Parquet)
    ├── Table Name: "events"
    │
    └── Session ──▶ (session with table)
```

### Mount JSON/NDJSON

```
Mount JSON
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to JSON)
    ├── Table Name: "logs"
    │
    └── Session ──▶ (session with table)
```

### Register LanceDB Tables

Use data from your LanceDB databases:

```
Register Lance Table
    │
    ├── Session: (DataFusion session)
    ├── Database: (LanceDB connection)
    ├── Alias: "customers"
    │
    └── Session ──▶ (session with table)
```

### Register CSV Tables (In-Memory)

For CSVTable data already in memory:

```
Register CSV Table
    │
    ├── Session: (DataFusion session)
    ├── CSV Table: (CSVTable from previous node)
    ├── Table Name: "processed_data"
    │
    └── Session ──▶ (session with table)
```

## Connecting to Databases

DataFusion supports federated queries across multiple databases.

### PostgreSQL

```
Register PostgreSQL
    │
    ├── Session: (DataFusion session)
    ├── Host: "db.example.com"
    ├── Port: 5432
    ├── Database: "analytics"
    ├── Schema: "public"
    ├── User: (secret reference)
    ├── Password: (secret reference)
    ├── Table: "transactions"
    ├── Alias: "txns"
    ├── SSL Mode: "require"
    │
    └── Session ──▶ (session with database table)
```

### MySQL

```
Register MySQL
    │
    ├── Session: (DataFusion session)
    ├── Host: "mysql.example.com"
    ├── Port: 3306
    ├── Database: "app_db"
    ├── User: (secret)
    ├── Password: (secret)
    ├── Table: "users"
    ├── Alias: "users"
    │
    └── Session ──▶ (session with table)
```

### Other Databases

| Database | Node | Notes |
|----------|------|-------|
| SQLite | Register SQLite | File-based, local only |
| DuckDB | Register DuckDB | Embedded analytics |
| ClickHouse | Register ClickHouse | Column-oriented OLAP |
| Oracle | Register Oracle | Enterprise database |

## Data Lakes

### Delta Lake

Query Delta Lake tables with time travel:

```
Register Delta Lake
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to delta table)
    ├── Table Name: "orders"
    │
    └── Session ──▶ (session with Delta table)
```

**Time Travel:**
```
Delta Time Travel
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to delta table)
    ├── Table Name: "orders_historical"
    ├── Version: 5  (or timestamp)
    │
    └── Session ──▶ (session with historical version)
```

Query data as it was at a specific point in time!

### Apache Iceberg

```
Register Iceberg
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to Iceberg table)
    ├── Table Name: "events"
    │
    └── Session ──▶ (session with Iceberg table)
```

### Hive-Partitioned Data

For partitioned Parquet/JSON files:

```
Register Hive Parquet
    │
    ├── Session: (DataFusion session)
    ├── Path: (FlowPath to partitioned data)
    ├── Table Name: "partitioned_data"
    ├── Partition Columns: ["year", "month"]
    │
    └── Session ──▶ (session with partitioned table)
```

## Cloud Services

### AWS Athena

Query data in AWS Athena:

```
Register Athena
    │
    ├── Session: (DataFusion session)
    ├── Region: "us-east-1"
    ├── Database: "default"
    ├── Table: "web_logs"
    ├── Output Location: "s3://my-bucket/athena-results/"
    ├── Access Key: (secret)
    ├── Secret Key: (secret)
    │
    └── Session ──▶ (session with Athena table)
```

### Arrow Flight SQL

For high-performance data transfer:

```
Register Flight SQL
    │
    ├── Session: (DataFusion session)
    ├── Endpoint: "grpc://flight-server:8815"
    ├── Table Name: "realtime_data"
    │
    └── Session ──▶ (session with Flight table)
```

## Executing Queries

### SQL Query Node

Execute SQL and get structured results:

```
SQL Query
    │
    ├── Session: (DataFusion session)
    ├── Query: "SELECT region, SUM(revenue) as total
    │           FROM sales
    │           WHERE year = 2025
    │           GROUP BY region
    │           ORDER BY total DESC"
    │
    ├── End ──▶ (query complete)
    ├── CSV Table ──▶ (columnar results)
    ├── Rows ──▶ (array of row objects)
    └── Schema ──▶ (column definitions)
```

### Execute SQL (Markdown Output)

For AI agents or text-based output:

```
Execute SQL
    │
    ├── Session: (DataFusion session)
    ├── Query: "SELECT * FROM sales LIMIT 10"
    │
    ├── End ──▶ (query complete)
    ├── Result ──▶ (markdown-formatted table)
    ├── Row Count ──▶ (number of rows)
    └── Column Count ──▶ (number of columns)
```

This is perfect for feeding results to LLMs!

## Time Series Queries

DataFusion excels at time series analysis.

### Time Bin Aggregation

Aggregate by time intervals:

```
Time Bin Aggregation
    │
    ├── Session: (DataFusion session)
    ├── Table: "events"
    ├── Time Column: "timestamp"
    ├── Interval: "1 hour"
    ├── Value Column: "count"
    ├── Aggregation: "SUM"
    │
    └── Results ──▶ (time-binned data)
```

**Supported intervals:** second, minute, hour, day, week, month, year

### Date Truncation

Group by date parts:

```
Date Trunc Aggregation
    │
    ├── Session: (DataFusion session)
    ├── Table: "sales"
    ├── Time Column: "order_date"
    ├── Granularity: "month"
    ├── Value Column: "revenue"
    ├── Aggregation: "SUM"
    │
    └── Results ──▶ (monthly aggregates)
```

### Time Range Filtering

Filter to a specific time window:

```
Time Range Filter
    │
    ├── Session: (DataFusion session)
    ├── Table: "logs"
    ├── Time Column: "timestamp"
    ├── Start: "2025-01-01T00:00:00"
    ├── End: "2025-01-31T23:59:59"
    │
    └── Results ──▶ (filtered data)
```

## Utility Nodes

### List Tables

See all registered tables:

```
List Tables
    │
    ├── Session: (DataFusion session)
    │
    └── Tables ──▶ ["sales", "customers", "orders"]
```

### Describe Table

Get table schema:

```
Describe Table
    │
    ├── Session: (DataFusion session)
    ├── Table Name: "sales"
    │
    └── Schema ──▶ [
        {name: "id", type: "Int64"},
        {name: "amount", type: "Float64"},
        {name: "date", type: "Timestamp"}
    ]
```

## Writing Results

### Write to Delta Lake

Persist query results:

```
Write Delta
    │
    ├── Session: (DataFusion session)
    ├── Query: "SELECT * FROM processed_data"
    ├── Path: (FlowPath to output)
    ├── Mode: "overwrite"  (or "append")
    │
    └── End
```

## Complete Example: Multi-Source Analytics

Here's a real-world example joining data from multiple sources:

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Create Session                                             │
│       │                                                     │
│       ▼                                                     │
│  Register PostgreSQL (orders from production DB)            │
│       │                                                     │
│       ▼                                                     │
│  Mount CSV (product catalog from file)                      │
│       │                                                     │
│       ▼                                                     │
│  Register Delta Lake (historical analytics)                 │
│       │                                                     │
│       ▼                                                     │
│  SQL Query:                                                 │
│    "SELECT                                                  │
│       p.category,                                           │
│       COUNT(o.id) as order_count,                          │
│       SUM(o.amount) as revenue,                            │
│       AVG(h.avg_delivery_days) as avg_delivery             │
│     FROM orders o                                           │
│     JOIN products p ON o.product_id = p.id                 │
│     LEFT JOIN historical h ON p.category = h.category      │
│     GROUP BY p.category"                                    │
│       │                                                     │
│       ▼                                                     │
│  Results ──▶ Dashboard / Report                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## SQL Features Supported

DataFusion supports a rich SQL dialect:

### Aggregations
```sql
SELECT
    category,
    COUNT(*), SUM(amount), AVG(price),
    MIN(date), MAX(date),
    STDDEV(rating), VARIANCE(score)
FROM products
GROUP BY category
HAVING COUNT(*) > 10
```

### Window Functions
```sql
SELECT
    date,
    revenue,
    SUM(revenue) OVER (ORDER BY date ROWS 6 PRECEDING) as rolling_7day,
    RANK() OVER (PARTITION BY region ORDER BY revenue DESC) as rank
FROM sales
```

### Joins
```sql
SELECT * FROM orders o
INNER JOIN customers c ON o.customer_id = c.id
LEFT JOIN shipping s ON o.id = s.order_id
```

### Common Table Expressions (CTEs)
```sql
WITH monthly_sales AS (
    SELECT DATE_TRUNC('month', date) as month, SUM(amount) as total
    FROM sales
    GROUP BY 1
)
SELECT month, total,
       total - LAG(total) OVER (ORDER BY month) as change
FROM monthly_sales
```

## Best Practices

### 1. Push Down Filters
Let DataFusion push filters to data sources:
```sql
-- Good: filter will be pushed to Postgres
SELECT * FROM postgres_table WHERE status = 'active'

-- Less efficient: filter applied after full scan
SELECT * FROM (SELECT * FROM postgres_table) WHERE status = 'active'
```

### 2. Use Appropriate Data Formats
| Scenario | Recommended Format |
|----------|-------------------|
| Frequent full scans | Parquet |
| Frequent updates | Delta Lake |
| Small lookup tables | CSV (in-memory) |
| Real-time data | Database connection |

### 3. Limit Result Sets
Always use LIMIT when exploring:
```sql
SELECT * FROM large_table LIMIT 100
```

### 4. Index Database Tables
Ensure source database tables are properly indexed for the columns you filter on.

### 5. Monitor Memory
Set memory limits for large queries to prevent crashes.

## Troubleshooting

### "Table not found"
- Check table registration succeeded
- Verify table name (case-sensitive)
- Use List Tables to see registered tables

### "Query is slow"
- Check data source is accessible
- Use EXPLAIN to analyze query plan
- Consider partitioned data for large datasets

### "Memory error"
- Set memory limits on session creation
- Use LIMIT clauses
- Process data in chunks

## Next Steps

Now that you can query any data source:

- **[Machine Learning](/topics/datascience/ml/)** – Build ML models on query results
- **[Data Visualization](/topics/datascience/visualization/)** – Create charts from SQL results
- **[AI-Powered Analysis](/topics/datascience/ai-analysis/)** – Let AI agents query your data
