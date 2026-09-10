use std::collections::BTreeSet;

use super::types::NodeMetadata;

#[derive(Debug, Clone)]
pub struct SearchQueryAnalysis {
    pub normalized: String,
    pub tokens: Vec<String>,
    pub expanded_tokens: Vec<String>,
}

pub fn analyze_search_query(query: &str) -> SearchQueryAnalysis {
    let normalized = query.trim().to_lowercase();
    let tokens = tokenize_query_text(query);

    let mut expanded = BTreeSet::new();
    for token in &tokens {
        expanded.insert(token.clone());
        for synonym in token_synonyms(token) {
            expanded.insert(synonym.to_string());
        }
    }

    if normalized.contains("send email") || normalized.contains("email send") {
        expanded.insert("smtp".to_string());
        expanded.insert("send".to_string());
        expanded.insert("mail".to_string());
    }

    if normalized.contains("read email")
        || normalized.contains("check inbox")
        || normalized.contains("read inbox")
        || normalized.contains("fetch email")
        || normalized.contains("fetch mail")
        || normalized.contains("past 2 days")
    {
        expanded.insert("imap".to_string());
        expanded.insert("inbox".to_string());
        expanded.insert("fetch".to_string());
        expanded.insert("list".to_string());
        expanded.insert("mail".to_string());
        expanded.insert("message".to_string());
    }

    if normalized.contains("open database")
        || normalized.contains("local db")
        || normalized.contains("local database")
    {
        expanded.insert("open".to_string());
        expanded.insert("local".to_string());
        expanded.insert("db".to_string());
        expanded.insert("database".to_string());
        expanded.insert("lance".to_string());
        expanded.insert("lancedb".to_string());
    }

    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "store"
                | "stores"
                | "stored"
                | "storing"
                | "write"
                | "writes"
                | "writing"
                | "written"
                | "persist"
                | "persists"
                | "persisted"
                | "persisting"
                | "put"
                | "puts"
                | "putting"
        )
    }) || (tokens.iter().any(|token| token == "vector")
        && tokens
            .iter()
            .any(|token| matches!(token.as_str(), "db" | "database")))
    {
        expanded.insert("open".to_string());
        expanded.insert("local".to_string());
        expanded.insert("db".to_string());
        expanded.insert("database".to_string());
        expanded.insert("table".to_string());
        expanded.insert("row".to_string());
        expanded.insert("record".to_string());
        expanded.insert("batch".to_string());
        expanded.insert("insert".to_string());
        expanded.insert("upsert".to_string());
    }

    if normalized.contains("datafusion sql") || normalized.contains("sql query") {
        expanded.insert("datafusion".to_string());
        expanded.insert("sql".to_string());
        expanded.insert("query".to_string());
        expanded.insert("session".to_string());
        expanded.insert("table".to_string());
    }

    if normalized.contains("hybrid search") {
        expanded.insert("hybrid".to_string());
        expanded.insert("vector".to_string());
        expanded.insert("full".to_string());
        expanded.insert("text".to_string());
        expanded.insert("fts".to_string());
        expanded.insert("search".to_string());
    }

    if normalized.contains("build index")
        || normalized.contains("create index")
        || normalized.contains("vector index")
        || normalized.contains("index build")
        || normalized.contains("vector db")
        || normalized.contains("vector database")
    {
        expanded.insert("build".to_string());
        expanded.insert("create".to_string());
        expanded.insert("index".to_string());
        expanded.insert("vector".to_string());
        expanded.insert("full".to_string());
        expanded.insert("text".to_string());
    }

    if normalized.contains("sentiment") {
        expanded.insert("classification".to_string());
        expanded.insert("classify".to_string());
        expanded.insert("label".to_string());
        expanded.insert("positive".to_string());
        expanded.insert("negative".to_string());
        expanded.insert("embedding".to_string());
    }

    SearchQueryAnalysis {
        normalized,
        tokens,
        expanded_tokens: expanded.into_iter().collect(),
    }
}

