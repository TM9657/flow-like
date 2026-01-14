use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json, Value};
use super::chart_data_utils::{extract_from_csv_table, parse_csv_text, parse_column_ref};

/// Converts CSV data or CSVTable (from DataFusion) to Nivo Treemap/Sunburst hierarchical format.
///
/// **Output Format:** `{ name: "root", children: [{ name: "A", value: 100 }, { name: "B", children: [...] }] }`
///
/// **Documentation:**
/// - Treemap: https://nivo.rocks/treemap/
/// - Sunburst: https://nivo.rocks/sunburst/
/// - Circle Packing: https://nivo.rocks/circle-packing/
///
/// **Accepts:**
/// - Raw CSV text with headers
/// - CSVTable struct from DataFusion SQL queries
#[crate::register_node]
#[derive(Default)]
pub struct CsvToTreeData;

impl CsvToTreeData {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for CsvToTreeData {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_csv_to_tree_data",
            "CSV to Tree Data",
            "Converts CSV or DataFusion CSVTable to hierarchical format for Treemap/Sunburst/Circle Packing. Docs: https://nivo.rocks/treemap/",
            "A2UI/Elements/Charts/Hierarchical",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "csv",
            "CSV",
            "CSV with path and value columns. Path uses separator (e.g., 'Root/Category/Item')",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "table",
            "Table",
            "Alternative: CSVTable from DataFusion query",
            VariableType::Struct,
        )
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "path_column",
            "Path Column",
            "Column name or 0-based index for hierarchical path (default: 0)",
            VariableType::String,
        )
        .set_default_value(Some(json!("0")));

        node.add_input_pin(
            "value_column",
            "Value Column",
            "Column name or 0-based index for values (default: 1)",
            VariableType::String,
        )
        .set_default_value(Some(json!("1")));

        node.add_input_pin(
            "path_separator",
            "Path Separator",
            "Character separating path levels (default: /)",
            VariableType::String,
        )
        .set_default_value(Some(json!("/")));

        node.add_input_pin(
            "root_name",
            "Root Name",
            "Name for the root node (default: root)",
            VariableType::String,
        )
        .set_default_value(Some(json!("root")));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "CSV column delimiter for CSV text (default: comma)",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

        node.add_output_pin("data", "Data", "Hierarchical tree data object", VariableType::Generic);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let path_col: String = context.evaluate_pin("path_column").await?;
        let value_col: String = context.evaluate_pin("value_column").await?;
        let path_sep: String = context.evaluate_pin("path_separator").await?;
        let root_name: String = context.evaluate_pin("root_name").await?;
        let delimiter: String = context.evaluate_pin("delimiter").await?;

        let (headers, rows) = if let Ok(table_value) = context.evaluate_pin::<Value>("table").await {
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
            context.set_pin_value("data", json!({ "name": root_name, "children": [] })).await?;
            return Ok(());
        }

        let path_idx = parse_column_ref(&path_col, &headers);
        let value_idx = parse_column_ref(&value_col, &headers);

        let mut root = TreeNode::new(&root_name);

        for row in &rows {
            let path = row.get(path_idx).cloned().unwrap_or_default();
            let value: f64 = row.get(value_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);

            let parts: Vec<&str> = path.split(&path_sep).filter(|s| !s.is_empty()).collect();
            root.insert(&parts, value);
        }

        context.set_pin_value("data", root.to_json()).await?;

        Ok(())
    }
}

struct TreeNode {
    name: String,
    value: Option<f64>,
    children: Vec<TreeNode>,
}

impl TreeNode {
    fn new(name: &str) -> Self {
        Self { name: name.to_string(), value: None, children: Vec::new() }
    }

    fn insert(&mut self, path: &[&str], value: f64) {
        if path.is_empty() {
            self.value = Some(value);
            return;
        }

        let name = path[0];
        let rest = &path[1..];

        let child_idx = self.children.iter().position(|c| c.name == name);

        if let Some(idx) = child_idx {
            self.children[idx].insert(rest, value);
        } else {
            let mut child = TreeNode::new(name);
            child.insert(rest, value);
            self.children.push(child);
        }
    }

    fn to_json(&self) -> Value {
        if self.children.is_empty() {
            json!({
                "name": self.name,
                "value": self.value.unwrap_or(0.0)
            })
        } else {
            let children: Vec<Value> = self.children.iter().map(|c| c.to_json()).collect();
            json!({
                "name": self.name,
                "children": children
            })
        }
    }
}
