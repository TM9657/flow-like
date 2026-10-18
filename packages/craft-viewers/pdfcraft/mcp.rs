// Upstream owns the catalog, argument validation, dispatcher and result encoding.
impl McpServer {
    pub fn handle_live(
        session: &mut pdfcraft_engine::Session,
        message: &Value,
        read_only: bool,
    ) -> Value {
        let mut server = Self::new(Automation::new());
        server.automation.live_read_only = Some(read_only);
        std::mem::swap(session, &mut server.automation.session);
        let mut response = server.handle(message).unwrap_or(Value::Null);
        std::mem::swap(session, &mut server.automation.session);
        if let Some(tools) = response
            .pointer_mut("/result/tools")
            .and_then(Value::as_array_mut)
        {
            tools.retain(|tool| tool["name"].as_str().is_some_and(crate::live_tool_allowed));
        }
        response
    }
}
