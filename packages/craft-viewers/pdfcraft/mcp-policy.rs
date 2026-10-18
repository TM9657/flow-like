// The embedded host owns file I/O and the document lifecycle. Keep the upstream
// schemas intact; only advertise tools that can operate on this live session.
fn live_tool_allowed(name: &str) -> bool {
    if matches!(
        name,
        "doc_open"
            | "doc_open_revision"
            | "doc_create"
            | "doc_close"
            | "doc_save"
            | "doc_combine"
            | "doc_create_multiple"
            | "doc_split"
            | "page_extract"
            | "link_follow"
            | "print_document"
            | "doc_print"
            | "printers"
            | "printer_options"
    ) {
        return false;
    }
    let Some(tool) = tools::find(name) else {
        return false;
    };
    fn uses_files(schema: &Value) -> bool {
        match schema {
            Value::Object(map) => map.iter().any(|(key, value)| {
                (matches!(
                    key.as_str(),
                    "path"
                        | "paths"
                        | "file"
                        | "files"
                        | "folder"
                        | "directory"
                        | "directories"
                        | "out"
                        | "out_dir"
                        | "output"
                ) && value.get("type").and_then(Value::as_str) != Some("array"))
                    || uses_files(value)
            }),
            Value::Array(values) => values.iter().any(uses_files),
            _ => false,
        }
    }
    !uses_files(&tool.input_schema)
}