pub fn tokenize_query_text(text: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(text.len());
    let mut previous_was_lower_or_digit = false;

    for ch in text.chars() {
        if ch.is_ascii_uppercase() {
            if previous_was_lower_or_digit {
                normalized.push(' ');
            }
            normalized.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = false;
        } else if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            normalized.push(' ');
            previous_was_lower_or_digit = false;
        }
    }

    normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn enrich_node_metadata(mut metadata: NodeMetadata) -> NodeMetadata {
    metadata.required_inputs = metadata
        .inputs
        .iter()
        .filter(|pin| pin.data_type != "Execution" && pin.default_value.is_none())
        .map(|pin| pin.name.clone())
        .collect();

    let mut capability_tags = BTreeSet::new();
    let haystack = format!(
        "{} {} {} {}",
        metadata.name.to_lowercase(),
        metadata.friendly_name.to_lowercase(),
        metadata.description.to_lowercase(),
        metadata.category.clone().unwrap_or_default().to_lowercase(),
    );

    if haystack.contains("email") || haystack.contains("mail") {
        capability_tags.insert("email".to_string());
    }
    if haystack.contains("smtp") {
        capability_tags.insert("smtp".to_string());
        capability_tags.insert("send-email".to_string());
    }
    if haystack.contains("imap") {
        capability_tags.insert("imap".to_string());
        capability_tags.insert("read-email".to_string());
    }
    if haystack.contains("gmail") {
        capability_tags.insert("gmail".to_string());
    }
    if haystack.contains("inbox") || haystack.contains("mailbox") {
        capability_tags.insert("inbox".to_string());
    }
    if haystack.contains("fetch") {
        capability_tags.insert("fetch".to_string());
    }
    if haystack.contains("webhook") || haystack.contains("http event") {
        capability_tags.insert("trigger".to_string());
        capability_tags.insert("webhook".to_string());
    }
    if haystack.contains("notification") {
        capability_tags.insert("notification".to_string());
    }
    // Category-gated so the tags stay strong evidence: without them the path accessors fall under
    // the declaration acceptance gate for the plain-language queries that should find them.
    if metadata
        .category
        .as_deref()
        .is_some_and(|category| category.starts_with("Data/Files/Path"))
    {
        capability_tags.insert("file".to_string());
        capability_tags.insert("path".to_string());
        if haystack.contains("filename") {
            capability_tags.insert("filename".to_string());
            capability_tags.insert("basename".to_string());
            capability_tags.insert("stem".to_string());
        }
        if haystack.contains("extension") {
            capability_tags.insert("extension".to_string());
        }
        if haystack.contains("parent") || haystack.contains("child") || haystack.contains("folder")
        {
            capability_tags.insert("folder".to_string());
        }
    }
    if haystack.contains("http") || haystack.contains("request") {
        capability_tags.insert("http".to_string());
    }

    metadata.capability_tags = capability_tags.into_iter().collect();
    metadata.companion_nodes = companion_nodes_for(&metadata.name);
    metadata
}

pub fn score_catalog_metadata(metadata: &NodeMetadata, query: &str) -> i32 {
    let pin_matches = catalog_pin_search_matches(metadata, query);
    score_catalog_metadata_with_pin_matches(metadata, query, &pin_matches)
}

