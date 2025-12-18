use super::provider::{MICROSOFT_PROVIDER_ID, MicrosoftGraphProvider};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{JsonSchema, Value, async_trait, json::json, reqwest};
use serde::{Deserialize, Serialize};

// =============================================================================
// OneDrive Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OneDriveItem {
    pub id: String,
    pub name: String,
    pub size: Option<i64>,
    pub created_date_time: Option<String>,
    pub last_modified_date_time: Option<String>,
    pub web_url: Option<String>,
    pub is_folder: bool,
    pub parent_reference: Option<OneDriveParentReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OneDriveParentReference {
    pub id: Option<String>,
    pub path: Option<String>,
    pub drive_id: Option<String>,
}

fn parse_item(value: &Value) -> Option<OneDriveItem> {
    Some(OneDriveItem {
        id: value["id"].as_str()?.to_string(),
        name: value["name"].as_str()?.to_string(),
        size: value["size"].as_i64(),
        created_date_time: value["createdDateTime"].as_str().map(String::from),
        last_modified_date_time: value["lastModifiedDateTime"].as_str().map(String::from),
        web_url: value["webUrl"].as_str().map(String::from),
        is_folder: value.get("folder").is_some(),
        parent_reference: value
            .get("parentReference")
            .map(|p| OneDriveParentReference {
                id: p["id"].as_str().map(String::from),
                path: p["path"].as_str().map(String::from),
                drive_id: p["driveId"].as_str().map(String::from),
            }),
    })
}

// =============================================================================
// List OneDrive Items Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct ListOneDriveItemsNode {}

