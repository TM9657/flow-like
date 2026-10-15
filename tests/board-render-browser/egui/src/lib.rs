use eframe::egui::{self, Color32, FontId, Pos2, Rect, Shape, Stroke, pos2, vec2};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::prelude::*;
mod drag;
mod lod;
use drag::{DragAnchor, PointerEvent, pointer_frame};
use lod::Detail;

#[derive(Deserialize)]
struct Node {
    #[allow(dead_code)]
    id: String,
    x: f32,
    y: f32,
    label: String,
    kind: String,
    #[serde(skip)]
    title: Option<(u32, Arc<egui::Galley>)>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Edge {
    #[allow(dead_code)]
    id: String,
    source: usize,
    target: usize,
    source_pin: usize,
    target_pin: usize,
    kind: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    node_width: f32,
    node_height: f32,
    pin_start: f32,
    pin_step: f32,
    pin_labels: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Palette {
    background: [u8; 4],
    card: [u8; 4],
    foreground: [u8; 4],
    border: [u8; 4],
    primary: [u8; 4],
    tertiary: [u8; 4],
    pin_execution: [u8; 4],
    pin_string: [u8; 4],
    pin_float: [u8; 4],
    pin_integer: [u8; 4],
}

fn color(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn alpha(rgba: [u8; 4], opacity: f32) -> Color32 {
    color([rgba[0], rgba[1], rgba[2], (rgba[3] as f32 * opacity) as u8])
}

fn mix(a: [u8; 4], b: [u8; 4], amount: f32) -> [u8; 4] {
    std::array::from_fn(|i| (a[i] as f32 * (1.0 - amount) + b[i] as f32 * amount) as u8)
}

struct EdgeGeometry {
    points: Vec<Pos2>,
    coarse_points: Vec<Pos2>,
    coarse_error: f32,
    lengths: Vec<f32>,
    bounds: Rect,
}

impl EdgeGeometry {
    fn new(edge: &Edge, graph: &Fixture) -> Self {
        let source = &graph.nodes[edge.source];
        let target = &graph.nodes[edge.target];
        let a = pos2(
            source.x + graph.node_width,
            source.y + graph.pin_start + edge.source_pin as f32 * graph.pin_step,
        );
        let d = pos2(
            target.x,
            target.y + graph.pin_start + edge.target_pin as f32 * graph.pin_step,
        );
        let distance = d.x - a.x;
        let offset = if distance >= 0.0 {
            distance * 0.5
        } else {
            0.25 * 25.0 * (-distance).sqrt()
        };
        let b = a + vec2(offset, 0.0);
        let c = d - vec2(offset, 0.0);
        let mut points = Vec::with_capacity(33);
        let mut lengths = Vec::with_capacity(33);
        let mut length = 0.0;
        for i in 0..=32 {
            let t = i as f32 / 32.0;
            let u = 1.0 - t;
            let p = (a.to_vec2() * u.powi(3)
                + b.to_vec2() * (3.0 * u * u * t)
                + c.to_vec2() * (3.0 * u * t * t)
                + d.to_vec2() * t.powi(3))
            .to_pos2();
            if let Some(previous) = points.last() {
                length += p.distance(*previous);
            }
            points.push(p);
            lengths.push(length);
        }
        // The control hull is conservative even when both endpoints are outside the viewport.
        let bounds = Rect::from_points(&[a, b, c, d]).expand(4.0);
        let coarse_points: Vec<_> = points.iter().step_by(4).copied().collect();
        let mut coarse_error: f32 = 0.0;
        for (i, point) in points.iter().enumerate().take(32) {
            let segment = i / 4;
            let start = coarse_points[segment];
            let delta = coarse_points[segment + 1] - start;
            let t = ((*point - start).dot(delta) / delta.length_sq().max(0.001)).clamp(0.0, 1.0);
            coarse_error = coarse_error.max(point.distance(start + delta * t));
        }
        Self {
            points,
            coarse_points,
            coarse_error,
            lengths,
            bounds,
        }
    }

    fn section(&self, start: f32, end: f32) -> Vec<Pos2> {
        let mut points = Vec::new();
        for i in 1..self.points.len() {
            let lo = self.lengths[i - 1];
            let hi = self.lengths[i];
            if hi < start || lo > end || hi <= lo {
                continue;
            }
            let a = self.points[i - 1];
            let delta = self.points[i] - a;
            if points.is_empty() {
                points.push(a + delta * ((start - lo) / (hi - lo)).clamp(0.0, 1.0));
            }
            points.push(a + delta * ((end - lo) / (hi - lo)).clamp(0.0, 1.0));
        }
        points
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    frames: u64,
    paint_cpu_ms: Vec<f64>,
    paint_cpu_total_ms: f64,
    rendered_nodes: usize,
    rendered_edges: usize,
    node_count: usize,
    edge_count: usize,
    viewport: [f32; 3],
    activity_tick: Option<u32>,
    drag_events: u64,
    reconnect_count: u64,
    latest_mutation_at_ms: f64,
    lod_enabled: bool,
    lod_level: &'static str,
    lod_details: &'static str,
    labels_drawn: usize,
    pins_drawn: usize,
    edge_segments_drawn: usize,
}

struct Board {
    canvas: web_sys::HtmlCanvasElement,
    graph: Fixture,
    palette: Palette,
    geometry: Vec<EdgeGeometry>,
    incident: Vec<Vec<usize>>,
    viewport: [f32; 3],
    motion: bool,
    motion_started_at: f64,
    webkit: bool,
    lod_enabled: bool,
    activity: Option<u32>,
    stats: Stats,
    ctx: Option<egui::Context>,
    dragging_node: Option<usize>,
    gesture_anchor: Option<DragAnchor>,
    gesture_button: Option<u8>,
    pin_text: Option<(u32, Vec<Arc<egui::Galley>>)>,
    medium_font: bool,
    selected_edge: Option<usize>,
    reconnecting: bool,
    canvas_size: [f32; 2],
}

impl Board {
    fn wake(&self) {
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    fn move_nodes(&mut self, indices: &[usize], dx: f32, dy: f32) {
        let mut affected = Vec::new();
        for &index in indices {
            if let Some(node) = self.graph.nodes.get_mut(index) {
                node.x += dx;
                node.y += dy;
                affected.extend_from_slice(&self.incident[index]);
            }
        }
        affected.sort_unstable();
        affected.dedup();
        for index in affected {
            self.geometry[index] = EdgeGeometry::new(&self.graph.edges[index], &self.graph);
        }
        self.wake();
    }

    fn paint(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        self.canvas_size = [rect.width(), rect.height()];
        ui.allocate_rect(rect, egui::Sense::drag());
        let (pointer, events, any_down, scroll, pinch) = ctx.input(|i| {
            let events = pointer_frame(
                i.raw.events.iter().filter_map(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        ..
                    } => Some(if *pressed {
                        PointerEvent::Press {
                            position: [pos.x, pos.y],
                            button: *button as u8,
                        }
                    } else {
                        PointerEvent::Release {
                            position: [pos.x, pos.y],
                            button: *button as u8,
                        }
                    }),
                    egui::Event::PointerMoved(pos) => Some(PointerEvent::Move([pos.x, pos.y])),
                    _ => None,
                }),
                self.gesture_button,
            );
            let pointer = events
                .release
                .or(events.latest)
                .map(|p| pos2(p[0], p[1]))
                .or(i.pointer.interact_pos());
            (
                pointer,
                events,
                i.pointer.any_down(),
                i.smooth_scroll_delta,
                i.zoom_delta(),
            )
        });
        // A canceled pointer or lost window focus may end the gesture without a release event.
        if !any_down && events.press.is_none() && events.release.is_none() {
            self.gesture_anchor = None;
            self.gesture_button = None;
            self.dragging_node = None;
            self.reconnecting = false;
        }
        if let Some(pointer) = pointer {
            if rect.contains(pointer) {
                let old_zoom = self.viewport[2];
                let zoom = (old_zoom * pinch * (scroll.y * 0.002).exp()).clamp(0.01, 3.0);
                if (zoom - old_zoom).abs() > f32::EPSILON {
                    let relative = pointer - rect.min;
                    let world = (relative - vec2(self.viewport[0], self.viewport[1])) / old_zoom;
                    let translated = relative - world * zoom;
                    self.viewport = [translated.x, translated.y, zoom];
                }
            }
        }
        // Use the actual button-down coordinate, even if moves and release arrived before this frame.
        if let Some((press, button)) = events.press {
            let press = pos2(press[0], press[1]);
            if rect.contains(press) {
                self.gesture_button = Some(button);
                let world = (press - rect.min - vec2(self.viewport[0], self.viewport[1]))
                    / self.viewport[2];
                self.reconnecting = self.selected_edge.is_some_and(|index| {
                    let edge = &self.graph.edges[index];
                    let target = &self.graph.nodes[edge.target];
                    world.to_pos2().distance(pos2(
                        target.x,
                        target.y
                            + self.graph.pin_start
                            + edge.target_pin as f32 * self.graph.pin_step,
                    )) * self.viewport[2]
                        <= 10.0
                });
                self.dragging_node = if self.reconnecting {
                    None
                } else {
                    self.graph.nodes.iter().rposition(|n| {
                        Rect::from_min_size(
                            pos2(n.x, n.y),
                            vec2(self.graph.node_width, self.graph.node_height),
                        )
                        .contains(world.to_pos2())
                    })
                };
                if !self.reconnecting && self.dragging_node.is_none() {
                    let cursor = world.to_pos2();
                    let mut closest = 8.0 / self.viewport[2];
                    let mut selected = None;
                    for (index, geometry) in self.geometry.iter().enumerate() {
                        if !geometry.bounds.expand(closest).contains(cursor) {
                            continue;
                        }
                        for segment in geometry.points.windows(2) {
                            let delta = segment[1] - segment[0];
                            let t = ((cursor - segment[0]).dot(delta)
                                / delta.length_sq().max(0.001))
                            .clamp(0.0, 1.0);
                            let distance = cursor.distance(segment[0] + delta * t);
                            if distance < closest {
                                closest = distance;
                                selected = Some(index);
                            }
                        }
                    }
                    self.selected_edge = selected;
                }
                self.gesture_anchor = if self.reconnecting {
                    None
                } else {
                    let (start, scale) = if let Some(index) = self.dragging_node {
                        let node = &self.graph.nodes[index];
                        ([node.x, node.y], self.viewport[2])
                    } else {
                        ([self.viewport[0], self.viewport[1]], 1.0)
                    };
                    Some(DragAnchor {
                        press: [press.x, press.y],
                        start,
                        scale,
                    })
                };
            }
        }
        // Derive displacement from gesture start so frame coalescing cannot lose movement.
        // Apply this before clearing a released gesture to include the final release coordinate.
        if let (Some(anchor), Some(pointer)) = (self.gesture_anchor, pointer) {
            let target = anchor.position_at([pointer.x, pointer.y]);
            if let Some(index) = self.dragging_node {
                let node = &self.graph.nodes[index];
                let dx = target[0] - node.x;
                let dy = target[1] - node.y;
                if dx != 0.0 || dy != 0.0 {
                    self.move_nodes(&[index], dx, dy);
                    self.stats.drag_events += 1;
                    self.stats.latest_mutation_at_ms = now();
                }
            } else {
                self.viewport[0] = target[0];
                self.viewport[1] = target[1];
            }
        }
        if events.release.is_some() {
            if self.reconnecting {
                if let (Some(index), Some(pointer)) = (self.selected_edge, pointer) {
                    let world = ((pointer - rect.min - vec2(self.viewport[0], self.viewport[1]))
                        / self.viewport[2])
                        .to_pos2();
                    let edge = &self.graph.edges[index];
                    let mut candidate = None;
                    let mut nearest = 10.0 / self.viewport[2];
                    for (node_index, node) in self.graph.nodes.iter().enumerate() {
                        if node_index == edge.source {
                            continue;
                        }
                        for pin in 0..4 {
                            if (edge.kind == "execution") != (pin == 0) {
                                continue;
                            }
                            let distance = world.distance(pos2(
                                node.x,
                                node.y + self.graph.pin_start + pin as f32 * self.graph.pin_step,
                            ));
                            if distance < nearest {
                                nearest = distance;
                                candidate = Some((node_index, pin));
                            }
                        }
                    }
                    if let Some((target, pin)) = candidate {
                        let old_target = self.graph.edges[index].target;
                        self.incident[old_target].retain(|&i| i != index);
                        self.incident[target].push(index);
                        self.graph.edges[index].target = target;
                        self.graph.edges[index].target_pin = pin;
                        self.geometry[index] =
                            EdgeGeometry::new(&self.graph.edges[index], &self.graph);
                        self.stats.reconnect_count += 1;
                        self.stats.latest_mutation_at_ms = now();
                    }
                }
            }
            self.reconnecting = false;
            self.dragging_node = None;
            self.gesture_anchor = None;
            self.gesture_button = None;
        }

        let zoom = self.viewport[2];
        let detail = Detail::for_zoom(self.lod_enabled, zoom);
        let translation = rect.min.to_vec2() + vec2(self.viewport[0], self.viewport[1]);
        let transform = |p: Pos2| (p.to_vec2() * zoom + translation).to_pos2();
        let view = Rect::from_min_max(
            ((rect.min.to_vec2() - translation) / zoom).to_pos2(),
            ((rect.max.to_vec2() - translation) / zoom).to_pos2(),
        );
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, color(self.palette.background));
        let phase = ((now() - self.motion_started_at) / 3000.0).fract() as f32;
        let ease = phase * phase * (3.0 - 2.0 * phase);
        let mut rendered_edges = 0;
        let mut edge_segments_drawn = 0;
        for (index, (edge, geometry)) in self.graph.edges.iter().zip(&self.geometry).enumerate() {
            if !geometry.bounds.intersects(view) {
                continue;
            }
            rendered_edges += 1;
            if self.reconnecting && self.selected_edge == Some(index) {
                continue;
            }
            let curve =
                if lod::coarse_curve_allowed(detail.coarse_edges(), geometry.coarse_error, zoom) {
                    &geometry.coarse_points
                } else {
                    &geometry.points
                };
            edge_segments_drawn += curve.len() - 1;
            let points: Vec<_> = curve.iter().map(|&p| transform(p)).collect();
            if edge.kind == "execution" {
                painter.add(Shape::line(
                    points,
                    Stroke::new(2.0 * zoom, alpha(self.palette.pin_execution, 0.7)),
                ));
                if self.motion && !self.webkit {
                    let pulse = geometry.section(
                        ease * 1000.0,
                        ease * 1000.0 + 30.0 * (phase * 10.0).min(1.0).min((1.0 - phase) * 10.0),
                    );
                    if pulse.len() > 1 {
                        let opacity = (phase * 10.0).min(1.0).min((1.0 - phase) * 10.0);
                        painter.add(Shape::line(
                            pulse.into_iter().map(transform).collect(),
                            Stroke::new(
                                3.0 * zoom,
                                alpha(mix(self.palette.pin_execution, [255; 4], 0.3), opacity),
                            ),
                        ));
                    }
                }
            } else {
                let base = match edge.source_pin {
                    1 => self.palette.pin_string,
                    2 => self.palette.pin_integer,
                    3 => self.palette.pin_float,
                    _ => self.palette.pin_execution,
                };
                if detail.coarse_edges() {
                    painter.add(Shape::line(
                        points,
                        Stroke::new(1.5 * zoom, alpha(base, 0.35)),
                    ));
                } else {
                    painter.extend(Shape::dashed_line(
                        &points,
                        Stroke::new(1.5 * zoom, alpha(base, 0.35)),
                        1.5 * zoom,
                        4.0 * zoom,
                    ));
                    if !self.webkit {
                        painter.extend(Shape::dashed_line(
                            &points,
                            Stroke::new(zoom, alpha(mix(base, [255; 4], 0.3), 0.5)),
                            1.5 * zoom,
                            4.0 * zoom,
                        ));
                    }
                }
            }
        }
        if let Some(index) = self.selected_edge {
            let edge = &self.graph.edges[index];
            let target = &self.graph.nodes[edge.target];
            let anchor = transform(pos2(
                target.x,
                target.y + self.graph.pin_start + edge.target_pin as f32 * self.graph.pin_step,
            ));
            painter.circle_stroke(anchor, 7.0, Stroke::new(1.5, color(self.palette.primary)));
            if self.reconnecting {
                if let Some(pointer) = pointer {
                    let source = &self.graph.nodes[edge.source];
                    let source = transform(pos2(
                        source.x + self.graph.node_width,
                        source.y
                            + self.graph.pin_start
                            + edge.source_pin as f32 * self.graph.pin_step,
                    ));
                    let offset = ((pointer.x - source.x) * 0.5).abs().max(30.0);
                    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                        [
                            source,
                            source + vec2(offset, 0.0),
                            pointer - vec2(offset, 0.0),
                            pointer,
                        ],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(2.0, color(self.palette.primary)),
                    ));
                }
            }
        }
        let zoom_key = zoom.to_bits();
        if detail == Detail::Full && self.pin_text.as_ref().map(|(key, _)| *key) != Some(zoom_key) {
            self.pin_text = Some((
                zoom_key,
                self.graph
                    .pin_labels
                    .iter()
                    .map(|label| {
                        painter.layout_no_wrap(
                            label.clone(),
                            FontId::proportional(8.0 * zoom),
                            color(self.palette.foreground),
                        )
                    })
                    .collect(),
            ));
        }
        let pin_text = self
            .pin_text
            .as_ref()
            .map(|(_, labels)| labels.as_slice())
            .unwrap_or(&[]);
        let mut rendered_nodes = 0;
        let mut labels_drawn = 0;
        let mut pins_drawn = 0;
        for (index, node) in self.graph.nodes.iter_mut().enumerate() {
            let world_rect = Rect::from_min_size(
                pos2(node.x, node.y),
                vec2(self.graph.node_width, self.graph.node_height),
            );
            if !world_rect.expand(5.0).intersects(view) {
                continue;
            }
            rendered_nodes += 1;
            let node_rect =
                Rect::from_min_max(transform(world_rect.min), transform(world_rect.max));
            let tint = match node.kind.as_str() {
                "event" => self.palette.primary,
                "function" => [139, 92, 246, 255],
                "pure" => self.palette.tertiary,
                _ => self.palette.card,
            };
            if detail == Detail::Tiny {
                painter.rect_filled(node_rect, 0.0, color(mix(self.palette.card, tint, 0.65)));
                if self
                    .activity
                    .is_some_and(|tick| index % 100 == tick as usize % 100)
                {
                    painter.circle_filled(
                        node_rect.center(),
                        (2.0 * zoom).max(0.8),
                        Color32::from_rgb(34, 197, 94),
                    );
                }
                continue;
            }
            painter.rect(
                node_rect,
                6.0 * zoom,
                color(self.palette.card),
                Stroke::new(zoom, color(self.palette.border)),
                egui::StrokeKind::Inside,
            );
            let right_color = tint;
            let mut header = egui::Mesh::default();
            header.colored_vertex(node_rect.min + vec2(zoom, zoom), color(self.palette.card));
            header.colored_vertex(
                node_rect.min + vec2((self.graph.node_width - 1.0) * zoom, zoom),
                color(right_color),
            );
            header.colored_vertex(
                node_rect.min + vec2((self.graph.node_width - 1.0) * zoom, 16.0 * zoom),
                color(right_color),
            );
            header.colored_vertex(
                node_rect.min + vec2(zoom, 16.0 * zoom),
                color(self.palette.card),
            );
            header.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
            painter.add(header);
            let origin = node_rect.min;
            let point = |x, y| origin + vec2(x, y) * zoom;
            painter.line_segment(
                [point(1.0, 16.0), point(self.graph.node_width - 1.0, 16.0)],
                Stroke::new(zoom, color(self.palette.border)),
            );
            if detail == Detail::Full {
                let icon = Stroke::new(zoom * 0.5, color(self.palette.foreground));
                painter.add(Shape::line(
                    vec![
                        point(6.0 + 7.0 / 3.0, 4.0 + 11.0 / 3.0),
                        point(6.0 + 7.0 / 3.0, 4.0 + 15.0 / 3.0),
                        point(9.0, 4.0 + 17.0 / 3.0),
                        point(6.0 + 13.0 / 3.0, 4.0 + 17.0 / 3.0),
                    ],
                    icon,
                ));
                for (x, y) in [(7.0, 5.0), (6.0 + 13.0 / 3.0, 4.0 + 13.0 / 3.0)] {
                    painter.rect_stroke(
                        Rect::from_min_size(point(x, y), vec2(8.0 / 3.0, 8.0 / 3.0) * zoom),
                        2.0 / 3.0 * zoom,
                        icon,
                        egui::StrokeKind::Middle,
                    );
                }
            }
            if detail.title() {
                if node.title.as_ref().map(|(key, _)| *key) != Some(zoom_key) {
                    let family = if self.medium_font {
                        egui::FontFamily::Name("Inter Medium".into())
                    } else {
                        egui::FontFamily::Proportional
                    };
                    node.title = Some((
                        zoom_key,
                        painter.layout_no_wrap(
                            node.label.clone(),
                            FontId::new(10.0 * zoom, family),
                            color(self.palette.foreground),
                        ),
                    ));
                }
                let title = &node.title.as_ref().unwrap().1;
                painter
                    .with_clip_rect(Rect::from_min_max(point(18.0, 0.0), point(134.0, 16.0)))
                    .galley(
                        point(18.0, 8.0 - title.size().y / zoom * 0.5),
                        title.clone(),
                        color(self.palette.foreground),
                    );
                labels_drawn += 1;
            }
            if self
                .activity
                .is_some_and(|tick| index % 100 == tick as usize % 100)
            {
                painter.circle_filled(
                    point(140.0, 8.0),
                    2.0 * zoom,
                    Color32::from_rgb(34, 197, 94),
                );
            }
            if detail.pins() {
                for pin in 0..4 {
                    let y = self.graph.pin_start + pin as f32 * self.graph.pin_step;
                    let pin_color = color(match pin {
                        0 => self.palette.pin_execution,
                        1 => self.palette.pin_string,
                        2 => self.palette.pin_integer,
                        _ => self.palette.pin_float,
                    });
                    for x in [0.0, self.graph.node_width] {
                        pins_drawn += 1;
                        let center = point(x, y);
                        if pin == 0 {
                            painter.add(Shape::convex_polygon(
                                vec![
                                    center + vec2(0.0, -5.657) * zoom,
                                    center + vec2(5.657, 0.0) * zoom,
                                    center + vec2(0.0, 5.657) * zoom,
                                    center + vec2(-5.657, 0.0) * zoom,
                                ],
                                pin_color,
                                Stroke::NONE,
                            ));
                        } else {
                            painter.circle_filled(center, 3.5 * zoom, pin_color);
                        }
                    }
                    if detail == Detail::Full {
                        if let Some(galley) = pin_text.get(pin) {
                            let y = y - galley.size().y / zoom * 0.5;
                            painter.galley(
                                point(10.0, y),
                                galley.clone(),
                                color(self.palette.foreground),
                            );
                            painter.galley(
                                point(self.graph.node_width - 10.0 - galley.size().x / zoom, y),
                                galley.clone(),
                                color(self.palette.foreground),
                            );
                            labels_drawn += 2;
                        }
                    }
                }
            }
        }
        self.stats.rendered_nodes = rendered_nodes;
        self.stats.rendered_edges = rendered_edges;
        self.stats.viewport = self.viewport;
        self.stats.activity_tick = self.activity;
        self.stats.lod_enabled = self.lod_enabled;
        self.stats.lod_level = detail.name();
        self.stats.lod_details = detail.description();
        self.stats.labels_drawn = labels_drawn;
        self.stats.pins_drawn = pins_drawn;
        self.stats.edge_segments_drawn = edge_segments_drawn;
        if self.motion && !self.webkit {
            ctx.request_repaint();
        }
    }
}

