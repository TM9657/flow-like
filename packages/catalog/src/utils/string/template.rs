
/// # Render Jinja Templates

use flow_like::{
    flow::{
        board::Board,
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Value, async_trait, json::json, minijinja};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};


#[derive(Default)]
pub struct TemplateStringNode {}

impl TemplateStringNode {
    pub fn new() -> Self {
        TemplateStringNode {}
    }
}


#[async_trait]
impl NodeLogic for TemplateStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "render_template",
            "Render Template",
            "Template Engine based on Jinja Templates",
            "Utils/String",
        );
        node.add_icon("/flow/icons/string.svg");

        // inputs
        node.add_input_pin(
            "template",
            "Template",
            "Jinja Template String",
            VariableType::String,
        );

        // outputs
        node.add_output_pin(
            "rendered",
            "Rendered",
            "Rendered String",
            VariableType::String,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {

        // load inputs & templates
        let template_string: String = context.evaluate_pin("template").await?;
        let mut jinja_env = minijinja::Environment::new();
        
        // collect placeholders & values
        jinja_env.add_template("template", &template_string).unwrap();
        let template = jinja_env.get_template("template").unwrap();
        let placeholders = template.undeclared_variables(false);
        context.log_message(&format!("extracted placholders: {:?}", placeholders), LogLevel::Debug);
        
        let mut template_context = HashMap::new();
        for placeholder in placeholders {
            let value: flow_like_types::Value = context.evaluate_pin(&placeholder).await?;
            template_context.insert(placeholder, value);
        }

        // render template
        let rendered = template.render(template_context).unwrap();

        // set outputs
        context
            .set_pin_value("rendered", json!(rendered))
            .await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let pins: Vec<_> = node
            .pins
            .values()
            .filter(|p| p.name != "template" && p.pin_type == PinType::Input)
            .collect();

        let template_string: String = node
            .get_pin_by_name("template")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| flow_like_types::json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        let mut current_placeholders = pins
            .iter()
            .map(|p| (p.name.clone(), *p))
            .collect::<HashMap<_, _>>();

        let mut jinja_env = minijinja::Environment::new();
        jinja_env.add_template("template", &template_string).unwrap();
        let template = jinja_env.get_template("template").unwrap();
        let template_placeholders = template.undeclared_variables(false);
        let mut all_placeholders = HashSet::new();
        let mut missing_placeholders = HashSet::new();

        for placeholder in template_placeholders {
            all_placeholders.insert(placeholder.clone());
            if current_placeholders.remove(&placeholder).is_none() {
                missing_placeholders.insert(placeholder.clone());
            }
        }

        let ids_to_remove = current_placeholders
            .values()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>();
        ids_to_remove.iter().for_each(|id| {
            node.pins.remove(id);
        });

        for placeholder in missing_placeholders {
            node.add_input_pin(&placeholder, &placeholder, "", VariableType::Generic);
        }

        all_placeholders.iter().for_each(|placeholder| {
            let _ = node.match_type(placeholder, board.clone(), None, None);
        })

    }
}
