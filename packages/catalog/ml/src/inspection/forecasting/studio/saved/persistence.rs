use super::*;
use crate::inspection::forecasting::studio::{StudioForecastInputError, preparation};
use flow_like_storage::{
    arrow_array::{ArrayRef, LargeStringArray, RecordBatch, StringArray, UInt64Array},
    arrow_schema::{DataType, Field, Schema},
    arrow_utils::record_batch_to_value,
    databases::workbench::{
        WorkbenchSurface, WorkbenchView,
        saved_query::{SavedQueryKind, SavedQuerySurface, list_saved_queries},
        source_access::{self, AccessChecker, SourceColumnRequirement},
    },
    lancedb::{
        self, Connection, Table,
        query::{ColumnOrdering, ExecutableQuery, QueryBase, QueryExecutionOptions, Select},
    },
};
use flow_like_storage_contracts::column_metadata::ColumnReader;
use flow_like_types::{Result, Value, anyhow};
use futures::TryStreamExt;
use std::{collections::BTreeSet, sync::Arc, time::Duration};

const MAXIMUM_POLICY_BYTES: usize = 64 * 1024;
const METADATA_COLUMNS: &[&str] = &[
    "id",
    "app_id",
    "name",
    "description",
    "created_by",
    "created_at",
    "model_bit_id",
    "horizon",
    "run_id",
    "access_version",
    "source_access_json",
];

fn input(error: impl std::fmt::Display) -> flow_like_types::Error {
    StudioForecastInputError(error.to_string()).into()
}
fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn app_filter(app_id: &str) -> String {
    format!("app_id = {}", literal(app_id))
}
fn id_filter(app_id: &str, id: &str) -> String {
    format!("{} AND id = {}", app_filter(app_id), literal(id))
}

fn identity(app_id: &str, id: Option<&str>) -> Result<()> {
    if !validation::identifier(app_id) || id.is_some_and(|id| !validation::identifier(id)) {
        return Err(input(
            "Saved forecast identifiers must be nonempty and at most 256 bytes",
        ));
    }
    Ok(())
}

