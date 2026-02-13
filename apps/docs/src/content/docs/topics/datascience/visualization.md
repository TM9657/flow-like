---
title: Data Visualization
description: Create charts and dashboards with Nivo and Plotly in A2UI
sidebar:
  order: 5
---

Flow-Like includes powerful visualization capabilities through **Nivo** and **Plotly** chart libraries. Create interactive charts directly in your A2UI interfaces—no frontend coding required.

## Chart Libraries

Flow-Like supports two chart libraries:

| Library | Best For | Chart Types |
|---------|----------|-------------|
| **Nivo** | Beautiful, declarative charts | 17 types |
| **Plotly** | Scientific, interactive charts | 9 types |

## Available Chart Types

### Nivo Charts

| Chart Type | Use Case |
|------------|----------|
| **Bar** | Compare categories |
| **Line** | Trends over time |
| **Pie** | Parts of a whole |
| **Scatter** | Correlations |
| **Radar** | Multi-dimensional comparison |
| **Heatmap** | Intensity matrices |
| **Funnel** | Conversion flows |
| **Treemap** | Hierarchical proportions |
| **Sunburst** | Hierarchical rings |
| **Calendar** | Daily patterns |
| **Bump** | Ranking changes |
| **Area Bump** | Area-based rankings |
| **Sankey** | Flow diagrams |
| **Stream** | Stacked time series |
| **Waffle** | Part-of-whole grids |
| **Radial Bar** | Circular bar charts |
| **Chord** | Relationship flows |

### Plotly Charts

| Chart Type | Use Case |
|------------|----------|
| **Bar** | Category comparison |
| **Line** | Time series |
| **Scatter** | Correlations, clusters |
| **Pie** | Proportions |
| **Area** | Cumulative trends |
| **Histogram** | Distribution |
| **Heatmap** | Matrix visualization |
| **Box** | Statistical distribution |
| **Violin** | Distribution shape |

## Creating Charts in A2UI

Charts are rendered using special markdown code blocks in A2UI content.

### Basic Syntax

```markdown
```nivo
type: bar
title: Sales by Region
---
region,sales
North,150
South,230
East,180
West,290
```
```

Or for Plotly:

```markdown
```plotly
type: bar
title: Sales by Region
---
region,sales
North,150
South,230
East,180
West,290
```
```

### Chart Configuration

The header section (before `---`) configures the chart:

| Option | Description | Default |
|--------|-------------|---------|
| `type` | Chart type | Auto-detected |
| `title` | Chart title | None |
| `xLabel` | X-axis label | None |
| `yLabel` | Y-axis label | None |
| `colors` | Color scheme or array | `nivo` |
| `height` | Chart height (pixels) | 400 |
| `showLegend` | Show legend | true |
| `legendPosition` | top, bottom, left, right | bottom |
| `stacked` | Stack bars/areas | false |
| `layout` | vertical, horizontal | vertical |
| `animate` | Enable animation | true |

### Color Schemes

Use predefined schemes or custom colors:

**Named schemes:**
- `nivo` – Default Nivo palette
- `paired` – Contrasting pairs
- `category10` – D3 category colors
- `accent` – Bold accent colors
- `dark2` – Dark palette
- `set1`, `set2`, `set3` – D3 sets
- `pastel1`, `pastel2` – Soft colors
- `spectral` – Rainbow spectrum

**Custom colors:**
```
colors: ["#ff6b6b", "#4ecdc4", "#45b7d1", "#96ceb4"]
```

## Chart Examples

### Bar Chart

```markdown
```nivo
type: bar
title: Quarterly Revenue
colors: paired
layout: vertical
---
quarter,revenue,profit
Q1,150000,45000
Q2,230000,67000
Q3,180000,52000
Q4,290000,84000
```
```

### Line Chart

```markdown
```nivo
type: line
title: User Growth
xLabel: Month
yLabel: Active Users
colors: set2
---
month,users,premium
Jan,1200,150
Feb,1450,180
Mar,1800,220
Apr,2100,290
May,2600,350
Jun,3200,420
```
```

### Pie Chart

```markdown
```nivo
type: pie
title: Market Share
colors: category10
showLegend: true
legendPosition: right
---
company,share
Acme Corp,35
TechCo,28
StartupX,22
Others,15
```
```

### Heatmap

```markdown
```nivo
type: heatmap
title: Weekly Activity
colors: spectral
---
day,Morning,Afternoon,Evening
Mon,45,82,65
Tue,52,88,58
Wed,48,75,70
Thu,55,90,62
Fri,42,85,78
Sat,30,65,85
Sun,28,55,72
```
```

### Scatter Plot

```markdown
```plotly
type: scatter
title: Price vs Quality
xLabel: Price ($)
yLabel: Rating
---
price,rating,product
29,4.2,Widget A
45,4.5,Widget B
35,3.8,Widget C
52,4.8,Widget D
28,3.5,Widget E
```
```

### Histogram

```markdown
```plotly
type: histogram
title: Age Distribution
xLabel: Age
yLabel: Count
---
age
25
28
32
35
29
41
38
45
33
27
36
42
31
39
44
```
```

### Box Plot

```markdown
```plotly
type: box
title: Salary by Department
yLabel: Salary ($K)
---
department,salary
Engineering,85
Engineering,92
Engineering,78
Engineering,105
Engineering,88
Sales,65
Sales,72
Sales,68
Sales,58
Sales,75
Marketing,70
Marketing,62
Marketing,68
Marketing,75
Marketing,71
```
```

