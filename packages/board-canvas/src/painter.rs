use crate::geometry::{Curve, CurveKey, Detail, header_outline};
use crate::model::{
    ConnectionKind, Direction, Endpoint, Node, Overlay, PaintLayer, Palette, PositionUpdate, Rgba,
    Scene, SceneIndex,
};
use crate::text_cache::{TextCache, ZoomQuality};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Shape, Stroke, pos2, vec2};
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};
use wasm_bindgen::prelude::*;

const PULSE_DURATION_MS: f64 = 1_200.0;
const PULSE_FRAME_INTERVAL: Duration = Duration::from_millis(42);
const MAX_PULSE_EDGES: usize = 32;

fn color(value: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(value[0], value[1], value[2], value[3])
}

fn alpha(value: Rgba, opacity: f32) -> Color32 {
    color([
        value[0],
        value[1],
        value[2],
        (value[3] as f32 * opacity) as u8,
    ])
}

fn mix(a: Rgba, b: Rgba, amount: f32) -> Rgba {
    std::array::from_fn(|i| (a[i] as f32 * (1.0 - amount) + b[i] as f32 * amount) as u8)
}

fn error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

type NodeText = TextCache<Arc<egui::Galley>>;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    frames: u64,
    last_paint_cpu_ms: f64,
    paint_cpu_total_ms: f64,
    rendered_nodes: usize,
    rendered_edges: usize,
    node_count: usize,
    edge_count: usize,
    unresolved_edges: usize,
    geometry_rebuilds: u64,
    viewport: [f32; 3],
    lod_enabled: bool,
    lod_level: &'static str,
    labels_drawn: usize,
    text_layout_count: u64,
    pins_drawn: usize,
    edge_segments_drawn: usize,
    paint_layer: &'static str,
    activation_pulse: bool,
    pulse_edges_drawn: usize,
    pulse_count: u64,
}

struct Board {
    canvas: web_sys::HtmlCanvasElement,
    scene: Scene,
    index: SceneIndex,
    palette: Palette,
    curves: Vec<Option<Curve>>,
    text: Vec<NodeText>,
    text_zoom: ZoomQuality,
    overlay: Overlay,
    overlay_curve: Option<Curve>,
    viewport: [f32; 3],
    lod: bool,
    layer: PaintLayer,
    medium_font: bool,
    destroyed: bool,
    pulse_started: Option<f64>,
    ctx: Option<egui::Context>,
    stats: Stats,
}

impl Board {
    fn wake(&self) {
        if !self.destroyed
            && let Some(ctx) = &self.ctx
        {
            ctx.request_repaint();
        }
    }

    fn ensure_alive(&self) -> Result<(), JsValue> {
        if self.destroyed {
            Err(error("Board canvas has been destroyed"))
        } else {
            Ok(())
        }
    }

    fn clear_text(&mut self) {
        self.text = (0..self.scene.nodes.len())
            .map(|_| NodeText::default())
            .collect();
    }

    fn replace_scene(&mut self, scene: Scene, index: SceneIndex) {
        let mut previous: HashMap<_, _> = self
            .scene
            .edges
            .iter()
            .map(|edge| edge.id.clone())
            .zip(std::mem::take(&mut self.curves))
            .collect();
        self.curves = scene
            .edges
            .iter()
            .map(|edge| {
                let key = CurveKey::for_edge(edge, &scene, &index)?;
                if let Some(Some(curve)) = previous.remove(&edge.id)
                    && curve.key == key
                {
                    return Some(curve);
                }
                self.stats.geometry_rebuilds += 1;
                Some(Curve::new(key))
            })
            .collect();
        self.stats.node_count = scene.nodes.len();
        self.stats.edge_count = scene.edges.len();
        self.stats.unresolved_edges = self.curves.iter().filter(|curve| curve.is_none()).count();
        self.scene = scene;
        self.index = index;
        self.clear_text();
        self.wake();
    }