struct App(Rc<RefCell<Board>>);

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let start = now();
        let mut board = self.0.borrow_mut();
        let css = board.canvas.get_bounding_client_rect();
        let dpr = web_sys::window().unwrap().device_pixel_ratio();
        let width = (css.width() * dpr).round().max(1.0) as u32;
        let height = (css.height() * dpr).round().max(1.0) as u32;
        // Browser DPR emulation can disagree with ResizeObserver's physical pixel size.
        // Keep the backing buffer consistent with eframe's reported pixels per point.
        if board.canvas.width() != width || board.canvas.height() != height {
            board.canvas.set_width(width);
            board.canvas.set_height(height);
            ui.ctx().request_repaint();
            return;
        }
        board.paint(ui);
        let elapsed = now() - start;
        board.stats.frames += 1;
        board.stats.paint_cpu_total_ms += elapsed;
        if board.stats.paint_cpu_ms.len() < 2048 {
            board.stats.paint_cpu_ms.push(elapsed);
        } else {
            let index = (board.stats.frames as usize - 1) % 2048;
            board.stats.paint_cpu_ms[index] = elapsed;
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.0.borrow().palette.background.map(|v| v as f32 / 255.0)
    }
}

fn now() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
    board: Rc<RefCell<Board>>,
}

#[wasm_bindgen]
impl WebHandle {
    pub fn set_viewport(&self, x: f32, y: f32, zoom: f32) {
        let mut board = self.board.borrow_mut();
        board.viewport = [x, y, zoom.clamp(0.01, 3.0)];
        board.wake();
    }

