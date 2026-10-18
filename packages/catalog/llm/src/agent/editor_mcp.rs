use flow_like_types::JsonSchema;
#[cfg(any(feature = "execute", test))]
use flow_like_types::{Value, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// One mounted document. Reopening the element creates a different session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditorMcpConfig {
    pub element_ref: String,
    pub session_id: String,
    pub editor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_filter: Option<HashSet<String>>,
}

#[cfg(any(feature = "execute", test))]
pub(crate) fn validate_element_ref(value: &str) -> flow_like_types::Result<()> {
    let Some((surface, component)) = value.split_once('/') else {
        return Err(anyhow!(
            "Editor Element must be a full surface/component reference"
        ));
    };
    if surface.is_empty()
        || component.is_empty()
        || value.len() > 1024
        || value.chars().any(char::is_control)
    {
        return Err(anyhow!(
            "Editor Element must be a full surface/component reference"
        ));
    }
    Ok(())
}

#[cfg(any(feature = "execute", test))]
pub(crate) fn element_reference(value: &Value) -> flow_like_types::Result<String> {
    let reference = match value {
        Value::String(reference) => Some(reference.as_str()),
        Value::Object(element) => element
            .get("__element_id")
            .or_else(|| element.get("id"))
            .and_then(Value::as_str),
        _ => None,
    }
    .ok_or_else(|| {
        anyhow!("Editor Element requires an element from Get Element or a full surface/component reference")
    })?;
    validate_element_ref(reference)?;
    Ok(reference.to_owned())
}

#[cfg(any(feature = "execute", test))]
pub(crate) fn device_value(reply: Value) -> flow_like_types::Result<Value> {
    if reply.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(anyhow!(
            "{}",
            reply
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("The open editor could not answer the request")
        ));
    }
    Ok(reply.get("value").cloned().unwrap_or(Value::Null))
}

impl EditorMcpConfig {
    #[cfg(any(feature = "execute", test))]
    pub(crate) fn from_session(
        element_ref: String,
        session: Value,
        tool_filter: Option<HashSet<String>>,
    ) -> flow_like_types::Result<Self> {
        validate_element_ref(&element_ref)?;
        let field = |name: &str| -> flow_like_types::Result<String> {
            session
                .get(name)
                .and_then(Value::as_str)
                .filter(|value| {
                    !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
                })
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("The open editor returned an invalid {name}"))
        };
        Ok(Self {
            element_ref,
            session_id: field("sessionId")?,
            editor: field("editor")?,
            tool_filter,
        })
    }

    #[cfg(feature = "execute")]
    pub(crate) fn validate(&self) -> flow_like_types::Result<()> {
        Self::from_session(
            self.element_ref.clone(),
            flow_like_types::json::json!({"sessionId": self.session_id, "editor": self.editor}),
            self.tool_filter.clone(),
        )
        .map(|_| ())
    }
}

#[cfg(feature = "execute")]
mod transport;

#[cfg(feature = "execute")]
impl transport::EditorMcpTransport {
    pub(crate) fn new(
        context: &flow_like::flow::execution::context::ExecutionContext,
        config: &EditorMcpConfig,
    ) -> flow_like_types::Result<Self> {
        use flow_like_types::json;
        use std::sync::Arc;
        use transport::{CANCEL_TIMEOUT, REQUEST_TIMEOUT, Requester};

        config.validate()?;
        let cancellation = context
            .cancellation_token()
            .unwrap_or_default()
            .child_token();
        let requester = |context: flow_like::flow::execution::context::ExecutionContext,
                         timeout|
         -> Requester {
            let config = config.clone();
            Arc::new(move |message| {
                let mut context = context.clone();
                let config = config.clone();
                Box::pin(async move {
                    device_value(
                        context
                            .request_device(
                                "craft.mcp",
                                json::json!({
                                    "elementRef": config.element_ref,
                                    "sessionId": config.session_id,
                                    "editor": config.editor,
                                    "message": message,
                                }),
                                timeout,
                            )
                            .await?,
                    )
                })
            })
        };
        // Closing this peer must still be able to cancel its request in the invoking page.
        let cancel_requester = requester(context.clone(), CANCEL_TIMEOUT);
        let mut child_context = context.clone();
        child_context.set_cancellation_token(cancellation.clone());
        Ok(Self::with_requester(
            requester(child_context, REQUEST_TIMEOUT),
            cancel_requester,
            cancellation,
        ))
    }
}

#[cfg(feature = "execute")]
pub(crate) use transport::EditorMcpTransport;

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::json::{self, json};

    #[test]
    fn get_element_uses_the_resolved_reference_instead_of_the_component_id() {
        let element = json!({
            "id": "editor",
            "__element_id": "active-page/editor",
            "component": {"type": "filePreview", "fileType": "document"}
        });
        assert_eq!(element_reference(&element).unwrap(), "active-page/editor");
        for reference in [
            json!("storage:app/folder/drawing.solidcraft"),
            json!({"id": "storage:app/folder/drawing.solidcraft"}),
        ] {
            assert_eq!(
                element_reference(&reference).unwrap(),
                "storage:app/folder/drawing.solidcraft"
            );
        }
    }

    #[test]
    fn missing_or_malformed_elements_cannot_select_another_document() {
        for element in [
            Value::Null,
            json!({"id": "editor"}),
            json!({"component": {"type": "filePreview"}}),
            json!({"__element_id": null, "id": "page/other-editor"}),
            json!({"__element_id": "", "id": "page/other-editor"}),
            json!(["page/editor"]),
        ] {
            assert!(element_reference(&element).is_err(), "{element}");
        }
    }

    #[test]
    fn registration_requires_a_qualified_element_and_a_live_session() {
        for reference in ["editor", "/editor", "page/", "page/editor\n"] {
            assert!(validate_element_ref(reference).is_err());
        }
        let config = EditorMcpConfig::from_session(
            "page/editor".into(),
            json!({"sessionId":"live-1","editor":"solidcraft"}),
            None,
        )
        .unwrap();
        assert_eq!(config.session_id, "live-1");
        assert!(
            EditorMcpConfig::from_session(
                "page/editor".into(),
                json!({"editor":"solidcraft"}),
                None
            )
            .is_err()
        );
        let serialized = json::to_value(config).unwrap();
        assert!(serialized.get("uri").is_none());
        assert!(serialized.get("tool_filter").is_none());
    }

    #[test]
    fn missing_or_refused_frontend_acknowledgements_fail() {
        assert!(device_value(json!({"value":{}})).is_err());
        let error = device_value(
            json!({"ok":false,"error":{"code":"stale_session","message":"Document was reopened"}}),
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "Document was reopened");
        assert_eq!(
            device_value(json!({"ok":true,"value":null})).unwrap(),
            Value::Null
        );
    }
}
