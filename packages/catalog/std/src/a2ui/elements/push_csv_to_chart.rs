use super::chart_data_utils::{clean_field_name, extract_from_csv_table, parse_csv_text};
use super::element_utils::extract_element_id;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::Map, json::json};

/// Supported chart libraries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartLibrary {
    Nivo,
    Plotly,
}

impl ChartLibrary {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "nivo" => Some(Self::Nivo),
            "plotly" => Some(Self::Plotly),
            _ => None,
        }
    }
}

/// Supported chart types for automatic data transformation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
    Area,
    Radar,
    Heatmap,
    Calendar,
    Sankey,
    Tree,
}

impl ChartType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bar" => Some(Self::Bar),
            "line" => Some(Self::Line),
            "pie" => Some(Self::Pie),
            "scatter" => Some(Self::Scatter),
            "area" => Some(Self::Area),
            "radar" => Some(Self::Radar),
            "heatmap" => Some(Self::Heatmap),
            "calendar" => Some(Self::Calendar),
            "sankey" => Some(Self::Sankey),
            "tree" | "treemap" => Some(Self::Tree),
            _ => None,
        }
    }
}

/// Push CSV/table data to a chart element (Nivo or Plotly).
///
/// Automatically transforms data to the correct format based on chart type and library.
#[crate::register_node]
#[derive(Default)]
pub struct PushCsvToChart;

impl PushCsvToChart {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for PushCsvToChart {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_push_csv_to_chart",
            "Push CSV to Chart",
            "Push CSV or table data to a chart. Supports Nivo and Plotly.",
            "UI/Elements/Charts",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Chart",
            "Reference to the chart element",
            VariableType::Struct,
        )
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin("library", "Library", "Chart library", VariableType::String)
            .set_options(
                PinOptions::new()
                    .set_valid_values(vec!["Nivo".to_string(), "Plotly".to_string()])
                    .build(),
            )
            .set_default_value(Some(json!("Nivo")));

        node.add_input_pin(
            "chart_type",
            "Chart Type",
            "Type of chart",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "Bar".to_string(),
                    "Line".to_string(),
                    "Pie".to_string(),
                    "Scatter".to_string(),
                    "Area".to_string(),
                    "Radar".to_string(),
                    "Heatmap".to_string(),
                    "Calendar".to_string(),
                    "Sankey".to_string(),
                    "Tree".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("Bar")));

        node.add_input_pin("csv", "CSV", "CSV text with headers", VariableType::String)
            .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "table",
            "Table",
            "Table data from DataFusion query",
            VariableType::Struct,
        )
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "CSV delimiter (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin("exec_out", "▶", "", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid chart element reference"))?;

        let library_str: String = context.evaluate_pin("library").await?;
        let library = ChartLibrary::from_str(&library_str)
            .ok_or_else(|| flow_like_types::anyhow!("Unknown library: {}", library_str))?;

        let chart_type_str: String = context.evaluate_pin("chart_type").await?;
        let chart_type = ChartType::from_str(&chart_type_str)
            .ok_or_else(|| flow_like_types::anyhow!("Unknown chart type: {}", chart_type_str))?;

        let delimiter: String = context.evaluate_pin("delimiter").await?;

        // Get data from either table or CSV
        let (headers, rows) = if let Ok(table_value) = context.evaluate_pin::<Value>("table").await
        {
            if !table_value.is_null() {
                extract_from_csv_table(&table_value)?
            } else {
                let csv_text: String = context.evaluate_pin("csv").await?;
                parse_csv_text(&csv_text, delimiter.chars().next().unwrap_or(','))?
            }
        } else {
            let csv_text: String = context.evaluate_pin("csv").await?;
            parse_csv_text(&csv_text, delimiter.chars().next().unwrap_or(','))?
        };

        if headers.is_empty() || rows.is_empty() {
            let update_value = match library {
                ChartLibrary::Nivo => json!({ "type": "setNivoData", "data": [] }),
                ChartLibrary::Plotly => json!({ "type": "setChartData", "data": [] }),
            };
            context.upsert_element(&element_id, update_value).await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        match library {
            ChartLibrary::Nivo => {
                push_nivo_data(context, &element_id, chart_type, &headers, &rows).await?
            }
            ChartLibrary::Plotly => {
                push_plotly_data(context, &element_id, chart_type, &headers, &rows).await?
            }
        }

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

async fn push_nivo_data(
    context: &mut ExecutionContext,
    element_id: &str,
    chart_type: ChartType,
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<()> {
    let (data, config) = transform_for_nivo(chart_type, headers, rows)?;

    context
        .upsert_element(element_id, json!({ "type": "setNivoData", "data": data }))
        .await?;

    if let Some(cfg) = config {
        context
            .upsert_element(
                element_id,
                json!({ "type": "setNivoConfig", "config": cfg }),
            )
            .await?;
    }

    Ok(())
}

async fn push_plotly_data(
    context: &mut ExecutionContext,
    element_id: &str,
    chart_type: ChartType,
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<()> {
    let traces = transform_for_plotly(chart_type, headers, rows)?;

    context
        .upsert_element(
            element_id,
            json!({ "type": "setChartData", "data": traces }),
        )
        .await?;

    Ok(())
}

// ============================================================================
// NIVO TRANSFORMATIONS
// ============================================================================

fn transform_for_nivo(
    chart_type: ChartType,
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    match chart_type {
        ChartType::Bar | ChartType::Radar => nivo_bar(headers, rows),
        ChartType::Line | ChartType::Area => nivo_line(headers, rows),
        ChartType::Pie => nivo_pie(headers, rows),
        ChartType::Scatter => nivo_scatter(headers, rows),
        ChartType::Heatmap => nivo_heatmap(headers, rows),
        ChartType::Calendar => nivo_calendar(headers, rows),
        ChartType::Sankey => nivo_sankey(headers, rows),
        ChartType::Tree => nivo_tree(headers, rows),
    }
}

fn nivo_bar(
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let index_field = clean_field_name(headers.first().map(|s| s.as_str()).unwrap_or("category"));
    let keys: Vec<String> = headers[1..].iter().map(|h| clean_field_name(h)).collect();

    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut obj = Map::new();
            obj.insert(
                index_field.clone(),
                json!(row.first().cloned().unwrap_or_default()),
            );
            for (i, key) in keys.iter().enumerate() {
                let val: f64 = row.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                obj.insert(key.clone(), json!(val));
            }
            Value::Object(obj)
        })
        .collect();

    Ok((
        json!(data),
        Some(json!({ "keys": keys, "indexBy": index_field })),
    ))
}

fn nivo_line(
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let series: Vec<Value> = (1..headers.len())
        .map(|y_idx| {
            let name = clean_field_name(&headers[y_idx]);
            let points: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let x = row.first().cloned().unwrap_or_default();
                    let y: f64 = row.get(y_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    json!({ "x": x, "y": y })
                })
                .collect();
            json!({ "id": name, "data": points })
        })
        .collect();

    Ok((json!(series), None))
}