async fn open(connection: &Connection) -> Result<Option<Table>> {
    let table = match connection.open_table(SAVED_FORECASTS_TABLE).execute().await {
        Ok(table) => table,
        Err(lancedb::Error::TableNotFound { .. }) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    table.checkout_latest().await?;
    Ok(Some(table))
}

fn schema() -> Arc<Schema> {
    let mut fields = METADATA_COLUMNS
        .iter()
        .map(|name| {
            Field::new(
                *name,
                if matches!(*name, "horizon" | "access_version") {
                    DataType::UInt64
                } else {
                    DataType::Utf8
                },
                *name == "description",
            )
        })
        .collect::<Vec<_>>();
    fields.push(Field::new("definition_json", DataType::LargeUtf8, false));
    Arc::new(Schema::new(fields))
}

async fn initialize(connection: &Connection) -> Result<Table> {
    if let Some(table) = open(connection).await? {
        return Ok(table);
    }
    match connection
        .create_empty_table(SAVED_FORECASTS_TABLE, schema())
        .execute()
        .await
    {
        Ok(table) => Ok(table),
        Err(lancedb::Error::TableAlreadyExists { .. }) => open(connection)
            .await?
            .ok_or_else(|| anyhow!("Saved forecast table disappeared during initialization")),
        Err(error) => Err(error.into()),
    }
}

fn batch(snapshot: &SavedStudioForecast, policy: &str, definition: &str) -> Result<RecordBatch> {
    let text = |value: &str| -> ArrayRef { Arc::new(StringArray::from(vec![value])) };
    RecordBatch::try_new(
        schema(),
        vec![
            text(&snapshot.id),
            text(&snapshot.app_id),
            text(&snapshot.name),
            Arc::new(StringArray::from(vec![snapshot.description.as_deref()])),
            text(&snapshot.created_by),
            text(&snapshot.created_at),
            text(&snapshot.result.model.bit_id),
            Arc::new(UInt64Array::from(vec![snapshot.result.horizon as u64])),
            text(&snapshot.result.run_id),
            Arc::new(UInt64Array::from(vec![1])),
            text(policy),
            Arc::new(LargeStringArray::from(vec![definition])),
        ],
    )
    .map_err(Into::into)
}

fn access(row: &Value) -> Option<Vec<SourceColumnRequirement>> {
    if row.get("access_version")?.as_u64()? != 1 {
        return None;
    }
    let json = row.get("source_access_json")?.as_str()?;
    if json.len() > MAXIMUM_POLICY_BYTES {
        return None;
    }
    serde_json::from_str(json).ok()
}

fn summary(row: Value) -> Result<SavedStudioForecastSummary> {
    Ok(serde_json::from_value(row)?)
}

async fn metadata(table: &Table, app_id: &str, id: &str) -> Result<Option<Value>> {
    let stream = table
        .query()
        .only_if(id_filter(app_id, id))
        .select(Select::columns(METADATA_COLUMNS))
        .limit(2)
        .execute()
        .await?;
    let batches = stream.try_collect::<Vec<_>>().await?;
    let mut rows = Vec::new();
    for batch in batches {
        rows.extend(record_batch_to_value(&batch)?);
    }
    if rows.len() > 1 {
        return Err(anyhow!("Saved forecast identifier is not unique"));
    }
    Ok(rows.pop())
}

/// Saves an immutable user-authored artifact. Source requirements are derived on
/// the server from the SQL and current saved views, never accepted from a payload.
pub async fn create(
    connection: &Connection,
    app_id: &str,
    created_by: &str,
    reader: &ColumnReader,
    payload: CreateStudioForecast,
) -> Result<SavedStudioForecast> {
    identity(app_id, Some(created_by))?;
    validation::payload(&payload).map_err(input)?;
    let views = list_saved_queries(connection)
        .await?
        .into_iter()
        .filter(|view| {
            view.app_id == app_id
                && view.kind == SavedQueryKind::View
                && view.surface == SavedQuerySurface::Native
        })
        .map(|view| WorkbenchView {
            name: view.name,
            sql: view.sql,
        })
        .collect::<Vec<_>>();
    let mut requirements = BTreeSet::new();
    for (sql, params) in std::iter::once((&payload.request.sql, &payload.request.params)).chain(
        payload
            .request
            .future_sql
            .as_ref()
            .map(|sql| (sql, &payload.request.future_params)),
    ) {
        requirements.extend(
            source_access::capture(
                connection,
                &WorkbenchSurface::Native,
                &views,
                reader,
                sql,
                params,
            )
            .await?,
        );
    }
    let requirements = requirements.into_iter().collect::<Vec<_>>();
    let policy = serde_json::to_string(&requirements)?;
    if policy.len() > MAXIMUM_POLICY_BYTES {
        return Err(input(
            "Saved forecast has too many source access requirements",
        ));
    }
    if !source_access::allows(connection, reader, &requirements).await? {
        return Err(
            flow_like_storage::databases::vector::schema::ColumnAccessRejected(
                "A forecast source is no longer readable".into(),
            )
            .into(),
        );
    }
    let snapshot = SavedStudioForecast {
        id: flow_like_types::create_id(),
        app_id: app_id.into(),
        name: payload.name.trim().into(),
        description: payload
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        created_by: created_by.into(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        request: payload.request,
        result: payload.result,
    };
    preparation::validate_serialized_size(&snapshot, MAX_SNAPSHOT_BYTES).map_err(input)?;
    let definition = serde_json::to_string(&snapshot)?;
    let table = initialize(connection).await?;
    if table.count_rows(Some(app_filter(app_id))).await? >= MAX_SAVED_FORECASTS {
        return Err(input(format!(
            "This data scope already contains {MAX_SAVED_FORECASTS} saved forecasts; delete one before saving another"
        )));
    }
    table
        .add(vec![batch(&snapshot, &policy, &definition)?])
        .execute()
        .await?;
    Ok(snapshot)
}

/// Offsets count only forecasts the reader may open. Listing never reads the
/// definition_json column containing historical values and predictions.
pub async fn list(
    connection: &Connection,
    app_id: &str,
    reader: &ColumnReader,
    offset: usize,
    limit: usize,
) -> Result<Vec<SavedStudioForecastSummary>> {
    identity(app_id, None)?;
    if !(1..=100).contains(&limit) || offset > MAX_SAVED_FORECASTS {
        return Err(input(format!(
            "Saved forecast pagination requires limit 1 to 100 and offset at most {MAX_SAVED_FORECASTS}"
        )));
    }
    let Some(table) = open(connection).await? else {
        return Ok(vec![]);
    };
    // Concurrent processes can pass the create-time count check together.
    // Report that condition before returning any page instead of hiding rows.
    if table.count_rows(Some(app_filter(app_id))).await? > MAX_SAVED_FORECASTS {
        return Err(input(
            "Saved forecast count exceeds the listing ceiling; remove an existing snapshot by ID before listing again",
        ));
    }
    let mut options = QueryExecutionOptions::default();
    options.max_batch_length = 16;
    options.timeout = Some(Duration::from_secs(30));
    let mut stream = table
        .query()
        .only_if(app_filter(app_id))
        .select(Select::columns(METADATA_COLUMNS))
        .order_by(Some(vec![
            ColumnOrdering::desc_nulls_last("created_at".into()),
            ColumnOrdering::desc_nulls_last("id".into()),
        ]))
        .limit(MAX_SAVED_FORECASTS + 1)
        .execute_with_options(options)
        .await?;
    let mut checker = AccessChecker::new(connection, reader);
    let mut output = Vec::new();
    let mut visible = 0usize;
    let mut scanned = 0usize;
    while let Some(batch) = stream.try_next().await? {
        for row in record_batch_to_value(&batch)? {
            scanned += 1;
            if scanned > MAX_SAVED_FORECASTS {
                return Err(input(
                    "Saved forecast count exceeds the listing ceiling; remove an existing snapshot before listing again",
                ));
            }
            let Some(requirements) = access(&row) else {
                continue;
            };
            if !checker.allows(&requirements).await? {
                continue;
            }
            if visible >= offset {
                output.push(summary(row)?);
            }
            visible += 1;
            if output.len() == limit {
                return Ok(output);
            }
        }
    }
    Ok(output)
}

pub async fn get(
    connection: &Connection,
    app_id: &str,
    id: &str,
    reader: &ColumnReader,
) -> Result<Option<SavedStudioForecast>> {
    identity(app_id, Some(id))?;
    let Some(table) = open(connection).await? else {
        return Ok(None);
    };
    let Some(row) = metadata(&table, app_id, id).await? else {
        return Ok(None);
    };
    let Some(requirements) = access(&row) else {
        return Ok(None);
    };
    if !source_access::allows(connection, reader, &requirements).await? {
        return Ok(None);
    }
    let expected = summary(row)?;
    let batches = table
        .query()
        .only_if(id_filter(app_id, id))
        .select(Select::columns(&["definition_json"]))
        .limit(2)
        .execute()
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    let mut rows = Vec::new();
    for batch in batches {
        rows.extend(record_batch_to_value(&batch)?);
    }
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() != 1 {
        return Err(anyhow!("Saved forecast identifier is not unique"));
    }
    let definition = rows[0]
        .get("definition_json")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Saved forecast definition is missing"))?;
    if definition.len() > MAX_SNAPSHOT_BYTES {
        return Err(anyhow!(
            "Saved forecast exceeds the supported snapshot size"
        ));
    }
    let saved: SavedStudioForecast = serde_json::from_str(definition)?;
    if saved.id != expected.id
        || saved.app_id != expected.app_id
        || saved.name != expected.name
        || saved.description != expected.description
        || saved.created_by != expected.created_by
        || saved.created_at != expected.created_at
        || saved.result.model.bit_id != expected.model_bit_id
        || saved.result.horizon != expected.horizon
        || saved.result.run_id != expected.run_id
    {
        return Err(anyhow!(
            "Saved forecast metadata do not match its definition"
        ));
    }
    Ok(Some(saved))
}

pub async fn delete(
    connection: &Connection,
    app_id: &str,
    id: &str,
    reader: &ColumnReader,
) -> Result<bool> {
    identity(app_id, Some(id))?;
    let Some(table) = open(connection).await? else {
        return Ok(false);
    };
    let Some(row) = metadata(&table, app_id, id).await? else {
        return Ok(false);
    };
    // Administrators may clean up orphaned snapshots after a source table is
    // removed. Deletion returns no saved values; read checks stay unchanged.
    if !reader.is_unrestricted() {
        let Some(requirements) = access(&row) else {
            return Ok(false);
        };
        if !source_access::allows(connection, reader, &requirements).await? {
            return Ok(false);
        }
    }
    Ok(table.delete(&id_filter(app_id, id)).await?.num_deleted_rows > 0)
}
