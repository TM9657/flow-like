//! Column metadata and column read restrictions, per todo/column-metadata.md.

use std::path::PathBuf;

use flow_like_storage::contracts::column_metadata::{
    ColumnMetadata, ColumnReader, DESCRIPTION_KEY, DataClassification, DataSensitivity,
    READ_ATTRIBUTES_KEY,
};
use flow_like_storage::contracts::database::DatabaseSelector;
use flow_like_storage::databases::vector::{
    VectorStore,
    lancedb::LanceDBVectorStore,
    schema::{
        ColumnAccessRejected, DatabaseSchemaField, TableInputRejected,
        database_fields_to_arrow_schema,
    },
};
use flow_like_storage::datafusion::prelude::SessionContext;
use flow_like_storage::lancedb::table::ColumnAlteration;
use flow_like_types::{Error, Result, Value, create_id, json::json};

const TABLE: &str = "people";

struct TempDb(PathBuf);

impl TempDb {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("flow-like-column-metadata-{}", create_id())))
    }

    async fn open(&self) -> Result<LanceDBVectorStore> {
        LanceDBVectorStore::new(self.0.clone(), TABLE.into()).await
    }

    async fn open_as(&self, reader: ColumnReader) -> Result<LanceDBVectorStore> {
        Ok(self.open().await?.with_column_reader(reader))
    }

    #[cfg(feature = "graph")]
    async fn connection(&self) -> Result<flow_like_storage::lancedb::Connection> {
        Ok(
            flow_like_storage::lancedb::connect(&self.0.to_string_lossy())
                .execute()
                .await?,
        )
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn field(name: &str, data_type: &str, nullable: bool, primary_key: bool) -> DatabaseSchemaField {
    DatabaseSchemaField {
        name: name.into(),
        data_type: data_type.into(),
        nullable,
        vector_size: None,
        primary_key,
    }
}

fn fields() -> Vec<DatabaseSchemaField> {
    vec![
        field("id", "string", false, true),
        field("name", "string", true, false),
        field("salary", "int64", true, false),
    ]
}

fn restricted_to(attributes: &[&str]) -> ColumnMetadata {
    ColumnMetadata {
        description: Some("Gross annual salary".into()),
        sensitivity: DataSensitivity {
            classification: Some(DataClassification::Confidential),
            pii: true,
        },
        read_attributes: attributes
            .iter()
            .map(|attribute| attribute.to_string())
            .collect(),
        ..ColumnMetadata::default()
    }
}

fn sales() -> ColumnReader {
    ColumnReader::with_attributes(["sales"])
}

fn hr() -> ColumnReader {
    ColumnReader::with_attributes(["hr"])
}

/// `people(id, name, salary)` with two rows and `salary` readable by `hr` only.
async fn salaries(db: &TempDb) -> Result<LanceDBVectorStore> {
    let mut store = db.open().await?;
    store
        .create_empty_table(database_fields_to_arrow_schema(&fields())?, false)
        .await?;
    store
        .insert(vec![
            json!({ "id": "a", "name": "Ada", "salary": 120 }),
            json!({ "id": "b", "name": "Bob", "salary": 80 }),
        ])
        .await?;
    store
        .set_column_metadata("salary", restricted_to(&["hr"]))
        .await?;
    Ok(store)
}

fn column_names(schema: &flow_like_storage::arrow_schema::Schema) -> Vec<String> {
    schema
        .fields()
        .iter()
        .map(|field| field.name().clone())
        .collect()
}

fn expect_hidden(error: Error) {
    assert!(
        error.downcast_ref::<TableInputRejected>().is_some(),
        "expected a hidden column to read as missing, got: {error:#}"
    );
    assert!(error.to_string().contains("has no column"), "{error:#}");
}

async fn sql_rows(store: &LanceDBVectorStore, sql: &str) -> Result<Vec<Value>> {
    let context = SessionContext::new();
    context.register_table(TABLE, store.to_datafusion().await?)?;
    let batches = context.sql(sql).await?.collect().await?;
    flow_like_storage::databases::vector::lancedb::record_batches_to_vec(Some(batches))
}

#[tokio::test]
async fn metadata_round_trips_and_keeps_the_table_key() -> Result<()> {
    let db = TempDb::new();
    let store = salaries(&db).await?;

    let stored = store.column_metadata("salary").await?;
    assert_eq!(stored, restricted_to(&["hr"]));
    assert_eq!(store.primary_key().await?.as_deref(), Some("id"));

    let schema = store.schema().await?;
    let salary = schema.field_with_name("salary")?;
    assert_eq!(
        salary
            .metadata()
            .get(READ_ATTRIBUTES_KEY)
            .map(String::as_str),
        Some(r#"["hr"]"#)
    );

    store
        .set_column_metadata("salary", ColumnMetadata::default())
        .await?;
    assert!(store.column_metadata("salary").await?.is_empty());
    assert_eq!(store.primary_key().await?.as_deref(), Some("id"));

    let id = store
        .set_column_metadata(
            "id",
            ColumnMetadata {
                display_name: Some("  Ticket ".into()),
                read_attributes: vec!["hr".into(), " hr ".into()],
                ..ColumnMetadata::default()
            },
        )
        .await?;
    assert_eq!(id.display_name.as_deref(), Some("Ticket"));
    assert_eq!(id.read_attributes, vec!["hr"]);
    assert_eq!(store.primary_key().await?.as_deref(), Some("id"));
    Ok(())
}

#[tokio::test]
async fn unknown_columns_and_oversized_values_are_rejected() -> Result<()> {
    let db = TempDb::new();
    let store = salaries(&db).await?;

    expect_hidden(
        store
            .set_column_metadata("missing", ColumnMetadata::default())
            .await
            .unwrap_err(),
    );
    let error = store
        .set_column_metadata(
            "name",
            ColumnMetadata {
                description: Some("x".repeat(5000)),
                ..ColumnMetadata::default()
            },
        )
        .await
        .unwrap_err();
    assert!(
        error.downcast_ref::<TableInputRejected>().is_some(),
        "{error:#}"
    );
    Ok(())
}

#[tokio::test]
async fn metadata_survives_schema_changes_and_create_if_not_exists() -> Result<()> {
    let db = TempDb::new();
    let mut store = salaries(&db).await?;

    store
        .make_column_nullable("salary", false)
        .await
        .unwrap_or_else(|error| panic!("make salary required: {error:#}"));
    store.add_typed_column("bonus", "int64", None).await?;
    store
        .alter_column(&[ColumnAlteration::new("salary".into()).rename("pay".into())])
        .await?;
    assert_eq!(store.column_metadata("pay").await?, restricted_to(&["hr"]));

    let mut fields = fields();
    fields[2].name = "pay".into();
    fields[2].nullable = false;
    fields.push(field("bonus", "int64", true, false));
    let created = store
        .create_empty_table(database_fields_to_arrow_schema(&fields)?, true)
        .await?;
    assert!(!created);
    Ok(())
}

#[tokio::test]
async fn restricted_readers_never_see_hidden_columns() -> Result<()> {
    let db = TempDb::new();
    salaries(&db).await?;
    let store = db.open_as(sales()).await?;

    assert_eq!(column_names(&store.schema().await?), vec!["id", "name"]);
    let rows = store.list(None, 10, 0).await?;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.get("salary").is_none()));
    let rows = store.filter("name = 'Ada'", None, 10, 0).await?;
    assert_eq!(rows, vec![json!({ "id": "a", "name": "Ada" })]);
    assert_eq!(store.count(Some("name = 'Bob'".into())).await?, 1);

    expect_hidden(store.filter("salary > 100", None, 10, 0).await.unwrap_err());
    expect_hidden(
        store
            .filter("`Salary` > 100", None, 10, 0)
            .await
            .unwrap_err(),
    );
    expect_hidden(
        store
            .list(Some(vec!["salary".into()]), 10, 0)
            .await
            .unwrap_err(),
    );
    expect_hidden(store.count(Some("salary > 100".into())).await.unwrap_err());
    expect_hidden(store.column_metadata("salary").await.unwrap_err());

    let visible = sql_rows(&store, "SELECT * FROM people ORDER BY id").await?;
    assert_eq!(
        visible,
        vec![
            json!({ "id": "a", "name": "Ada" }),
            json!({ "id": "b", "name": "Bob" })
        ]
    );
    assert_eq!(
        sql_rows(&store, "SELECT COUNT(*) AS rows FROM people").await?,
        vec![json!({ "rows": 2 })]
    );
    let error = sql_rows(&store, "SELECT id FROM people WHERE salary > 100")
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("salary"), "{error}");

    let reader = db.open_as(hr()).await?;
    assert_eq!(
        column_names(&reader.schema().await?),
        vec!["id", "name", "salary"]
    );
    assert_eq!(reader.filter("salary > 100", None, 10, 0).await?.len(), 1);
    assert_eq!(
        db.open()
            .await?
            .filter("salary > 100", None, 10, 0)
            .await?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn restricted_writers_cannot_probe_or_copy_hidden_columns() -> Result<()> {
    let db = TempDb::new();
    salaries(&db).await?;
    let store = db.open_as(sales()).await?;

    expect_hidden(store.delete("salary > 100").await.unwrap_err());
    expect_hidden(
        store
            .update("salary > 100", [("name".to_string(), json!("Rich"))].into())
            .await
            .unwrap_err(),
    );
    expect_hidden(
        store
            .update("id = 'a'", [("salary".to_string(), json!(1))].into())
            .await
            .unwrap_err(),
    );
    expect_hidden(store.add_column("copy", "salary * 1").await.unwrap_err());
    expect_hidden(store.drop_columns(&["salary"]).await.unwrap_err());
    expect_hidden(
        store
            .make_column_nullable("salary", true)
            .await
            .unwrap_err(),
    );
    expect_hidden(store.set_primary_key("salary").await.unwrap_err());
    assert_eq!(db.open().await?.count(None).await?, 2);
    Ok(())
}

#[tokio::test]
async fn only_unrestricted_readers_change_who_may_read() -> Result<()> {
    let db = TempDb::new();
    salaries(&db).await?;
    let hr_store = db.open_as(hr()).await?;

    let described = hr_store
        .set_column_metadata(
            "salary",
            ColumnMetadata {
                display_name: Some("Salary".into()),
                read_attributes: vec!["hr".into()],
                ..restricted_to(&["hr"])
            },
        )
        .await?;
    assert_eq!(described.display_name.as_deref(), Some("Salary"));

    let error = hr_store
        .set_column_metadata("salary", restricted_to(&[]))
        .await
        .unwrap_err();
    assert!(
        error.downcast_ref::<ColumnAccessRejected>().is_some(),
        "{error:#}"
    );
    let error = hr_store
        .set_column_metadata("name", restricted_to(&["hr"]))
        .await
        .unwrap_err();
    assert!(
        error.downcast_ref::<ColumnAccessRejected>().is_some(),
        "{error:#}"
    );

    db.open()
        .await?
        .set_column_metadata("salary", restricted_to(&[]))
        .await?;
    let schema = db.open_as(sales()).await?.schema().await?;
    assert_eq!(column_names(&schema), vec!["id", "name", "salary"]);
    assert_eq!(
        schema
            .field_with_name("salary")?
            .metadata()
            .get(DESCRIPTION_KEY)
            .map(String::as_str),
        Some("Gross annual salary")
    );
    Ok(())
}

#[tokio::test]
async fn history_never_reveals_a_newly_restricted_column() -> Result<()> {
    let db = TempDb::new();
    let mut store = db.open().await?;
    store
        .create_empty_table(database_fields_to_arrow_schema(&fields())?, false)
        .await?;
    store
        .insert(vec![json!({ "id": "a", "name": "Ada", "salary": 120 })])
        .await?;
    let before = store.reference().await?.version;
    store
        .set_column_metadata("salary", restricted_to(&["hr"]))
        .await?;

    let pinned = db
        .open_as(sales())
        .await?
        .checkout(DatabaseSelector {
            version: Some(before),
            ..DatabaseSelector::default()
        })
        .await?;
    assert_eq!(column_names(&pinned.schema().await?), vec!["id", "name"]);
    assert!(
        pinned.list(None, 10, 0).await?[0].get("salary").is_none(),
        "an old version must not show the column"
    );
    Ok(())
}

#[tokio::test]
async fn restoring_an_older_version_keeps_column_metadata() -> Result<()> {
    let db = TempDb::new();
    let mut store = db.open().await?;
    store
        .create_empty_table(database_fields_to_arrow_schema(&fields())?, false)
        .await?;
    store
        .insert(vec![json!({ "id": "a", "name": "Ada", "salary": 120 })])
        .await?;
    let before = store.reference().await?.version;
    store
        .set_column_metadata("salary", restricted_to(&["hr"]))
        .await?;
    store
        .insert(vec![json!({ "id": "b", "name": "Bob", "salary": 80 })])
        .await?;

    store
        .checkout(DatabaseSelector {
            version: Some(before),
            ..DatabaseSelector::default()
        })
        .await?
        .restore()
        .await?;

    let latest = db.open().await?;
    assert_eq!(latest.count(None).await?, 1);
    assert_eq!(
        latest.column_metadata("salary").await?,
        restricted_to(&["hr"])
    );
    assert_eq!(
        column_names(&db.open_as(sales()).await?.schema().await?),
        vec!["id", "name"]
    );
    Ok(())
}

#[tokio::test]
async fn full_text_search_skips_hidden_columns() -> Result<()> {
    let db = TempDb::new();
    let mut store = db.open().await?;
    store
        .create_empty_table(
            database_fields_to_arrow_schema(&[
                field("id", "string", false, true),
                field("bio", "string", true, false),
                field("notes", "string", true, false),
            ])?,
            false,
        )
        .await?;
    store
        .insert(vec![
            json!({ "id": "a", "bio": "cat lover", "notes": "secret cat" }),
            json!({ "id": "b", "bio": "dog walker", "notes": "secret dog" }),
        ])
        .await?;
    store.index("notes", Some("fulltext")).await?;
    store
        .set_column_metadata("notes", restricted_to(&["hr"]))
        .await?;

    let restricted = db.open_as(sales()).await?;
    let error = restricted
        .fts_search("secret", None, None, None, 10, 0)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("full-text"), "{error:#}");
    expect_hidden(
        restricted
            .fts_search("secret", None, None, Some(vec!["notes".into()]), 10, 0)
            .await
            .unwrap_err(),
    );

    let context = SessionContext::new();
    flow_like_storage::databases::register_sql_functions(&context);
    context.register_table(TABLE, restricted.to_datafusion().await?)?;
    for query in [
        r#"SELECT id FROM fts('people', '{"match":{"column":"notes","terms":"secret"}}')"#,
        r#"SELECT id FROM fts('people', '{"match":{"terms":"secret"}}')"#,
    ] {
        let error = match context.sql(query).await {
            Ok(frame) => frame.collect().await.unwrap_err().to_string(),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("fts()"), "{query}: {error}");
    }

    assert_eq!(
        db.open()
            .await?
            .fts_search("secret", None, None, None, 10, 0)
            .await?
            .len(),
        2
    );
    Ok(())
}

