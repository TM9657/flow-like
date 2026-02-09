---
title: Data Loading & Storage
description: Import, store, and manage data from files, databases, and cloud sources
sidebar:
  order: 2
---

Every data science project starts with data. Flow-Like provides comprehensive tools for loading data from various sources, storing it efficiently, and managing your data assets.

## Loading CSV Files

CSV is the most common data format. Flow-Like offers two approaches:

### Simple Read: Read to String

For smaller files, read the entire contents:

```
Read to String
    │
    ├── Path: (FlowPath to CSV)
    │
    └── Content ──▶ (string with CSV data)
```

### Streaming Read: Buffered CSV Reader

For large files, stream data in chunks to avoid memory issues:

```
Buffered CSV Reader
    │
    ├── Path: (FlowPath to CSV)
    ├── Chunk Size: 10000  (rows per batch)
    ├── Delimiter: ","
    │
    ├── On Chunk ──▶ (triggers for each batch)
    ├── Chunk ──▶ (current batch data)
    │
    └── Done ──▶ (file fully processed)
```

**When to use each:**

| Approach | File Size | Memory Usage | Use Case |
|----------|-----------|--------------|----------|
| Read to String | < 50MB | High | Quick analysis, small datasets |
| Buffered Reader | Any size | Controlled | ETL pipelines, large datasets |

## Loading Excel Files

Flow-Like provides comprehensive Excel support:

### Basic Operations

| Node | Purpose |
|------|---------|
| **Get Sheet Names** | List all sheets in a workbook |
| **Get Row** | Read a specific row |
| **Loop Rows** | Iterate through all rows |
| **Read Cell** | Read a specific cell value |

### Intelligent Table Extraction

The **Try Extract Tables** node automatically detects tables in Excel:

```
Try Extract Tables
    │
    ├── Path: (FlowPath to Excel)
    ├── Min Table Cells: 4
    ├── Max Header Rows: 3
    ├── Drop Totals: true
    ├── Group Similar Headers: true
    │
    └── Tables ──▶ (array of detected tables)
```

This is powerful for messy spreadsheets with:
- Multiple tables per sheet
- Headers spanning multiple rows
- Merged cells
- Total/summary rows

### Excel Workflow Example

```
Get Sheet Names ──▶ For Each Sheet ──▶ Try Extract Tables ──▶ Process
        │                  │                    │
        │                  │                    └── tables array
        │                  └── sheet name
        └── ["Sheet1", "Data", "Summary"]
```

## Loading JSON Files

### Parse with Schema Validation

For structured JSON, validate against a schema:

```
Parse with Schema
    │
    ├── JSON: (JSON string)
    ├── Schema: (JSON Schema definition)
    │
    ├── Valid ──▶ (parsing succeeded)
    ├── Result ──▶ (parsed object)
    │
    └── Invalid ──▶ (validation failed)
```

### Repair Malformed JSON

The **Repair Parse** node fixes common JSON issues:

```
Repair Parse
    │
    ├── Input: "{name: 'John', age: 30}"  (invalid JSON)
    │
    └── Result ──▶ {"name": "John", "age": 30}  (fixed)
```

Handles:
- Unquoted keys
- Single quotes
- Trailing commas
- Missing brackets

## Working with Parquet

Parquet is ideal for large analytical datasets:

```
Mount Parquet to DataFusion
    │
    ├── Path: (FlowPath to .parquet)
    ├── Table Name: "analytics"
    │
    └── Session ──▶ (DataFusion session with table)
```

Then query with SQL:
```sql
SELECT * FROM analytics WHERE date > '2025-01-01'
```

## Using App Storage

Every Flow-Like app has dedicated storage for files and databases.

### Uploading Files

1. Go to your app's **Storage** section
2. Click **Upload** or drag-and-drop files
3. Files are now accessible via FlowPath

### FlowPath Explained

FlowPath is Flow-Like's unified path system:

| Path Type | Example | Description |
|-----------|---------|-------------|
| **App Storage** | `storage://data/sales.csv` | Files in your app's storage |
| **Temp** | `temp://processing/output.csv` | Temporary files (cleared on restart) |
| **Absolute** | `/Users/me/data.csv` | Local filesystem (desktop only) |

### Creating Paths in Flows

```
Make FlowPath
    │
    ├── Scheme: "storage"
    ├── Path: "data/sales.csv"
    │
    └── Path ──▶ (FlowPath object)
```

## Database Storage (LanceDB)

Flow-Like includes LanceDB, a vector database for storing structured data:

### Opening/Creating a Database

```
Open Database
    │
    ├── Name: "my_dataset"
    │
    └── Database ──▶ (connection reference)
```

### Inserting Data

**Single Record:**
```
Insert
    │
    ├── Database: (connection)
    ├── Data: {"name": "John", "age": 30, "city": "NYC"}
    │
    └── End
```

