use serde_json::Value;

pub fn file_arguments(value: &Value) -> bool {
    external_arguments(value, false)
}

pub fn file_paths(value: &Value) -> bool {
    external_arguments(value, true)
}

pub fn command_file_arguments(editor: &str, command: &str, value: &Value) -> bool {
    // These commands store a hyperlink in the document and do not fetch it.
    let stores_link = matches!(
        (editor, command.to_ascii_lowercase().as_str()),
        ("wordcraft", "insert.link")
            | ("gridcraft", "insert.link")
            | ("deckcraft", "insert.hyperlink")
    );
    external_arguments(value, stores_link)
}

fn external_arguments(value: &Value, allow_url: bool) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            let external = matches!(
                key.as_str(),
                "path"
                    | "paths"
                    | "filename"
                    | "directory"
                    | "allow_paths"
                    | "output_path"
                    | "input_path"
            ) || (!allow_url && matches!(key.as_str(), "url" | "urls"));
            (external
                && (value.is_string()
                    || value
                        .as_array()
                        .is_some_and(|values| values.iter().any(Value::is_string))))
                || external_arguments(value, allow_url)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| external_arguments(value, allow_url)),
        _ => false,
    }
}

pub fn allowed_command(command: &str) -> bool {
    let id = command.to_ascii_lowercase();
    !id.starts_with("file.")
        && !id.starts_with("doc.")
        && !id.starts_with("app.")
        && !id.starts_with("help.")
        && !id.starts_with("diagnostics.")
        && !id.starts_with("ui.")
        && !id.split(['.', '_', '-']).any(|part| {
            matches!(
                part,
                "script" | "scripts" | "javascript" | "runscript" | "execscript"
            )
        })
        && !["document.", "session."].iter().any(|prefix| {
            id.strip_prefix(prefix).is_some_and(|suffix| {
                [
                    "new", "open", "save", "close", "export", "import", "revert", "switch",
                    "recover",
                ]
                .iter()
                .any(|operation| suffix.starts_with(operation))
            })
        })
        && !matches!(
            id.as_str(),
            "new"
                | "qnew"
                | "open"
                | "close"
                | "closeall"
                | "save"
                | "qsave"
                | "saveas"
                | "recover"
                | "export"
                | "exportpdf"
                | "plot"
                | "wblock"
                | "quit"
                | "exit"
        )
}

pub fn command_read_only(id: &str) -> bool {
    matches!(
        id,
        "document.text"
            | "document.inspect"
            | "drawing.inspect"
            | "session.inspect"
            | "entities"
            | "engine.commands"
            | "model.edges"
            | "model.faces"
            | "inspect.measure"
            | "sketch.inspect"
            | "slide.inspect"
            | "story.get"
            | "cell.get"
            | "formulas.evaluate"
            | "formulas.functions"
    ) || id.ends_with(".inspect")
}

pub fn tool_allowed(tool: &Value) -> bool {
    let name = tool["name"].as_str().unwrap_or("");
    !name.is_empty()
        && !matches!(
            name,
            "screenshot"
                | "ui_screenshot"
                | "open_file"
                | "save_file"
                | "export_png"
                | "place_image"
                | "insert_picture"
                | "menu_invoke"
                | "dialog_set"
                | "dialog_confirm"
                | "dialog_cancel"
                | "new_document"
                | "open_document"
                | "save_document"
                | "new_presentation"
                | "open_presentation"
                | "save_presentation"
                | "new_workbook"
                | "open_workbook"
                | "save_workbook"
                | "new_drawing"
                | "new_design"
                | "new_session"
                | "open_session"
                | "save_session"
                | "open"
                | "save"
                | "export"
                | "import_audio"
                | "bounce_mix"
                | "crash_reports"
                | "command_line"
                | "script"
        )
}

pub fn tool_read_only(tool: &Value) -> bool {
    tool["annotations"]["readOnlyHint"]
        .as_bool()
        .unwrap_or_else(|| {
            matches!(
                tool["name"].as_str(),
                Some(
                    "list_commands"
                        | "inspect_drawing"
                        | "query_entities"
                        | "render"
                        | "ui_inspect"
                        | "inspect_session"
                        | "parity"
                )
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lifecycle_and_scripts_cannot_bypass_case_or_namespace_guards() {
        for command in [
            "FILE.NEW",
            "document.saveAs",
            "session.open",
            "script.run",
            "timeline.export_script",
            "engine.runScript",
        ] {
            assert!(!allowed_command(command), "{command}");
        }
        for command in [
            "solid.box",
            "document.description",
            "audio.transcription",
            "clip.setDescription",
        ] {
            assert!(allowed_command(command), "{command}");
        }
    }

    #[test]
    fn geometry_path_ids_are_not_filesystem_paths() {
        assert!(!file_arguments(
            &json!({"path": 42, "points": [[0, 1], [2, 3]]})
        ));
        assert!(!file_arguments(&json!({"path": [[0, 1], [2, 3]]})));
        assert!(file_arguments(
            &json!({"commands": [{"params": {"PATH": "/tmp/model"}}]})
        ));
    }

    #[test]
    fn hyperlinks_store_urls_without_allowing_media_reads() {
        let link = json!({"params": {"url": "https://example.org", "text": "Example"}});
        assert!(!command_file_arguments("wordcraft", "insert.link", &link));
        assert!(!command_file_arguments(
            "deckcraft",
            "INSERT.HYPERLINK",
            &link
        ));
        assert!(command_file_arguments("wordcraft", "insert.picture", &link));
        assert!(command_file_arguments("vectorcraft", "insert.link", &link));
        assert!(command_file_arguments(
            "wordcraft",
            "insert.link",
            &json!({"params": {"path": "/tmp/file"}})
        ));
    }
}