#[tokio::test]
async fn sql_insert_overwrite_is_refused_on_a_described_table() -> Result<()> {
    let db = TempDb::new();
    let mut store = db.open().await?;
    store
        .create_empty_table(
            database_fields_to_arrow_schema(&[
                field("id", "string", true, false),
                field("name", "string", true, false),
            ])?,
            false,
        )
        .await?;
    store
        .insert(vec![json!({ "id": "a", "name": "Ada" })])
        .await?;
    store
        .set_column_metadata(
            "name",
            ColumnMetadata {
                description: Some("Full name".into()),
                ..ColumnMetadata::default()
            },
        )
        .await?;

    let context = SessionContext::new();
    context.register_table(TABLE, store.to_datafusion().await?)?;
    let error = match context
        .sql("INSERT OVERWRITE people VALUES ('z', 'Zed')")
        .await
    {
        Ok(frame) => frame.collect().await.unwrap_err().to_string(),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("metadata of column 'name'"), "{error}");
    assert_eq!(store.count(None).await?, 1);
    Ok(())
}

#[cfg(feature = "graph")]
#[tokio::test]
async fn summaries_hide_columns_and_roll_up_sensitivity() -> Result<()> {
    use flow_like_storage::databases::table_summary::summarize_tables_for;

    let db = TempDb::new();
    let store = salaries(&db).await?;
    store
        .set_column_metadata(
            "name",
            ColumnMetadata {
                sensitivity: DataSensitivity {
                    classification: Some(DataClassification::Internal),
                    pii: false,
                },
                ..ColumnMetadata::default()
            },
        )
        .await?;
    let connection = db.connection().await?;

    let [restricted] = summarize_tables_for(&connection, vec![TABLE.into()], &sales())
        .await
        .try_into()
        .expect("one summary");
    let names: Vec<_> = restricted
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(names, vec!["id", "name"]);
    assert_eq!(
        restricted.sensitivity,
        DataSensitivity {
            classification: Some(DataClassification::Internal),
            pii: false,
        }
    );

    let [full] = summarize_tables_for(&connection, vec![TABLE.into()], &ColumnReader::Unrestricted)
        .await
        .try_into()
        .expect("one summary");
    assert_eq!(full.columns.len(), 3);
    assert_eq!(
        full.sensitivity,
        DataSensitivity {
            classification: Some(DataClassification::Confidential),
            pii: true,
        }
    );
    Ok(())
}

