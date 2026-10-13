//! Column read restrictions on ontology overlays.

use std::collections::{HashMap, HashSet};

use flow_like_storage_contracts::column_metadata::ColumnReader;
use flow_like_types::{Result, anyhow};
use lancedb::Connection;

use super::{GraphOverlayDef, effective_node_id_column_checked, freeze_remote_contract_projection};
use crate::databases::column_access::{hidden_columns, is_hidden};

/// Hidden columns per table, as `reader` sees the latest version of each.
pub(crate) async fn hidden_columns_by_table<'a>(
    connection: &Connection,
    tables: impl IntoIterator<Item = &'a String>,
    reader: &ColumnReader,
) -> Result<HashMap<String, Vec<String>>> {
    let mut hidden = HashMap::new();
    if reader.is_unrestricted() {
        return Ok(hidden);
    }
    for table in tables {
        if hidden.contains_key(table) {
            continue;
        }
        let schema = connection
            .open_table(table)
            .execute()
            .await
            .map_err(|error| anyhow!("Failed to open table '{table}': {error}"))?
            .schema()
            .await
            .map_err(|error| anyhow!("Failed to read schema for '{table}': {error}"))?;
        hidden.insert(table.clone(), hidden_columns(&schema, reader));
    }
    Ok(hidden)
}

/// The overlay as `reader` sees it. Hidden columns leave every property list and object
/// view, a hidden display column falls back to the identity, and an object type whose
/// identity is hidden disappears together with the relationships that need it or whose
/// join columns are hidden. The projection is frozen to the visible columns, so no read
/// falls back to the live schema and picks a hidden column up again.
pub async fn restrict_overlay(
    connection: &Connection,
    mut overlay: GraphOverlayDef,
    reader: &ColumnReader,
) -> Result<GraphOverlayDef> {
    let tables: Vec<String> = overlay
        .nodes
        .iter()
        .map(|node| node.table.clone())
        .chain(overlay.edges.iter().map(|edge| edge.table.clone()))
        .collect();
    let hidden_by_table = hidden_columns_by_table(connection, &tables, reader).await?;
    if hidden_by_table.values().all(Vec::is_empty) {
        return Ok(overlay);
    }
    let hidden = |table: &str, column: &str| {
        hidden_by_table
            .get(table)
            .is_some_and(|hidden| is_hidden(hidden, column))
    };

    freeze_remote_contract_projection(connection, &mut overlay).await?;
    let identities = overlay
        .nodes
        .iter()
        .map(|node| {
            let identity = effective_node_id_column_checked(&overlay, &node.label)?
                .ok_or_else(|| anyhow!("Object type '{}' has no identity column", node.label))?;
            Ok((node.label.clone(), identity))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let node_tables: HashMap<String, String> = overlay
        .nodes
        .iter()
        .map(|node| (node.label.clone(), node.table.clone()))
        .collect();

    let removed: HashSet<String> = overlay
        .nodes
        .iter()
        .filter(|node| {
            hidden(&node.table, &node.id_column) || hidden(&node.table, &identities[&node.label])
        })
        .map(|node| node.label.clone())
        .collect();
    overlay.nodes.retain(|node| !removed.contains(&node.label));
    for node in &mut overlay.nodes {
        // Relationships that carried an identity override may be dropped below.
        node.id_column = identities[&node.label].clone();
        if node
            .display_column
            .as_deref()
            .is_some_and(|column| hidden(&node.table, column))
        {
            node.display_column = None;
        }
        node.property_columns
            .retain(|property| !hidden(&node.table, &property.name));
    }

    let endpoint_hidden = |label: &str, column: Option<&str>| {
        column.is_some_and(|column| {
            node_tables
                .get(label)
                .is_some_and(|table| hidden(table, column))
        })
    };
    overlay.edges.retain(|edge| {
        !hidden(&edge.table, &edge.src_column)
            && !hidden(&edge.table, &edge.dst_column)
            && !removed.contains(&edge.src_label)
            && !removed.contains(&edge.dst_label)
            && !endpoint_hidden(&edge.src_label, edge.src_node_column.as_deref())
            && !endpoint_hidden(&edge.dst_label, edge.dst_node_column.as_deref())
    });
    for edge in &mut overlay.edges {
        edge.property_columns
            .retain(|property| !hidden(&edge.table, &property.name));
    }

    overlay.object_views.retain(|view| {
        node_tables.contains_key(&view.object_type) && !removed.contains(&view.object_type)
    });
    for view in &mut overlay.object_views {
        let table = &node_tables[&view.object_type];
        if view
            .title_property
            .as_deref()
            .is_some_and(|column| hidden(table, column))
        {
            view.title_property = None;
        }
        view.prominent_properties
            .retain(|column| !hidden(table, column));
    }
    Ok(overlay)
}
