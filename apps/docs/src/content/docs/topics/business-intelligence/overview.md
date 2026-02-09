---
title: Business Intelligence
description: Build reports, dashboards, and self-service analytics with Flow-Like
sidebar:
  order: 1
---

Flow-Like provides a complete Business Intelligence toolkit—connect to any data source, query with SQL, build interactive dashboards, and share insights across your organization.

## What You Can Build

| Solution | Description |
|----------|-------------|
| **Executive Dashboards** | KPIs, trends, real-time metrics |
| **Operational Reports** | Daily/weekly/monthly business reports |
| **Self-Service Analytics** | Let users explore data themselves |
| **Embedded Analytics** | Add BI to your existing apps |
| **Automated Reports** | Scheduled delivery via email/Slack |
| **Ad-Hoc Analysis** | Quick data exploration and insights |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Data Sources                         │
├─────────────────────────────────────────────────────────┤
│  Databases    │  Files     │  APIs      │  Data Lakes  │
│  PostgreSQL   │  CSV       │  REST      │  Delta Lake  │
│  MySQL        │  Excel     │  GraphQL   │  Iceberg     │
│  ClickHouse   │  Parquet   │  Webhooks  │  S3/Azure    │
└───────────────┴────────────┴────────────┴──────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│              DataFusion SQL Engine                      │
│  • Federated queries across sources                     │
│  • Real-time aggregations                               │
│  • Window functions & analytics                         │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                  Visualization Layer                    │
│  • 25+ chart types (Nivo)                              │
│  • Interactive tables                                   │
│  • Filters & drill-downs                               │
│  • Export to PDF/CSV                                    │
└─────────────────────────────────────────────────────────┘
```

## Connecting Data Sources

### Databases

Connect to production databases or data warehouses:

```
Create DataFusion Session
    │
    ▼
Register PostgreSQL ("sales_db", connection_string)
    │
    ▼
Register ClickHouse ("analytics_dw", connection_string)
    │
    ▼
Ready to query both with SQL
```

**Supported databases:**
- PostgreSQL, MySQL, SQLite
- ClickHouse, DuckDB
- Oracle, SQL Server (via connectors)

### Files

Query files directly as tables:

```
Register CSV ("monthly_targets", /data/targets.csv)
    │
    ▼