fn nivo_pie(
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let value_idx = if headers.len() > 1 { 1 } else { 0 };
    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            let label = row.first().cloned().unwrap_or_default();
            let value: f64 = row
                .get(value_idx)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            json!({ "id": clean_field_name(&label), "label": label, "value": value })
        })
        .collect();

    Ok((json!(data), None))
}

fn nivo_scatter(
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    // Group by series if 3+ columns, otherwise single series
    if headers.len() >= 3 {
        let mut groups: std::collections::HashMap<String, Vec<Value>> =
            std::collections::HashMap::new();
        for row in rows {
            let x: f64 = row.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y: f64 = row.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let group = row.get(2).cloned().unwrap_or_else(|| "default".to_string());
            groups
                .entry(group)
                .or_default()
                .push(json!({ "x": x, "y": y }));
        }
        let series: Vec<Value> = groups
            .into_iter()
            .map(|(id, data)| json!({ "id": id, "data": data }))
            .collect();
        Ok((json!(series), None))
    } else {
        let points: Vec<Value> = rows
            .iter()
            .map(|row| {
                let x: f64 = row.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let y: f64 = row.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                json!({ "x": x, "y": y })
            })
            .collect();
        Ok((json!([{ "id": "data", "data": points }]), None))
    }
}

fn nivo_heatmap(
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let col_headers: Vec<String> = headers[1..].iter().map(|s| clean_field_name(s)).collect();
    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            let row_id = clean_field_name(row.first().map(|s| s.as_str()).unwrap_or("row"));
            let cells: Vec<Value> = col_headers
                .iter()
                .enumerate()
                .map(|(i, col)| {
                    let val: f64 = row.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    json!({ "x": col, "y": val })
                })
                .collect();
            json!({ "id": row_id, "data": cells })
        })
        .collect();

    Ok((json!(data), None))
}

fn nivo_calendar(
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let value_idx = if headers.len() > 1 { 1 } else { 0 };
    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            let day = row.first().cloned().unwrap_or_default();
            let value: f64 = row
                .get(value_idx)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            json!({ "day": day, "value": value })
        })
        .collect();

    Ok((json!(data), None))
}

fn nivo_sankey(
    _headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let mut nodes_set = std::collections::HashSet::new();
    let links: Vec<Value> = rows
        .iter()
        .map(|row| {
            let source = row.first().cloned().unwrap_or_default();
            let target = row.get(1).cloned().unwrap_or_default();
            let value: f64 = row.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            nodes_set.insert(source.clone());
            nodes_set.insert(target.clone());
            json!({ "source": source, "target": target, "value": value })
        })
        .collect();

    let nodes: Vec<Value> = nodes_set
        .into_iter()
        .map(|id| json!({ "id": id }))
        .collect();
    Ok((json!({ "nodes": nodes, "links": links }), None))
}

