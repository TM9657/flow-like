//! A2UI Copilot tools for AI-powered UI generation

use crate::a2ui::SurfaceComponent;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ============================================================================
// Tool Error Types
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Get component schema tool error")]
pub struct GetComponentSchemaToolError;

#[derive(Debug, thiserror::Error)]
#[error("Get style examples tool error")]
pub struct GetStyleExamplesToolError;

#[derive(Debug, thiserror::Error)]
#[error("Modify component tool error: {0}")]
pub struct ModifyComponentToolError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Emit surface tool error: {0}")]
pub struct EmitSurfaceToolError(pub String);

// ============================================================================
// Tool Arguments
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct GetComponentSchemaArgs {
    /// The component type to get schema for (e.g., "button", "card", "column")
    pub component_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetStyleExamplesArgs {
    /// Style category: "spacing", "colors", "effects", "layout", "responsive"
    pub category: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModifyComponentArgs {
    /// ID of the component to modify
    pub component_id: String,
    /// Type of modification: "add_child", "update_style", "update_props", "wrap", "delete"
    pub modification_type: String,
    /// New values to apply (depends on modification_type)
    #[serde(default)]
    pub values: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EmitSurfaceArgs {
    /// The complete list of surface components to emit
    pub components: Vec<SurfaceComponent>,
    /// Optional root component ID (first component if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_id: Option<String>,
}

// ============================================================================
// Tools
// ============================================================================

/// Tool to get component schema documentation
#[derive(Debug)]
pub struct GetComponentSchemaTool;

impl Tool for GetComponentSchemaTool {
    const NAME: &'static str = "get_component_schema";

    type Args = GetComponentSchemaArgs;
    type Output = String;
    type Error = GetComponentSchemaToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the schema and documentation for a specific A2UI component type. Use this to understand what properties a component accepts.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "component_type": {
                        "type": "string",
                        "description": "Component type name: column, row, grid, text, button, card, image, icon, textField, select, checkbox, switch, slider, tabs, modal, etc."
                    }
                },
                "required": ["component_type"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(get_component_schema(&args.component_type))
    }
}

/// Tool to get style examples
#[derive(Debug)]
pub struct GetStyleExamplesTool;

impl Tool for GetStyleExamplesTool {
    const NAME: &'static str = "get_style_examples";

    type Args = GetStyleExamplesArgs;
    type Output = String;
    type Error = GetStyleExamplesToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get Tailwind CSS style examples for a specific category. Useful for consistent styling.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Style category: spacing, colors, effects, layout, responsive, typography"
                    }
                },
                "required": ["category"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(get_style_examples(&args.category))
    }
}

/// Tool to modify existing components
#[derive(Debug)]
pub struct ModifyComponentTool {
    pub current_components: Option<Vec<SurfaceComponent>>,
}

impl Tool for ModifyComponentTool {
    const NAME: &'static str = "modify_component";

    type Args = ModifyComponentArgs;
    type Output = String;
    type Error = ModifyComponentToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify an existing component in the surface. Can add children, update styles, update properties, wrap in container, or delete.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "component_id": {
                        "type": "string",
                        "description": "ID of the component to modify"
                    },
                    "modification_type": {
                        "type": "string",
                        "enum": ["add_child", "update_style", "update_props", "wrap", "delete"],
                        "description": "Type of modification to perform"
                    },
                    "values": {
                        "type": "object",
                        "description": "New values to apply. For add_child: {child: SurfaceComponent}, for update_style: {className: '...'}, for wrap: {wrapper: SurfaceComponent}"
                    }
                },
                "required": ["component_id", "modification_type"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(modify_component(&args, self.current_components.as_ref()))
    }
}

/// Tool to emit the final surface components
#[derive(Debug)]
pub struct EmitSurfaceTool;

impl Tool for EmitSurfaceTool {
    const NAME: &'static str = "emit_surface";