    fn paint(&mut self, ui: &mut egui::Ui) {
        let area = ui.max_rect();
        let painter = ui.painter().with_clip_rect(area);
        let [offset_x, offset_y, zoom] = self.viewport;
        let origin = area.min + vec2(offset_x, offset_y);
        let transform = |point: [f32; 2]| origin + vec2(point[0], point[1]) * zoom;
        let view = Rect::from_min_max(
            pos2(-offset_x / zoom, -offset_y / zoom),
            pos2(
                (area.width() - offset_x) / zoom,
                (area.height() - offset_y) / zoom,
            ),
        );
        let detail = Detail::for_zoom(self.lod, zoom);
        let text_settle_delay = self.text_zoom.settle_delay_ms(now());
        painter.rect_filled(area, 0.0, color(self.palette.background));

        // Each decorative sweep is finite and uses cached curves with a fixed edge budget.
        let pulse = self.pulse_started.and_then(|started| {
            let progress = ((now() - started) / PULSE_DURATION_MS) as f32;
            (progress < 1.0).then_some(progress.clamp(0.0, 1.0))
        });
        if pulse.is_none() {
            self.pulse_started = None;
        }
        let mut pulse_edges_drawn = 0;
        let pulse_stride = self.stats.rendered_edges.div_ceil(MAX_PULSE_EDGES).max(1);

        let mut rendered_edges = 0;
        let mut edge_segments_drawn = 0;
        for (edge, curve) in self.scene.edges.iter().zip(&self.curves) {
            if self.layer == PaintLayer::Nodes || edge.dom {
                continue;
            }
            let Some(curve) = curve else {
                continue;
            };
            let bounds = Rect::from_min_max(
                pos2(curve.bounds[0], curve.bounds[1]),
                pos2(curve.bounds[2], curve.bounds[3]),
            );
            if !bounds.intersects(view) {
                continue;
            }
            rendered_edges += 1;
            let points: Vec<_> = curve
                .samples(detail.simplify_edges() && !edge.selected, zoom)
                .iter()
                .map(|point| transform(*point))
                .collect();
            edge_segments_drawn += points.len().saturating_sub(1);
            let selected = edge.selected;
            let base = edge.color;
            match edge.kind {
                ConnectionKind::Execution => {
                    painter.add(Shape::line(
                        points,
                        Stroke::new(
                            if selected { 3.0 } else { 2.0 } * zoom,
                            alpha(base, if selected { 0.9 } else { 0.7 }),
                        ),
                    ));
                }
                ConnectionKind::Reference => {
                    if !detail.simplify_edges() {
                        painter.add(Shape::line(
                            points.clone(),
                            Stroke::new(
                                if selected { 8.0 } else { 5.0 } * zoom,
                                alpha(base, if selected { 0.08 } else { 0.04 }),
                            ),
                        ));
                    }
                    painter.add(Shape::line(
                        points.clone(),
                        Stroke::new(
                            if selected { 2.5 } else { 1.5 } * zoom,
                            alpha(base, if selected { 0.5 } else { 0.3 }),
                        ),
                    ));
                    if !detail.simplify_edges() {
                        painter.add(Shape::line(
                            points,
                            Stroke::new(
                                if selected { 1.2 } else { 0.8 } * zoom,
                                alpha(mix(base, [255; 4], 0.2), if selected { 0.7 } else { 0.4 }),
                            ),
                        ));
                    }
                }
                ConnectionKind::Data => {
                    if detail.simplify_edges() && !selected {
                        painter.add(Shape::line(
                            points,
                            Stroke::new(1.5 * zoom, alpha(base, 0.35)),
                        ));
                    } else {
                        painter.extend(Shape::dashed_line(
                            &points,
                            Stroke::new(
                                if selected { 2.5 } else { 1.5 } * zoom,
                                alpha(base, if selected { 0.5 } else { 0.35 }),
                            ),
                            1.5 * zoom,
                            4.0 * zoom,
                        ));
                        if !edge.lite {
                            painter.extend(Shape::dashed_line(
                                &points,
                                Stroke::new(
                                    if selected { 1.5 } else { 1.0 } * zoom,
                                    alpha(
                                        mix(base, [255; 4], 0.3),
                                        if selected { 0.8 } else { 0.5 },
                                    ),
                                ),
                                1.5 * zoom,
                                4.0 * zoom,
                            ));
                        }
                    }
                }
            }
            if let Some(progress) = pulse
                && pulse_edges_drawn < MAX_PULSE_EDGES
                && (rendered_edges + self.stats.pulse_count as usize).is_multiple_of(pulse_stride)
            {
                let opacity = (std::f32::consts::PI * progress).sin();
                let position = progress * (curve.points.len() - 1) as f32;
                let first = (position.floor() as usize).min(curve.points.len() - 2);
                let fraction = position - first as f32;
                let a = transform(curve.points[first]);
                let b = transform(curve.points[first + 1]);
                let head = a.lerp(b, fraction);
                // Clip each trail to the visible curve; off-screen edges add no shapes.
                if area.expand(12.0).contains(head) {
                    let mut trail: Vec<_> = curve.points[first.saturating_sub(2)..=first]
                        .iter()
                        .map(|point| transform(*point))
                        .collect();
                    trail.push(head);
                    let tint = mix(base, self.palette.primary, 0.6);
                    let width = (2.0 * zoom).clamp(1.0, 2.5);
                    painter.add(Shape::line(
                        trail.clone(),
                        Stroke::new(width + 5.0, alpha(tint, opacity * 0.10)),
                    ));
                    painter.add(Shape::line(
                        trail,
                        Stroke::new(width, alpha(tint, opacity * 0.65)),
                    ));
                    painter.circle_filled(head, width * 1.1, alpha(tint, opacity * 0.85));
                    pulse_edges_drawn += 1;
                }
            }
        }
        self.stats.activation_pulse = pulse.is_some() && rendered_edges > 0;
        self.stats.pulse_edges_drawn = pulse_edges_drawn;
        if self.stats.activation_pulse {
            // egui subtracts its frame prediction; compensate to retain the idle interval.
            let prediction = Duration::try_from_secs_f32(ui.input(|input| input.predicted_dt))
                .unwrap_or_default();
            ui.ctx()
                .request_repaint_after(PULSE_FRAME_INTERVAL + prediction);
        } else if rendered_edges == 0 {
            self.pulse_started = None;
        }

        let mut rendered_nodes = 0;
        let mut labels_drawn = 0;
        let mut text_layouts = 0;
        let mut pins_drawn = 0;
        for (node, text) in self.scene.nodes.iter().zip(&mut self.text) {
            if self.layer == PaintLayer::Edges || node.dom {
                continue;
            }
            let bounds = Rect::from_min_size(pos2(node.x, node.y), vec2(node.width, node.height));
            if !bounds.expand(6.0).intersects(view) {
                continue;
            }
            rendered_nodes += 1;
            let rect = Rect::from_min_max(
                transform([node.x, node.y]),
                transform([node.x + node.width, node.y + node.height]),
            );
            // Keep selected content legible when the host is not rendering a DOM editor.
            let node_detail = if node.selected { Detail::Full } else { detail };
            let (labels, pins, layouts) = paint_node(
                &painter,
                node,
                rect,
                zoom,
                text_settle_delay.is_none(),
                node_detail,
                &self.palette,
                text,
                self.medium_font,
            );
            labels_drawn += labels;
            pins_drawn += pins;
            text_layouts += layouts;
        }
        if labels_drawn > 0
            && let Some(delay_ms) = text_settle_delay
        {
            // Restore exact font hinting once zoom stops, without keeping idle frames alive.
            let prediction = Duration::try_from_secs_f32(ui.input(|input| input.predicted_dt))
                .unwrap_or_default();
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(delay_ms / 1000.0) + prediction);
        }