    pub fn set_motion(&self, enabled: bool) {
        let mut board = self.board.borrow_mut();
        if enabled && !board.motion {
            board.motion_started_at = now();
        }
        board.motion = enabled;
        board.wake();
    }

    pub fn set_activity(&self, tick: u32) {
        let mut board = self.board.borrow_mut();
        board.activity = (tick != u32::MAX).then_some(tick);
        board.wake();
    }

    pub fn set_lod(&self, enabled: bool) {
        let mut board = self.board.borrow_mut();
        board.lod_enabled = enabled;
        board.wake();
    }

    pub fn move_nodes(&self, indices_json: &str, dx: f32, dy: f32) -> Result<(), JsValue> {
        let indices: Vec<usize> = serde_json::from_str(indices_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.board.borrow_mut().move_nodes(&indices, dx, dy);
        Ok(())
    }

    pub fn set_font(&self, regular: &[u8]) {
        self.set_fonts(regular, &[]);
    }

    pub fn set_webkit(&self, enabled: bool) {
        let mut board = self.board.borrow_mut();
        board.webkit = enabled;
        board.wake();
    }

    pub fn set_fonts(&self, regular: &[u8], medium: &[u8]) {
        let mut board = self.board.borrow_mut();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".into(),
            egui::FontData::from_owned(regular.to_vec()).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Inter".into());
        if !medium.is_empty() {
            fonts.font_data.insert(
                "Inter Medium".into(),
                egui::FontData::from_owned(medium.to_vec()).into(),
            );
            fonts.families.insert(
                egui::FontFamily::Name("Inter Medium".into()),
                vec!["Inter Medium".into()],
            );
        }
        board.medium_font = !medium.is_empty();
        for node in &mut board.graph.nodes {
            node.title = None;
        }
        board.pin_text = None;
        if let Some(ctx) = &board.ctx {
            ctx.set_fonts(fonts);
        }
        board.wake();
    }

    pub fn stats(&self) -> String {
        serde_json::to_string(&self.board.borrow().stats).unwrap()
    }

    pub fn select_edge(&self, index: usize) {
        let mut board = self.board.borrow_mut();
        board.selected_edge = (index < board.graph.edges.len()).then_some(index);
        board.wake();
    }

    pub fn node_position(&self, index: usize) -> String {
        let board = self.board.borrow();
        board
            .graph
            .nodes
            .get(index)
            .map(|node| serde_json::json!({"x": node.x, "y": node.y}).to_string())
            .unwrap_or_else(|| "null".into())
    }

    pub fn edge_target(&self, index: usize) -> String {
        let board = self.board.borrow();
        board
            .graph
            .edges
            .get(index)
            .map(|edge| {
                serde_json::json!({"targetIndex": edge.target, "targetPin": edge.target_pin})
                    .to_string()
            })
            .unwrap_or_else(|| "null".into())
    }

    pub fn interaction_targets(&self) -> String {
        let board = self.board.borrow();
        let [x, y, zoom] = board.viewport;
        let [width, height] = board.canvas_size;
        let screen = |x1: f32, y1: f32| pos2(x + x1 * zoom, y + y1 * zoom);
        let center = pos2(width * 0.5, height * 0.5);
        let visible =
            |p: Pos2| p.x > 30.0 && p.y > 30.0 && p.x < width - 30.0 && p.y < height - 30.0;
        let node = board
            .graph
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let p = screen(node.x + board.graph.node_width * 0.5, node.y + 8.0);
                visible(p).then_some((index, p))
            })
            .min_by(|a, b| a.1.distance_sq(center).total_cmp(&b.1.distance_sq(center)));
        let Some((index, mut node_point)) = node else {
            return "null".into();
        };
        let reconnect = board.graph.edges.iter().enumerate().find(|(_, edge)| edge.target == index && edge.kind == "execution").and_then(|(edge_index, edge)| {
            let old_target = &board.graph.nodes[index];
            let from = screen(old_target.x, old_target.y + board.graph.pin_start);
            board.graph.nodes.iter().enumerate().filter(|(target, _)| *target != index && *target != edge.source).filter_map(|(target, node)| {
                let to = screen(node.x, node.y + board.graph.pin_start);
                visible(to).then_some((target, to))
            }).min_by(|a, b| a.1.distance_sq(from).total_cmp(&b.1.distance_sq(from))).map(|(target, to)| serde_json::json!({"edgeIndex": edge_index, "from": {"x": from.x,"y": from.y},"to": {"x":to.x,"y":to.y},"targetIndex": target,"targetPin": 0}))
        });
        let mut drag_index = index;
        if let Some(reconnect) = &reconnect {
            let edge_index = reconnect["edgeIndex"].as_u64().unwrap() as usize;
            let target = reconnect["targetIndex"].as_u64().unwrap() as usize;
            let edge = &board.graph.edges[edge_index];
            if let Some((alternative, point)) = board
                .graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != edge.source && *i != edge.target && *i != target)
                .filter_map(|(i, node)| {
                    let point = screen(node.x + board.graph.node_width * 0.5, node.y + 8.0);
                    visible(point).then_some((i, point))
                })
                .min_by(|a, b| a.1.distance_sq(center).total_cmp(&b.1.distance_sq(center)))
            {
                drag_index = alternative;
                node_point = point;
            }
        }
        serde_json::json!({"node":{"index":drag_index,"x":node_point.x,"y":node_point.y},"reconnect":reconnect}).to_string()
    }

    pub fn reset_stats(&self) {
        let mut board = self.board.borrow_mut();
        board.stats = Stats {
            node_count: board.graph.nodes.len(),
            edge_count: board.graph.edges.len(),
            viewport: board.viewport,
            ..Stats::default()
        };
    }

    pub fn destroy(&self) {
        self.runner.destroy();
    }
}