**Batch Insert:**
```
Batch Insert
    │
    ├── Database: (connection)
    ├── Values: [array of records]
    │
    └── End
```

**From CSV:**
```
Batch Insert CSV
    │
    ├── Database: (connection)
    ├── CSV: (CSVTable data)
    │
    └── End
```

### Querying Data

| Node | Purpose | Use Case |
|------|---------|----------|
| **Filter** | SQL WHERE clause | Exact matches, ranges |
| **List** | Paginated listing | Browse all data |
| **Vector Search** | Similarity search | Find similar items |
| **FTS Search** | Full-text search | Keyword matching |
| **Hybrid Search** | Vector + FTS | Best of both |

```
Filter Database
    │
    ├── Database: (connection)
    ├── SQL Filter: "age > 25 AND city = 'NYC'"
    ├── Limit: 100
    │
    └── Results ──▶ (matching records)
```

### Database Maintenance

| Node | Purpose |
|------|---------|
| **Index** | Create indexes for faster queries |
| **Optimize** | Compact and optimize storage |
| **Purge** | Remove deleted records permanently |
| **Get Schema** | Inspect table structure |
| **Count** | Get record count |

## External Data Sources

### Cloud Storage

Connect to cloud object stores:

```
S3 Store
    │
    ├── Bucket: "my-data-bucket"
    ├── Region: "us-east-1"
    ├── Access Key: (secret)
    ├── Secret Key: (secret)
    │
    └── Store ──▶ (object store connection)
```

**Supported Providers:**
- AWS S3
- Azure Blob Storage
- Google Cloud Storage
- S3-compatible (MinIO, etc.)

### SaaS Integrations

Flow-Like connects to popular services:

| Service | Capabilities |
|---------|-------------|
| **GitHub** | Clone repos, issues, PRs, releases |
| **Notion** | Pages, databases, search |
| **Confluence** | Pages, spaces, comments |
| **Google Workspace** | Sheets, Drive, Calendar |
| **Microsoft 365** | Excel, OneDrive, SharePoint |
| **Databricks** | Query Databricks tables |

### Database Connections

Connect directly to databases for federated queries:

```
Register PostgreSQL
    │
    ├── Host: "db.example.com"
    ├── Port: 5432
    ├── Database: "analytics"
    ├── User: (secret)
    ├── Password: (secret)
    ├── Table: "transactions"
    ├── Alias: "txns"
    │
    └── Session ──▶ (DataFusion session)
```

Now query with SQL: `SELECT * FROM txns WHERE amount > 1000`

## Data Transformation

### File Operations

| Node | Purpose |
|------|---------|
| **Copy** | Duplicate a file |
| **Rename** | Change file name |
| **Delete** | Remove a file |
| **Exists** | Check if file exists |
| **List Paths** | List directory contents |
| **Sign URL** | Generate temporary download URL |

### Writing Output

**Write String:**
```
Write String
    │
    ├── Path: (FlowPath)
    ├── Content: "CSV data..."
    │
    └── End
```

**Write Bytes:**
```
Write Bytes
    │
    ├── Path: (FlowPath)
    ├── Bytes: (binary data)
    │
    └── End
```

## Best Practices

### 1. Use Appropriate Chunk Sizes
For streaming reads, balance memory vs. performance:
- Small chunks (1000): Low memory, slower
- Large chunks (50000): Fast, more memory

### 2. Index Your Databases
Create indexes on columns you filter frequently:
```
Index Database
    │
    ├── Database: (connection)
    ├── Columns: ["user_id", "date"]
```

### 3. Use Parquet for Analytics
Convert large CSVs to Parquet for:
- Faster queries (columnar)
- Better compression
- Type preservation

### 4. Organize Storage Logically
```
storage://
├── raw/           # Original files
├── processed/     # Cleaned data
├── models/        # Trained ML models
└── exports/       # Output files
```

### 5. Handle Errors Gracefully
Always check for file existence before reading:
```
Exists ──▶ Branch ──▶ Read File
              │
              └── (File not found) ──▶ Error handling
```

## Common Issues

### "File not found"
- Check the FlowPath scheme (storage://, temp://, etc.)
- Verify the file was uploaded to app storage
- Check for typos in the path

### "Out of memory on large files"
- Use Buffered CSV Reader with smaller chunk sizes
- Process data incrementally instead of loading all at once
- Consider converting to Parquet format

### "CSV parsing errors"
- Check delimiter settings (comma vs. semicolon)
- Verify encoding (UTF-8 is recommended)
- Look for unquoted special characters in data

## Next Steps

With your data loaded, continue to:

- **[DataFusion & SQL](/topics/datascience/datafusion/)** – Query and transform with SQL
- **[Machine Learning](/topics/datascience/ml/)** – Build predictive models
- **[Data Visualization](/topics/datascience/visualization/)** – Create charts and dashboards