/// Reuse pin matches when declaration assessment needs the same field evidence as ranking.
pub fn score_catalog_metadata_with_pin_matches(
    metadata: &NodeMetadata,
    query: &str,
    pin_matches: &[String],
) -> i32 {
    let analysis = analyze_search_query(query);
    let name_lower = metadata.name.to_lowercase();
    let friendly_lower = metadata.friendly_name.to_lowercase();
    let desc_lower = metadata.description.to_lowercase();
    let category = metadata.category.clone().unwrap_or_default().to_lowercase();
    let qualified_lower = metadata
        .qualified_spelling()
        .unwrap_or_default()
        .to_lowercase();
    let symbol_tokens = tokenize_query_text(&metadata.name);
    let friendly_tokens = tokenize_query_text(&metadata.friendly_name);
    let qualified_tokens = tokenize_query_text(&qualified_lower);

    let mut score = 0i32;

    if !analysis.normalized.is_empty() {
        if name_lower.contains(&analysis.normalized) {
            score += 100;
        }
        if !qualified_lower.is_empty() && qualified_lower.contains(&analysis.normalized) {
            score += 100;
        }
        if friendly_lower.contains(&analysis.normalized) {
            score += 90;
        }
        if desc_lower.contains(&analysis.normalized) {
            score += 35;
        }
    }

    for token in &analysis.expanded_tokens {
        if name_lower.contains(token) {
            score += 30;
        }
        if !qualified_lower.is_empty() && qualified_lower.contains(token) {
            score += 30;
        }
        if friendly_lower.contains(token) {
            score += 25;
        }
        if category.contains(token) {
            score += 20;
        }
        if desc_lower.contains(token) {
            score += 10;
        }
        if metadata.capability_tags.iter().any(|tag| tag == token) {
            score += 18;
        }
        let fuzzy_symbol_match = symbol_tokens
            .iter()
            .chain(friendly_tokens.iter())
            .chain(qualified_tokens.iter())
            .any(|candidate| candidate.len() > 3 && strsim::jaro_winkler(candidate, token) >= 0.9);
        if fuzzy_symbol_match {
            score += 16;
        }
    }

    let name_parts: Vec<&str> = name_lower.split([':', '_']).collect();
    for token in &analysis.tokens {
        if name_parts.iter().any(|part| part == token) {
            score += 15;
        }
    }

    if analysis.normalized.contains("send email") && name_lower.contains("email_smtp_send") {
        score += 120;
    }

    if (analysis.normalized.contains("smtp") || analysis.normalized.contains("gmail"))
        && name_lower.contains("email_smtp_connect")
    {
        score += 80;
    }

    if (analysis.normalized.contains("read email")
        || analysis.normalized.contains("check inbox")
        || analysis.normalized.contains("imap")
        || analysis.normalized.contains("unread"))
        && (name_lower.contains("email_imap_connect")
            || name_lower.contains("mail_imap_list")
            || name_lower.contains("email_imap_inbox_fetch_mail")
            || name_lower.contains("mail_imap_inbox"))
    {
        score += 70;
    }

    // Pin text improves recall without outweighing a matching operation or service name.
    score + (pin_matches.len().min(4) as i32) * 12
}

const MAX_PIN_SEARCH_PINS: usize = 64;
const MAX_PIN_SEARCH_CHARS: usize = 8_192;
const MAX_PIN_SEARCH_FIELD_CHARS: usize = 512;
const MAX_PIN_SEARCH_SCHEMA_BYTES: usize = 16_384;
const MAX_PIN_SEARCH_SCHEMA_NODES: usize = 128;

struct PinSearchText {
    tokens: BTreeSet<String>,
    remaining_chars: usize,
}

impl PinSearchText {
    fn add(&mut self, text: &str) {
        let limit = self.remaining_chars.min(MAX_PIN_SEARCH_FIELD_CHARS);
        if limit == 0 {
            return;
        }
        let mut chars = text.chars();
        let bounded: String = chars.by_ref().take(limit).collect();
        self.remaining_chars -= bounded.chars().count();
        let mut tokens = tokenize_query_text(&bounded);
        // A clipped identifier must not become an exact match for its shorter prefix.
        if chars.next().is_some_and(|ch| ch.is_ascii_alphanumeric())
            && bounded.ends_with(|ch: char| ch.is_ascii_alphanumeric())
        {
            tokens.pop();
        }
        self.tokens.extend(tokens);
    }
}

