"use client";

import { TextEditor } from "@tm9657/flow-like-ui";
import { useState } from "react";

const EXAMPLE_FULL_MARKDOWN = `# Markdown Debug Preview

This is a **comprehensive** markdown test with _various_ features.

## Text Formatting

Regular text, **bold**, *italic*, ~~strikethrough~~, \`inline code\`, and ***bold italic***.

## Lists

- Bullet item 1
- Bullet item 2
  - Nested item
- Bullet item 3

1. Numbered item
2. Another item
3. Third item

## Code Blocks

\`\`\`typescript
const greeting = "Hello, World!";
console.log(greeting);
\`\`\`

## Charts

### Nivo Bar Chart

\`\`\`nivo
type: bar
title: Monthly Sales
---
month,sales,expenses,profit
Jan,4200,3100,1100
Feb,5100,3400,1700
Mar,4800,3200,1600
Apr,6200,4100,2100
\`\`\`

### Plotly Line Chart

\`\`\`plotly
type: line
title: Temperature Trends
---
month,New York,London
Jan,-2,5
Feb,0,6
Mar,5,9
Apr,12,12
May,18,15
\`\`\`

## Tables

| Feature | Status | Notes |
|---------|--------|-------|
| Tables | ✅ | Working |
| Charts | ✅ | Nivo & Plotly |
| Code | ✅ | Syntax highlighting |

## Blockquotes

> This is a blockquote.
> It can span multiple lines.

## Links and Images

[Link to Google](https://google.com)

---

*End of markdown preview*
`;

const EXAMPLE_NIVO_CSV = `\`\`\`nivo
type: bar
title: Monthly Sales
---
month,sales,expenses,profit
Jan,4200,3100,1100
Feb,5100,3400,1700
Mar,4800,3200,1600
Apr,6200,4100,2100
May,5800,3900,1900
Jun,7100,4500,2600
\`\`\``;

const EXAMPLE_NIVO_LINE = `\`\`\`nivo
type: line
title: Stock Performance
colors: paired
---
date,AAPL,GOOGL,MSFT
Jan,150,140,310
Feb,155,145,320
Mar,148,150,315
Apr,160,155,330
May,165,148,340
Jun,170,160,355
\`\`\``;

const EXAMPLE_NIVO_PIE = `\`\`\`nivo
type: pie
title: Market Share
---
company,share
Apple,28
Samsung,21
Xiaomi,14
Oppo,10
Others,27
\`\`\``;

const EXAMPLE_NIVO_RADAR = `\`\`\`nivo
type: radar
title: Team Skills Assessment
---
skill,Frontend,Backend,DevOps
JavaScript,95,60,40
Python,30,90,70
React,90,20,15
Docker,25,70,95
SQL,40,85,50
AWS,35,65,90
\`\`\``;

const EXAMPLE_NIVO_HEATMAP = `\`\`\`nivo
type: heatmap
title: Weekly Activity
---
day,9am,12pm,3pm,6pm,9pm
Mon,45,78,62,38,15
Tue,58,95,71,42,22
Wed,52,88,68,45,18
Thu,65,92,75,55,28
Fri,48,85,60,72,45
\`\`\``;

const EXAMPLE_NIVO_JSON = `\`\`\`nivo
{
  "chartType": "bar",
  "data": [
    { "country": "USA", "burgers": 131, "fries": 85, "sandwiches": 72 },
    { "country": "Germany", "burgers": 95, "fries": 108, "sandwiches": 86 },
    { "country": "France", "burgers": 72, "fries": 102, "sandwiches": 95 },
    { "country": "UK", "burgers": 88, "fries": 95, "sandwiches": 110 }
  ],
  "indexBy": "country",
  "keys": ["burgers", "fries", "sandwiches"]
}
\`\`\``;

const EXAMPLE_PLOTLY_CSV = `\`\`\`plotly
type: bar
title: Quarterly Revenue
xLabel: Quarter
yLabel: Revenue ($M)
---
quarter,2023,2024
Q1,120,145
Q2,135,160
Q3,150,175
Q4,180,210
\`\`\``;

const EXAMPLE_PLOTLY_LINE = `\`\`\`plotly
type: line
title: Temperature Trends
xLabel: Month
yLabel: Temperature (°C)
---
month,New York,London,Tokyo
Jan,-2,5,6
Feb,0,6,7
Mar,5,9,11
Apr,12,12,16
May,18,15,20
Jun,23,18,24
\`\`\``;

