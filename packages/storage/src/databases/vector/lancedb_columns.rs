//! Column metadata and column read restrictions on [`LanceDBVectorStore`].

use std::collections::BTreeSet;

use arrow_schema::{Schema, SchemaRef};
use flow_like_types::{Result, anyhow};
use lancedb::Table;

use super::{
    ColumnMetadata, ColumnReader, IndexConfigDto, LanceDBVectorStore, VectorStore,
    is_vector_data_type, list_table_indices,
};
use crate::databases::column_access::{
    hidden_columns, is_hidden, no_such_column, reject_hidden_columns, reject_hidden_references,
    visible_columns,
};
use crate::databases::vector::schema::{
    ColumnAccessRejected, TableInputRejected, column_metadata_update,
};

/// What the store's reader may select, filter and search in one table.
pub(super) struct ReadScope<'a> {
    table: &'a str,
    hidden: Vec<String>,
    schema: Option<SchemaRef>,
}

impl ReadScope<'_> {
    pub(super) fn hidden(&self) -> &[String] {
        &self.hidden
    }

    pub(super) fn is_restricted(&self) -> bool {
        !self.hidden.is_empty()
    }

    pub(super) fn check_expression(&self, expression: &str) -> Result<()> {
        reject_hidden_references(expression, &self.hidden, self.table)
    }

    /// The columns to return: the requested ones, which must all be visible, or every
    /// visible column when a table with hidden columns is read without a column list.
    pub(super) fn select(&self, requested: Option<Vec<String>>) -> Result<Option<Vec<String>>> {
        match (requested, &self.schema) {
            (Some(columns), _) => {
                for column in &columns {
                    self.check_expression(column)?;
                }
                Ok(Some(columns))
            }
            (None, Some(schema)) if self.is_restricted() => {
                Ok(Some(visible_columns(schema, &self.hidden)))
            }
            (None, _) => Ok(None),
        }
    }

    /// The full-text columns to search: the requested ones, which must all be visible, or
    /// every visible column with a full-text index when a table with hidden columns is
    /// searched without naming any.
    pub(super) async fn search_fields(
        &self,
        table: &Table,
        requested: Option<Vec<String>>,
    ) -> Result<Option<Vec<String>>> {
        if let Some(fields) = requested.filter(|fields| !fields.is_empty()) {
            reject_hidden_columns(&fields, &self.hidden, self.table)?;
            return Ok(Some(fields));
        }
        if !self.is_restricted() {
            return Ok(None);
        }
        let fields: Vec<String> = list_table_indices(table)
            .await?
            .into_iter()
            .filter(is_full_text_index)
            .flat_map(|index| index.columns)
            .filter(|column| !is_hidden(&self.hidden, column))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if fields.is_empty() {
            return Err(TableInputRejected(format!(
                "Table '{}' has no full-text indexed column to search",
                self.table
            ))
            .into());
        }
        Ok(Some(fields))
    }

    /// Refuses a vector search that would rank rows by a hidden vector column. Without a
    /// named column Lance picks the table's vector column itself, so any hidden one counts.
    pub(super) fn check_vector_column(&self, column: Option<&str>) -> Result<()> {
        if !self.is_restricted() {
            return Ok(());
        }
        if let Some(column) = column {
            return reject_hidden_columns(&[column], &self.hidden, self.table);
        }
        let hides_vector = self.schema.as_ref().is_some_and(|schema| {
            schema.fields().iter().any(|field| {
                is_vector_data_type(field.data_type()) && is_hidden(&self.hidden, field.name())
            })
        });
        if hides_vector {
            return Err(TableInputRejected(format!(
                "Table '{}' has no vector column to search",
                self.table
            ))
            .into());
        }
        Ok(())
    }
}

fn is_full_text_index(index: &IndexConfigDto) -> bool {
    let kind = index
        .index_type
        .to_ascii_lowercase()
        .replace([' ', '_'], "");
    kind == "fts" || kind == "fulltext" || kind == "inverted"
}

impl LanceDBVectorStore {
    /// Apply `reader`'s column read restrictions to every read through this store and to
    /// stores derived from it.
    pub fn with_column_reader(mut self, reader: ColumnReader) -> Self {
        self.column_reader = reader;
        self
    }

    pub fn column_reader(&self) -> &ColumnReader {
        &self.column_reader
    }

    /// Refuses a filter naming a column the reader may not read.
    pub(super) async fn check_filter(&self, filter: &str) -> Result<()> {
        if self.column_reader.is_unrestricted() {
            return Ok(());
        }
        let table = self.require_readable_table().await?;
        self.read_scope(&table).await?.check_expression(filter)
    }