fn collect_schema_pin_search_text(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    depth: usize,
    remaining_nodes: &mut usize,
    visited_refs: &mut BTreeSet<String>,
    text: &mut PinSearchText,
) {
    if depth >= 8 || *remaining_nodes == 0 || text.remaining_chars == 0 {
        return;
    }
    let Some(object) = schema.as_object() else {
        return;
    };
    *remaining_nodes -= 1;
    for field in ["title", "description", "type"] {
        if let Some(value) = object.get(field).and_then(serde_json::Value::as_str) {
            text.add(value);
        }
    }
    if let Some(types) = object.get("type").and_then(serde_json::Value::as_array) {
        for kind in types.iter().take(16).filter_map(serde_json::Value::as_str) {
            text.add(kind);
        }
    }
    if let Some(properties) = object
        .get("properties")
        .and_then(serde_json::Value::as_object)
    {
        for (name, property) in properties.iter().take(64) {
            text.add(name);
            collect_schema_pin_search_text(
                root,
                property,
                depth + 1,
                remaining_nodes,
                visited_refs,
                text,
            );
        }
    }
    for field in ["items", "additionalProperties"] {
        if let Some(child) = object.get(field) {
            collect_schema_pin_search_text(
                root,
                child,
                depth + 1,
                remaining_nodes,
                visited_refs,
                text,
            );
        }
    }
    for field in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = object.get(field).and_then(serde_json::Value::as_array) {
            for child in children.iter().take(16) {
                collect_schema_pin_search_text(
                    root,
                    child,
                    depth + 1,
                    remaining_nodes,
                    visited_refs,
                    text,
                );
            }
        }
    }
    if let Some(reference) = object.get("$ref").and_then(serde_json::Value::as_str)
        && let Some(pointer) = reference.strip_prefix("#/")
        && visited_refs.insert(reference.to_string())
        && let Some(target) = root.pointer(&format!("/{pointer}"))
    {
        collect_schema_pin_search_text(
            root,
            target,
            depth + 1,
            remaining_nodes,
            visited_refs,
            text,
        );
    }
}

/// Bounded pin tokens prepared once for a catalog snapshot and reused across queries.
#[derive(Debug, Clone)]
pub struct CatalogPinSearchIndex {
    tokens: BTreeSet<String>,
}

impl CatalogPinSearchIndex {
    pub fn new(metadata: &NodeMetadata) -> Self {
        let mut text = PinSearchText {
            tokens: BTreeSet::new(),
            remaining_chars: MAX_PIN_SEARCH_CHARS,
        };
        let pins = || {
            metadata
                .inputs
                .iter()
                .chain(&metadata.outputs)
                .take(MAX_PIN_SEARCH_PINS)
        };
        // Collect every pin's own metadata before spending the remaining budget on schemas.
        for pin in pins() {
            for value in [
                &pin.name,
                &pin.friendly_name,
                &pin.description,
                &pin.data_type,
                &pin.value_type,
            ] {
                text.add(value);
            }
        }
        let mut remaining_schema_bytes = MAX_PIN_SEARCH_SCHEMA_BYTES;
        let mut remaining_nodes = MAX_PIN_SEARCH_SCHEMA_NODES;
        for pin in pins() {
            let Some(schema) = pin
                .schema
                .as_deref()
                .filter(|schema| schema.len() <= remaining_schema_bytes)
            else {
                continue;
            };
            if text.remaining_chars == 0 || remaining_nodes == 0 {
                break;
            }
            remaining_schema_bytes -= schema.len();
            if let Ok(schema) = serde_json::from_str::<serde_json::Value>(schema) {
                collect_schema_pin_search_text(
                    &schema,
                    &schema,
                    0,
                    &mut remaining_nodes,
                    &mut BTreeSet::new(),
                    &mut text,
                );
            }
        }
        Self {
            tokens: text.tokens,
        }
    }

    /// Return exact query tokens in first-occurrence order, without expanding synonyms.
    pub fn matches(&self, query: &str) -> Vec<String> {
        self.match_tokens(tokenize_query_text(query))
    }

