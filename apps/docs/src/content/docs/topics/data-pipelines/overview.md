---
title: Data Pipelines & ETL
description: Build reliable data pipelines for extraction, transformation, and loading
sidebar:
  order: 1
---

Flow-Like provides powerful ETL (Extract, Transform, Load) capabilities—move data between systems, transform it on the fly, and keep everything in sync with scheduled pipelines.

## Pipeline Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Extract   │────▶│  Transform  │────▶│    Load     │
│             │     │             │     │             │
│ - APIs      │     │ - Clean     │     │ - Database  │
│ - Databases │     │ - Map       │     │ - Data Lake │
│ - Files     │     │ - Aggregate │     │ - API       │
│ - Streams   │     │ - Enrich    │     │ - Files     │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Extract: Data Sources

### Databases

Query any database with DataFusion:

```
Create DataFusion Session
    │
    ▼
Register PostgreSQL ("source_db", connection_string)
    │
    ▼
SQL Query: "SELECT * FROM orders WHERE created_at > {last_sync}"
    │
    ▼
Result rows ──▶ Transform
```

**Supported databases:**
- PostgreSQL, MySQL, SQLite
- ClickHouse, DuckDB, Oracle
- SQL Server (via ODBC)

### APIs

Pull data from REST APIs:

```
HTTP Request
├── URL: "https://api.example.com/users"
├── Method: GET
├── Headers: { Authorization: "Bearer {token}" }
└── Pagination: handle next_page
    │
    ▼
Parse JSON ──▶ Array of records
```

**Pagination pattern:**
```
Variables:
├── all_records: Array = []
└── cursor: String | null

While (has_more_pages)
    │
    ▼
API Request (cursor)
    │
    ▼
Append results to all_records
    │
    ▼
Update cursor from response
    │
    ▼
Check has_more_pages
```

### Files

Read from various file sources:

```
┌─────────────────────────────────────────────────────────┐
│ File Sources                                            │
├─────────────────────────────────────────────────────────┤
│ Local Files:     Read to String, Buffered CSV Reader   │
│ S3:              S3 Provider ──▶ Get File              │
│ Azure Blob:      Azure Provider ──▶ Get File           │
│ GCS:             GCS Provider ──▶ Get File             │
│ SFTP:            HTTP Request (or custom)              │
│ SharePoint:      Microsoft Graph ──▶ Get File          │
└─────────────────────────────────────────────────────────┘
```

### Data Lakes

Query Delta Lake and Iceberg tables:

```
Create DataFusion Session
    │
    ▼
Register Delta Lake ("transactions", s3://bucket/delta/transactions)
    │
    ▼
SQL Query: "SELECT * FROM transactions WHERE partition_date = '2024-01-15'"
```

### Streaming Sources

Process data as it arrives:

```
HTTP Event (webhook)
    │
    ▼
Validate payload
    │
    ▼
Transform ──▶ Load ──▶ Acknowledge
```

## Transform: Data Processing

### Mapping & Cleaning

```
For Each record
    │
    ▼
Map fields:
├── id: record.customer_id
├── name: Trim(record.full_name)
├── email: Lowercase(record.email)
├── created_at: Parse Date(record.created)
└── status: Map status code to label
    │
    ▼
Transformed record
```

### SQL Transformations

Use DataFusion for complex transformations:

```
SQL Query:
"""
SELECT
    customer_id,
    DATE_TRUNC('month', order_date) as month,
    SUM(amount) as total_spent,
    COUNT(*) as order_count,
    AVG(amount) as avg_order_value
FROM orders
WHERE order_date >= '2024-01-01'
GROUP BY customer_id, DATE_TRUNC('month', order_date)
"""
```

### Joins & Enrichment

Combine data from multiple sources:

```
Create DataFusion Session
    │
    ▼
Register CSV ("orders", orders.csv)
    │
    ▼
Register PostgreSQL ("customers", connection)
    │
    ▼
SQL Query:
"""
SELECT
    o.*,
    c.name as customer_name,
    c.segment,
    c.lifetime_value
FROM orders o
JOIN customers c ON o.customer_id = c.id
"""
```

### AI-Powered Transformations

Use AI for complex parsing:

```
For Each raw_address
    │
    ▼
Extract Knowledge
├── Schema:
│   ├── street: String
│   ├── city: String
│   ├── state: String
│   ├── zip: String
│   └── country: String
└── Input: raw_address
    │
    ▼
Structured address
```

### Deduplication

Remove duplicate records:

```
SQL Query:
"""
SELECT DISTINCT ON (email) *
FROM users
ORDER BY email, created_at DESC
"""
```

Or in a flow:

```
Group By (email)
    │
    ▼
For Each group
    │
    ▼
Take first (most recent) ──▶ Deduplicated records
```

### Validation

Ensure data quality:

```
For Each record
    │
    ▼
Validate:
├── email: Matches email regex?
├── phone: Valid format?
├── amount: Positive number?
└── required_fields: All present?
    │
    ├── Valid ──▶ Continue to load
    │
    └── Invalid ──▶ Log to error_records
                      │
                      ▼
                 Alert / Review queue
```

## Load: Destinations

### Databases

Insert or upsert to databases:

```
For Each batch (1000 records)
    │
    ▼
SQL Execute:
"""
INSERT INTO customers (id, name, email, updated_at)
VALUES ($1, $2, $3, NOW())
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    email = EXCLUDED.email,
    updated_at = NOW()
"""
```

### Data Warehouses

Load to analytics platforms:

```
Transform data
    │
    ▼
Write to Parquet (staging)
    │
    ▼
Upload to S3
    │
    ▼
Databricks Execute SQL:
"""
COPY INTO bronze.customers
FROM 's3://bucket/staging/customers/'
FILEFORMAT = PARQUET
"""
```

### APIs

Push data to external systems:

```
For Each record
    │
    ▼
HTTP Request
├── URL: "https://api.crm.com/contacts"
├── Method: POST
├── Body: record
└── Handle rate limits (retry with backoff)
```

### Files

Write to file destinations:

```
Transform data
    │
    ▼
Write to CSV / Parquet / JSON
    │
    ▼
Upload to destination:
├── S3 Upload
├── Azure Blob Upload
├── SharePoint Upload
└── SFTP Upload
```

## Pipeline Patterns

### Full Sync

Replace all data:

```
Scheduled Event (daily at 2am)
    │
    ▼
Extract all from source
    │
    ▼
Transform
    │
    ▼
Truncate destination table
    │
    ▼
Load all records
    │
    ▼
Log: "Full sync complete: {count} records"
```

### Incremental Sync

Only process changes:

```
Scheduled Event (every 15 minutes)
    │
    ▼
Get last_sync_timestamp (from state)
    │
    ▼
Extract WHERE updated_at > last_sync_timestamp
    │
    ▼
Transform
    │
    ▼
Upsert to destination
    │
    ▼
Update last_sync_timestamp
    │
    ▼
Log: "Incremental sync: {count} records"
```

### CDC (Change Data Capture)

React to database changes:

```
HTTP Event (database webhook / Debezium)
    │
    ▼
Parse change event:
├── operation: INSERT/UPDATE/DELETE
├── before: previous state
└── after: new state
    │
    ▼
Transform
    │
    ▼
Apply to destination
```

### Fan-Out

Load to multiple destinations:

```
Extract
    │
    ▼
Transform
    │
    ├──▶ Load to Data Warehouse
    │
    ├──▶ Load to Search Index
    │
    ├──▶ Load to Cache
    │
    └──▶ Notify downstream systems
```

### Fan-In

Combine multiple sources:

```
┌─────────────────────────────────────┐
│ Extract from Source A               │
└───────────────┬─────────────────────┘
                │
┌───────────────┴─────────────────────┐
│ Extract from Source B               │
└───────────────┬─────────────────────┘
                │
┌───────────────┴─────────────────────┐
│ Extract from Source C               │
└───────────────┬─────────────────────┘
                │
                ▼
        Merge & Deduplicate
                │
                ▼
           Transform
                │
                ▼
        Load to destination
```

## Error Handling

### Retry Logic

```
Retry
├── max_attempts: 3
├── delay: 5000ms
├── backoff: exponential
└── retry_on: [timeout, 429, 503]
    │
    ▼
API Call / Database Query
```

### Dead Letter Queue

```
Try
    │
    ▼
Process record
    │
    └── Catch
            │
            ▼
        Insert to dead_letter_queue:
        ├── original_record
        ├── error_message
        ├── timestamp
        └── retry_count
```

### Checkpointing

Resume from failure:

```
For Each batch
    │
    ▼
Process batch
    │
    ▼
Save checkpoint:
├── batch_id: current_batch
├── processed_count: running_total
└── timestamp: now()
    │
    ▼
Next batch

// On restart:
Get last checkpoint ──▶ Resume from batch_id
```