fn nivo_tree(
    _headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<(Value, Option<Value>)> {
    let nodes: Vec<Value> = rows
        .iter()
        .map(|row| {
            let id = row.first().cloned().unwrap_or_default();
            let parent = row.get(1).cloned().unwrap_or_default();
            let value: f64 = row.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            json!({
                "id": id,
                "parent": if parent.is_empty() { Value::Null } else { json!(parent) },
                "value": value
            })
        })
        .collect();

    Ok((json!(nodes), None))
}

// ============================================================================
// PLOTLY TRANSFORMATIONS
// ============================================================================

fn transform_for_plotly(
    chart_type: ChartType,
    headers: &[String],
    rows: &[Vec<String>],
) -> flow_like_types::Result<Value> {
    match chart_type {
        ChartType::Bar => plotly_bar(headers, rows),
        ChartType::Line => plotly_line(headers, rows),
        ChartType::Pie => plotly_pie(headers, rows),
        ChartType::Scatter => plotly_scatter(headers, rows),
        ChartType::Area => plotly_area(headers, rows),
        ChartType::Heatmap => plotly_heatmap(headers, rows),
        // Plotly doesn't have native equivalents for these - fallback to bar
        ChartType::Radar | ChartType::Calendar | ChartType::Sankey | ChartType::Tree => {
            plotly_bar(headers, rows)
        }
    }
}

fn plotly_bar(headers: &[String], rows: &[Vec<String>]) -> flow_like_types::Result<Value> {
    let x: Vec<String> = rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();

    let traces: Vec<Value> = (1..headers.len())
        .map(|i| {
            let y: Vec<f64> = rows
                .iter()
                .map(|r| r.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0))
                .collect();
            json!({
                "x": x,
                "y": y,
                "name": clean_field_name(&headers[i]),
                "type": "bar"
            })
        })
        .collect();

    Ok(json!(traces))
}

fn plotly_line(headers: &[String], rows: &[Vec<String>]) -> flow_like_types::Result<Value> {
    let x: Vec<String> = rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();

    let traces: Vec<Value> = (1..headers.len())
        .map(|i| {
            let y: Vec<f64> = rows
                .iter()
                .map(|r| r.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0))
                .collect();
            json!({
                "x": x,
                "y": y,
                "name": clean_field_name(&headers[i]),
                "type": "scatter",
                "mode": "lines+markers"
            })
        })
        .collect();

    Ok(json!(traces))
}

fn plotly_scatter(headers: &[String], rows: &[Vec<String>]) -> flow_like_types::Result<Value> {
    if headers.len() >= 3 {
        let mut groups: std::collections::HashMap<String, (Vec<f64>, Vec<f64>)> =
            std::collections::HashMap::new();
        for row in rows {
            let x: f64 = row.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y: f64 = row.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let group = row.get(2).cloned().unwrap_or_else(|| "data".to_string());
            let entry = groups.entry(group).or_insert_with(|| (vec![], vec![]));
            entry.0.push(x);
            entry.1.push(y);
        }
        let traces: Vec<Value> = groups
            .into_iter()
            .map(|(name, (x, y))| {
                json!({ "x": x, "y": y, "name": name, "type": "scatter", "mode": "markers" })
            })
            .collect();
        Ok(json!(traces))
    } else {
        let x: Vec<f64> = rows
            .iter()
            .map(|r| r.first().and_then(|s| s.parse().ok()).unwrap_or(0.0))
            .collect();
        let y: Vec<f64> = rows
            .iter()
            .map(|r| r.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0))
            .collect();
        Ok(json!([{ "x": x, "y": y, "type": "scatter", "mode": "markers" }]))
    }
}

fn plotly_area(headers: &[String], rows: &[Vec<String>]) -> flow_like_types::Result<Value> {
    let x: Vec<String> = rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();

    let traces: Vec<Value> = (1..headers.len())
        .map(|i| {
            let y: Vec<f64> = rows
                .iter()
                .map(|r| r.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0))
                .collect();
            json!({
                "x": x,
                "y": y,
                "name": clean_field_name(&headers[i]),
                "type": "scatter",
                "fill": "tozeroy"
            })
        })
        .collect();

    Ok(json!(traces))
}

fn plotly_pie(headers: &[String], rows: &[Vec<String>]) -> flow_like_types::Result<Value> {
    let labels: Vec<String> = rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();
    let value_idx = if headers.len() > 1 { 1 } else { 0 };
    let values: Vec<f64> = rows
        .iter()
        .map(|r| r.get(value_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0))
        .collect();

    Ok(json!([{ "labels": labels, "values": values, "type": "pie" }]))
}

fn plotly_heatmap(headers: &[String], rows: &[Vec<String>]) -> flow_like_types::Result<Value> {
    let y: Vec<String> = rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();
    let x: Vec<String> = headers[1..].iter().map(|h| clean_field_name(h)).collect();

    let z: Vec<Vec<f64>> = rows
        .iter()
        .map(|row| {
            (1..headers.len())
                .map(|i| row.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0))
                .collect()
        })
        .collect();

    Ok(json!([{ "x": x, "y": y, "z": z, "type": "heatmap" }]))
}