    fn match_tokens(&self, query_tokens: Vec<String>) -> Vec<String> {
        let mut seen = BTreeSet::new();
        query_tokens
            .into_iter()
            .take(64)
            .filter(|token| self.tokens.contains(token) && seen.insert(token.clone()))
            .collect()
    }
}

/// Original query tokens found in bounded pin metadata, in first-occurrence order. Matching uses
/// `tokenize_query_text` only, without synonyms, fuzzy matching or semantic canonicalization.
/// These are weak retrieval signals, never proof that a node performs the requested operation.
/// Schemas contribute known metadata and reachable properties, never defaults or example values.
pub fn catalog_pin_search_matches(metadata: &NodeMetadata, query: &str) -> Vec<String> {
    let query_tokens = tokenize_query_text(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }
    CatalogPinSearchIndex::new(metadata).match_tokens(query_tokens)
}

pub fn search_result_hint_lines(metadata: &NodeMetadata) -> Vec<String> {
    let mut hints = Vec::new();

    if !metadata.required_inputs.is_empty() {
        hints.push(format!("requires: {}", metadata.required_inputs.join(", ")));
    }

    if !metadata.companion_nodes.is_empty() {
        hints.push(format!(
            "pairs with: {}",
            metadata.companion_nodes.join(", ")
        ));
    }

    if !metadata.capability_tags.is_empty() {
        hints.push(format!("tags: {}", metadata.capability_tags.join(", ")));
    }

    hints
}

