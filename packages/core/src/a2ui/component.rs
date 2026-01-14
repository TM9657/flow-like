use flow_like_types::{
    Value,
    json::{Deserialize, Serialize},
    proto,
};
use schemars::JsonSchema;

/// Default value for path bindings - stores the preview value
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PathDefault {
    String(String),
    Number(f64),
    Bool(bool),
}

/// Path binding with optional default value for preview
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathBinding {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<PathDefault>,
}

/// Represents a value that can be either a literal or a data path binding
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", untagged)]
pub enum BoundValue {
    #[serde(rename_all = "camelCase")]
    PathBinding(PathBinding),
    LiteralString { #[serde(rename = "literalString")] value: String },
    LiteralNumber { #[serde(rename = "literalNumber")] value: f64 },
    LiteralBool { #[serde(rename = "literalBool")] value: bool },
}

impl BoundValue {
    pub fn literal_string(value: impl Into<String>) -> Self {
        Self::LiteralString { value: value.into() }
    }

    pub fn literal_number(value: f64) -> Self {
        Self::LiteralNumber { value }
    }

    pub fn literal_bool(value: bool) -> Self {
        Self::LiteralBool { value }
    }

    pub fn path(value: impl Into<String>) -> Self {
        Self::PathBinding(PathBinding { path: value.into(), default_value: None })
    }

    pub fn path_with_default(path: impl Into<String>, default: PathDefault) -> Self {
        Self::PathBinding(PathBinding { path: path.into(), default_value: Some(default) })
    }

    pub fn is_literal(&self) -> bool {
        !matches!(self, Self::PathBinding(_))
    }

    pub fn is_path(&self) -> bool {
        matches!(self, Self::PathBinding(_))
    }

    pub fn get_path(&self) -> Option<&str> {
        match self {
            Self::PathBinding(pb) => Some(&pb.path),
            _ => None,
        }
    }

    pub fn get_default(&self) -> Option<&PathDefault> {
        match self {
            Self::PathBinding(pb) => pb.default_value.as_ref(),
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::LiteralString { value } => Value::String(value.clone()),
            Self::LiteralNumber { value } => {
                Value::Number(serde_json::Number::from_f64(*value).unwrap_or(serde_json::Number::from(0)))
            }
            Self::LiteralBool { value } => Value::Bool(*value),
            Self::PathBinding(pb) => Value::String(pb.path.clone()),
        }
    }
}

impl From<&proto::BoundValue> for BoundValue {
    fn from(proto: &proto::BoundValue) -> Self {
        match &proto.value {
            Some(proto::bound_value::Value::LiteralString(s)) => Self::LiteralString { value: s.clone() },
            Some(proto::bound_value::Value::LiteralNumber(n)) => Self::LiteralNumber { value: *n },
            Some(proto::bound_value::Value::LiteralBool(b)) => Self::LiteralBool { value: *b },
            Some(proto::bound_value::Value::Path(p)) => Self::PathBinding(PathBinding { path: p.clone(), default_value: None }),
            None => Self::LiteralString { value: String::new() },
        }
    }
}

impl From<proto::BoundValue> for BoundValue {
    fn from(proto: proto::BoundValue) -> Self {
        match proto.value {
            Some(proto::bound_value::Value::LiteralString(s)) => Self::LiteralString { value: s },
            Some(proto::bound_value::Value::LiteralNumber(n)) => Self::LiteralNumber { value: n },
            Some(proto::bound_value::Value::LiteralBool(b)) => Self::LiteralBool { value: b },
            Some(proto::bound_value::Value::Path(p)) => Self::PathBinding(PathBinding { path: p, default_value: None }),
            None => Self::LiteralString { value: String::new() },
        }
    }
}

impl From<BoundValue> for proto::BoundValue {
    fn from(value: BoundValue) -> Self {
        proto::BoundValue {
            value: Some(match value {
                BoundValue::LiteralString { value } => proto::bound_value::Value::LiteralString(value),
                BoundValue::LiteralNumber { value } => proto::bound_value::Value::LiteralNumber(value),
                BoundValue::LiteralBool { value } => proto::bound_value::Value::LiteralBool(value),
                BoundValue::PathBinding(pb) => proto::bound_value::Value::Path(pb.path),
            }),
        }
    }
}

/// Action definition for user interactions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub name: String,
    #[serde(default)]
    pub context: std::collections::HashMap<String, BoundValue>,
    pub target_surface_id: Option<String>,
}

impl Action {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            context: std::collections::HashMap::new(),
            target_surface_id: None,
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: BoundValue) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    pub fn with_target(mut self, surface_id: impl Into<String>) -> Self {
        self.target_surface_id = Some(surface_id.into());
        self
    }
}

impl From<&proto::Action> for Action {
    fn from(proto: &proto::Action) -> Self {
        Self {
            name: proto.name.clone(),
            context: proto
                .context
                .iter()
                .map(|(k, v)| (k.clone(), BoundValue::from(v)))
                .collect(),
            target_surface_id: proto.target_surface_id.clone(),
        }
    }
}

impl From<Action> for proto::Action {
    fn from(value: Action) -> Self {
        proto::Action {
            name: value.name,
            context: value
                .context
                .into_iter()
                .map(|(k, v)| (k, proto::BoundValue::from(v)))
                .collect(),
            target_surface_id: value.target_surface_id,
        }
    }
}

/// Children definition - explicit list or template
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Children {
    ExplicitList(Vec<String>),
    Template {
        data_binding: String,
        component_id: String,
        item_id_path: Option<String>,
    },
}

impl Children {
    pub fn explicit(ids: Vec<String>) -> Self {
        Self::ExplicitList(ids)
    }

    pub fn template(data_binding: impl Into<String>, component_id: impl Into<String>) -> Self {
        Self::Template {
            data_binding: data_binding.into(),
            component_id: component_id.into(),
            item_id_path: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::ExplicitList(ids) => ids.is_empty(),
            Self::Template { .. } => false,
        }
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::ExplicitList(Vec::new())
    }
}

impl From<&proto::Children> for Children {
    fn from(proto: &proto::Children) -> Self {
        match &proto.children_type {
            Some(proto::children::ChildrenType::ExplicitList(list)) => {
                Self::ExplicitList(list.component_ids.clone())
            }
            Some(proto::children::ChildrenType::Template(t)) => Self::Template {
                data_binding: t.data_binding.clone(),
                component_id: t.component_id.clone(),
                item_id_path: t.item_id_path.clone(),
            },
            None => Self::ExplicitList(Vec::new()),
        }
    }
}

impl From<Children> for proto::Children {
    fn from(value: Children) -> Self {
        proto::Children {
            children_type: Some(match value {
                Children::ExplicitList(ids) => {
                    proto::children::ChildrenType::ExplicitList(proto::ExplicitList {
                        component_ids: ids,
                    })
                }
                Children::Template {
                    data_binding,
                    component_id,
                    item_id_path,
                } => proto::children::ChildrenType::Template(proto::Template {
                    data_binding,
                    component_id,
                    item_id_path,
                }),
            }),
        }
    }
}