## Auto-Detection

When you don't specify `type`, the system auto-detects:

| Data Pattern | Detected Type |
|--------------|---------------|
| 2 columns, ≤6 rows, numeric second | `pie` |
| 2 columns, >6 rows | `bar` |
| Time-like first column | `line` |
| 3+ columns | `bar` |

## JSON Mode (Full Control)

For complete customization, use JSON format:

### Nivo JSON

```markdown
```nivo
{
  "type": "bar",
  "data": [
    {"region": "North", "sales": 150, "profit": 45},
    {"region": "South", "sales": 230, "profit": 67}
  ],
  "keys": ["sales", "profit"],
  "indexBy": "region",
  "colors": {"scheme": "paired"},
  "margin": {"top": 50, "right": 130, "bottom": 50, "left": 60}
}
```
```

### Plotly JSON

```markdown
```plotly
{
  "data": [
    {
      "x": ["Jan", "Feb", "Mar", "Apr"],
      "y": [10, 15, 13, 17],
      "type": "scatter",
      "mode": "lines+markers",
      "name": "2024"
    },
    {
      "x": ["Jan", "Feb", "Mar", "Apr"],
      "y": [8, 12, 11, 15],
      "type": "scatter",
      "mode": "lines+markers",
      "name": "2023"
    }
  ],
  "layout": {
    "title": "Year over Year Comparison",
    "xaxis": {"title": "Month"},
    "yaxis": {"title": "Sales"}
  }
}
```
```

## Dynamic Charts from Data

In your flows, generate chart markdown dynamically:

### From SQL Query

```
SQL Query ──▶ Format as CSV ──▶ Build Chart Markdown ──▶ A2UI Output
     │              │                    │
     │              │                    └── "```nivo\ntype: bar\n---\n{csv}"
     │              └── "region,sales\nNorth,150\n..."
     └── CSVTable results
```

### Example Flow

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  SQL Query: "SELECT region, SUM(revenue) FROM sales        │
│              GROUP BY region"                               │
│       │                                                     │
│       ▼                                                     │
│  CSV Table ──▶ Convert to CSV String                       │
│       │              │                                      │
│       │              ▼                                      │
│       │         "region,revenue                            │
│       │          North,150000                              │
│       │          South,230000..."                          │
│       │              │                                      │
│       │              ▼                                      │
│       │         Concat Strings:                            │
│       │         "```nivo                                   │
│       │          type: bar                                 │
│       │          title: Revenue by Region                  │
│       │          ---                                       │
│       │          {csv_data}                                │
│       │          ```"                                      │
│       │              │                                      │
│       │              ▼                                      │
│       │         Push to A2UI                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Building Dashboards

Combine multiple charts in A2UI for dashboards:

```markdown
# Sales Dashboard

## Revenue Overview

```nivo
type: line
title: Monthly Revenue
---
month,revenue
Jan,150000
Feb,175000
...
```

## Regional Breakdown

```nivo
type: pie
title: Revenue by Region
---
region,revenue
North,450000
South,380000
...
```

## Performance Metrics

| Metric | Value | Change |
|--------|-------|--------|
| Total Revenue | $2.1M | +15% |
| Orders | 12,500 | +8% |
| Avg Order Value | $168 | +6% |
```

## Chart Selection Guide

| Data Type | Recommended Chart |
|-----------|-------------------|
| Compare categories | Bar |
| Show trend over time | Line |
| Parts of a whole | Pie, Waffle |
| Correlation | Scatter |
| Distribution | Histogram, Box, Violin |
| Hierarchical data | Treemap, Sunburst |
| Flow/conversion | Funnel, Sankey |
| Multi-dimensional comparison | Radar |
| Time patterns | Calendar, Stream |
| Ranking changes | Bump |
| Intensity/matrix | Heatmap |

## Best Practices

### 1. Choose the Right Chart
- Don't use pie charts for more than 5-6 categories
- Use line charts only for continuous data
- Use bar charts for discrete categories

### 2. Keep It Simple
- Remove unnecessary decorations
- Use clear, descriptive titles
- Label axes meaningfully

### 3. Use Consistent Colors
- Stick to one color scheme per dashboard
- Use color meaningfully (e.g., green=good, red=bad)
- Consider colorblind-friendly palettes

### 4. Consider Data Density
- Don't overcrowd charts with too many data points
- Aggregate data if needed
- Use interactive features (Plotly) for exploration

### 5. Provide Context
- Add titles that explain the insight
- Include comparison periods when relevant
- Show targets/benchmarks when useful

## Troubleshooting

### "Chart not rendering"
- Check markdown code block syntax (triple backticks)
- Verify CSV format (comma-separated, newline rows)
- Check for special characters in data

### "Wrong chart type"
- Explicitly set `type` in configuration
- Check data format matches chart requirements

### "Colors look wrong"
- Verify color scheme name spelling
- For custom colors, use valid hex codes
- Check theme compatibility (light/dark mode)

### "Data not showing correctly"
- Ensure numeric columns contain only numbers
- Check for header row in CSV data
- Verify column names match between config and data

## Next Steps

With visualization skills:

- **[DataFusion & SQL](/topics/datascience/datafusion/)** – Query data for charts
- **[AI-Powered Analysis](/topics/datascience/ai-analysis/)** – Generate insights with AI
- **[Machine Learning](/topics/datascience/ml/)** – Visualize ML results