/// Compact model-facing rendering of catalog search results (one line per node + usage hints).
/// Single source for the `catalog_search` tool output across every backend executor.
pub fn render_catalog_search_results(results: &[NodeMetadata]) -> String {
    if results.is_empty() {
        return "No nodes found matching your query. Try different keywords.".to_string();
    }

    results
        .iter()
        .map(|meta| {
            let hints = search_result_hint_lines(meta);
            if hints.is_empty() {
                meta.to_compact()
            } else {
                format!("{} [{}]", meta.to_compact(), hints.join("; "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn token_synonyms(token: &str) -> &'static [&'static str] {
    match token {
        "email" | "mail" => &["smtp", "imap", "gmail", "outlook", "message"],
        "gmail" => &["smtp", "imap", "google", "email"],
        "inbox" => &["imap", "mailbox", "email", "message"],
        "unread" => &["new", "unseen", "imap", "inbox"],
        "db" => &["database", "lance", "lancedb", "local", "table"],
        "database" => &["db", "lance", "lancedb", "local", "table"],
        "lance" | "lancedb" => &["database", "db", "vector", "table"],
        "sql" => &["datafusion", "query", "table"],
        "datafusion" => &["sql", "query", "session", "table"],
        "embed" | "embedding" => &["vector", "model", "document", "query"],
        "vector" => &["embedding", "search", "index", "lancedb"],
        "fts" => &["full", "text", "search", "index"],
        "hybrid" => &["vector", "full", "text", "search", "rerank"],
        "index" => &["build", "create", "optimize", "vector", "full", "text"],
        "batch" => &["many", "bulk", "array", "insert", "upsert"],
        "put" | "store" | "write" | "persist" => {
            &["database", "insert", "upsert", "local", "table", "record"]
        }
        "row" | "record" => &["database", "insert", "table", "struct"],
        "notification" => &["email", "mail", "send"],
        "receipt" => &["email", "mail", "send"],
        "followup" | "follow-up" => &["email", "send", "notification"],
        "webhook" => &["trigger", "http", "event"],
        "file" => &["path", "flowpath", "filename", "extension", "storage"],
        "filename" | "basename" => &["file", "path", "name", "extension", "stem"],
        "extension" | "ext" => &["file", "path", "filename", "suffix"],
        "folder" | "directory" | "dir" => &["path", "parent", "child", "list", "storage"],
        "stem" => &["filename", "file", "path", "extension"],
        _ => &[],
    }
}

fn companion_nodes_for(node_name: &str) -> Vec<String> {
    match node_name {
        "email_smtp_connect" => vec!["email_smtp_send".to_string()],
        "email_smtp_send" => vec!["email_smtp_connect".to_string()],
        "email_imap_connect" => vec![
            "mail_imap_list_inboxes".to_string(),
            "mail_imap_inbox".to_string(),
            "mail_imap_list".to_string(),
        ],
        "mail_imap_inbox" => vec!["mail_imap_list".to_string()],
        "mail_imap_list" => vec![
            "email_imap_inbox_fetch_mail".to_string(),
            "email_imap_mark_seen".to_string(),
        ],
        "email_imap_inbox_fetch_mail" => vec![
            "email_imap_connect".to_string(),
            "mail_imap_inbox".to_string(),
            "mail_imap_list".to_string(),
            "email_get_headers".to_string(),
            "email_get_content".to_string(),
            "mail_address_fields".to_string(),
            "email_imap_mark_seen".to_string(),
            "email_imap_move_message".to_string(),
        ],
        // A FlowPath carries no file attributes, so every producer of one advertises the accessors.
        // Without this the model reaches for a generic struct-field read and silently gets null.
        "path_from_upload_dir"
        | "path_from_storage_dir"
        | "path_from_cache_dir"
        | "path_from_user_dir"
        | "path_virtual_dir"
        | "path_list_paths"
        | "path_list_folders"
        | "pathbuf_to_path"
        | "a2ui_get_file_input_files"
        | "events_chat_attachment_from_signed_url" => vec![
            "filename".to_string(),
            "extension".to_string(),
            "raw_path".to_string(),
            "parent".to_string(),
            "child".to_string(),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::PinMetadata;
    use super::*;
    use serde_json::json;

    fn pin(name: &str, description: &str) -> PinMetadata {
        PinMetadata {
            name: name.into(),
            friendly_name: name.into(),
            description: description.into(),
            data_type: "String".into(),
            value_type: "Normal".into(),
            default_value: None,
            schema: None,
            is_generic: false,
            valid_values: None,
            enforce_schema: false,
        }
    }

    fn metadata(
        name: &str,
        friendly_name: &str,
        description: &str,
        namespace: &str,
        alias: &str,
    ) -> NodeMetadata {
        enrich_node_metadata(NodeMetadata {
            name: name.into(),
            friendly_name: friendly_name.into(),
            description: description.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            category: None,
            required_inputs: Vec::new(),
            companion_nodes: Vec::new(),
            capability_tags: Vec::new(),
            namespace: Some(namespace.into()),
            alias: Some(alias.into()),
            receiver: None,
        })
    }

    #[test]
    fn prepared_pin_index_preserves_query_order_limits_and_wrapper_results() {
        let mut metadata = metadata("fields", "Fields", "Read fields", "payload", "fields");
        let mut payload = pin("payload", "");
        payload.schema = Some(
            json!({
                "description": "Idempotency envelope",
                "properties": {"correlationKey": {"type": "string"}},
                "default": "privatevalue"
            })
            .to_string(),
        );
        metadata.inputs.push(payload);
        metadata.outputs.push(pin("deltaLink", ""));
        let index = CatalogPinSearchIndex::new(&metadata);
        for (query, expected) in [
            ("", vec![]),
            ("  --  ", vec![]),
            (
                "deltaLink correlationKey deltaLink",
                vec!["delta", "link", "correlation", "key"],
            ),
            ("Idempotency privatevalue", vec!["idempotency"]),
            ("key delta key", vec!["key", "delta"]),
            ("unmatched", vec![]),
        ] {
            assert_eq!(index.matches(query), expected, "{query}");
            assert_eq!(
                index.matches(query),
                catalog_pin_search_matches(&metadata, query),
                "{query}"
            );
        }
        for (prefix_len, expected) in [(63, vec!["idempotency"]), (64, vec![])] {
            let query = format!("{}idempotency", "unmatched ".repeat(prefix_len));
            assert_eq!(index.matches(&query), expected);
            assert_eq!(
                index.matches(&query),
                catalog_pin_search_matches(&metadata, &query)
            );
        }
    }

    #[test]
    fn prepared_pin_index_owns_the_metadata_snapshot() {
        let mut metadata = metadata("fields", "Fields", "Read fields", "payload", "fields");
        let mut payload = pin("payload", "");
        payload.schema = Some(json!({"description": "original"}).to_string());
        metadata.inputs.push(payload);
        let original_index = CatalogPinSearchIndex::new(&metadata);
        metadata.inputs[0].schema = Some(json!({"description": "updated"}).to_string());
        let updated_index = CatalogPinSearchIndex::new(&metadata);

        assert_eq!(original_index.matches("original updated"), ["original"]);
        assert_eq!(updated_index.matches("original updated"), ["updated"]);
        assert_eq!(
            catalog_pin_search_matches(&metadata, "original updated"),
            ["updated"]
        );
    }

    #[test]
    fn pin_queries_recall_real_catalog_contract_fields() {
        // Names and descriptions mirror the generated email, data and data-studio declarations.
        let mut attachment = metadata(
            "attachment_fields",
            "Attachment → Fields",
            "Access filename, content_type and data",
            "email",
            "attachmentToFields",
        );
        attachment
            .outputs
            .push(pin("content_type", "MIME content type"));
        let mut graph = metadata(
            "data_microsoft_graph_request",
            "Graph Request",
            "Call any Microsoft Graph endpoint with optional collection pagination",
            "microsoft",
            "graphRequest",
        );
        graph.outputs.push(pin("delta_link", "@odata.deltaLink"));
        let mut action = metadata(
            "ontology_action_input",
            "Ontology Action Input",
            "Reads the typed objects and parameters the ontology action was invoked with",
            "ontology",
            "actionInput",
        );
        action
            .outputs
            .push(pin("idempotency_key", "Client-supplied retry key, if any"));
        for (metadata, query, expected) in [
            (attachment, "MIME", vec!["mime"]),
            (graph, "deltaLink", vec!["delta", "link"]),
            (action, "idempotency", vec!["idempotency"]),
        ] {
            let matches = catalog_pin_search_matches(&metadata, query);
            assert_eq!(matches, expected, "{query}");
            assert_eq!(
                score_catalog_metadata_with_pin_matches(&metadata, query, &[]),
                0,
                "the query should depend on pin metadata: {query}"
            );
            assert!(
                score_catalog_metadata(&metadata, query) > 0,
                "missing recall for {query}"
            );
            assert_eq!(
                score_catalog_metadata(&metadata, query),
                score_catalog_metadata_with_pin_matches(&metadata, query, &matches)
            );
        }
    }

    #[test]
    fn schema_recall_follows_known_properties_without_indexing_values_or_unused_definitions() {
        let mut metadata = metadata(
            "payload_fields",
            "Payload Fields",
            "Read typed payload fields",
            "payload",
            "fields",
        );
        let mut payload = pin("payload", "Input payload");
        payload.data_type = "Struct".into();
        payload.value_type = "Array".into();
        payload.friendly_name = "Envelope".into();
        payload.default_value = Some("privateDefault".into());
        payload.valid_values = Some(vec!["privateEnum".into()]);
        payload.schema = Some(json!({
            "$ref": "#/$defs/Envelope",
            "$defs": {
                "Envelope": {
                    "type": "object",
                    "properties": {
                        "correlationKey": {"type": "string", "description": "Idempotency key for deduplication", "default": "privateSchemaDefault", "examples": ["privateExample"], "enum": ["privateChoice"]},
                        "rows": {"type": "array", "items": {"properties": {"expiresAt": {"description": "Expiration timestamp"}}}},
                        "next": {"$ref": "#/$defs/Envelope"}
                    },
                    "unknown": {"description": "privateUnknown"}
                },
                "Unused": {"description": "privateUnused"}
            }
        }).to_string());
        metadata.inputs.push(payload);
        assert_eq!(
            catalog_pin_search_matches(
                &metadata,
                "Idempotency correlationKey Envelope Array Struct expiresAt Idempotency"
            ),
            [
                "idempotency",
                "correlation",
                "key",
                "envelope",
                "array",
                "struct",
                "expires",
                "at"
            ]
        );
        assert!(catalog_pin_search_matches(&metadata, "privateDefault privateEnum privateSchemaDefault privateExample privateChoice privateUnknown privateUnused").is_empty());
    }

    #[test]
    fn pin_mentions_do_not_outrank_the_named_operation() {
        let upsert = metadata(
            "upsert_local_db",
            "Upsert",
            "Inserts if the Item does not exist, Updates if it does",
            "db",
            "upsert",
        );
        let mut inspect = metadata(
            "inspect_local_db",
            "Inspect",
            "Reads existing records",
            "db",
            "inspect",
        );
        inspect.inputs =
            vec![pin("upsert", "Upsert operation being inspected"); MAX_PIN_SEARCH_PINS];
        assert_eq!(
            catalog_pin_search_matches(&inspect, "upsert upsert"),
            ["upsert"]
        );
        assert!(
            score_catalog_metadata(&upsert, "upsert") > score_catalog_metadata(&inspect, "upsert")
        );
        assert_eq!(
            score_catalog_metadata(&inspect, "upsert")
                - score_catalog_metadata_with_pin_matches(&inspect, "upsert", &[]),
            12
        );
    }

    #[test]
    fn pin_and_schema_search_budgets_keep_oversized_or_recursive_metadata_bounded() {
        let mut metadata = metadata(
            "payload_fields",
            "Payload Fields",
            "Read payload fields",
            "payload",
            "fields",
        );
        let mut payload = pin("known", "");
        payload.schema = Some(format!(
            "{{\"description\":\"{} oversized\"}}",
            "x".repeat(MAX_PIN_SEARCH_SCHEMA_BYTES)
        ));
        metadata.inputs.push(payload);
        assert_eq!(
            catalog_pin_search_matches(&metadata, "known oversized"),
            ["known"]
        );
        let mut deep = json!({"description": "unreachable"});
        for _ in 0..8 {
            deep = json!({"items": deep});
        }
        metadata.inputs[0].schema = Some(deep.to_string());
        assert!(catalog_pin_search_matches(&metadata, "unreachable").is_empty());
        metadata.inputs = vec![pin("known", ""); MAX_PIN_SEARCH_PINS];
        metadata.outputs.push(pin("outside", ""));
        assert!(catalog_pin_search_matches(&metadata, "outside").is_empty());
    }

    #[test]
    fn input_and_output_do_not_imply_database_storage() {
        for query in ["input", "output schema", "throughput", "storefront"] {
            let analysis = analyze_search_query(query);
            assert!(
                !analysis
                    .expanded_tokens
                    .iter()
                    .any(|token| token == "database" || token == "upsert"),
                "{query}"
            );
        }
        for query in ["put output", "store record", "persisted data", "vector db"] {
            assert!(
                analyze_search_query(query)
                    .expanded_tokens
                    .iter()
                    .any(|token| token == "upsert"),
                "{query}"
            );
        }
    }

    #[test]
    fn imap_fetch_companions_include_the_required_upstream_chain() {
        let companions = companion_nodes_for("email_imap_inbox_fetch_mail");
        for required in [
            "email_imap_connect",
            "mail_imap_inbox",
            "mail_imap_list",
            "email_get_headers",
            "email_get_content",
            "mail_address_fields",
            "email_imap_mark_seen",
        ] {
            assert!(
                companions.iter().any(|companion| companion == required),
                "missing {required} in {companions:?}"
            );
        }
    }
}
