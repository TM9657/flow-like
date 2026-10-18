use serde_json::{Value, json};
use wasm_bindgen::JsValue;

const HOST_FILES: &str =
    "Use the document controls above the editor to open, save, or export files.";

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

fn tool_error(message: impl ToString) -> Value {
    json!({"content": [{"type": "text", "text": message.to_string()}], "isError": true})
}

#[path = "mcp_policy.rs"]
pub mod policy;
use policy::{
    allowed_command, command_file_arguments, command_read_only, file_arguments, file_paths,
    tool_allowed, tool_read_only,
};

struct LiveBackend<'a> {
    call: &'a mut dyn FnMut(&str, Value) -> Result<Value, String>,
    read_only: bool,
    deadline: f64,
    commands: Option<Value>,
}

impl LiveBackend<'_> {
    fn canonical_command(&mut self, command: &str) -> Result<String, String> {
        if !allowed_command(command) {
            return Err(HOST_FILES.into());
        }
        if self.commands.is_none() {
            self.commands = Some((self.call)("engine.commands", json!({}))?);
        }
        let canonical = self
            .commands
            .as_ref()
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .filter_map(|item| item["id"].as_str())
                    .find(|id| id.eq_ignore_ascii_case(command))
            })
            .ok_or("Use a command id from list_commands.")?
            .to_owned();
        if !allowed_command(&canonical) {
            return Err(HOST_FILES.into());
        }
        if self.read_only && !command_read_only(&canonical) {
            return Err("This document is read-only.".into());
        }
        Ok(canonical)
    }
}

impl upstream::Backend for LiveBackend<'_> {
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        "The open Flow-Like editor document".into()
    }

    fn call(&mut self, method: &str, mut params: Value) -> Result<Value, String> {
        if js_sys::Date::now() >= self.deadline {
            return Err("The editor request expired before it could run.".into());
        }
        let external_files = if method == "engine.batch" {
            file_paths(&params)
        } else if matches!(method, "engine.execute" | "ui.menu.invoke") {
            let command = params
                .get("command")
                .or_else(|| params.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            command_file_arguments(EDITOR, command, &params)
        } else {
            file_arguments(&params)
        };
        if external_files {
            return Err(HOST_FILES.into());
        }
        match method {
            "engine.commands" => {
                let mut commands = (self.call)(method, params)?;
                if let Some(items) = commands.as_array_mut() {
                    items.retain(|item| item["id"].as_str().is_some_and(allowed_command));
                }
                Ok(commands)
            }
            "engine.execute" | "ui.menu.invoke" => {
                let command = params
                    .get("command")
                    .or_else(|| params.get("id"))
                    .and_then(Value::as_str)
                    .ok_or("Missing command.")?;
                let canonical = self.canonical_command(command)?;
                params["command"] = json!(canonical);
                (self.call)(method, params)
            }
            "engine.batch" => {
                let commands = params["commands"]
                    .as_array_mut()
                    .ok_or("Missing batch commands.")?;
                // Validate every step before upstream starts the undo transaction.
                for step in commands {
                    let command = step
                        .get("command")
                        .or_else(|| step.get("id"))
                        .and_then(Value::as_str)
                        .ok_or("Missing batch command.")?;
                    let canonical = self.canonical_command(command)?;
                    if command_file_arguments(EDITOR, &canonical, step) {
                        return Err(HOST_FILES.into());
                    }
                    step["command"] = json!(canonical);
                }
                (self.call)(method, params)
            }
            "document.inspect" | "drawing.inspect" | "session.inspect" | "sheet.read"
            | "engine.parity" | "ui.parity" | "ui.inspect" | "ui.render" | "ui.menu.list" => {
                (self.call)(method, params)
            }
            "ui.click" | "ui.pointer" | "ui.key" | "ui.text" | "ui.tool.select" | "ui.set"
            | "ui.view" | "ui.drag" | "cmdline.key"
                if !self.read_only =>
            {
                (self.call)(method, params)
            }
            _ => Err("This editor operation is unavailable in the embedded document.".into()),
        }
    }
}

pub fn receive(
    message: &JsValue,
    read_only: bool,
    call: &mut dyn FnMut(&str, Value) -> Result<Value, String>,
) -> Option<Value> {
    let encoded = js_sys::JSON::stringify(message).ok()?.as_string()?;
    let envelope: Value = serde_json::from_str(&encoded).ok()?;
    let request = &envelope["message"];
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let deadline = envelope["deadlineAtMs"].as_f64().unwrap_or(0.0);
    let error = |code, message: &str| json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}});
    let response = if js_sys::Date::now() >= deadline {
        error(-32000, "The editor request expired before it could run.")
    } else {
        let tools: Vec<Value> = definitions()
            .as_array()
            .into_iter()
            .flatten()
            .filter(|tool| tool_allowed(tool))
            .cloned()
            .map(|mut tool| {
                if tool["annotations"]["readOnlyHint"].is_null() {
                    let read_only = tool_read_only(&tool);
                    if !tool["annotations"].is_object() {
                        tool["annotations"] = json!({});
                    }
                    tool["annotations"]["readOnlyHint"] = json!(read_only);
                }
                tool
            })
            .collect();
        match request["method"].as_str() {
            Some("tools/list") => json!({"jsonrpc": "2.0", "id": id, "result": {"tools": tools}}),
            Some("tools/call") => {
                let name = request["params"]["name"].as_str().unwrap_or("");
                let args = request["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let result = if !tools
                    .iter()
                    .any(|tool| tool["name"] == name && (!read_only || tool_read_only(tool)))
                {
                    tool_error("This tool is unavailable for this document or access mode.")
                } else if file_paths(&args) {
                    tool_error(HOST_FILES)
                } else {
                    call_upstream(
                        LiveBackend {
                            call,
                            read_only,
                            deadline,
                            commands: None,
                        },
                        name,
                        &args,
                    )
                };
                json!({"jsonrpc": "2.0", "id": id, "result": result})
            }
            _ => error(-32601, "Unsupported editor method."),
        }
    };
    Some(json!({"requestId": envelope["requestId"], "response": response}))
}