const EXAMPLE_PLOTLY_SCATTER = `\`\`\`plotly
type: scatter
title: Height vs Weight
xLabel: Height (cm)
yLabel: Weight (kg)
---
height,weight
160,55
165,62
170,68
175,75
180,82
185,88
172,70
168,65
178,78
\`\`\``;

const EXAMPLE_PLOTLY_PIE = `\`\`\`plotly
type: pie
title: Browser Market Share
---
browser,share
Chrome,65
Safari,18
Firefox,8
Edge,5
Others,4
\`\`\``;

const EXAMPLE_PLOTLY_JSON = `\`\`\`plotly
{
  "data": [
    {
      "x": ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
      "y": [20, 14, 25, 16, 18, 22],
      "type": "scatter",
      "mode": "lines+markers",
      "name": "Series A",
      "marker": { "color": "#8884d8" }
    },
    {
      "x": ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
      "y": [12, 18, 15, 22, 14, 20],
      "type": "scatter",
      "mode": "lines+markers",
      "name": "Series B",
      "marker": { "color": "#82ca9d" }
    }
  ],
  "layout": {
    "title": "Custom Plotly Chart",
    "xaxis": { "title": "Month" },
    "yaxis": { "title": "Value" }
  }
}
\`\`\``;

const CHART_EXAMPLES = [
	{ title: "Nivo Bar (CSV)", content: EXAMPLE_NIVO_CSV },
	{ title: "Nivo Line (CSV)", content: EXAMPLE_NIVO_LINE },
	{ title: "Nivo Pie (CSV)", content: EXAMPLE_NIVO_PIE },
	{ title: "Nivo Radar (CSV)", content: EXAMPLE_NIVO_RADAR },
	{ title: "Nivo Heatmap (CSV)", content: EXAMPLE_NIVO_HEATMAP },
	{ title: "Nivo Bar (JSON)", content: EXAMPLE_NIVO_JSON },
	{ title: "Plotly Bar (CSV)", content: EXAMPLE_PLOTLY_CSV },
	{ title: "Plotly Line (CSV)", content: EXAMPLE_PLOTLY_LINE },
	{ title: "Plotly Scatter (CSV)", content: EXAMPLE_PLOTLY_SCATTER },
	{ title: "Plotly Pie (CSV)", content: EXAMPLE_PLOTLY_PIE },
	{ title: "Plotly Custom (JSON)", content: EXAMPLE_PLOTLY_JSON },
];

export default function DebugMarkdownPage() {
	const [customMarkdown, setCustomMarkdown] = useState(EXAMPLE_FULL_MARKDOWN);

	return (
		<div className="container mx-auto py-8 space-y-8 px-2 md:px-4">
			<div>
				<h1 className="text-3xl font-bold mb-2">Markdown Debug Preview</h1>
				<p className="text-muted-foreground">
					Debug page for testing markdown rendering including{" "}
					<code>```nivo```</code> and <code>```plotly```</code> chart code
					blocks.
				</p>
			</div>

			{/* Live Editor */}
			<section className="space-y-4">
				<h2 className="text-xl font-semibold">Live Editor</h2>
				<div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
					<div>
						<label className="block text-sm font-medium mb-2">
							Raw Markdown
						</label>
						<textarea
							className="w-full h-[600px] p-4 font-mono text-sm bg-muted/50 rounded-md border resize-none"
							value={customMarkdown}
							onChange={(e) => setCustomMarkdown(e.target.value)}
						/>
					</div>
					<div>
						<label className="block text-sm font-medium mb-2">
							Rendered Output
						</label>
						<div className="h-[600px] p-4 bg-background border rounded-md overflow-auto">
							<TextEditor
								key={customMarkdown}
								initialContent={customMarkdown}
								isMarkdown={true}
								editable={false}
							/>
						</div>
					</div>
				</div>
			</section>

			{/* Chart Examples */}
			<section className="space-y-4">
				<h2 className="text-xl font-semibold">Chart Examples</h2>
				<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
					{CHART_EXAMPLES.map((example) => (
						<div
							key={example.title}
							className="border rounded-lg overflow-hidden"
						>
							<div className="bg-muted/50 px-4 py-2 flex items-center justify-between">
								<h3 className="font-medium text-sm">{example.title}</h3>
								<button
									type="button"
									className="text-xs text-muted-foreground hover:text-foreground"
									onClick={() => setCustomMarkdown(example.content)}
								>
									Load
								</button>
							</div>
							<div className="p-4 min-h-[300px]">
								<TextEditor
									initialContent={example.content}
									isMarkdown={true}
									editable={false}
								/>
							</div>
						</div>
					))}
				</div>
			</section>
		</div>
	);
}