#[cfg(feature = "graph")]
#[tokio::test]
async fn workbench_sql_hides_columns_from_restricted_readers() -> Result<()> {
    use flow_like_storage::databases::workbench::{WorkbenchSurface, execute_readonly_sql};

    let db = TempDb::new();
    salaries(&db).await?;
    let connection = db.connection().await?;
    let run = |sql: &'static str, reader: ColumnReader| {
        let connection = connection.clone();
        async move {
            execute_readonly_sql(
                &connection,
                WorkbenchSurface::Native,
                Vec::new(),
                sql,
                &Value::Null,
                Some(10),
                &reader,
            )
            .await
        }
    };

    let visible = run("SELECT * FROM people ORDER BY id", sales()).await?;
    let names: Vec<_> = visible
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(names, vec!["id", "name"]);
    assert!(run("SELECT salary FROM people", sales()).await.is_err());
    assert_eq!(run("SELECT salary FROM people", hr()).await?.row_count, 2);
    Ok(())
}

#[cfg(feature = "graph")]
#[tokio::test]
async fn overlays_drop_hidden_properties_and_identities() -> Result<()> {
    use flow_like_storage::databases::graph::lancegraph::{
        GraphOverlayDef, PropertyProjectionMode, restrict_overlay,
    };

    let db = TempDb::new();
    salaries(&db).await?;
    let connection = db.connection().await?;
    let overlay: GraphOverlayDef = flow_like_types::json::from_value(json!({
        "id": "staff",
        "name": "Staff",
        "description": null,
        "nodes": [{
            "label": "Person",
            "table": TABLE,
            "id_column": "id",
            "display_column": "salary",
            "property_columns": [],
            "style": {}
        }],
        "edges": [],
        "default_limit": 100,
        "created_at": "",
        "updated_at": ""
    }))?;

    let restricted = restrict_overlay(&connection, overlay.clone(), &sales()).await?;
    let person = &restricted.nodes[0];
    assert_eq!(
        restricted.property_projection_mode,
        PropertyProjectionMode::Frozen
    );
    assert_eq!(person.display_column, None);
    let properties: Vec<_> = person
        .property_columns
        .iter()
        .map(|property| property.name.as_str())
        .collect();
    assert_eq!(properties, vec!["name"]);

    let unrestricted = restrict_overlay(&connection, overlay.clone(), &hr()).await?;
    assert_eq!(
        unrestricted.nodes[0].display_column.as_deref(),
        Some("salary")
    );
    assert_eq!(
        unrestricted.property_projection_mode,
        PropertyProjectionMode::Dynamic
    );

    db.open()
        .await?
        .set_column_metadata("id", restricted_to(&["hr"]))
        .await?;
    let without_identity = restrict_overlay(&connection, overlay, &sales()).await?;
    assert!(without_identity.nodes.is_empty());
    Ok(())
}