        if self.layer != PaintLayer::Edges {
            if let Some(rect) = self.overlay.selection_rect {
                let a = transform([rect[0], rect[1]]);
                let b = transform([rect[0] + rect[2], rect[1] + rect[3]]);
                painter.rect(
                    Rect::from_two_pos(a, b),
                    0.0,
                    alpha(self.palette.primary, 0.1),
                    Stroke::new(1.0, alpha(self.palette.primary, 0.8)),
                    egui::StrokeKind::Inside,
                );
            }
            if let (Some(connection), Some(curve)) = (&self.overlay.connection, &self.overlay_curve)
            {
                let tint = if connection.valid == Some(false) {
                    [239, 68, 68, 255]
                } else {
                    connection.color
                };
                painter.add(Shape::line(
                    curve.points.iter().map(|point| transform(*point)).collect(),
                    Stroke::new((2.0 * zoom).max(1.0), color(tint)),
                ));
            }
            if let Some(hovered) = &self.overlay.hovered_pin {
                let endpoint = Endpoint {
                    node_id: hovered.node_id.clone(),
                    pin_id: hovered.pin_id.clone(),
                };
                if let Some((node, pin)) = self.index.endpoint(&self.scene, &endpoint) {
                    let center = transform([node.x + pin.x, node.y + pin.y]);
                    let tint = if hovered.valid {
                        self.palette.primary
                    } else {
                        [239, 68, 68, 255]
                    };
                    painter.circle_stroke(
                        center,
                        (7.0 * zoom).max(5.0),
                        Stroke::new(1.5, color(tint)),
                    );
                }
            }
        }

