use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::geo::GeoCoordinate;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub enum RouteProfile {
    #[default]
    Car,
    Bike,
    Foot,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct RouteStep {
    pub instruction: String,
    pub distance: f64,
    pub duration: f64,
    pub name: String,
    pub maneuver_type: String,
    pub coordinate: GeoCoordinate,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct RouteLeg {
    pub distance: f64,
    pub duration: f64,
    pub summary: String,
    pub steps: Vec<RouteStep>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct RouteResult {
    pub distance: f64,
    pub duration: f64,
    pub geometry: Vec<GeoCoordinate>,
    pub legs: Vec<RouteLeg>,
    pub weight_name: String,
}

#[crate::register_node]
#[derive(Default)]
pub struct PlanRouteNode {}

impl PlanRouteNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for PlanRouteNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "geo_plan_route",
            "Plan Route",
            "Plans a route between two points using the OSRM routing service. Returns turn-by-turn directions, distance, and duration.",
            "Web/Geo/Routing",
        );
        node.add_icon("/flow/icons/map.svg");

        node.add_input_pin(
            "exec_in",
            "Execute",
            "Initiate route planning",
            VariableType::Execution,
        );
        node.add_input_pin(
            "start",
            "Start",
            "Starting coordinate for the route",
            VariableType::Struct,
        )
        .set_schema::<GeoCoordinate>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "end",
            "End",
            "Ending coordinate for the route",
            VariableType::Struct,
        )
        .set_schema::<GeoCoordinate>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "waypoints",
            "Waypoints",
            "Optional intermediate waypoints to pass through",
            VariableType::Struct,
        )
        .set_schema::<GeoCoordinate>()
        .set_default_value(Some(json!([])));

        node.add_input_pin(
            "profile",
            "Profile",
            "Transportation mode: Car, Bike, or Foot",
            VariableType::Struct,
        )
        .set_schema::<RouteProfile>()
        .set_default_value(Some(json!(RouteProfile::Car)));

        node.add_input_pin(
            "alternatives",
            "Alternatives",
            "Request alternative routes",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_success",
            "Success",
            "Triggered when route planning completes successfully",
            VariableType::Execution,
        );
        node.add_output_pin(
            "exec_error",
            "Error",
            "Triggered when route planning fails",
            VariableType::Execution,
        );
        node.add_output_pin(
            "route",
            "Route",
            "The primary calculated route",
            VariableType::Struct,
        )
        .set_schema::<RouteResult>();

        node.add_output_pin(
            "alternatives_out",
            "Alternative Routes",
            "Alternative routes if requested",
            VariableType::Struct,
        )
        .set_schema::<RouteResult>();

        node.add_output_pin(
            "distance",
            "Distance",
            "Total route distance in meters",
            VariableType::Float,
        );

        node.add_output_pin(
            "duration",
            "Duration",
            "Estimated travel time in seconds",
            VariableType::Float,
        );

        node.add_output_pin(
            "geometry",
            "Geometry",
            "Route geometry as array of coordinates",
            VariableType::Struct,
        )
        .set_schema::<GeoCoordinate>();

        node.set_scores(
            NodeScores::new()
                .set_privacy(7)
                .set_security(9)
                .set_performance(7)
                .set_reliability(8)
                .set_cost(10)
                .build(),
        );

        node
    }

    #[cfg(feature = "execute")]
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        use flow_like_types::reqwest;

        context.deactivate_exec_pin("exec_success").await?;
        context.activate_exec_pin("exec_error").await?;

        let start: GeoCoordinate = context.evaluate_pin("start").await?;
        let end: GeoCoordinate = context.evaluate_pin("end").await?;
        let waypoints: Vec<GeoCoordinate> = context.evaluate_pin("waypoints").await?;
        let profile: RouteProfile = context.evaluate_pin("profile").await?;
        let alternatives: bool = context.evaluate_pin("alternatives").await?;

        let profile_str = match profile {
            RouteProfile::Car => "driving",
            RouteProfile::Bike => "cycling",
            RouteProfile::Foot => "foot",
        };

        let mut coordinates = vec![format!("{},{}", start.longitude, start.latitude)];
        for wp in &waypoints {
            coordinates.push(format!("{},{}", wp.longitude, wp.latitude));
        }
        coordinates.push(format!("{},{}", end.longitude, end.latitude));
        let coords_str = coordinates.join(";");

        let client = reqwest::Client::builder()
            .user_agent("FlowLike/1.0")
            .build()?;

        let url = format!(
            "https://router.project-osrm.org/route/v1/{}/{}?overview=full&geometries=geojson&steps=true&alternatives={}",
            profile_str, coords_str, alternatives
        );

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(flow_like_types::anyhow!(
                "OSRM API returned status: {}",
                response.status()
            ));
        }

        let body: OsrmResponse = response.json().await?;

        if body.code != "Ok" {
            return Err(flow_like_types::anyhow!(
                "OSRM returned error: {}",
                body.message.unwrap_or_else(|| body.code.clone())
            ));
        }

        let routes: Vec<RouteResult> = body
            .routes
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                let geometry = r
                    .geometry
                    .coordinates
                    .iter()
                    .map(|c| GeoCoordinate::new(c[1], c[0]))
                    .collect();

                let legs = r
                    .legs
                    .into_iter()
                    .map(|leg| {
                        let steps = leg
                            .steps
                            .into_iter()
                            .map(|step| RouteStep {
                                instruction: step
                                    .maneuver
                                    .as_ref()
                                    .map(|m| m.modifier.clone().unwrap_or_else(|| m.r#type.clone()))
                                    .unwrap_or_default(),
                                distance: step.distance,
                                duration: step.duration,
                                name: step.name,
                                maneuver_type: step
                                    .maneuver
                                    .as_ref()
                                    .map(|m| m.r#type.clone())
                                    .unwrap_or_default(),
                                coordinate: step
                                    .maneuver
                                    .as_ref()
                                    .map(|m| GeoCoordinate::new(m.location[1], m.location[0]))
                                    .unwrap_or_default(),
                            })
                            .collect();

                        RouteLeg {
                            distance: leg.distance,
                            duration: leg.duration,
                            summary: leg.summary,
                            steps,
                        }
                    })
                    .collect();

                RouteResult {
                    distance: r.distance,
                    duration: r.duration,
                    geometry,
                    legs,
                    weight_name: r.weight_name,
                }
            })
            .collect();

        let primary_route = routes.first().cloned().unwrap_or_default();
        let alt_routes: Vec<RouteResult> = routes.into_iter().skip(1).collect();

        context.set_pin_value("route", json!(primary_route)).await?;
        context
            .set_pin_value("alternatives_out", json!(alt_routes))
            .await?;
        context
            .set_pin_value("distance", json!(primary_route.distance))
            .await?;
        context
            .set_pin_value("duration", json!(primary_route.duration))
            .await?;
        context
            .set_pin_value("geometry", json!(primary_route.geometry))
            .await?;

        context.deactivate_exec_pin("exec_error").await?;
        context.activate_exec_pin("exec_success").await?;

        Ok(())
    }

    #[cfg(not(feature = "execute"))]
    async fn run(&self, _context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        Err(flow_like_types::anyhow!(
            "This node requires the 'execute' feature"
        ))
    }
}

#[derive(Deserialize)]
struct OsrmResponse {
    code: String,
    message: Option<String>,
    routes: Option<Vec<OsrmRoute>>,
}

#[derive(Deserialize)]
struct OsrmRoute {
    distance: f64,
    duration: f64,
    geometry: OsrmGeometry,
    legs: Vec<OsrmLeg>,
    weight_name: String,
}

#[derive(Deserialize)]
struct OsrmGeometry {
    coordinates: Vec<Vec<f64>>,
}

#[derive(Deserialize)]
struct OsrmLeg {
    distance: f64,
    duration: f64,
    summary: String,
    steps: Vec<OsrmStep>,
}

#[derive(Deserialize)]
struct OsrmStep {
    distance: f64,
    duration: f64,
    name: String,
    maneuver: Option<OsrmManeuver>,
}

#[derive(Deserialize)]
struct OsrmManeuver {
    #[serde(rename = "type")]
    r#type: String,
    modifier: Option<String>,
    location: Vec<f64>,
}
