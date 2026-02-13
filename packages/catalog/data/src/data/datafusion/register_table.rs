use crate::data::datafusion::session::DataFusionSession;
use crate::data::excel::CSVTable;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct RegisterCSVTableNode {}

impl RegisterCSVTableNode {
    pub fn new() -> Self {
        RegisterCSVTableNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterCSVTableNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_csv_table",
            "Register Table",
            "Register a CSVTable (from Excel/CSV extraction) into a DataFusion session for SQL queries. Converts the table to an in-memory Arrow table.",
            "Data/DataFusion",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Trigger execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "session",
            "Session",
            "DataFusion session to register the table into",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "table",
            "Table",
            "CSVTable to register (from Excel/CSV extraction nodes)",
            VariableType::Struct,
        )
        .set_schema::<CSVTable>();

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register the table as in the DataFusion catalog",
            VariableType::String,
        )
        .set_default_value(Some(json!("data")));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table registered successfully",
            VariableType::Execution,
        );

        node.scores = Some(NodeScores {
            privacy: 10,
            security: 10,
            performance: 8,
            governance: 9,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let table: CSVTable = context.evaluate_pin("table").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;

        let cached_session = session.load(context).await?;

        table.register_with_datafusion(&cached_session.ctx, &table_name)?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