    pub(super) async fn read_scope(&self, table: &Table) -> Result<ReadScope<'_>> {
        if self.column_reader.is_unrestricted() {
            return Ok(ReadScope {
                table: &self.table_name,
                hidden: Vec::new(),
                schema: None,
            });
        }
        let schema = table.schema().await?;
        Ok(ReadScope {
            table: &self.table_name,
            hidden: self.hidden_columns_in(&schema).await?,
            schema: Some(schema),
        })
    }

    /// Columns of `schema` the reader may not read. A restriction recorded on the latest
    /// version of `main` or of the selected branch also hides the column in older versions
    /// and other branches, so history cannot reveal a newly restricted column.
    pub(super) async fn hidden_columns_in(&self, schema: &Schema) -> Result<Vec<String>> {
        if self.column_reader.is_unrestricted() {
            return Ok(Vec::new());
        }
        let mut hidden = hidden_columns(schema, &self.column_reader);
        for latest in self.latest_reference_schemas().await? {
            for column in hidden_columns(&latest, &self.column_reader) {
                if !is_hidden(&hidden, &column) {
                    hidden.push(column);
                }
            }
        }
        Ok(hidden)
    }

    async fn latest_reference_schemas(&self) -> Result<Vec<SchemaRef>> {
        let on_main = self.selector.branch == "main";
        let mut schemas = Vec::new();
        if !on_main || self.selector.is_pinned() {
            let main = self
                .connection
                .open_table(&self.table_name)
                .execute()
                .await?;
            schemas.push(main.schema().await?);
        }
        if !on_main && self.selector.is_pinned() {
            let branch = self
                .connection
                .open_table(&self.table_name)
                .branch(&self.selector.branch)
                .execute()
                .await?;
            schemas.push(branch.schema().await?);
        }
        Ok(schemas)
    }

    /// Replace `column`'s metadata; its other field metadata, such as the table key, stays.
    /// Only an unrestricted reader (an admin) may change who can read the column.
    pub async fn set_column_metadata(
        &self,
        column: &str,
        metadata: ColumnMetadata,
    ) -> Result<ColumnMetadata> {
        self.ensure_unmanaged("schema changes")?;
        self.ensure_writable()?;
        let table = self.table.clone().ok_or_else(|| {
            anyhow!(
                "Table '{}' does not exist; create it before describing its columns",
                self.table_name
            )
        })?;
        let mut metadata = metadata
            .normalized(column)
            .map_err(|error| TableInputRejected(error.to_string()))?;
        table.checkout_latest().await?;
        let schema = table.schema().await?;
        let hidden = self.hidden_columns_in(&schema).await?;
        let field = schema
            .field_with_name(column)
            .ok()
            .filter(|_| !is_hidden(&hidden, column))
            .ok_or_else(|| no_such_column(&self.table_name, column))?;
        let current = ColumnMetadata::from_field_metadata(field.metadata());

        let readers = |attributes: &[String]| attributes.iter().cloned().collect::<BTreeSet<_>>();
        if readers(&current.read_attributes) == readers(&metadata.read_attributes) {
            metadata.read_attributes = current.read_attributes.clone();
        } else if !self.column_reader.is_unrestricted() {
            return Err(ColumnAccessRejected(format!(
                "Only admins can change who may read column '{column}' of table '{}'",
                self.table_name
            ))
            .into());
        }
        if metadata == current {
            return Ok(metadata);
        }
        table
            .update_field_metadata(&[column_metadata_update(column, &metadata)])
            .await
            .map_err(|error| {
                anyhow!(
                    "Updating the metadata of column '{column}' in table '{}' failed: {error}",
                    self.table_name
                )
            })?;
        Ok(metadata)
    }

    /// `column`'s metadata, if the reader may read the column.
    pub async fn column_metadata(&self, column: &str) -> Result<ColumnMetadata> {
        let schema = VectorStore::schema(self).await?;
        let field = schema
            .field_with_name(column)
            .map_err(|_| no_such_column(&self.table_name, column))?;
        Ok(ColumnMetadata::from_field_metadata(field.metadata()))
    }
}

/// A restore commits the restored version's schema. Re-apply the column metadata `latest`
/// carried, so restoring data never lifts a read restriction or drops a description.
pub(super) async fn carry_column_metadata_forward(latest: &Schema, restored: &Table) -> Result<()> {
    let schema = restored.schema().await?;
    let updates: Vec<_> = latest
        .fields()
        .iter()
        .filter_map(|field| {
            let restored_field = schema.field_with_name(field.name()).ok()?;
            let metadata = ColumnMetadata::from_field_metadata(field.metadata());
            (ColumnMetadata::from_field_metadata(restored_field.metadata()) != metadata)
                .then(|| column_metadata_update(field.name(), &metadata))
        })
        .collect();
    if !updates.is_empty() {
        restored.update_field_metadata(&updates).await?;
    }
    Ok(())
}

/// A table key on a hidden column would let its duplicates be counted.
pub(super) fn reject_hidden_key(hidden: &[String], column: &str, table: &str) -> Result<()> {
    if is_hidden(hidden, column) {
        return Err(no_such_column(table, column).into());
    }
    Ok(())
}