### Alerting

```
Pipeline fails?
    │
    ▼
Slack Send Message
├── channel: "#data-alerts"
└── text: "❌ Pipeline failed: {pipeline_name}
           Error: {error_message}
           Failed at: {stage}
           Records processed: {count}"
```

## Scheduling

### Time-Based

```
Scheduled Event:
├── Every 15 minutes
├── Daily at 3:00 AM
├── Weekly on Monday at 6:00 AM
└── Monthly on 1st at midnight
```

### Event-Based

```
HTTP Event (trigger endpoint)
    │
    ▼
Validate trigger source
    │
    ▼
Run pipeline
```

### Dependency-Based

```
Pipeline A completes
    │
    ▼
Trigger Pipeline B
    │
    ▼
Trigger Pipeline C (parallel)
    │
    ▼
Wait for B and C
    │
    ▼
Trigger Pipeline D
```

## Monitoring & Observability

### Logging

```
Throughout pipeline:
├── Log: "Starting extract from {source}"
├── Log: "Extracted {count} records"
├── Log: "Transform complete: {success}/{total}"
├── Log: "Load complete: {inserted} inserted, {updated} updated"
└── Log: "Pipeline finished in {duration}ms"
```

### Metrics

Track pipeline health:

```
After each run:
    │
    ▼
Insert to pipeline_metrics:
├── pipeline_id
├── run_id
├── start_time
├── end_time
├── records_extracted
├── records_transformed
├── records_loaded
├── errors_count
└── status: success/failure
```

### Data Quality Metrics

```
After load:
    │
    ▼
Run validation queries:
├── Row count matches expected?
├── No NULL values in required fields?
├── Values within expected ranges?
├── Referential integrity intact?
    │
    ▼
Log/Alert on anomalies
```

## Example: Full ETL Pipeline

```
Board: CustomerDataPipeline
├── Variables:
│   ├── last_sync: DateTime
│   ├── batch_size: 1000
│   └── error_count: 0
│
└── Scheduled Event (every hour)
        │
        ▼
    ─────────────────────────────────
    │         EXTRACT              │
    ─────────────────────────────────
        │
        ▼
    Get last_sync from state
        │
        ▼
    API Request: Get customers (updated_since: last_sync)
        │
        ▼
    ─────────────────────────────────
    │        TRANSFORM             │
    ─────────────────────────────────
        │
        ▼
    For Each customer
        │
        ▼
    Try:
        │
        ├── Validate required fields
        │
        ├── Normalize phone numbers
        │
        ├── Standardize addresses
        │
        ├── Calculate customer_segment
        │
        └── Enrich with external data
                │
                ├── Success ──▶ Add to valid_records
                │
                └── Error ──▶ Add to error_records
                                │
                                ▼
                            Increment error_count
        │
        ▼
    ─────────────────────────────────
    │          LOAD                │
    ─────────────────────────────────
        │
        ▼
    For Each batch of valid_records
        │
        ▼
    Upsert to PostgreSQL
        │
        ▼
    ─────────────────────────────────
    │        CLEANUP               │
    ─────────────────────────────────
        │
        ▼
    Update last_sync to now()
        │
        ▼
    error_count > 0?
    ├── Yes ──▶ Send alert with error summary
        │
        ▼
    Log pipeline summary
```

## Best Practices

### 1. Idempotency
Design pipelines that can be safely re-run:
```
// Use UPSERT instead of INSERT
// Include run_id for deduplication
// Store processed record IDs
```

### 2. Batching
Process in batches to manage memory:
```
For Each batch (size: 1000)
    │
    ▼
Process batch ──▶ Commit ──▶ Next batch
```

### 3. Schema Evolution
Handle schema changes gracefully:
```
New field in source?
├── Add with default value
├── Backfill if needed
└── Update downstream consumers
```

### 4. Testing
Test pipelines with sample data:
```
// Test extract with mock API
// Test transform with edge cases
// Test load with rollback
```

### 5. Documentation
Document each pipeline:
```
// Source: What and where
// Schedule: When and why
// Transform: Business logic
// Dependencies: What must run first
// Contacts: Who to alert
```

## Next Steps

- **[DataFusion](/topics/datascience/datafusion/)** – SQL transformations
- **[API Integrations](/topics/api-integrations/overview/)** – Connect to sources
- **[Document Processing](/topics/document-processing/overview/)** – Process files
- **[Building Internal Tools](/topics/internal-tools/overview/)** – Pipeline dashboards
