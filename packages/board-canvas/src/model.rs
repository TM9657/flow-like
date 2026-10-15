use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub type Rgba = [u8; 4];

fn default_pin_font_size() -> f32 {
    9.6
}
fn default_label_font_size() -> f32 {
    10.0
}
fn default_header_height() -> f32 {
    16.0
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Input,
    Output,
}

impl Direction {
    pub fn sign(self) -> f32 {
        match self {
            Self::Input => -1.0,
            Self::Output => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionKind {
    Execution,
    Data,
    Reference,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub id: String,
    pub direction: Direction,
    pub x: f32,
    pub y: f32,
    pub label: String,
    pub label_max_width: Option<f32>,
    #[serde(default = "default_pin_font_size")]
    pub font_size: f32,
    pub color: Rgba,
    pub kind: ConnectionKind,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: String,
    #[serde(default = "default_label_font_size")]
    pub label_font_size: f32,
    #[serde(default = "default_header_height")]
    pub header_height: f32,
    pub corner_radius: Option<f32>,
    pub kind: String,
    #[serde(default)]
    pub pins: Vec<Pin>,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub dimmed: bool,
    #[serde(default)]
    pub dom: bool,
}

impl Node {
    pub fn corner_radius(&self) -> f32 {
        self.corner_radius
            .unwrap_or(if self.kind == "reroute" { 6.0 } else { 4.0 })
            .min(self.width * 0.5)
            .min(self.height * 0.5)
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub node_id: String,
    pub pin_id: String,
}

#[derive(Clone, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source: Endpoint,
    pub target: Endpoint,
    pub kind: ConnectionKind,
    #[serde(default)]
    pub selected: bool,
    pub color: Rgba,
    #[serde(default)]
    pub dom: bool,
    #[serde(default)]
    pub lite: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum PaintLayer {
    #[default]
    All,
    Edges,
    Nodes,
}

impl PaintLayer {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "all" => Ok(Self::All),
            "edges" => Ok(Self::Edges),
            "nodes" => Ok(Self::Nodes),
            _ => Err(format!("Unknown paint layer: {value}")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Edges => "edges",
            Self::Nodes => "nodes",
        }
    }
}

#[derive(Default, Deserialize)]
pub struct Scene {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Deserialize)]
pub struct Palette {
    pub background: Rgba,
    pub card: Rgba,
    pub foreground: Rgba,
    pub muted: Rgba,
    pub border: Rgba,
    pub primary: Rgba,
    pub tertiary: Rgba,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPreview {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub from_direction: Direction,
    pub kind: Option<ConnectionKind>,
    pub valid: Option<bool>,
    pub color: Rgba,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoveredPin {
    pub node_id: String,
    pub pin_id: String,
    pub valid: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overlay {
    pub connection: Option<ConnectionPreview>,
    pub hovered_pin: Option<HoveredPin>,
    pub selection_rect: Option<[f32; 4]>,
}

impl Overlay {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .connection
            .as_ref()
            .is_some_and(|c| c.from.iter().chain(&c.to).any(|value| !value.is_finite()))
            || self
                .selection_rect
                .is_some_and(|rect| rect.iter().any(|value| !value.is_finite()))
        {
            return Err("Overlay coordinates must be finite".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct PositionUpdate {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

pub struct SceneIndex {
    pub nodes: HashMap<String, usize>,
    pub pins: Vec<HashMap<String, usize>>,
    pub incident: Vec<Vec<usize>>,
}

impl SceneIndex {
    pub fn new(scene: &Scene) -> Result<Self, String> {
        let mut nodes = HashMap::with_capacity(scene.nodes.len());
        let mut pins = Vec::with_capacity(scene.nodes.len());
        for (index, node) in scene.nodes.iter().enumerate() {
            if node.id.is_empty() || nodes.insert(node.id.clone(), index).is_some() {
                return Err(format!("Duplicate or empty node id: {}", node.id));
            }
            if [node.x, node.y, node.width, node.height]
                .iter()
                .any(|v| !v.is_finite())
                || node.width <= 0.0
                || node.height <= 0.0
                || !node.label_font_size.is_finite()
                || node.label_font_size <= 0.0
                || !node.header_height.is_finite()
                || node.header_height <= 0.0
                || node
                    .corner_radius
                    .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
            {
                return Err(format!("Invalid node dimensions: {}", node.id));
            }
            let mut by_id = HashMap::with_capacity(node.pins.len());
            for (pin_index, pin) in node.pins.iter().enumerate() {
                if pin.id.is_empty() || by_id.insert(pin.id.clone(), pin_index).is_some() {
                    return Err(format!("Duplicate or empty pin id on node {}", node.id));
                }
                if !pin.x.is_finite()
                    || !pin.y.is_finite()
                    || !pin.font_size.is_finite()
                    || pin.font_size <= 0.0
                    || pin
                        .label_max_width
                        .is_some_and(|width| !width.is_finite() || width < 0.0)
                {
                    return Err(format!("Invalid pin position: {}", pin.id));
                }
            }
            pins.push(by_id);
        }
        let mut incident = vec![Vec::new(); scene.nodes.len()];
        let mut edge_ids = HashSet::with_capacity(scene.edges.len());
        for (index, edge) in scene.edges.iter().enumerate() {
            if edge.id.is_empty() || !edge_ids.insert(&edge.id) {
                return Err(format!("Duplicate or empty edge id: {}", edge.id));
            }
            for endpoint in [&edge.source, &edge.target] {
                if let Some(&node) = nodes.get(&endpoint.node_id) {
                    incident[node].push(index);
                }
            }
        }
        Ok(Self {
            nodes,
            pins,
            incident,
        })
    }

    pub fn endpoint<'a>(
        &self,
        scene: &'a Scene,
        endpoint: &Endpoint,
    ) -> Option<(&'a Node, &'a Pin)> {
        let &node_index = self.nodes.get(&endpoint.node_id)?;
        let &pin_index = self.pins[node_index].get(&endpoint.pin_id)?;
        Some((
            &scene.nodes[node_index],
            &scene.nodes[node_index].pins[pin_index],
        ))
    }

    pub fn apply_positions(
        &self,
        scene: &mut Scene,
        updates: &[PositionUpdate],
    ) -> Result<Vec<usize>, String> {
        // Validate the complete batch so one bad id cannot leave a partially moved scene.
        for update in updates {
            if !self.nodes.contains_key(&update.id)
                || !update.x.is_finite()
                || !update.y.is_finite()
            {
                return Err(format!("Invalid position update: {}", update.id));
            }
        }
        let mut affected = Vec::new();
        for update in updates {
            let index = self.nodes[&update.id];
            let node = &mut scene.nodes[index];
            if node.x != update.x || node.y != update.y {
                node.x = update.x;
                node.y = update.y;
                affected.extend_from_slice(&self.incident[index]);
            }
        }
        affected.sort_unstable();
        affected.dedup();
        Ok(affected)
    }
}

#[cfg(test)]
pub fn fixture() -> Scene {
    serde_json::from_value(serde_json::json!({
        "nodes": [
            {"id":"a","x":0,"y":0,"width":100,"height":60,"label":"A","kind":"event",
             "pins":[{"id":"out","direction":"output","x":100,"y":30,"label":"exec","color":[255,255,255,255],"kind":"execution"}]},
            {"id":"b","x":200,"y":0,"width":100,"height":60,"label":"B","kind":"standard",
             "pins":[{"id":"in","direction":"input","x":0,"y":30,"label":"exec","color":[255,255,255,255],"kind":"execution"}]},
            {"id":"c","x":400,"y":0,"width":100,"height":60,"label":"C","kind":"pure","pins":[]}
        ],
        "edges":[{"id":"ab","source":{"nodeId":"a","pinId":"out"},"target":{"nodeId":"b","pinId":"in"},"kind":"execution","color":[255,255,255,255]}]
    })).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_metrics_preserve_square_styles_and_clamp_to_node_bounds() {
        let mut scene = fixture();
        assert_eq!(scene.nodes[0].corner_radius(), 4.0);
        scene.nodes[0].kind = "reroute".into();
        assert_eq!(scene.nodes[0].corner_radius(), 6.0);
        scene.nodes[0].corner_radius = Some(0.0);
        assert_eq!(scene.nodes[0].corner_radius(), 0.0);
        assert!(SceneIndex::new(&scene).is_ok());
        scene.nodes[0].corner_radius = Some(200.0);
        assert_eq!(scene.nodes[0].corner_radius(), 30.0);
        scene.nodes[0].corner_radius = Some(-1.0);
        assert!(SceneIndex::new(&scene).is_err());
        scene.nodes[0].corner_radius = Some(f32::NAN);
        assert!(SceneIndex::new(&scene).is_err());
    }

    #[test]
    fn typography_defaults_and_explicit_invalid_metrics_are_validated() {
        let mut scene = fixture();
        assert_eq!(scene.nodes[0].label_font_size, 10.0);
        assert_eq!(scene.nodes[0].header_height, 16.0);
        assert_eq!(scene.nodes[0].pins[0].font_size, 9.6);
        scene.nodes[0].header_height = 20.0;
        scene.nodes[0].pins[0].font_size = 12.0;
        assert!(SceneIndex::new(&scene).is_ok());
        scene.nodes[0].pins[0].font_size = 0.0;
        assert!(SceneIndex::new(&scene).is_err());
        scene.nodes[0].pins[0].font_size = 12.0;
        scene.nodes[0].label_font_size = f32::INFINITY;
        assert!(SceneIndex::new(&scene).is_err());
    }

    #[test]
    fn moving_multiple_endpoints_invalidates_an_edge_once() {
        let mut scene = fixture();
        let index = SceneIndex::new(&scene).unwrap();
        let changed = index
            .apply_positions(
                &mut scene,
                &[
                    PositionUpdate {
                        id: "a".into(),
                        x: 10.0,
                        y: 20.0,
                    },
                    PositionUpdate {
                        id: "b".into(),
                        x: 210.0,
                        y: 20.0,
                    },
                ],
            )
            .unwrap();
        assert_eq!(changed, vec![0]);
        assert_eq!(scene.nodes[0].x, 10.0);
        assert!(
            index
                .apply_positions(
                    &mut scene,
                    &[PositionUpdate {
                        id: "c".into(),
                        x: 500.0,
                        y: 0.0
                    }]
                )
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn invalid_position_batch_is_atomic() {
        let mut scene = fixture();
        let index = SceneIndex::new(&scene).unwrap();
        assert!(
            index
                .apply_positions(
                    &mut scene,
                    &[
                        PositionUpdate {
                            id: "a".into(),
                            x: 10.0,
                            y: 20.0
                        },
                        PositionUpdate {
                            id: "gone".into(),
                            x: 210.0,
                            y: 20.0
                        },
                    ]
                )
                .is_err()
        );
        assert_eq!(scene.nodes[0].x, 0.0);
    }

    #[test]
    fn duplicate_identifiers_are_rejected() {
        let mut scene = fixture();
        scene.nodes[1].id = "a".into();
        assert!(SceneIndex::new(&scene).is_err());
    }

    #[test]
    fn dom_edges_keep_canonical_endpoint_geometry() {
        let mut scene = fixture();
        scene.edges[0].dom = true;
        let index = SceneIndex::new(&scene).unwrap();
        assert!(index.endpoint(&scene, &scene.edges[0].source).is_some());
        assert_eq!(index.incident[0], vec![0]);
        assert_eq!(PaintLayer::default().name(), "all");
        assert_eq!(PaintLayer::parse("edges").unwrap().name(), "edges");
        assert!(PaintLayer::parse("invalid").is_err());
    }
}