        self.stats.rendered_nodes = rendered_nodes;
        self.stats.rendered_edges = rendered_edges;
        self.stats.viewport = self.viewport;
        self.stats.lod_enabled = self.lod;
        self.stats.lod_level = detail.name();
        self.stats.labels_drawn = labels_drawn;
        self.stats.text_layout_count += text_layouts;
        self.stats.pins_drawn = pins_drawn;
        self.stats.edge_segments_drawn = edge_segments_drawn;
        self.stats.paint_layer = self.layer.name();
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_node(
    painter: &egui::Painter,
    node: &Node,
    rect: Rect,
    zoom: f32,
    text_settled: bool,
    detail: Detail,
    palette: &Palette,
    text: &mut NodeText,
    medium_font: bool,
) -> (usize, usize, u64) {
    let opacity = if node.dimmed { 0.45 } else { 1.0 };
    let foreground = if node.dimmed {
        palette.muted
    } else {
        palette.foreground
    };
    let tint = match node.kind.as_str() {
        "event" => palette.primary,
        "function" => [139, 92, 246, 255],
        "pure" => palette.tertiary,
        _ => palette.card,
    };
    if node.kind == "reroute" {
        let tint = node.pins.first().map_or(palette.border, |pin| pin.color);
        painter.rect(
            rect,
            node.corner_radius() * zoom,
            alpha(if node.selected { tint } else { palette.card }, opacity),
            Stroke::new(zoom, alpha(tint, opacity)),
            egui::StrokeKind::Inside,
        );
        let mut pins = 0;
        if matches!(detail, Detail::Full | Detail::Compact) {
            for pin in &node.pins {
                // Reroutes hide execution diamonds and retain colored data handles.
                if pin.kind != ConnectionKind::Execution {
                    paint_pin(
                        painter,
                        rect.min + vec2(pin.x, pin.y) * zoom,
                        zoom,
                        pin.kind,
                        pin.color,
                        opacity,
                        detail == Detail::Full,
                    );
                    pins += 1;
                }
            }
        }
        return (0, pins, 0);
    }
    if detail == Detail::Tiny {
        painter.rect_filled(rect, 0.0, alpha(mix(palette.card, tint, 0.65), opacity));
        return (0, 0, 0);
    }
    let point = |x, y| rect.min + vec2(x, y) * zoom;
    let border = if node.selected {
        palette.primary
    } else {
        palette.border
    };
    painter.rect(
        rect,
        node.corner_radius() * zoom,
        alpha(palette.card, opacity),
        Stroke::new(
            if node.selected { 1.5 } else { 1.0 } * zoom,
            alpha(border, opacity),
        ),
        egui::StrokeKind::Inside,
    );
    let header_height = node.header_height.min(node.height);
    let mut header = egui::Mesh::default();
    let header_width = (node.width - 2.0).max(0.0);
    for [x, y] in header_outline(
        header_width,
        (header_height - 1.0).max(0.0),
        (node.corner_radius() - 1.0).max(0.0),
    ) {
        header.colored_vertex(
            point(x + 1.0, y + 1.0),
            alpha(mix(palette.card, tint, x / header_width.max(1.0)), opacity),
        );
    }
    for index in 2..header.vertices.len() as u32 {
        header.add_triangle(0, index - 1, index);
    }
    painter.add(header);
    painter.line_segment(
        [
            point(1.0, header_height),
            point(node.width - 1.0, header_height),
        ],
        Stroke::new(zoom, alpha(palette.border, opacity)),
    );

    let mut labels = 0;
    let mut pins = 0;
    let mut text_layouts = 0;
    let raster_zoom = if detail.labels() {
        text.prepare(zoom, painter.ctx().pixels_per_point(), text_settled)
    } else {
        zoom
    };
    let text_scale = zoom / raster_zoom;
    if detail.labels() {
        let title = text.title.get_or_insert_with(|| {
            text_layouts += 1;
            let family = if medium_font {
                egui::FontFamily::Name("Inter Medium".into())
            } else {
                egui::FontFamily::Proportional
            };
            layout_label(
                painter,
                node.label.clone(),
                FontId::new(node.label_font_size * raster_zoom, family),
                alpha(foreground, opacity),
                (node.width - 24.0).max(0.0) * raster_zoom,
            )
        });
        let title = text
            .scaled_title
            .get_or_insert_with(|| scale_galley(title, text_scale));
        painter
            .with_clip_rect(painter.clip_rect().intersect(Rect::from_min_max(
                point(18.0, 0.0),
                point((node.width - 6.0).max(18.0), header_height),
            )))
            .galley(
                point(18.0, header_height * 0.5 - title.size().y / zoom * 0.5),
                title.clone(),
                alpha(foreground, opacity),
            );
        labels += 1;
    }
    if detail == Detail::Full {
        let stroke = Stroke::new(0.5 * zoom, alpha(foreground, opacity));
        painter.add(Shape::line(
            vec![
                point(8.33, 7.67),
                point(8.33, 9.0),
                point(9.0, 9.67),
                point(10.33, 9.67),
            ],
            stroke,
        ));
        for (x, y) in [(7.0, 5.0), (10.33, 8.33)] {
            painter.rect_stroke(
                Rect::from_min_size(point(x, y), vec2(2.67, 2.67) * zoom),
                0.67 * zoom,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        if text.pins.len() != node.pins.len() {
            text_layouts += node.pins.len() as u64;
            text.pins = node
                .pins
                .iter()
                .map(|pin| {
                    layout_label(
                        painter,
                        pin.label.clone(),
                        FontId::proportional(pin.font_size * raster_zoom),
                        alpha(foreground, opacity),
                        pin.label_max_width
                            .unwrap_or((node.width * 0.5 - 12.0).max(0.0))
                            * raster_zoom,
                    )
                })
                .collect();
        }
        if text.scaled_pins.len() != text.pins.len() {
            text.scaled_pins = text
                .pins
                .iter()
                .map(|galley| scale_galley(galley, text_scale))
                .collect();
        }
    }
    if matches!(detail, Detail::Full | Detail::Compact) {
        for (index, pin) in node.pins.iter().enumerate() {
            let center = point(pin.x, pin.y);
            paint_pin(
                painter,
                center,
                zoom,
                pin.kind,
                pin.color,
                opacity,
                detail == Detail::Full,
            );
            pins += 1;
            if detail == Detail::Full {
                let galley = &text.scaled_pins[index];
                let available = pin
                    .label_max_width
                    .unwrap_or((node.width * 0.5 - 12.0).max(0.0));
                let (left, right, label_x) = match pin.direction {
                    Direction::Input => (pin.x + 10.0, pin.x + 10.0 + available, pin.x + 10.0),
                    Direction::Output => (
                        pin.x - 10.0 - available,
                        pin.x - 10.0,
                        pin.x - 10.0 - galley.size().x / zoom,
                    ),
                };
                let clip = Rect::from_min_max(
                    point(left.max(0.0), pin.y - 8.0),
                    point(right.min(node.width), pin.y + 8.0),
                );
                painter
                    .with_clip_rect(painter.clip_rect().intersect(clip))
                    .galley(
                        point(label_x, pin.y - galley.size().y / zoom * 0.5),
                        galley.clone(),
                        alpha(foreground, opacity),
                    );
                labels += 1;
            }
        }
    }
    (labels, pins, text_layouts)
}

fn scale_galley(galley: &Arc<egui::Galley>, scale: f32) -> Arc<egui::Galley> {
    if scale == 1.0 {
        return galley.clone();
    }
    let mut shape = egui::epaint::TextShape::new(Pos2::ZERO, galley.clone(), Color32::PLACEHOLDER);
    shape.transform(egui::emath::TSTransform::from_scaling(scale));
    shape.galley
}

fn layout_label(
    painter: &egui::Painter,
    label: String,
    font: FontId,
    color: Color32,
    width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(label, font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    painter.layout_job(job)
}

fn paint_pin(
    painter: &egui::Painter,
    center: Pos2,
    zoom: f32,
    kind: ConnectionKind,
    base: Rgba,
    opacity: f32,
    glossy: bool,
) {
    let radius = match kind {
        ConnectionKind::Execution => 5.657,
        ConnectionKind::Data => 3.5,
        ConnectionKind::Reference => 6.0,
    };
    let tint = alpha(base, opacity);
    if !glossy {
        match kind {
            ConnectionKind::Data => {
                painter.circle_filled(center, radius * zoom, tint);
            }
            ConnectionKind::Reference => {
                painter.rect_filled(
                    Rect::from_center_size(center, vec2(12.0, 12.0) * zoom),
                    2.0 * zoom,
                    tint,
                );
            }
            ConnectionKind::Execution => {
                painter.add(Shape::convex_polygon(
                    vec![
                        center + vec2(0.0, -radius) * zoom,
                        center + vec2(radius, 0.0) * zoom,
                        center + vec2(0.0, radius) * zoom,
                        center + vec2(-radius, 0.0) * zoom,
                    ],
                    tint,
                    Stroke::NONE,
                ));
            }
        }
        return;
    }
    if kind == ConnectionKind::Reference {
        painter.rect_filled(
            Rect::from_center_size(center, vec2(12.0, 12.0) * zoom),
            2.0 * zoom,
            tint,
        );
        painter.rect_stroke(
            Rect::from_center_size(center, vec2(10.0, 10.0) * zoom),
            zoom,
            Stroke::new(zoom, alpha(mix(base, [255; 4], 0.12), opacity)),
            egui::StrokeKind::Inside,
        );
        return;
    }
    // Vertex colors reproduce the small glossy pin without a CSS filter or texture readback.
    let segments = if kind == ConnectionKind::Execution {
        4
    } else {
        20
    };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(
        center - vec2(radius * 0.3, radius * 0.3) * zoom,
        alpha(mix(base, [255; 4], 0.2), opacity),
    );
    for index in 0..segments {
        let angle =
            index as f32 * std::f32::consts::TAU / segments as f32 - std::f32::consts::FRAC_PI_2;
        mesh.colored_vertex(
            center + vec2(angle.cos(), angle.sin()) * radius * zoom,
            tint,
        );
    }
    for index in 0..segments {
        mesh.add_triangle(0, index + 1, (index + 1) % segments + 1);
    }
    painter.add(mesh);
    if kind == ConnectionKind::Data {
        painter.circle_stroke(center, radius * zoom, Stroke::new(0.6 * zoom, tint));
    }
}

struct App(Rc<RefCell<Board>>);

impl eframe::App for App {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        // React owns gestures, focus navigation and keyboard shortcuts.
        input.events.clear();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let started = now();
        let mut board = self.0.borrow_mut();
        let bounds = board.canvas.get_bounding_client_rect();
        let dpr = web_sys::window()
            .map(|window| window.device_pixel_ratio())
            .unwrap_or(1.0);
        let width = (bounds.width() * dpr).round().max(1.0) as u32;
        let height = (bounds.height() * dpr).round().max(1.0) as u32;
        // Keep eframe's points and canvas backing pixels aligned after DPR changes.
        if board.canvas.width() != width || board.canvas.height() != height {
            board.canvas.set_width(width);
            board.canvas.set_height(height);
            ui.ctx().request_repaint();
            return;
        }
        board.paint(ui);
        let elapsed = now() - started;
        board.stats.frames += 1;
        board.stats.last_paint_cpu_ms = elapsed;
        board.stats.paint_cpu_total_ms += elapsed;
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // WebGL composites premultiplied pixels. The background shape applies
        // palette alpha once; a colored transparent clear would leak RGB.
        [0.0; 4]
    }
}

fn now() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

#[wasm_bindgen]
pub struct Handle {
    runner: eframe::WebRunner,
    board: Rc<RefCell<Board>>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[wasm_bindgen]
impl Handle {
    pub fn set_scene(&self, scene_json: &str) -> Result<(), JsValue> {
        let scene: Scene = serde_json::from_str(scene_json).map_err(error)?;
        let index = SceneIndex::new(&scene).map_err(error)?;
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        board.replace_scene(scene, index);
        Ok(())
    }

    pub fn set_positions(&self, positions_json: &str) -> Result<(), JsValue> {
        let positions: Vec<PositionUpdate> = serde_json::from_str(positions_json).map_err(error)?;
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        let Board { scene, index, .. } = &mut *board;
        let affected = index.apply_positions(scene, &positions).map_err(error)?;
        for index in affected {
            board.curves[index] =
                CurveKey::for_edge(&board.scene.edges[index], &board.scene, &board.index)
                    .map(Curve::new);
            board.stats.geometry_rebuilds += 1;
        }
        board.wake();
        Ok(())
    }

    pub fn set_viewport(&self, x: f32, y: f32, zoom: f32) -> Result<(), JsValue> {
        if !x.is_finite() || !y.is_finite() || !zoom.is_finite() || zoom <= 0.0 {
            return Err(error(
                "Viewport coordinates must be finite and zoom must be positive",
            ));
        }
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        board.text_zoom.update(zoom, now());
        if board.viewport != [x, y, zoom] {
            board.viewport = [x, y, zoom];
        }
        // The host also resubmits the viewport when a hidden canvas becomes drawable.
        board.wake();
        Ok(())
    }

    pub fn set_overlay(&self, overlay_json: &str) -> Result<(), JsValue> {
        let overlay: Overlay = serde_json::from_str(overlay_json).map_err(error)?;
        overlay.validate().map_err(error)?;
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        board.overlay_curve = overlay.connection.as_ref().map(|connection| {
            Curve::new(CurveKey {
                from: connection.from,
                to: connection.to,
                from_direction: connection.from_direction,
                to_direction: match connection.from_direction {
                    Direction::Input => Direction::Output,
                    Direction::Output => Direction::Input,
                },
                from_vertical: connection.kind == Some(ConnectionKind::Reference),
                to_vertical: connection.kind == Some(ConnectionKind::Reference),
            })
        });
        board.overlay = overlay;
        board.wake();
        Ok(())
    }

    pub fn set_palette(&self, palette_json: &str) -> Result<(), JsValue> {
        let palette = serde_json::from_str(palette_json).map_err(error)?;
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        board.palette = palette;
        board.clear_text();
        board.wake();
        Ok(())
    }

    pub fn set_lod(&self, enabled: bool) -> Result<(), JsValue> {
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        if board.lod != enabled {
            board.lod = enabled;
            board.wake();
        }
        Ok(())
    }

    pub fn pulse(&self) -> Result<(), JsValue> {
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        if board.layer != PaintLayer::Nodes && !board.scene.edges.is_empty() {
            board.pulse_started = Some(now());
            board.stats.pulse_count += 1;
            board.wake();
        }
        Ok(())
    }

    pub fn stop_pulse(&self) -> Result<(), JsValue> {
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        if board.pulse_started.take().is_some() {
            board.stats.activation_pulse = false;
            board.stats.pulse_edges_drawn = 0;
            board.wake();
        }
        Ok(())
    }

    /// Separate transparent canvases preserve DOM comments between edges and nodes.
    pub fn set_layer(&self, layer: &str) -> Result<(), JsValue> {
        let layer = PaintLayer::parse(layer).map_err(error)?;
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
        if board.layer != layer {
            board.layer = layer;
            board.wake();
        }
        Ok(())
    }

    pub fn set_fonts(&self, regular: &[u8], medium: &[u8]) -> Result<(), JsValue> {
        if regular.is_empty() {
            return Err(error("Regular font data must not be empty"));
        }
        let mut board = self.board.borrow_mut();
        board.ensure_alive()?;
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
                vec!["Inter Medium".into(), "Inter".into()],
            );
        }
        if let Some(ctx) = &board.ctx {
            ctx.set_fonts(fonts);
        }
        board.medium_font = !medium.is_empty();
        board.clear_text();
        board.wake();
        Ok(())
    }

    pub fn stats(&self) -> String {
        serde_json::to_string(&self.board.borrow().stats).expect("Finite board statistics")
    }

    pub fn destroy(&self) {
        let mut board = self.board.borrow_mut();
        if !board.destroyed {
            board.destroyed = true;
            board.ctx = None;
            board.curves = Vec::new();
            board.text = Vec::new();
            board.overlay = Overlay::default();
            board.overlay_curve = None;
            board.scene = Scene::default();
            board.index.nodes = HashMap::new();
            board.index.pins = Vec::new();
            board.index.incident = Vec::new();
            drop(board);
            self.runner.destroy();
        }
    }
}

#[wasm_bindgen]
/// The host owns input on a separate focusable surface. Give this canvas
/// `pointer-events: none` and `tabIndex = -1` to bypass eframe's DOM input handlers.
pub async fn start(
    canvas: web_sys::HtmlCanvasElement,
    scene_json: &str,
    palette_json: &str,
) -> Result<Handle, JsValue> {
    let tab_index = canvas.tab_index();
    let previous_sibling = canvas.next_element_sibling();
    let document = canvas.owner_document();
    let previous_focus = document
        .as_ref()
        .and_then(|document| document.active_element());
    let scene: Scene = serde_json::from_str(scene_json).map_err(error)?;
    let index = SceneIndex::new(&scene).map_err(error)?;
    let palette = serde_json::from_str(palette_json).map_err(error)?;
    let curves: Vec<_> = scene
        .edges
        .iter()
        .map(|edge| CurveKey::for_edge(edge, &scene, &index).map(Curve::new))
        .collect();
    let stats = Stats {
        node_count: scene.nodes.len(),
        edge_count: scene.edges.len(),
        unresolved_edges: curves.iter().filter(|curve| curve.is_none()).count(),
        geometry_rebuilds: curves.iter().filter(|curve| curve.is_some()).count() as u64,
        viewport: [0.0, 0.0, 1.0],
        lod_level: "full",
        paint_layer: "all",
        ..Stats::default()
    };
    let text = (0..scene.nodes.len())
        .map(|_| NodeText::default())
        .collect();
    let board = Rc::new(RefCell::new(Board {
        canvas: canvas.clone(),
        scene,
        index,
        palette,
        curves,
        text,
        text_zoom: ZoomQuality::new(1.0),
        overlay: Overlay::default(),
        overlay_curve: None,
        viewport: [0.0, 0.0, 1.0],
        lod: false,
        layer: PaintLayer::All,
        medium_font: false,
        destroyed: false,
        pulse_started: None,
        ctx: None,
        stats,
    }));
    let app_board = board.clone();
    let runner = eframe::WebRunner::new();
    let result = runner
        .start(
            canvas.clone(),
            eframe::WebOptions {
                renderer: eframe::Renderer::Glow,
                webgl_context_option: eframe::WebGlContextOption::WebGl2,
                dithering: false,
                should_stop_propagation: Box::new(|_| false),
                should_prevent_default: Box::new(|_| false),
                ..Default::default()
            },
            Box::new(move |cc| {
                app_board.borrow_mut().ctx = Some(cc.egui_ctx.clone());
                Ok(Box::new(App(app_board)))
            }),
        )
        .await;
    canvas.set_tab_index(tab_index);
    // eframe 0.36 inserts and focuses a hidden input immediately after the canvas.
    // This painter has no text editing; keep the host's DOM editor and tab order intact.
    if let Some(agent) = canvas
        .next_element_sibling()
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        && !previous_sibling
            .as_ref()
            .is_some_and(|previous| previous.is_same_node(Some(&agent)))
        && agent.get_attribute("autocapitalize").as_deref() == Some("off")
        && agent.style().get_property_value("width").ok().as_deref() == Some("1px")
    {
        let restore_focus = document
            .as_ref()
            .and_then(|document| document.active_element())
            .is_some_and(|active| {
                active.is_same_node(Some(&agent)) || active.is_same_node(Some(&canvas))
            });
        agent.remove();
        if restore_focus
            && let Some(previous) =
                previous_focus.and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = previous.focus_with_options(&options);
        }
    }
    if let Err(cause) = result {
        runner.destroy();
        return Err(cause);
    }
    Ok(Handle { runner, board })
}