#[wasm_bindgen]
pub async fn start(
    canvas: web_sys::HtmlCanvasElement,
    fixture_json: &str,
    style_json: &str,
) -> Result<WebHandle, JsValue> {
    let graph: Fixture = serde_json::from_str(fixture_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let palette: Palette =
        serde_json::from_str(style_json).map_err(|error| JsValue::from_str(&error.to_string()))?;
    for edge in &graph.edges {
        if edge.source >= graph.nodes.len() || edge.target >= graph.nodes.len() {
            return Err(JsValue::from_str("Edge endpoint is outside the node array"));
        }
    }
    let geometry = graph
        .edges
        .iter()
        .map(|edge| EdgeGeometry::new(edge, &graph))
        .collect();
    let mut incident = vec![Vec::new(); graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        incident[edge.source].push(index);
        incident[edge.target].push(index);
    }
    let stats = Stats {
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        ..Stats::default()
    };
    let board = Rc::new(RefCell::new(Board {
        canvas: canvas.clone(),
        graph,
        palette,
        geometry,
        incident,
        viewport: [0.0, 0.0, 1.0],
        motion: false,
        motion_started_at: 0.0,
        webkit: false,
        lod_enabled: false,
        activity: None,
        stats,
        ctx: None,
        dragging_node: None,
        gesture_anchor: None,
        gesture_button: None,
        pin_text: None,
        medium_font: false,
        selected_edge: None,
        reconnecting: false,
        canvas_size: [0.0, 0.0],
    }));
    let app_board = board.clone();
    let runner = eframe::WebRunner::new();
    runner
        .start(
            canvas,
            eframe::WebOptions {
                renderer: eframe::Renderer::Glow,
                webgl_context_option: eframe::WebGlContextOption::WebGl2,
                dithering: false,
                ..Default::default()
            },
            Box::new(move |cc| {
                app_board.borrow_mut().ctx = Some(cc.egui_ctx.clone());
                Ok(Box::new(App(app_board)))
            }),
        )
        .await?;
    Ok(WebHandle { runner, board })
}
