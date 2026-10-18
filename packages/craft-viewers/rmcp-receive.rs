pub fn unwrap_control(reply: Value) -> Result<Value, String> {
    if reply["ok"].as_bool() == Some(true) {
        Ok(reply.get("result").cloned().unwrap_or(Value::Null))
    } else {
        Err(reply["error"]
            .as_str()
            .unwrap_or("The editor rejected the request.")
            .into())
    }
}

pub fn authorize_command(command: &str, params: &Value, read_only: bool) -> Result<(), String> {
    let id = command.to_ascii_lowercase();
    // FilmCraft puts project-item creation in File; these edits retain the open project.
    let project_item = EDITOR == "filmcraft"
        && matches!(
            id.as_str(),
            "file.newsequence"
                | "file.newbin"
                | "file.newadjustmentlayer"
                | "file.newcolormatte"
                | "file.newblackvideo"
                | "file.newtransparentvideo"
                | "file.newbarsandtone"
                | "file.newcountingleader"
        );
    if (!allowed_command(command) && !project_item)
        || file_arguments(params)
        || matches!(
            id.as_str(),
            "layer.smartobjects.editcontents" | "window.newwindowfordocument"
        )
    {
        return Err(HOST_FILES.into());
    }
    if read_only && !command_read_only(command) && command != "command.list" {
        return Err("This document is read-only.".into());
    }
    Ok(())
}

pub fn receive(
    message: &wasm_bindgen::JsValue,
    read_only: bool,
    call: &mut dyn FnMut(&str, Value) -> Result<Value, String>,
) -> Option<Value> {
    let text = js_sys::JSON::stringify(message).ok()?.as_string()?;
    let envelope: Value = serde_json::from_str(&text).ok()?;
    let request = &envelope["message"];
    let deadline = envelope["deadlineAtMs"].as_f64().unwrap_or(0.0);
    let (tx, rx) = mpsc::channel();
    let server = LiveServer::new(tx);
    let tools: Vec<_> = server
        .tool_router
        .list_all()
        .into_iter()
        .map(annotated)
        .collect();
    let result = match request["method"].as_str() {
        Some("tools/list") => json!({"tools":tools}),
        Some("tools/call") => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            let args = request["params"]
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let tool = tools.iter().find(|t| t.name == name);
            let denied = tool.is_none()
                || (read_only
                    && !tool.is_some_and(|t| {
                        t.annotations.as_ref().and_then(|a| a.read_only_hint) == Some(true)
                    }));
            let response = if denied {
                Ok(fail(
                    "This tool is unavailable for this document or access mode.",
                ))
            } else if file_arguments(&args) {
                Ok(fail(HOST_FILES))
            } else {
                let mut guarded = |method: &str, params: Value| {
                    if js_sys::Date::now() >= deadline {
                        return Err("The editor request expired.".into());
                    }
                    if file_arguments(&params) {
                        return Err(HOST_FILES.into());
                    }
                    match method {
                        "engine.execute" | "ui.menu.invoke" => {
                            let command = params
                                .get("command")
                                .or_else(|| params.get("id"))
                                .and_then(Value::as_str)
                                .ok_or("Missing command.")?;
                            authorize_command(command, &params, read_only)?;
                        }
                        "engine.commands" | "ui.inspect" | "ui.elements" | "ui.menu.list"
                        | "jobs.list" => {}
                        "ui.pointer" | "ui.set" | "jobs.cancel" if !read_only => {}
                        _ => {
                            return Err(
                                "This native editor operation is unavailable in the browser."
                                    .into(),
                            );
                        }
                    }
                    call(method, params)
                };
                match drive(server.call(name, args), &rx, &mut guarded) {
                    Ok(result) => result,
                    Err(error) => Ok(fail(error)),
                }
            };
            match response {
                Ok(result) => serde_json::to_value(result).unwrap_or_else(|error| json!({"content":[{"type":"text","text":error.to_string()}],"isError":true})),
                Err(error) => return Some(json!({"requestId":envelope["requestId"],"response":{"jsonrpc":"2.0","id":request["id"],"error":error}})),
            }
        }
        _ => return None,
    };
    Some(
        json!({"requestId":envelope["requestId"],"response":{"jsonrpc":"2.0","id":request["id"],"result":result}}),
    )
}