impl ListOneDriveItemsNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for ListOneDriveItemsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_list_items",
            "List OneDrive Items",
            "List files and folders in OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "folder_path",
            "Folder Path",
            "Path to folder (empty for root)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "items",
            "Items",
            "List of OneDrive items",
            VariableType::Struct,
        )
        .set_value_type(ValueType::Array)
        .set_schema::<OneDriveItem>();
        node.add_output_pin("count", "Count", "", VariableType::Integer);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.Read.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let folder_path: String = context
            .evaluate_pin("folder_path")
            .await
            .unwrap_or_default();

        let url = if folder_path.is_empty() {
            "https://graph.microsoft.com/v1.0/me/drive/root/children".to_string()
        } else {
            format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}:/children",
                folder_path
            )
        };

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let items: Vec<OneDriveItem> = body["value"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(parse_item).collect())
                    .unwrap_or_default();
                let count = items.len() as i64;
                context.set_pin_value("items", json!(items)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Get OneDrive Item Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GetOneDriveItemNode {}

impl GetOneDriveItemNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GetOneDriveItemNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_get_item",
            "Get OneDrive Item",
            "Get metadata for a OneDrive item",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "item_path",
            "Item Path",
            "Path to the item",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "item",
            "Item",
            "OneDrive item metadata",
            VariableType::Struct,
        )
        .set_schema::<OneDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.Read.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let item_path: String = context.evaluate_pin("item_path").await?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}",
                item_path
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(item) = parse_item(&body) {
                    context.set_pin_value("item", json!(item)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context
                        .set_pin_value("error_message", json!("Failed to parse response"))
                        .await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Download OneDrive File Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct DownloadOneDriveFileNode {}

impl DownloadOneDriveFileNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for DownloadOneDriveFileNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_download",
            "Download File",
            "Download a file from OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "item_path",
            "Item Path",
            "Path to the file",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "content",
            "Content",
            "File content (base64 for binary)",
            VariableType::String,
        );
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.Read.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let item_path: String = context.evaluate_pin("item_path").await?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}:/content",
                item_path
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().await?;
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                context.set_pin_value("content", json!(encoded)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Upload OneDrive File Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct UploadOneDriveFileNode {}

impl UploadOneDriveFileNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for UploadOneDriveFileNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_upload",
            "Upload File",
            "Upload a file to OneDrive (max 4MB)",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "file_path",
            "File Path",
            "Destination path including filename",
            VariableType::String,
        );
        node.add_input_pin(
            "content",
            "Content",
            "File content (base64 encoded)",
            VariableType::String,
        );
        node.add_input_pin(
            "conflict_behavior",
            "Conflict Behavior",
            "What to do on conflict",
            VariableType::String,
        )
        .set_default_value(Some(json!("rename")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "rename".to_string(),
                    "replace".to_string(),
                    "fail".to_string(),
                ])
                .build(),
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "item",
            "Item",
            "Uploaded item metadata",
            VariableType::Struct,
        )
        .set_schema::<OneDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.ReadWrite.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let file_path: String = context.evaluate_pin("file_path").await?;
        let content: String = context.evaluate_pin("content").await?;
        let conflict_behavior: String = context
            .evaluate_pin("conflict_behavior")
            .await
            .unwrap_or_else(|_| "rename".to_string());

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&content)
            .map_err(|e| flow_like_types::anyhow!("Invalid base64: {}", e))?;

        let client = reqwest::Client::new();
        let response = client
            .put(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}:/content",
                file_path
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/octet-stream")
            .query(&[(
                "@microsoft.graph.conflictBehavior",
                conflict_behavior.as_str(),
            )])
            .body(bytes)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(item) = parse_item(&body) {
                    context.set_pin_value("item", json!(item)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context
                        .set_pin_value("error_message", json!("Failed to parse response"))
                        .await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Create OneDrive Folder Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CreateOneDriveFolderNode {}

impl CreateOneDriveFolderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CreateOneDriveFolderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_create_folder",
            "Create Folder",
            "Create a new folder in OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "parent_path",
            "Parent Path",
            "Path to parent folder (empty for root)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));
        node.add_input_pin(
            "folder_name",
            "Folder Name",
            "Name of the new folder",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin(
            "item",
            "Item",
            "Created folder metadata",
            VariableType::Struct,
        )
        .set_schema::<OneDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.ReadWrite.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let parent_path: String = context
            .evaluate_pin("parent_path")
            .await
            .unwrap_or_default();
        let folder_name: String = context.evaluate_pin("folder_name").await?;

        let body = json!({
            "name": folder_name,
            "folder": {},
            "@microsoft.graph.conflictBehavior": "rename"
        });

        let url = if parent_path.is_empty() {
            "https://graph.microsoft.com/v1.0/me/drive/root/children".to_string()
        } else {
            format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}:/children",
                parent_path
            )
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(item) = parse_item(&body) {
                    context.set_pin_value("item", json!(item)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context
                        .set_pin_value("error_message", json!("Failed to parse response"))
                        .await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Delete OneDrive Item Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct DeleteOneDriveItemNode {}

impl DeleteOneDriveItemNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for DeleteOneDriveItemNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_delete",
            "Delete Item",
            "Delete a file or folder from OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "item_path",
            "Item Path",
            "Path to the item to delete",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.ReadWrite.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let item_path: String = context.evaluate_pin("item_path").await?;

        let client = reqwest::Client::new();
        let response = client
            .delete(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}",
                item_path
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 204 => {
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Move OneDrive Item Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct MoveOneDriveItemNode {}

impl MoveOneDriveItemNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for MoveOneDriveItemNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_move",
            "Move Item",
            "Move a file or folder to a new location in OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "item_path",
            "Item Path",
            "Path to the item to move",
            VariableType::String,
        );
        node.add_input_pin(
            "destination_path",
            "Destination Path",
            "Path to destination folder",
            VariableType::String,
        );
        node.add_input_pin(
            "new_name",
            "New Name",
            "Optional new name for the item",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("item", "Item", "Moved item metadata", VariableType::Struct)
            .set_schema::<OneDriveItem>();
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.ReadWrite.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let item_path: String = context.evaluate_pin("item_path").await?;
        let destination_path: String = context.evaluate_pin("destination_path").await?;
        let new_name: String = context.evaluate_pin("new_name").await.unwrap_or_default();

        // First get the destination folder ID
        let client = reqwest::Client::new();
        let dest_url = if destination_path.is_empty() {
            "https://graph.microsoft.com/v1.0/me/drive/root".to_string()
        } else {
            format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}",
                destination_path
            )
        };

        let dest_response = client
            .get(&dest_url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        let dest_id = match dest_response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                body["id"].as_str().unwrap_or("").to_string()
            }
            _ => {
                context
                    .set_pin_value("error_message", json!("Could not find destination folder"))
                    .await?;
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let mut body = json!({
            "parentReference": {
                "id": dest_id
            }
        });

        if !new_name.is_empty() {
            body["name"] = json!(new_name);
        }

        let response = client
            .patch(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}",
                item_path
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                if let Some(item) = parse_item(&body) {
                    context.set_pin_value("item", json!(item)).await?;
                    context.activate_exec_pin("exec_out").await?;
                } else {
                    context
                        .set_pin_value("error_message", json!("Failed to parse response"))
                        .await?;
                    context.activate_exec_pin("error").await?;
                }
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Copy OneDrive Item Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct CopyOneDriveItemNode {}

impl CopyOneDriveItemNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for CopyOneDriveItemNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_copy",
            "Copy Item",
            "Copy a file or folder in OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin(
            "item_path",
            "Item Path",
            "Path to the item to copy",
            VariableType::String,
        );
        node.add_input_pin(
            "destination_path",
            "Destination Path",
            "Path to destination folder",
            VariableType::String,
        );
        node.add_input_pin(
            "new_name",
            "New Name",
            "Optional name for the copy",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Success",
            "Copy operation started",
            VariableType::Execution,
        );
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.ReadWrite.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let item_path: String = context.evaluate_pin("item_path").await?;
        let destination_path: String = context.evaluate_pin("destination_path").await?;
        let new_name: String = context.evaluate_pin("new_name").await.unwrap_or_default();

        // Get destination folder ID
        let client = reqwest::Client::new();
        let dest_url = if destination_path.is_empty() {
            "https://graph.microsoft.com/v1.0/me/drive/root".to_string()
        } else {
            format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}",
                destination_path
            )
        };

        let dest_response = client
            .get(&dest_url)
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        let (dest_id, drive_id) = match dest_response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                (
                    body["id"].as_str().unwrap_or("").to_string(),
                    body["parentReference"]["driveId"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                )
            }
            _ => {
                context
                    .set_pin_value("error_message", json!("Could not find destination folder"))
                    .await?;
                context.activate_exec_pin("error").await?;
                return Ok(());
            }
        };

        let mut body = json!({
            "parentReference": {
                "driveId": drive_id,
                "id": dest_id
            }
        });

        if !new_name.is_empty() {
            body["name"] = json!(new_name);
        }

        let response = client
            .post(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root:{}:/copy",
                item_path
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 202 => {
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// Search OneDrive Node
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct SearchOneDriveNode {}

impl SearchOneDriveNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for SearchOneDriveNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_onedrive_search",
            "Search OneDrive",
            "Search for files and folders in OneDrive",
            "Data/Microsoft/OneDrive",
        );
        node.add_icon("/flow/icons/onedrive.svg");

        node.add_input_pin("exec_in", "Input", "Trigger", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_input_pin("query", "Query", "Search query", VariableType::String);

        node.add_output_pin("exec_out", "Success", "", VariableType::Execution);
        node.add_output_pin("error", "Error", "", VariableType::Execution);
        node.add_output_pin("items", "Items", "Search results", VariableType::Struct)
            .set_value_type(ValueType::Array)
            .set_schema::<OneDriveItem>();
        node.add_output_pin("count", "Count", "", VariableType::Integer);
        node.add_output_pin("error_message", "Error Message", "", VariableType::String);

        node.add_required_oauth_scopes(MICROSOFT_PROVIDER_ID, vec!["Files.Read.All"]);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("error").await?;

        let provider: MicrosoftGraphProvider = context.evaluate_pin("provider").await?;
        let query: String = context.evaluate_pin("query").await?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "https://graph.microsoft.com/v1.0/me/drive/root/search(q='{}')",
                urlencoding::encode(&query)
            ))
            .header("Authorization", format!("Bearer {}", provider.access_token))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await?;
                let items: Vec<OneDriveItem> = body["value"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(parse_item).collect())
                    .unwrap_or_default();
                let count = items.len() as i64;
                context.set_pin_value("items", json!(items)).await?;
                context.set_pin_value("count", json!(count)).await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Ok(resp) => {
                let error = resp.text().await.unwrap_or_default();
                context.set_pin_value("error_message", json!(error)).await?;
                context.activate_exec_pin("error").await?;
            }
            Err(e) => {
                context
                    .set_pin_value("error_message", json!(e.to_string()))
                    .await?;
                context.activate_exec_pin("error").await?;
            }
        }

        Ok(())
    }
}