    type Args = EmitSurfaceArgs;
    type Output = String;
    type Error = EmitSurfaceToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Emit the final surface components. ALWAYS use this tool to return your generated or modified UI. The components array should contain all SurfaceComponent objects for the UI.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "components": {
                        "type": "array",
                        "description": "Array of SurfaceComponent objects. Each component has: id (string), style (optional, with className), component (the component definition with type and props)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "style": {
                                    "type": "object",
                                    "properties": {
                                        "className": { "type": "string" }
                                    }
                                },
                                "component": {
                                    "type": "object",
                                    "description": "Component definition with type-specific properties"
                                }
                            },
                            "required": ["id", "component"]
                        }
                    },
                    "root_id": {
                        "type": "string",
                        "description": "ID of the root component (optional, defaults to first component)"
                    }
                },
                "required": ["components"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.components.is_empty() {
            return Err(EmitSurfaceToolError("emit_surface requires at least one component".to_string()));
        }
        Ok(format!(
            "Surface emitted successfully with {} components",
            args.components.len()
        ))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get component schema documentation
pub fn get_component_schema(component_type: &str) -> String {
    match component_type.to_lowercase().as_str() {
        "column" => r#"Column - Vertical flex container
Properties:
- type: "column" (required)
- gap: string (e.g., "16px", "1rem") - Space between children
- align: "start" | "center" | "end" | "stretch" | "baseline"
- justify: "start" | "center" | "end" | "between" | "around" | "evenly"
- wrap: boolean - Whether children can wrap
- children: { explicitList: ["child-id-1", "child-id-2"] }

Example:
{
  "id": "main-column",
  "style": { "className": "p-4 gap-4" },
  "component": {
    "type": "column",
    "gap": "16px",
    "children": { "explicitList": ["header", "content", "footer"] }
  }
}"#.to_string(),

        "row" => r#"Row - Horizontal flex container
Properties:
- type: "row" (required)
- gap: string - Space between children
- align: "start" | "center" | "end" | "stretch" | "baseline"
- justify: "start" | "center" | "end" | "between" | "around" | "evenly"
- wrap: boolean
- children: { explicitList: [...] }

Example:
{
  "id": "button-row",
  "style": { "className": "gap-2" },
  "component": {
    "type": "row",
    "gap": "8px",
    "justify": "end",
    "children": { "explicitList": ["cancel-btn", "submit-btn"] }
  }
}"#.to_string(),

        "grid" => r#"Grid - CSS Grid container
Properties:
- type: "grid" (required)
- columns: string (e.g., "repeat(3, 1fr)", "1fr 2fr")
- rows: string (optional)
- gap: string
- autoFlow: "row" | "column" | "dense"
- children: { explicitList: [...] }

Example:
{
  "id": "card-grid",
  "style": { "className": "gap-4" },
  "component": {
    "type": "grid",
    "columns": "repeat(auto-fill, minmax(250px, 1fr))",
    "gap": "16px",
    "children": { "explicitList": ["card-1", "card-2", "card-3"] }
  }
}"#.to_string(),

        "text" => r#"Text - Text display component
Properties:
- type: "text" (required)
- content: BoundValue - { literalString: "..." } or { path: "$.data.title" }
- variant: "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "body" | "caption" | "code" | "label"
- size: "xs" | "sm" | "md" | "lg" | "xl" | "2xl" | "3xl"
- weight: "normal" | "medium" | "semibold" | "bold"
- color: string (Tailwind color like "text-primary")
- align: "left" | "center" | "right"

Example:
{
  "id": "page-title",
  "style": { "className": "text-2xl font-bold text-primary" },
  "component": {
    "type": "text",
    "content": { "literalString": "Welcome" },
    "variant": "h1"
  }
}"#.to_string(),

        "button" => r#"Button - Interactive button
Properties:
- type: "button" (required)
- label: BoundValue - Button text
- variant: "default" | "secondary" | "outline" | "ghost" | "destructive" | "link"
- size: "sm" | "md" | "lg" | "icon"
- disabled: BoundValue (boolean)
- loading: BoundValue (boolean) - Shows loading spinner when true
- icon: BoundValue (string) - Lucide icon name (e.g., "send", "plus", "trash")
- iconPosition: BoundValue - "left" | "right" (default: "left")
- tooltip: BoundValue (string) - Tooltip text on hover
- actions: { onClick: { type: "emit", event: "..." } }

Example:
{
  "id": "submit-btn",
  "component": {
    "type": "button",
    "label": { "literalString": "Submit" },
    "variant": "default",
    "icon": { "literalString": "send" },
    "iconPosition": { "literalString": "left" }
  },
  "actions": {
    "onClick": { "type": "emit", "event": "form_submit" }
  }
}"#.to_string(),

        "card" => r#"Card - Content container with optional header/footer
Properties:
- type: "card" (required)
- title: BoundValue (optional)
- description: BoundValue (optional)
- footer: { explicitList: [...] } (optional)
- headerActions: { explicitList: [...] } (optional)
- children: { explicitList: [...] }

Example:
{
  "id": "user-card",
  "style": { "className": "p-6 shadow-md" },
  "component": {
    "type": "card",
    "title": { "literalString": "User Profile" },
    "children": { "explicitList": ["avatar", "user-info"] }
  }
}"#.to_string(),

        "textfield" | "text_field" => r#"TextField - Text input
Properties:
- type: "textField" (required)
- value: BoundValue - Current value
- placeholder: string
- inputType: "text" | "email" | "password" | "number" | "tel" | "url"
- multiline: boolean
- rows: number (for multiline)
- disabled: BoundValue (boolean)
- error: BoundValue (string, error message)
- label: string
- actions: { onChange: { type: "update", path: "..." } }

Example:
{
  "id": "email-input",
  "component": {
    "type": "textField",
    "value": { "path": "$.form.email" },
    "placeholder": "Enter email",
    "inputType": "email",
    "label": "Email Address"
  },
  "actions": {
    "onChange": { "type": "update", "path": "$.form.email" }
  }
}"#.to_string(),

        "select" => r#"Select - Dropdown selection
Properties:
- type: "select" (required)
- value: BoundValue - Selected value
- options: [{ value: string, label: string }]
- placeholder: string
- disabled: BoundValue (boolean)
- multiple: boolean
- actions: { onChange: { type: "update", path: "..." } }

Example:
{
  "id": "country-select",
  "component": {
    "type": "select",
    "value": { "path": "$.form.country" },
    "placeholder": "Select country",
    "options": [
      { "value": "us", "label": "United States" },
      { "value": "uk", "label": "United Kingdom" }
    ]
  }
}"#.to_string(),

        "image" => r#"Image - Image display
Properties:
- type: "image" (required)
- src: BoundValue - Image URL
- alt: string - Alt text (required for accessibility)
- fit: "cover" | "contain" | "fill" | "none" | "scale-down"
- loading: "lazy" | "eager"
- fallback: string - Fallback image URL

Example:
{
  "id": "profile-avatar",
  "style": { "className": "w-24 h-24 rounded-full" },
  "component": {
    "type": "image",
    "src": { "path": "$.user.avatar" },
    "alt": "User avatar",
    "fit": "cover"
  }
}"#.to_string(),

        "icon" => r#"Icon - Lucide icon
Properties:
- type: "icon" (required)
- name: string - Lucide icon name (e.g., "user", "settings", "chevron-right")
- size: "xs" | "sm" | "md" | "lg" | "xl" or number
- color: string - Tailwind color class

Example:
{
  "id": "settings-icon",
  "style": { "className": "text-muted-foreground" },
  "component": {
    "type": "icon",
    "name": "settings",
    "size": "md"
  }
}"#.to_string(),

        "checkbox" => r#"Checkbox - Boolean toggle with label
Properties:
- type: "checkbox" (required)
- checked: BoundValue (boolean)
- label: string
- disabled: BoundValue (boolean)
- actions: { onChange: { type: "update", path: "..." } }

Example:
{
  "id": "terms-checkbox",
  "component": {
    "type": "checkbox",
    "checked": { "path": "$.form.acceptTerms" },
    "label": "I accept the terms and conditions"
  }
}"#.to_string(),

        "switch" => r#"Switch - Toggle switch
Properties:
- type: "switch" (required)
- checked: BoundValue (boolean)
- label: string
- disabled: BoundValue (boolean)

Example:
{
  "id": "notifications-switch",
  "component": {
    "type": "switch",
    "checked": { "path": "$.settings.notifications" },
    "label": "Enable notifications"
  }
}"#.to_string(),

        "tabs" => r#"Tabs - Tabbed content container
Properties:
- type: "tabs" (required)
- value: BoundValue - Active tab value
- tabs: [{ value: string, label: string, icon?: string }]
- children: { explicitList: [...] } - Tab content panels

Example:
{
  "id": "settings-tabs",
  "component": {
    "type": "tabs",
    "value": { "path": "$.ui.activeTab" },
    "tabs": [
      { "value": "general", "label": "General", "icon": "settings" },
      { "value": "security", "label": "Security", "icon": "shield" }
    ],
    "children": { "explicitList": ["general-panel", "security-panel"] }
  }
}"#.to_string(),

        "modal" => r#"Modal - Dialog overlay
Properties:
- type: "modal" (required)
- open: BoundValue (boolean)
- title: BoundValue
- description: BoundValue (optional)
- children: { explicitList: [...] }
- actions: { onClose: { type: "update", path: "...", value: false } }

Example:
{
  "id": "confirm-modal",
  "component": {
    "type": "modal",
    "open": { "path": "$.ui.showConfirm" },
    "title": { "literalString": "Confirm Action" },
    "children": { "explicitList": ["modal-content", "modal-actions"] }
  }
}"#.to_string(),

        _ => format!(
            "Unknown component type: {}. Available types: column, row, grid, stack, text, image, icon, button, textField, select, checkbox, switch, slider, card, modal, tabs, accordion, divider, badge, avatar, progress, spinner, skeleton",
            component_type
        ),
    }
}

/// Get style examples for a category
pub fn get_style_examples(category: &str) -> String {
    match category.to_lowercase().as_str() {
        "spacing" => r#"Spacing Classes:
- Padding: p-1 p-2 p-3 p-4 p-5 p-6 p-8 p-10 p-12
- Padding X/Y: px-4 py-2 px-6 py-4
- Padding directional: pt-4 pr-4 pb-4 pl-4
- Margin: m-1 m-2 m-4 m-auto mx-auto my-4
- Gap: gap-1 gap-2 gap-3 gap-4 gap-6 gap-8

Common patterns:
- Card: "p-4" or "p-6"
- Button: "px-4 py-2"
- Section: "py-8" or "py-12"
- Container: "px-4 mx-auto max-w-screen-lg""#.to_string(),

        "colors" => r#"Color Classes:
Background:
- bg-background bg-card bg-popover bg-muted
- bg-primary bg-secondary bg-accent bg-destructive
- bg-white bg-black bg-transparent
- bg-gray-100 bg-gray-200 ... bg-gray-900

Text:
- text-foreground text-muted-foreground
- text-primary text-secondary text-destructive
- text-white text-black
- text-gray-500 text-gray-600 text-gray-700

Border:
- border-border border-input
- border-primary border-secondary
- border-gray-200 border-gray-300

Common patterns:
- Card: "bg-card text-card-foreground"
- Muted text: "text-muted-foreground"
- Primary button: "bg-primary text-primary-foreground"
- Hover: "hover:bg-accent hover:text-accent-foreground""#.to_string(),

        "effects" => r#"Effect Classes:
Border radius:
- rounded-none rounded-sm rounded rounded-md rounded-lg rounded-xl rounded-2xl rounded-full

Shadows:
- shadow-none shadow-sm shadow shadow-md shadow-lg shadow-xl shadow-2xl

Opacity:
- opacity-0 opacity-25 opacity-50 opacity-75 opacity-100

Transitions:
- transition-all transition-colors transition-opacity
- duration-150 duration-200 duration-300

Common patterns:
- Card: "rounded-lg shadow-md"
- Button: "rounded-md shadow-sm hover:shadow-md transition-all"
- Avatar: "rounded-full"
- Modal: "rounded-xl shadow-2xl""#.to_string(),

        "layout" => r#"Layout Classes:
Display:
- flex flex-row flex-col
- grid
- block inline inline-block hidden

Flex:
- items-start items-center items-end items-stretch
- justify-start justify-center justify-end justify-between justify-around
- flex-wrap flex-nowrap
- flex-1 flex-auto flex-none

Grid:
- grid-cols-1 grid-cols-2 grid-cols-3 grid-cols-4
- grid-rows-1 grid-rows-2
- col-span-2 col-span-full
- auto-rows-min auto-rows-max

Sizing:
- w-full w-1/2 w-1/3 w-auto w-screen
- h-full h-screen h-auto
- min-w-0 max-w-md max-w-lg max-w-screen-xl
- min-h-screen

Common patterns:
- Center content: "flex items-center justify-center"
- Space between: "flex justify-between items-center"
- Responsive grid: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4""#.to_string(),

        "responsive" => r#"Responsive Prefixes:
- sm: 640px and up
- md: 768px and up
- lg: 1024px and up
- xl: 1280px and up
- 2xl: 1536px and up

Examples:
- "p-2 md:p-4 lg:p-6" - Padding increases with screen size
- "grid-cols-1 md:grid-cols-2 lg:grid-cols-3" - Responsive grid
- "hidden md:block" - Hide on mobile, show on tablet+
- "text-sm md:text-base lg:text-lg" - Responsive text size
- "flex-col md:flex-row" - Stack on mobile, row on tablet+"#.to_string(),

        "typography" => r#"Typography Classes:
Size:
- text-xs text-sm text-base text-lg text-xl text-2xl text-3xl text-4xl

Weight:
- font-thin font-light font-normal font-medium font-semibold font-bold font-extrabold

Style:
- italic not-italic underline line-through

Line height:
- leading-none leading-tight leading-snug leading-normal leading-relaxed leading-loose

Alignment:
- text-left text-center text-right text-justify

Common patterns:
- Heading: "text-2xl font-bold"
- Subheading: "text-lg font-semibold"
- Body: "text-base font-normal"
- Caption: "text-sm text-muted-foreground"
- Code: "font-mono text-sm""#.to_string(),

        _ => format!(
            "Unknown style category: {}. Available: spacing, colors, effects, layout, responsive, typography",
            category
        ),
    }
}

/// Modify an existing component
pub fn modify_component(
    args: &ModifyComponentArgs,
    current_components: Option<&Vec<SurfaceComponent>>,
) -> String {
    let Some(components) = current_components else {
        return "Error: No current components to modify".to_string();
    };

    let component_exists = components.iter().any(|c| c.id == args.component_id);
    if !component_exists {
        return format!("Error: Component '{}' not found", args.component_id);
    }

    match args.modification_type.as_str() {
        "add_child" => format!(
            "To add a child to '{}', create a new component and add its ID to the parent's children.explicitList array.",
            args.component_id
        ),
        "update_style" => format!(
            "To update style of '{}', modify the style.className property with the new Tailwind classes.",
            args.component_id
        ),
        "update_props" => format!(
            "To update props of '{}', modify the component definition with the new property values.",
            args.component_id
        ),
        "wrap" => format!(
            "To wrap '{}', create a new container component, move '{}' to its children, and update any parent references.",
            args.component_id, args.component_id
        ),
        "delete" => format!(
            "To delete '{}', remove it from the components array and remove its ID from any parent children arrays.",
            args.component_id
        ),
        _ => format!("Unknown modification type: {}", args.modification_type),
    }
}
