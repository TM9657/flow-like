use super::provider::{GoogleProvider, GOOGLE_PROVIDER_ID};
use flow_like::{
    flow::{
        execution::{context::ExecutionContext, LogLevel},
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json, reqwest, JsonSchema, Value};
use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GoogleDriveItem {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: Option<i64>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub web_view_link: Option<String>,
    pub parents: Vec<String>,
    pub is_folder: bool,
}

fn parse_drive_item(item: &Value) -> Option<GoogleDriveItem> {
    let mime_type = item["mimeType"].as_str()?.to_string();
    let is_folder = mime_type == "application/vnd.google-apps.folder";
    Some(GoogleDriveItem {
        id: item["id"].as_str()?.to_string(),
        name: item["name"].as_str()?.to_string(),
        mime_type,
        size: item["size"].as_str().and_then(|s| s.parse().ok()),
        created_time: item["createdTime"].as_str().map(String::from),
        modified_time: item["modifiedTime"].as_str().map(String::from),
        web_view_link: item["webViewLink"].as_str().map(String::from),
        parents: item["parents"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|p| p.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        is_folder,
    })
}

// =============================================================================
// Create Folder Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CreateGoogleDriveFolderNode {}

impl CreateGoogleDriveFolderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CreateGoogleDriveFolderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_create_folder",
            "Create Folder",
            "Create a new folder in Google Drive",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/folder-plus.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("name", "Name", "Folder name", VariableType::String);
        node.add_input_pin("parent_id", "Parent ID", "Parent folder ID (empty for root)", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("folder_id", "Folder ID", "Created folder ID", VariableType::String);
        node.add_output_pin("folder", "Folder", "Created folder details", VariableType::Struct)
            .set_schema::<GoogleDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let name: String = context.evaluate_pin("name").await?;
        let parent_id: String = context.evaluate_pin("parent_id").await.unwrap_or_default();

        let mut metadata = json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder"
        });

        if !parent_id.is_empty() {
            metadata["parents"] = json!([parent_id]);
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://www.googleapis.com/drive/v3/files")
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .query(&[("fields", "id,name,mimeType,createdTime,modifiedTime,webViewLink,parents")])
            .json(&metadata)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(folder) = parse_drive_item(&body) {
                    let folder_id = folder.id.clone();
                    context.set_pin_value("folder_id", json!(folder_id)).await?;
                    context.set_pin_value("folder", json!(folder)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.set_pin_value("error_message", json!("Failed to parse response")).await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Delete File Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct DeleteGoogleDriveFileNode {}

impl DeleteGoogleDriveFileNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for DeleteGoogleDriveFileNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_delete_file",
            "Delete File",
            "Delete a file or folder from Google Drive",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/trash.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("file_id", "File ID", "ID of file/folder to delete", VariableType::String);

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let file_id: String = context.evaluate_pin("file_id").await?;

        let client = reqwest::Client::new();
        let response = client
            .delete(&format!("https://www.googleapis.com/drive/v3/files/{}", file_id))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() || resp.status() == 204 => {
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Copy File Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopyGoogleDriveFileNode {}

impl CopyGoogleDriveFileNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CopyGoogleDriveFileNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_copy_file",
            "Copy File",
            "Copy a file in Google Drive",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/copy.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("file_id", "File ID", "ID of file to copy", VariableType::String);
        node.add_input_pin("new_name", "New Name", "Name for the copy (empty to keep original)", VariableType::String)
            .set_default_value(Some(json!("")));
        node.add_input_pin("parent_id", "Parent ID", "Destination folder ID (empty for same location)", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("new_file_id", "New File ID", "ID of the copied file", VariableType::String);
        node.add_output_pin("file", "File", "Copied file details", VariableType::Struct)
            .set_schema::<GoogleDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let file_id: String = context.evaluate_pin("file_id").await?;
        let new_name: String = context.evaluate_pin("new_name").await.unwrap_or_default();
        let parent_id: String = context.evaluate_pin("parent_id").await.unwrap_or_default();

        let mut metadata = json!({});
        if !new_name.is_empty() {
            metadata["name"] = json!(new_name);
        }
        if !parent_id.is_empty() {
            metadata["parents"] = json!([parent_id]);
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&format!("https://www.googleapis.com/drive/v3/files/{}/copy", file_id))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .query(&[("fields", "id,name,mimeType,size,createdTime,modifiedTime,webViewLink,parents")])
            .json(&metadata)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(file) = parse_drive_item(&body) {
                    let new_id = file.id.clone();
                    context.set_pin_value("new_file_id", json!(new_id)).await?;
                    context.set_pin_value("file", json!(file)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.set_pin_value("error_message", json!("Failed to parse response")).await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Move File Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct MoveGoogleDriveFileNode {}

impl MoveGoogleDriveFileNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for MoveGoogleDriveFileNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_move_file",
            "Move File",
            "Move a file to a different folder in Google Drive",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/move.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("file_id", "File ID", "ID of file to move", VariableType::String);
        node.add_input_pin("new_parent_id", "New Parent ID", "Destination folder ID", VariableType::String);

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("file", "File", "Updated file details", VariableType::Struct)
            .set_schema::<GoogleDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let file_id: String = context.evaluate_pin("file_id").await?;
        let new_parent_id: String = context.evaluate_pin("new_parent_id").await?;

        // First get current parents
        let client = reqwest::Client::new();
        let meta_resp = client
            .get(&format!("https://www.googleapis.com/drive/v3/files/{}", file_id))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .query(&[("fields", "parents")])
            .send()
            .await;

        let current_parents = match meta_resp {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                body["parents"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|p| p.as_str()).collect::<Vec<_>>().join(","))
                    .unwrap_or_default()
            }
            _ => String::new(),
        };

        let response = client
            .patch(&format!("https://www.googleapis.com/drive/v3/files/{}", file_id))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .query(&[
                ("addParents", new_parent_id.as_str()),
                ("removeParents", current_parents.as_str()),
                ("fields", "id,name,mimeType,size,createdTime,modifiedTime,webViewLink,parents"),
            ])
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(file) = parse_drive_item(&body) {
                    context.set_pin_value("file", json!(file)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.set_pin_value("error_message", json!("Failed to parse response")).await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Search Files Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct SearchGoogleDriveNode {}

impl SearchGoogleDriveNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for SearchGoogleDriveNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_search",
            "Search Drive",
            "Search for files in Google Drive",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/search.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("query", "Query", "Search query (supports Drive query syntax)", VariableType::String);
        node.add_input_pin("page_size", "Page Size", "Max results (1-1000)", VariableType::Integer)
            .set_default_value(Some(json!(100)));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("files", "Files", "Search results", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<Vec<GoogleDriveItem>>();
        node.add_output_pin("count", "Count", "Number of results", VariableType::Integer);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive.readonly"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let query: String = context.evaluate_pin("query").await?;
        let page_size: i64 = context.evaluate_pin("page_size").await.unwrap_or(100);

        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/drive/v3/files")
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .query(&[
                ("q", query.as_str()),
                ("pageSize", &page_size.to_string()),
                ("fields", "files(id,name,mimeType,size,createdTime,modifiedTime,webViewLink,parents)"),
            ])
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let files: Vec<GoogleDriveItem> = body["files"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(parse_drive_item).collect())
                    .unwrap_or_default();
                let count = files.len() as i64;
                context.set_pin_value("files", json!(files)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Get File Metadata Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GetGoogleDriveFileMetadataNode {}

impl GetGoogleDriveFileMetadataNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetGoogleDriveFileMetadataNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_get_metadata",
            "Get File Metadata",
            "Get detailed metadata for a Google Drive file",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/info.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("file_id", "File ID", "File ID", VariableType::String);

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("file", "File", "File metadata", VariableType::Struct)
            .set_schema::<GoogleDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive.readonly"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let file_id: String = context.evaluate_pin("file_id").await?;

        let client = reqwest::Client::new();
        let response = client
            .get(&format!("https://www.googleapis.com/drive/v3/files/{}", file_id))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .query(&[("fields", "id,name,mimeType,size,createdTime,modifiedTime,webViewLink,parents")])
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(file) = parse_drive_item(&body) {
                    context.set_pin_value("file", json!(file)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context.set_pin_value("error_message", json!("Failed to parse response")).await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Download File Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct DownloadGoogleDriveFileNode {}

impl DownloadGoogleDriveFileNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for DownloadGoogleDriveFileNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_drive_download",
            "Download File",
            "Download file content from Google Drive",
            "Data/Google Drive",
        );
        node.add_icon("/flow/icons/download.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin("provider", "Provider", "Google Drive provider", VariableType::Struct)
            .set_schema::<GoogleProvider>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("file_id", "File ID", "File ID to download", VariableType::String);
        node.add_input_pin("export_mime_type", "Export MIME Type", "For Google Docs, export format (e.g., 'application/pdf')", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("content", "Content", "File content as bytes", VariableType::Byte);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(GOOGLE_PROVIDER_ID, vec!["https://www.googleapis.com/auth/drive.readonly"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: GoogleProvider = context.evaluate_pin("provider").await?;
        let file_id: String = context.evaluate_pin("file_id").await?;
        let export_mime_type: String = context.evaluate_pin("export_mime_type").await.unwrap_or_default();

        let client = reqwest::Client::new();

        // First get mime type to determine if export is needed
        let meta_resp = client
            .get(&format!("https://www.googleapis.com/drive/v3/files/{}", file_id))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .query(&[("fields", "mimeType")])
            .send()
            .await;

        let mime_type = match meta_resp {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                body["mimeType"].as_str().unwrap_or("").to_string()
            }
            _ => String::new(),
        };

        let is_google_doc = mime_type.starts_with("application/vnd.google-apps.");

        let url = if is_google_doc {
            let export_type = if export_mime_type.is_empty() {
                match mime_type.as_str() {
                    "application/vnd.google-apps.document" => "application/pdf",
                    "application/vnd.google-apps.spreadsheet" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    "application/vnd.google-apps.presentation" => "application/pdf",
                    _ => "application/pdf",
                }
            } else {
                &export_mime_type
            };
            format!(
                "https://www.googleapis.com/drive/v3/files/{}/export?mimeType={}",
                file_id,
                urlencoding::encode(export_type)
            )
        } else {
            format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id)
        };

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().await?;
                context.set_pin_value("content", json!(bytes.to_vec())).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context.set_pin_value("error_message", json!(e.to_string())).await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}