Register Parquet ("transactions", s3://bucket/transactions/)
    │
    ▼
SQL: "SELECT * FROM monthly_targets t
      JOIN transactions tx ON t.month = tx.month"
```

### Data Lakes

Connect to modern data platforms:

```
Register Delta Lake ("customers", s3://lake/customers)
    │
    ▼
Register Iceberg ("orders", s3://lake/orders)
    │
    ▼
Query with time travel, partitioning, schema evolution
```

### Live APIs

Pull real-time data from APIs:

```
HTTP Request (CRM API) ──▶ Parse JSON ──▶ Register as Table
    │
    ▼
Join with database data in single query
```

## SQL Analytics with DataFusion

### Basic Aggregations

```sql
SELECT
    region,
    COUNT(*) as order_count,
    SUM(amount) as total_revenue,
    AVG(amount) as avg_order_value
FROM orders
WHERE order_date >= '2024-01-01'
GROUP BY region
ORDER BY total_revenue DESC
```

### Time-Series Analysis

```sql
SELECT
    DATE_TRUNC('week', order_date) as week,
    SUM(amount) as weekly_revenue,
    SUM(amount) - LAG(SUM(amount)) OVER (ORDER BY DATE_TRUNC('week', order_date)) as wow_change
FROM orders
GROUP BY DATE_TRUNC('week', order_date)
ORDER BY week
```

### Window Functions

```sql
SELECT
    product_name,
    category,
    revenue,
    RANK() OVER (PARTITION BY category ORDER BY revenue DESC) as category_rank,
    revenue / SUM(revenue) OVER (PARTITION BY category) * 100 as pct_of_category
FROM product_sales
```

### Cross-Source Joins

Query across different databases in one statement:

```sql
SELECT
    c.name as customer,
    c.segment,
    SUM(o.amount) as total_spent,
    COUNT(DISTINCT o.id) as order_count
FROM postgres_crm.customers c
JOIN clickhouse_dw.orders o ON c.id = o.customer_id
WHERE o.order_date >= CURRENT_DATE - INTERVAL '90 days'
GROUP BY c.name, c.segment
ORDER BY total_spent DESC
LIMIT 100
```

### Cohort Analysis

```sql
WITH first_purchase AS (
    SELECT
        customer_id,
        DATE_TRUNC('month', MIN(order_date)) as cohort_month
    FROM orders
    GROUP BY customer_id
),
monthly_activity AS (
    SELECT
        customer_id,
        DATE_TRUNC('month', order_date) as activity_month
    FROM orders
)
SELECT
    fp.cohort_month,
    DATE_DIFF('month', fp.cohort_month, ma.activity_month) as months_since_first,
    COUNT(DISTINCT ma.customer_id) as active_customers
FROM first_purchase fp
JOIN monthly_activity ma ON fp.customer_id = ma.customer_id
GROUP BY fp.cohort_month, DATE_DIFF('month', fp.cohort_month, ma.activity_month)
ORDER BY fp.cohort_month, months_since_first
```

## Building Dashboards

### Dashboard Layout

```
Page: /analytics/sales
├── Grid (3 columns)
│   ├── KPI Card: Total Revenue
│   ├── KPI Card: Orders Today
│   └── KPI Card: Conversion Rate
│
├── Row
│   ├── Line Chart: Revenue Trend (60%)
│   └── Pie Chart: Revenue by Region (40%)
│
├── Row
│   ├── Bar Chart: Top Products
│   └── Heatmap: Sales by Day/Hour
│
└── Table: Recent Orders (full width)
```

### KPI Cards

```
Card Component
├── Text (label): "Total Revenue"
├── Text (value): {$metrics.total_revenue | currency}
├── Text (change): "↑ {$metrics.revenue_change}% vs last month"
├── Sparkline: {$metrics.revenue_trend}
└── Style: Conditional color based on change
```

### Interactive Filters

```
Row (filters)
├── Select: Date Range
│   └── Options: Today, 7 days, 30 days, 90 days, YTD, Custom
├── Select: Region
│   └── Options: All, North, South, East, West
├── Select: Product Category
│   └── Options: (dynamic from data)
└── Button: Apply Filters
    └── onClick: Refresh dashboard data
```

**Filter flow:**
```
Filter Change
    │
    ▼
Update filter variables
    │
    ▼
Re-run SQL queries with filters
    │
    ▼
Update all visualizations
```

### Drill-Down Navigation

```
Bar Chart: Revenue by Category
    │
    └── onClick (category)
            │
            ▼
        Navigate to /analytics/category/{category_id}
            │
            ▼
        Category detail page with:
        ├── Products in category
        ├── Trend over time
        └── Customer breakdown
```

## Chart Types for BI

### Comparison Charts

| Chart | Use Case |
|-------|----------|
| **Bar** | Compare categories |
| **Grouped Bar** | Compare categories across dimensions |
| **Stacked Bar** | Show composition within categories |
| **Bullet** | Actual vs target comparison |

### Trend Charts

| Chart | Use Case |
|-------|----------|
| **Line** | Time series trends |
| **Area** | Cumulative trends |
| **Stream** | Category trends over time |
| **Bump** | Ranking changes over time |

### Distribution Charts

| Chart | Use Case |
|-------|----------|
| **Histogram** | Value distribution |
| **Box Plot** | Statistical distribution |
| **Scatter** | Correlation analysis |
| **Swarm Plot** | Dense distributions |

### Part-to-Whole Charts

| Chart | Use Case |
|-------|----------|
| **Pie** | Simple proportions |
| **Donut** | Proportions with center metric |
| **Treemap** | Hierarchical proportions |
| **Sunburst** | Multi-level hierarchy |
| **Waffle** | Discrete proportions |

### Relationship Charts

| Chart | Use Case |
|-------|----------|
| **Sankey** | Flow between categories |
| **Chord** | Relationships between groups |
| **Network** | Connection networks |

### Specialized Charts

| Chart | Use Case |
|-------|----------|
| **Heatmap** | 2D value density |
| **Calendar** | Daily patterns |
| **Radar** | Multi-variable comparison |
| **Funnel** | Conversion funnels |
| **Gauge** | Single metric progress |

## Tables for Data Exploration

### Full-Featured Data Table

```
Table Component
├── data: {$query_results}
├── columns:
│   ├── customer (sortable, filterable)
│   ├── region (sortable, filterable, groupable)
│   ├── revenue (sortable, format: currency)
│   ├── orders (sortable)
│   └── last_order (sortable, format: date)
├── Features:
│   ├── Sorting (click headers)
│   ├── Filtering (column filters)
│   ├── Pagination (50 per page)
│   ├── Column resizing
│   ├── Column reordering
│   └── Export (CSV, Excel)
└── Actions:
    └── Row click → Customer detail
```

### Pivot Table Pattern

Create pivot-like views with SQL:

```sql
SELECT
    region,
    SUM(CASE WHEN month = 'Jan' THEN revenue END) as jan,
    SUM(CASE WHEN month = 'Feb' THEN revenue END) as feb,
    SUM(CASE WHEN month = 'Mar' THEN revenue END) as mar,
    SUM(revenue) as total
FROM monthly_sales
GROUP BY region
```

## Automated Reporting

### Scheduled Reports

```
Scheduled Event (Monday 8am)
    │
    ▼
Run analytics queries
    │
    ▼
Generate visualizations
    │
    ▼
Render report template
    │
    ▼
Convert to PDF
    │
    ├──▶ Email to stakeholders
    ├──▶ Upload to SharePoint
    └──▶ Post to Slack (#reports)
```

### Report Template

```markdown
# Weekly Sales Report
**Period:** {start_date} to {end_date}
**Generated:** {generated_at}

## Executive Summary
- Total Revenue: {total_revenue}
- Orders: {order_count}
- New Customers: {new_customers}

## Revenue Trend
{revenue_chart}

## Top Performing Products
{products_table}

## Regional Breakdown
{region_chart}

## Key Insights
{ai_generated_insights}
```

### AI-Powered Insights

```
Query results
    │
    ▼
Invoke LLM
├── prompt: "Analyze this sales data and provide 3-5 key insights:
│            {data_summary}
│            Focus on: trends, anomalies, opportunities"
└── model: GPT-4
    │
    ▼
Formatted insights for report
```

## Self-Service Analytics

### Query Builder UI

Let business users build queries visually:

```
Page: /analytics/explorer
├── Data Source Selector
│   └── Available tables and columns
│
├── Query Builder
│   ├── Select columns (drag & drop)
│   ├── Add filters (visual builder)
│   ├── Group by (drag columns)
│   └── Sort order
│
├── Preview
│   └── Live SQL preview
│
├── Results
│   ├── Table view
│   └── Chart view (select type)
│
└── Actions
    ├── Save as Report
    ├── Export Data
    └── Schedule
```

### Saved Reports

```
Board Variables:
├── saved_reports: Array<Report>
└── shared_reports: Array<Report>

Quick Action: Save Report
├── name: user_input
├── query: current_query
├── filters: current_filters
├── visualization: current_chart_config
└── permissions: private/team/public
```

### Sharing & Collaboration

```
Report Sharing
├── Private (only creator)
├── Team (specific team members)
├── Public (anyone with link)
└── Embedded (iframe in other apps)
```

## Real-Time Dashboards

### Live Data Updates

```
Page Load
    │
    ▼
Initial data fetch
    │
    ▼
Set up polling (every 30 seconds)
    │
    ▼
On poll:
├── Fetch updated metrics
├── Compare with previous
├── Update visualizations
└── Highlight changes (animations)
```

### Streaming Metrics

```
Scheduled Event (every 10 seconds)
    │
    ▼
Query real-time metrics:
├── Active users
├── Orders in progress
├── Current revenue
└── System health
    │
    ▼
Update dashboard variables
    │
    ▼
UI reactively updates
```

## Data Modeling

### Semantic Layer

Define business metrics once, use everywhere:

```
Board: MetricsDefinitions
├── Variables:
│   └── metric_definitions: {
│       "revenue": "SUM(order_amount)",
│       "aov": "AVG(order_amount)",
│       "conversion_rate": "COUNT(orders) / COUNT(visits) * 100",
│       "customer_lifetime_value": "SUM(amount) / COUNT(DISTINCT customer_id)"
│   }
│
└── Quick Action: Calculate Metric (metric_name, filters)
        │
        ▼
    Build SQL with metric definition
        │
        ▼
    Execute and return result
```

### Calculated Fields

```sql
-- Revenue with returns adjustment
SELECT
    date,
    gross_revenue,
    returns,
    gross_revenue - returns as net_revenue,
    (gross_revenue - returns) / gross_revenue * 100 as net_revenue_pct
FROM daily_sales

-- Customer health score
SELECT
    customer_id,
    recency_score * 0.3 +
    frequency_score * 0.3 +
    monetary_score * 0.4 as health_score
FROM customer_rfm
```

## Example: Sales Analytics Dashboard

```
Board: SalesDashboard
├── Variables:
│   ├── date_range: { start, end }
│   ├── region_filter: "all"
│   ├── metrics: {}
│   └── chart_data: {}
│
└── Init Event
        │
        ▼
    Create DataFusion Session
        │
        ▼
    Register data sources
        │
        ├──▶ Query: KPI Metrics
        │       │
        │       ▼
        │   Set metrics variable
        │
        ├──▶ Query: Revenue Trend
        │       │
        │       ▼
        │   Set chart_data.trend
        │
        ├──▶ Query: Revenue by Region
        │       │
        │       ▼
        │   Set chart_data.regions
        │
        └──▶ Query: Top Products
                │
                ▼
            Set chart_data.products

Page Layout:
├── Header
│   ├── Title: "Sales Analytics"
│   ├── Date Range Picker
│   └── Region Filter
│
├── KPI Row
│   ├── Card: Revenue ({metrics.revenue})
│   ├── Card: Orders ({metrics.orders})
│   ├── Card: AOV ({metrics.aov})
│   └── Card: Conversion ({metrics.conversion})
│
├── Charts Row
│   ├── LineChart: Revenue Trend (chart_data.trend)
│   └── PieChart: By Region (chart_data.regions)
│
└── Table: Top Products (chart_data.products)
```

## Best Practices

### 1. Optimize Queries
```sql
-- Use aggregations in the database, not in Flow-Like
-- Filter early, aggregate late
-- Use appropriate indexes
-- Cache frequently-used data
```

### 2. Design for Users
- Start with KPIs (what matters most)
- Progressive disclosure (summary → detail)
- Consistent formatting (dates, currencies)
- Clear labels (no jargon)

### 3. Performance
- Limit data returned (pagination, top N)
- Pre-aggregate when possible
- Cache slowly-changing data
- Use incremental updates

### 4. Governance
- Document metric definitions
- Version control reports
- Access control on sensitive data
- Audit trail for changes

## Integration Examples

### Embed in Existing Apps

```html
<iframe
  src="https://your-flow-like-app/embed/dashboard/sales"
  width="100%"
  height="600"
></iframe>
```

### Export to Data Tools

```
Dashboard data
    │
    ▼
Export as:
├── CSV (for Excel)
├── Parquet (for data tools)
├── JSON (for APIs)
└── PDF (for sharing)
```

### Connect to BI Tools

Flow-Like can feed data to traditional BI tools:

```
Scheduled Event
    │
    ▼
Run analytics queries
    │
    ▼
Write to PostgreSQL (analytics schema)
    │
    ▼
Tableau/PowerBI reads from PostgreSQL
```

## FAQ

### Can it replace Tableau/PowerBI?
For many use cases, yes. Flow-Like excels at custom dashboards and automated reporting. Traditional BI tools may still be better for very complex ad-hoc exploration.

### How does performance compare?
DataFusion is highly optimized. For most queries, performance is excellent. Very large datasets may benefit from a dedicated data warehouse.

### Can non-technical users build reports?
Yes—the visual query builder and pre-built templates make it accessible. Power users can write SQL directly.

### How do I handle large datasets?
Use aggregations, partitioning, and caching. Connect to a data warehouse for very large volumes.

## Next Steps

- **[DataFusion & SQL](/topics/datascience/datafusion/)** – Deep dive into queries
- **[Data Visualization](/topics/datascience/visualization/)** – Chart configuration
- **[Building Internal Tools](/topics/internal-tools/overview/)** – Dashboard building
- **[Data Pipelines](/topics/data-pipelines/overview/)** – ETL for analytics
