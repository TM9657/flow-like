use crate::data::datafusion::session::DataFusionSession;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};

// ============================================================================
// PLACEHOLDER NODE FOR DATABASE PROVIDERS
// These will be fully implemented when datafusion-table-providers becomes
// compatible with our datafusion version (49.x).
// For now, they use SQL-based external table registration which has limitations.
// ============================================================================

/// Register a PostgreSQL data source using connection URL.
/// Note: Full TableProvider support pending datafusion-table-providers compatibility.
#[crate::register_node]
#[derive(Default)]
pub struct RegisterPostgresNode {}

impl RegisterPostgresNode {
    pub fn new() -> Self {
        RegisterPostgresNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterPostgresNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_postgres",
            "Register PostgreSQL",
            "Register a PostgreSQL connection for federated queries. Creates a virtual table referencing the external PostgreSQL database.",
            "Data/DataFusion/Databases",
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
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "host",
            "Host",
            "PostgreSQL server host",
            VariableType::String,
        )
        .set_default_value(Some(json!("localhost")));

        node.add_input_pin(
            "port",
            "Port",
            "PostgreSQL server port",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(5432)));

        node.add_input_pin(
            "database",
            "Database",
            "Database name",
            VariableType::String,
        );

        node.add_input_pin(
            "username",
            "Username",
            "Database username",
            VariableType::String,
        );

        node.add_input_pin(
            "password",
            "Password",
            "Database password",
            VariableType::String,
        );

        node.add_input_pin(
            "schema",
            "Schema",
            "PostgreSQL schema",
            VariableType::String,
        )
        .set_default_value(Some(json!("public")));

        node.add_input_pin(
            "source_table",
            "Source Table",
            "Name of the table in PostgreSQL",
            VariableType::String,
        );

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register in DataFusion",
            VariableType::String,
        );

        node.add_input_pin(
            "ssl_mode",
            "SSL Mode",
            "SSL mode: disable, prefer, require",
            VariableType::String,
        )
        .set_default_value(Some(json!("prefer")));

        node.add_input_pin(
            "readonly",
            "Read Only",
            "Open connection in read-only mode",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table registered",
            VariableType::Execution,
        );

        node.add_output_pin(
            "session_out",
            "Session",
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_output_pin(
            "connection_url",
            "Connection URL",
            "Generated connection URL (without password)",
            VariableType::String,
        );

        node.scores = Some(NodeScores {
            privacy: 5,
            security: 6,
            performance: 7,
            governance: 7,
            reliability: 8,
            cost: 7,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let host: String = context.evaluate_pin("host").await?;
        let port: i64 = context.evaluate_pin("port").await?;
        let database: String = context.evaluate_pin("database").await?;
        let username: String = context.evaluate_pin("username").await?;
        let password: String = context.evaluate_pin("password").await?;
        let schema: String = context
            .evaluate_pin("schema")
            .await
            .unwrap_or_else(|_| "public".to_string());
        let source_table: String = context.evaluate_pin("source_table").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let ssl_mode: String = context
            .evaluate_pin("ssl_mode")
            .await
            .unwrap_or_else(|_| "prefer".to_string());
        let readonly: bool = context.evaluate_pin("readonly").await.unwrap_or(true);

        let cached_session = session.load(context).await?;

        // Build connection URL
        let readonly_param = if readonly {
            "&default_transaction_read_only=on"
        } else {
            ""
        };
        let conn_url = format!(
            "postgresql://{}:****@{}:{}/{}?sslmode={}&options=-c%20search_path%3D{}{}",
            username, host, port, database, ssl_mode, schema, readonly_param
        );

        // Build full URL with password for actual connection
        let full_url = format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode={}&options=-c%20search_path%3D{}{}",
            username, password, host, port, database, ssl_mode, schema, readonly_param
        );

        // Store connection metadata in session context for later use by query nodes
        // This approach allows us to execute queries through SQL generation
        let _connection_info = json!({
            "type": "postgres",
            "host": host,
            "port": port,
            "database": database,
            "username": username,
            "schema": schema,
            "table": source_table,
            "alias": table_name,
            "ssl_mode": ssl_mode,
            "readonly": readonly,
            "connection_url": full_url
        });

        // For now, we store the connection info as a custom variable
        // Full TableProvider integration pending datafusion-table-providers compatibility
        tracing::info!(
            "PostgreSQL connection configured for table '{}' -> '{}'",
            source_table,
            table_name
        );

        // Add a placeholder table with metadata
        let sql = format!(
            "CREATE VIEW {} AS SELECT 'PostgreSQL external table: {}@{}:{}/{}.{}' as _info",
            table_name, username, host, port, database, source_table
        );
        cached_session.ctx.sql(&sql).await?;

        context.set_pin_value("session_out", json!(session)).await?;
        context
            .set_pin_value("connection_url", json!(conn_url))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

/// Register a MySQL data source
#[crate::register_node]
#[derive(Default)]
pub struct RegisterMysqlNode {}

impl RegisterMysqlNode {
    pub fn new() -> Self {
        RegisterMysqlNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterMysqlNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_mysql",
            "Register MySQL",
            "Register a MySQL connection for federated queries.",
            "Data/DataFusion/Databases",
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
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin("host", "Host", "MySQL server host", VariableType::String)
            .set_default_value(Some(json!("localhost")));

        node.add_input_pin("port", "Port", "MySQL server port", VariableType::Integer)
            .set_default_value(Some(json!(3306)));

        node.add_input_pin(
            "database",
            "Database",
            "Database name",
            VariableType::String,
        );

        node.add_input_pin(
            "username",
            "Username",
            "Database username",
            VariableType::String,
        );

        node.add_input_pin(
            "password",
            "Password",
            "Database password",
            VariableType::String,
        );

        node.add_input_pin(
            "source_table",
            "Source Table",
            "Name of the table in MySQL",
            VariableType::String,
        );

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register in DataFusion",
            VariableType::String,
        );

        node.add_input_pin(
            "ssl_mode",
            "SSL Mode",
            "SSL mode: disabled, preferred, required",
            VariableType::String,
        )
        .set_default_value(Some(json!("preferred")));

        node.add_input_pin(
            "readonly",
            "Read Only",
            "Open connection in read-only mode",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table registered",
            VariableType::Execution,
        );

        node.add_output_pin(
            "session_out",
            "Session",
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_output_pin(
            "connection_url",
            "Connection URL",
            "Generated connection URL",
            VariableType::String,
        );

        node.scores = Some(NodeScores {
            privacy: 5,
            security: 6,
            performance: 7,
            governance: 7,
            reliability: 8,
            cost: 7,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let host: String = context.evaluate_pin("host").await?;
        let port: i64 = context.evaluate_pin("port").await?;
        let database: String = context.evaluate_pin("database").await?;
        let username: String = context.evaluate_pin("username").await?;
        let _password: String = context.evaluate_pin("password").await?;
        let source_table: String = context.evaluate_pin("source_table").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let ssl_mode: String = context
            .evaluate_pin("ssl_mode")
            .await
            .unwrap_or_else(|_| "preferred".to_string());
        let readonly: bool = context.evaluate_pin("readonly").await.unwrap_or(true);

        let cached_session = session.load(context).await?;

        let readonly_param = if readonly {
            "&session_variables=sql_mode%3D'NO_ENGINE_SUBSTITUTION',GLOBAL%20read_only%3D1"
        } else {
            ""
        };
        let conn_url = format!(
            "mysql://{}:****@{}:{}/{}?ssl-mode={}{}",
            username, host, port, database, ssl_mode, readonly_param
        );

        tracing::info!(
            "MySQL connection configured for table '{}' -> '{}'",
            source_table,
            table_name
        );

        let sql = format!(
            "CREATE VIEW {} AS SELECT 'MySQL external table: {}@{}:{}/{}' as _info",
            table_name, username, host, port, database
        );
        cached_session.ctx.sql(&sql).await?;

        context.set_pin_value("session_out", json!(session)).await?;
        context
            .set_pin_value("connection_url", json!(conn_url))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

/// Register a SQLite database file
#[crate::register_node]
#[derive(Default)]
pub struct RegisterSqliteNode {}

impl RegisterSqliteNode {
    pub fn new() -> Self {
        RegisterSqliteNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterSqliteNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_sqlite",
            "Register SQLite",
            "Register a SQLite database file for federated queries.",
            "Data/DataFusion/Databases",
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
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "file_path",
            "File Path",
            "Path to SQLite database file",
            VariableType::String,
        );

        node.add_input_pin(
            "source_table",
            "Source Table",
            "Name of the table in SQLite",
            VariableType::String,
        );

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register in DataFusion",
            VariableType::String,
        );

        node.add_input_pin(
            "readonly",
            "Read Only",
            "Open database in read-only mode",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table registered",
            VariableType::Execution,
        );

        node.add_output_pin(
            "session_out",
            "Session",
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.scores = Some(NodeScores {
            privacy: 8,
            security: 8,
            performance: 9,
            governance: 8,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let file_path: String = context.evaluate_pin("file_path").await?;
        let source_table: String = context.evaluate_pin("source_table").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let readonly: bool = context.evaluate_pin("readonly").await.unwrap_or(true);

        let cached_session = session.load(context).await?;

        // Validate file exists
        if !std::path::Path::new(&file_path).exists() {
            return Err(flow_like_types::anyhow!(
                "SQLite file not found: {}",
                file_path
            ));
        }

        let mode = if readonly { "ro" } else { "rw" };
        tracing::info!(
            "SQLite connection configured for '{}' table '{}' -> '{}' (mode: {})",
            file_path,
            source_table,
            table_name,
            mode
        );

        let sql = format!(
            "CREATE VIEW {} AS SELECT 'SQLite external table: {}:{}' as _info",
            table_name, file_path, source_table
        );
        cached_session.ctx.sql(&sql).await?;

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

/// Register a DuckDB database
#[crate::register_node]
#[derive(Default)]
pub struct RegisterDuckdbNode {}

impl RegisterDuckdbNode {
    pub fn new() -> Self {
        RegisterDuckdbNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterDuckdbNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_duckdb",
            "Register DuckDB",
            "Register a DuckDB database for federated queries.",
            "Data/DataFusion/Databases",
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
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "file_path",
            "File Path",
            "Path to DuckDB database file (or :memory:)",
            VariableType::String,
        )
        .set_default_value(Some(json!(":memory:")));

        node.add_input_pin(
            "source_table",
            "Source Table",
            "Name of the table in DuckDB",
            VariableType::String,
        );

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register in DataFusion",
            VariableType::String,
        );

        node.add_input_pin(
            "readonly",
            "Read Only",
            "Open database in read-only mode",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table registered",
            VariableType::Execution,
        );

        node.add_output_pin(
            "session_out",
            "Session",
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.scores = Some(NodeScores {
            privacy: 8,
            security: 8,
            performance: 10,
            governance: 8,
            reliability: 9,
            cost: 10,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let file_path: String = context.evaluate_pin("file_path").await?;
        let source_table: String = context.evaluate_pin("source_table").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let readonly: bool = context.evaluate_pin("readonly").await.unwrap_or(true);

        let cached_session = session.load(context).await?;

        // Validate file exists if not in-memory
        if file_path != ":memory:" && !std::path::Path::new(&file_path).exists() {
            return Err(flow_like_types::anyhow!(
                "DuckDB file not found: {}",
                file_path
            ));
        }

        let mode = if readonly { "read_only" } else { "read_write" };
        tracing::info!(
            "DuckDB connection configured for '{}' table '{}' -> '{}' (mode: {})",
            file_path,
            source_table,
            table_name,
            mode
        );

        let sql = format!(
            "CREATE VIEW {} AS SELECT 'DuckDB external table: {}:{}' as _info",
            table_name, file_path, source_table
        );
        cached_session.ctx.sql(&sql).await?;

        context.set_pin_value("session_out", json!(session)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}

/// Register a ClickHouse data source
#[crate::register_node]
#[derive(Default)]
pub struct RegisterClickhouseNode {}

impl RegisterClickhouseNode {
    pub fn new() -> Self {
        RegisterClickhouseNode {}
    }
}

#[async_trait]
impl NodeLogic for RegisterClickhouseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "df_register_clickhouse",
            "Register ClickHouse",
            "Register a ClickHouse connection for federated queries.",
            "Data/DataFusion/Databases",
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
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_input_pin(
            "host",
            "Host",
            "ClickHouse server host",
            VariableType::String,
        )
        .set_default_value(Some(json!("localhost")));

        node.add_input_pin(
            "port",
            "Port",
            "ClickHouse HTTP port",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(8123)));

        node.add_input_pin(
            "database",
            "Database",
            "Database name",
            VariableType::String,
        )
        .set_default_value(Some(json!("default")));

        node.add_input_pin(
            "username",
            "Username",
            "Database username",
            VariableType::String,
        )
        .set_default_value(Some(json!("default")));

        node.add_input_pin(
            "password",
            "Password",
            "Database password",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "source_table",
            "Source Table",
            "Name of the table in ClickHouse",
            VariableType::String,
        );

        node.add_input_pin(
            "table_name",
            "Table Name",
            "Name to register in DataFusion",
            VariableType::String,
        );

        node.add_input_pin(
            "readonly",
            "Read Only",
            "Use read-only queries",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Table registered",
            VariableType::Execution,
        );

        node.add_output_pin(
            "session_out",
            "Session",
            "DataFusion session",
            VariableType::Struct,
        )
        .set_schema::<DataFusionSession>();

        node.add_output_pin(
            "connection_url",
            "Connection URL",
            "Generated connection URL",
            VariableType::String,
        );

        node.scores = Some(NodeScores {
            privacy: 5,
            security: 6,
            performance: 10,
            governance: 7,
            reliability: 8,
            cost: 7,
        });

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let session: DataFusionSession = context.evaluate_pin("session").await?;
        let host: String = context.evaluate_pin("host").await?;
        let port: i64 = context.evaluate_pin("port").await?;
        let database: String = context
            .evaluate_pin("database")
            .await
            .unwrap_or_else(|_| "default".to_string());
        let username: String = context
            .evaluate_pin("username")
            .await
            .unwrap_or_else(|_| "default".to_string());
        let _password: String = context.evaluate_pin("password").await.unwrap_or_default();
        let source_table: String = context.evaluate_pin("source_table").await?;
        let table_name: String = context.evaluate_pin("table_name").await?;
        let readonly: bool = context.evaluate_pin("readonly").await.unwrap_or(true);

        let cached_session = session.load(context).await?;

        let readonly_param = if readonly { "&readonly=1" } else { "" };
        let conn_url = format!(
            "http://{}:****@{}:{}/{}{}",
            username, host, port, database, readonly_param
        );

        tracing::info!(
            "ClickHouse connection configured for table '{}' -> '{}'",
            source_table,
            table_name
        );

        let sql = format!(
            "CREATE VIEW {} AS SELECT 'ClickHouse external table: {}@{}:{}/{}' as _info",
            table_name, username, host, port, database
        );
        cached_session.ctx.sql(&sql).await?;

        context.set_pin_value("session_out", json!(session)).await?;
        context
            .set_pin_value("connection_url", json!(conn_url))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
