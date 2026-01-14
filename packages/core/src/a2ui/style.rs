use flow_like_types::{
    json::{Deserialize, Serialize},
    proto,
};
use schemars::JsonSchema;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::fmt;

/// Helper to deserialize either a string or an object with a value field
fn deserialize_string_or_value<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrValueVisitor;

    impl<'de> Visitor<'de> for StringOrValueVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an object with a 'value' field")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v)
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut value: Option<String> = None;
            while let Some(key) = map.next_key::<String>()? {
                if key == "value" {
                    value = Some(map.next_value()?);
                } else {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
            value.ok_or_else(|| de::Error::missing_field("value"))
        }
    }

    deserializer.deserialize_any(StringOrValueVisitor)
}

/// Overflow behavior
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl From<proto::Overflow> for Overflow {
    fn from(proto: proto::Overflow) -> Self {
        match proto {
            proto::Overflow::Visible => Self::Visible,
            proto::Overflow::Hidden => Self::Hidden,
            proto::Overflow::Scroll => Self::Scroll,
            proto::Overflow::Auto => Self::Auto,
        }
    }
}

impl From<Overflow> for proto::Overflow {
    fn from(value: Overflow) -> Self {
        match value {
            Overflow::Visible => proto::Overflow::Visible,
            Overflow::Hidden => proto::Overflow::Hidden,
            Overflow::Scroll => proto::Overflow::Scroll,
            Overflow::Auto => proto::Overflow::Auto,
        }
    }
}

/// Background type
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Background {
    Color(String),
    Gradient(Gradient),
    Image(BackgroundImage),
    Blur(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImage {
    pub url: super::BoundValue,
    pub size: Option<String>,
    pub position: Option<String>,
    pub repeat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Gradient {
    pub gradient_type: String,
    pub direction: Option<String>,
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GradientStop {
    pub color: String,
    pub position: f32,
}

/// Border styling
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Border {
    pub width: Option<String>,
    pub style: Option<String>,
    pub color: Option<String>,
    pub radius: Option<String>,
}

impl Border {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn with_radius(mut self, radius: impl Into<String>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    pub fn to_css(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref width) = self.width {
            parts.push(format!("border-width: {};", width));
        }
        if let Some(ref style) = self.style {
            parts.push(format!("border-style: {};", style));
        }
        if let Some(ref color) = self.color {
            parts.push(format!("border-color: {};", color));
        }
        if let Some(ref radius) = self.radius {
            parts.push(format!("border-radius: {};", radius));
        }

        parts.join(" ")
    }
}

/// Shadow styling
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Shadow {
    pub box_shadows: Vec<String>,
    pub text_shadow: Option<String>,
}

impl Shadow {
    pub fn to_css(&self) -> String {
        let mut parts = Vec::new();

        if !self.box_shadows.is_empty() {
            parts.push(format!("box-shadow: {};", self.box_shadows.join(", ")));
        }
        if let Some(ref text) = self.text_shadow {
            parts.push(format!("text-shadow: {};", text));
        }

        parts.join(" ")
    }
}

/// Spacing (padding/margin) - accepts both "20px" and { "value": "20px" }
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct Spacing {
    pub value: String,
}

impl<'de> Deserialize<'de> for Spacing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = deserialize_string_or_value(deserializer)?;
        Ok(Spacing { value })
    }
}

impl Spacing {
    pub fn new(value: impl Into<String>) -> Self {
        Self { value: value.into() }
    }
}

impl From<&str> for Spacing {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Size value - accepts both "20px" and { "value": "20px" }
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct Size {
    pub value: String,
}

impl<'de> Deserialize<'de> for Size {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = deserialize_string_or_value(deserializer)?;
        Ok(Size { value })
    }
}

impl Size {
    pub fn new(value: impl Into<String>) -> Self {
        Self { value: value.into() }
    }

    pub fn px(value: i32) -> Self {
        Self::new(format!("{}px", value))
    }

    pub fn percent(value: i32) -> Self {
        Self::new(format!("{}%", value))
    }

    pub fn auto() -> Self {
        Self::new("auto")
    }

    pub fn full() -> Self {
        Self::new("100%")
    }
}

/// Position styling
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub top: Option<String>,
    pub right: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
    pub position_type: String,
}

impl Position {
    pub fn absolute() -> Self {
        Self {
            position_type: "absolute".to_string(),
            ..Default::default()
        }
    }

    pub fn relative() -> Self {
        Self {
            position_type: "relative".to_string(),
            ..Default::default()
        }
    }

    pub fn fixed() -> Self {
        Self {
            position_type: "fixed".to_string(),
            ..Default::default()
        }
    }

    pub fn with_top(mut self, top: impl Into<String>) -> Self {
        self.top = Some(top.into());
        self
    }

    pub fn with_right(mut self, right: impl Into<String>) -> Self {
        self.right = Some(right.into());
        self
    }

    pub fn with_bottom(mut self, bottom: impl Into<String>) -> Self {
        self.bottom = Some(bottom.into());
        self
    }

    pub fn with_left(mut self, left: impl Into<String>) -> Self {
        self.left = Some(left.into());
        self
    }

    pub fn to_css(&self) -> String {
        let mut parts = vec![format!("position: {};", self.position_type)];

        if let Some(ref top) = self.top {
            parts.push(format!("top: {};", top));
        }
        if let Some(ref right) = self.right {
            parts.push(format!("right: {};", right));
        }
        if let Some(ref bottom) = self.bottom {
            parts.push(format!("bottom: {};", bottom));
        }
        if let Some(ref left) = self.left {
            parts.push(format!("left: {};", left));
        }

        parts.join(" ")
    }
}

/// Transform styling
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub translate: Option<String>,
    pub rotate: Option<f32>,
    pub scale: Option<String>,
    pub transform_origin: Option<String>,
    pub skew: Option<String>,
}

impl Transform {
    pub fn to_css(&self) -> String {
        let mut transforms = Vec::new();

        if let Some(ref t) = self.translate {
            transforms.push(format!("translate({})", t));
        }
        if let Some(r) = self.rotate {
            transforms.push(format!("rotate({}deg)", r));
        }
        if let Some(ref s) = self.scale {
            transforms.push(format!("scale({})", s));
        }
        if let Some(ref sk) = self.skew {
            transforms.push(format!("skew({})", sk));
        }

        if transforms.is_empty() {
            return String::new();
        }

        let mut result = format!("transform: {};", transforms.join(" "));
        if let Some(ref origin) = self.transform_origin {
            result.push_str(&format!(" transform-origin: {};", origin));
        }

        result
    }
}

/// Breakpoint style overrides
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointStyle {
    pub class_name: Option<String>,
    pub display: Option<String>,
    pub flex_direction: Option<String>,
    pub justify_content: Option<String>,
    pub align_items: Option<String>,
    pub gap: Option<String>,
    pub grid_cols: Option<i32>,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub padding: Option<Spacing>,
    pub margin: Option<Spacing>,
    pub hidden: Option<bool>,
    pub font_size: Option<String>,
    pub text_align: Option<String>,
    pub order: Option<i32>,
}

/// Responsive overrides for different breakpoints
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ResponsiveOverrides {
    pub sm: Option<BreakpointStyle>,
    pub md: Option<BreakpointStyle>,
    pub lg: Option<BreakpointStyle>,
    pub xl: Option<BreakpointStyle>,
    pub xxl: Option<BreakpointStyle>,
}

/// Complete style definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Style {
    pub class_name: Option<String>,
    pub background: Option<Background>,
    pub border: Option<Border>,
    pub shadow: Option<Shadow>,
    pub padding: Option<Spacing>,
    pub margin: Option<Spacing>,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub min_width: Option<Size>,
    pub max_width: Option<Size>,
    pub min_height: Option<Size>,
    pub max_height: Option<Size>,
    pub position: Option<Position>,
    pub z_index: Option<i32>,
    pub transform: Option<Transform>,
    pub opacity: Option<f32>,
    pub overflow: Option<Overflow>,
    pub cursor: Option<String>,
    pub responsive: Option<ResponsiveOverrides>,
    pub flex: Option<String>,
    pub align_self: Option<String>,
    pub grid_column: Option<String>,
    pub grid_row: Option<String>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class(mut self, class_name: impl Into<String>) -> Self {
        self.class_name = Some(class_name.into());
        self
    }

    pub fn with_padding(mut self, padding: impl Into<Spacing>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    pub fn with_margin(mut self, margin: impl Into<Spacing>) -> Self {
        self.margin = Some(margin.into());
        self
    }

    pub fn with_width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_height(mut self, height: Size) -> Self {
        self.height = Some(height);
        self
    }

    pub fn to_tailwind_classes(&self) -> String {
        let mut classes = Vec::new();

        if let Some(ref cn) = self.class_name {
            classes.push(cn.clone());
        }

        classes.join(" ")
    }

    pub fn to_inline_css(&self) -> String {
        let mut styles = Vec::new();

        if let Some(ref padding) = self.padding {
            styles.push(format!("padding: {};", padding.value));
        }
        if let Some(ref margin) = self.margin {
            styles.push(format!("margin: {};", margin.value));
        }
        if let Some(ref width) = self.width {
            styles.push(format!("width: {};", width.value));
        }
        if let Some(ref height) = self.height {
            styles.push(format!("height: {};", height.value));
        }
        if let Some(ref min_width) = self.min_width {
            styles.push(format!("min-width: {};", min_width.value));
        }
        if let Some(ref max_width) = self.max_width {
            styles.push(format!("max-width: {};", max_width.value));
        }
        if let Some(ref min_height) = self.min_height {
            styles.push(format!("min-height: {};", min_height.value));
        }
        if let Some(ref max_height) = self.max_height {
            styles.push(format!("max-height: {};", max_height.value));
        }
        if let Some(z) = self.z_index {
            styles.push(format!("z-index: {};", z));
        }
        if let Some(o) = self.opacity {
            styles.push(format!("opacity: {};", o));
        }
        if let Some(ref cursor) = self.cursor {
            styles.push(format!("cursor: {};", cursor));
        }
        if let Some(ref border) = self.border {
            let border_css = border.to_css();
            if !border_css.is_empty() {
                styles.push(border_css);
            }
        }
        if let Some(ref shadow) = self.shadow {
            let shadow_css = shadow.to_css();
            if !shadow_css.is_empty() {
                styles.push(shadow_css);
            }
        }
        if let Some(ref position) = self.position {
            styles.push(position.to_css());
        }
        if let Some(ref transform) = self.transform {
            let transform_css = transform.to_css();
            if !transform_css.is_empty() {
                styles.push(transform_css);
            }
        }
        if let Some(ref overflow) = self.overflow {
            let overflow_str = match overflow {
                Overflow::Visible => "visible",
                Overflow::Hidden => "hidden",
                Overflow::Scroll => "scroll",
                Overflow::Auto => "auto",
            };
            styles.push(format!("overflow: {};", overflow_str));
        }
        if let Some(ref flex) = self.flex {
            styles.push(format!("flex: {};", flex));
        }
        if let Some(ref align_self) = self.align_self {
            styles.push(format!("align-self: {};", align_self));
        }
        if let Some(ref grid_column) = self.grid_column {
            styles.push(format!("grid-column: {};", grid_column));
        }
        if let Some(ref grid_row) = self.grid_row {
            styles.push(format!("grid-row: {};", grid_row));
        }

        styles.join(" ")
    }

    pub fn merge_with(&self, other: &Style) -> Style {
        Style {
            class_name: other.class_name.clone().or_else(|| self.class_name.clone()),
            background: other.background.clone().or_else(|| self.background.clone()),
            border: other.border.clone().or_else(|| self.border.clone()),
            shadow: other.shadow.clone().or_else(|| self.shadow.clone()),
            padding: other.padding.clone().or_else(|| self.padding.clone()),
            margin: other.margin.clone().or_else(|| self.margin.clone()),
            width: other.width.clone().or_else(|| self.width.clone()),
            height: other.height.clone().or_else(|| self.height.clone()),
            min_width: other.min_width.clone().or_else(|| self.min_width.clone()),
            max_width: other.max_width.clone().or_else(|| self.max_width.clone()),
            min_height: other.min_height.clone().or_else(|| self.min_height.clone()),
            max_height: other.max_height.clone().or_else(|| self.max_height.clone()),
            position: other.position.clone().or_else(|| self.position.clone()),
            z_index: other.z_index.or(self.z_index),
            transform: other.transform.clone().or_else(|| self.transform.clone()),
            opacity: other.opacity.or(self.opacity),
            overflow: other.overflow.or(self.overflow),
            cursor: other.cursor.clone().or_else(|| self.cursor.clone()),
            responsive: other.responsive.clone().or_else(|| self.responsive.clone()),
            flex: other.flex.clone().or_else(|| self.flex.clone()),
            align_self: other.align_self.clone().or_else(|| self.align_self.clone()),
            grid_column: other.grid_column.clone().or_else(|| self.grid_column.clone()),
            grid_row: other.grid_row.clone().or_else(|| self.grid_row.clone()),
        }
    }
}

// ============================================================================
// Proto Conversions
// ============================================================================

impl From<Background> for proto::Background {
    fn from(value: Background) -> Self {
        proto::Background {
            background_type: Some(match value {
                Background::Color(c) => proto::background::BackgroundType::Color(c),
                Background::Gradient(g) => proto::background::BackgroundType::Gradient(g.into()),
                Background::Image(i) => proto::background::BackgroundType::Image(i.into()),
                Background::Blur(b) => proto::background::BackgroundType::Blur(b),
            }),
        }
    }
}

impl From<proto::Background> for Background {
    fn from(proto: proto::Background) -> Self {
        match proto.background_type {
            Some(proto::background::BackgroundType::Color(c)) => Background::Color(c),
            Some(proto::background::BackgroundType::Gradient(g)) => Background::Gradient(g.into()),
            Some(proto::background::BackgroundType::Image(i)) => Background::Image(i.into()),
            Some(proto::background::BackgroundType::Blur(b)) => Background::Blur(b),
            None => Background::Color(String::new()),
        }
    }
}

impl From<Gradient> for proto::Gradient {
    fn from(value: Gradient) -> Self {
        proto::Gradient {
            gradient_type: value.gradient_type,
            direction: value.direction,
            stops: value.stops.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<proto::Gradient> for Gradient {
    fn from(proto: proto::Gradient) -> Self {
        Gradient {
            gradient_type: proto.gradient_type,
            direction: proto.direction,
            stops: proto.stops.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<GradientStop> for proto::GradientStop {
    fn from(value: GradientStop) -> Self {
        proto::GradientStop {
            color: value.color,
            position: value.position,
        }
    }
}

impl From<proto::GradientStop> for GradientStop {
    fn from(proto: proto::GradientStop) -> Self {
        GradientStop {
            color: proto.color,
            position: proto.position,
        }
    }
}

impl From<BackgroundImage> for proto::BackgroundImage {
    fn from(value: BackgroundImage) -> Self {
        proto::BackgroundImage {
            url: Some(value.url.into()),
            size: value.size,
            position: value.position,
            repeat: value.repeat,
        }
    }
}

impl From<proto::BackgroundImage> for BackgroundImage {
    fn from(proto: proto::BackgroundImage) -> Self {
        BackgroundImage {
            url: proto.url.map(|u| (&u).into()).unwrap_or(super::BoundValue::literal_string("")),
            size: proto.size,
            position: proto.position,
            repeat: proto.repeat,
        }
    }
}

impl From<Border> for proto::Border {
    fn from(value: Border) -> Self {
        proto::Border {
            width: value.width,
            style: value.style,
            color: value.color,
            radius: value.radius,
        }
    }
}

impl From<proto::Border> for Border {
    fn from(proto: proto::Border) -> Self {
        Border {
            width: proto.width,
            style: proto.style,
            color: proto.color,
            radius: proto.radius,
        }
    }
}

impl From<Shadow> for proto::Shadow {
    fn from(value: Shadow) -> Self {
        proto::Shadow {
            box_shadows: value.box_shadows,
            text_shadow: value.text_shadow,
        }
    }
}

impl From<proto::Shadow> for Shadow {
    fn from(proto: proto::Shadow) -> Self {
        Shadow {
            box_shadows: proto.box_shadows,
            text_shadow: proto.text_shadow,
        }
    }
}

impl From<Spacing> for proto::Spacing {
    fn from(value: Spacing) -> Self {
        proto::Spacing { value: value.value }
    }
}

impl From<proto::Spacing> for Spacing {
    fn from(proto: proto::Spacing) -> Self {
        Spacing { value: proto.value }
    }
}

impl From<Size> for proto::Size {
    fn from(value: Size) -> Self {
        proto::Size { value: value.value }
    }
}

impl From<proto::Size> for Size {
    fn from(proto: proto::Size) -> Self {
        Size { value: proto.value }
    }
}

impl From<Position> for proto::Position {
    fn from(value: Position) -> Self {
        proto::Position {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            position_type: value.position_type,
        }
    }
}

impl From<proto::Position> for Position {
    fn from(proto: proto::Position) -> Self {
        Position {
            top: proto.top,
            right: proto.right,
            bottom: proto.bottom,
            left: proto.left,
            position_type: proto.position_type,
        }
    }
}

impl From<Transform> for proto::Transform {
    fn from(value: Transform) -> Self {
        proto::Transform {
            translate: value.translate,
            rotate: value.rotate,
            scale: value.scale,
            transform_origin: value.transform_origin,
            skew: value.skew,
        }
    }
}

impl From<proto::Transform> for Transform {
    fn from(proto: proto::Transform) -> Self {
        Transform {
            translate: proto.translate,
            rotate: proto.rotate,
            scale: proto.scale,
            transform_origin: proto.transform_origin,
            skew: proto.skew,
        }
    }
}

impl From<Style> for proto::Style {
    fn from(value: Style) -> Self {
        proto::Style {
            class_name: value.class_name,
            background: value.background.map(Into::into),
            border: value.border.map(Into::into),
            shadow: value.shadow.map(Into::into),
            padding: value.padding.map(Into::into),
            margin: value.margin.map(Into::into),
            width: value.width.map(Into::into),
            height: value.height.map(Into::into),
            min_width: value.min_width.map(Into::into),
            max_width: value.max_width.map(Into::into),
            min_height: value.min_height.map(Into::into),
            max_height: value.max_height.map(Into::into),
            position: value.position.map(Into::into),
            z_index: value.z_index,
            transform: value.transform.map(Into::into),
            opacity: value.opacity,
            overflow: value.overflow.map(|o| proto::Overflow::from(o) as i32),
            cursor: value.cursor,
            responsive: value.responsive.map(Into::into),
            flex: value.flex,
            align_self: value.align_self,
            grid_column: value.grid_column,
            grid_row: value.grid_row,
        }
    }
}

impl From<proto::Style> for Style {
    fn from(proto: proto::Style) -> Self {
        Style {
            class_name: proto.class_name,
            background: proto.background.map(Into::into),
            border: proto.border.map(Into::into),
            shadow: proto.shadow.map(Into::into),
            padding: proto.padding.map(Into::into),
            margin: proto.margin.map(Into::into),
            width: proto.width.map(Into::into),
            height: proto.height.map(Into::into),
            min_width: proto.min_width.map(Into::into),
            max_width: proto.max_width.map(Into::into),
            min_height: proto.min_height.map(Into::into),
            max_height: proto.max_height.map(Into::into),
            position: proto.position.map(Into::into),
            z_index: proto.z_index,
            transform: proto.transform.map(Into::into),
            opacity: proto.opacity,
            overflow: proto.overflow.and_then(|o| proto::Overflow::try_from(o).ok()).map(Into::into),
            cursor: proto.cursor,
            responsive: proto.responsive.map(Into::into),
            flex: proto.flex,
            align_self: proto.align_self,
            grid_column: proto.grid_column,
            grid_row: proto.grid_row,
        }
    }
}

impl From<ResponsiveOverrides> for proto::ResponsiveOverrides {
    fn from(value: ResponsiveOverrides) -> Self {
        proto::ResponsiveOverrides {
            sm: value.sm.map(Into::into),
            md: value.md.map(Into::into),
            lg: value.lg.map(Into::into),
            xl: value.xl.map(Into::into),
            xxl: value.xxl.map(Into::into),
        }
    }
}

impl From<proto::ResponsiveOverrides> for ResponsiveOverrides {
    fn from(proto: proto::ResponsiveOverrides) -> Self {
        ResponsiveOverrides {
            sm: proto.sm.map(Into::into),
            md: proto.md.map(Into::into),
            lg: proto.lg.map(Into::into),
            xl: proto.xl.map(Into::into),
            xxl: proto.xxl.map(Into::into),
        }
    }
}

impl From<BreakpointStyle> for proto::BreakpointStyle {
    fn from(value: BreakpointStyle) -> Self {
        proto::BreakpointStyle {
            class_name: value.class_name,
            display: value.display,
            flex_direction: value.flex_direction,
            justify_content: value.justify_content,
            align_items: value.align_items,
            gap: value.gap,
            grid_cols: value.grid_cols,
            width: value.width.map(Into::into),
            height: value.height.map(Into::into),
            padding: value.padding.map(Into::into),
            margin: value.margin.map(Into::into),
            hidden: value.hidden,
            font_size: value.font_size,
            text_align: value.text_align,
            order: value.order,
        }
    }
}

impl From<proto::BreakpointStyle> for BreakpointStyle {
    fn from(proto: proto::BreakpointStyle) -> Self {
        BreakpointStyle {
            class_name: proto.class_name,
            display: proto.display,
            flex_direction: proto.flex_direction,
            justify_content: proto.justify_content,
            align_items: proto.align_items,
            gap: proto.gap,
            grid_cols: proto.grid_cols,
            width: proto.width.map(Into::into),
            height: proto.height.map(Into::into),
            padding: proto.padding.map(Into::into),
            margin: proto.margin.map(Into::into),
            hidden: proto.hidden,
            font_size: proto.font_size,
            text_align: proto.text_align,
            order: proto.order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_default() {
        let size = Size::default();
        assert!(size.value.is_empty());
    }

    #[test]
    fn test_size_constructors() {
        let px = Size::px(100);
        assert_eq!(px.value, "100px");

        let percent = Size::percent(50);
        assert_eq!(percent.value, "50%");

        let auto = Size::auto();
        assert_eq!(auto.value, "auto");

        let full = Size::full();
        assert_eq!(full.value, "100%");
    }

    #[test]
    fn test_spacing_default() {
        let spacing = Spacing::default();
        assert!(spacing.value.is_empty());
    }

    #[test]
    fn test_spacing_new() {
        let spacing = Spacing::new("16px");
        assert_eq!(spacing.value, "16px");
    }

    #[test]
    fn test_spacing_from_str() {
        let spacing: Spacing = "8px".into();
        assert_eq!(spacing.value, "8px");
    }

    #[test]
    fn test_overflow_default() {
        let overflow = Overflow::default();
        assert_eq!(overflow, Overflow::Visible);
    }

    #[test]
    fn test_overflow_variants() {
        assert!(matches!(Overflow::Hidden, Overflow::Hidden));
        assert!(matches!(Overflow::Scroll, Overflow::Scroll));
        assert!(matches!(Overflow::Auto, Overflow::Auto));
    }

    #[test]
    fn test_position_default() {
        let pos = Position::default();
        assert!(pos.top.is_none());
        assert!(pos.right.is_none());
        assert!(pos.bottom.is_none());
        assert!(pos.left.is_none());
    }

    #[test]
    fn test_position_absolute() {
        let pos = Position::absolute();
        assert_eq!(pos.position_type, "absolute");
    }

    #[test]
    fn test_position_relative() {
        let pos = Position::relative();
        assert_eq!(pos.position_type, "relative");
    }

    #[test]
    fn test_position_fixed() {
        let pos = Position::fixed();
        assert_eq!(pos.position_type, "fixed");
    }

    #[test]
    fn test_transform_default() {
        let transform = Transform::default();
        assert!(transform.translate.is_none());
        assert!(transform.rotate.is_none());
        assert!(transform.scale.is_none());
        assert!(transform.transform_origin.is_none());
        assert!(transform.skew.is_none());
    }

    #[test]
    fn test_gradient_creation() {
        let gradient = Gradient {
            gradient_type: "linear".to_string(),
            direction: Some("to right".to_string()),
            stops: vec![],
        };
        assert_eq!(gradient.gradient_type, "linear");
        assert_eq!(gradient.direction.as_deref(), Some("to right"));
        assert!(gradient.stops.is_empty());
    }

    #[test]
    fn test_gradient_with_stops() {
        let gradient = Gradient {
            gradient_type: "linear".to_string(),
            direction: Some("45deg".to_string()),
            stops: vec![
                GradientStop {
                    color: "#ff0000".to_string(),
                    position: 0.0,
                },
                GradientStop {
                    color: "#0000ff".to_string(),
                    position: 1.0,
                },
            ],
        };
        assert_eq!(gradient.stops.len(), 2);
        assert_eq!(gradient.stops[0].color, "#ff0000");
        assert_eq!(gradient.stops[1].position, 1.0);
    }

    #[test]
    fn test_gradient_stop() {
        let stop = GradientStop {
            color: "#ff0000".to_string(),
            position: 0.5,
        };
        assert_eq!(stop.color, "#ff0000");
        assert_eq!(stop.position, 0.5);
    }

    #[test]
    fn test_background_color() {
        let bg = Background::Color("#ffffff".to_string());
        match bg {
            Background::Color(color) => assert_eq!(color, "#ffffff"),
            _ => panic!("Expected Color variant"),
        }
    }

    #[test]
    fn test_background_gradient() {
        let gradient = Gradient {
            gradient_type: "linear".to_string(),
            direction: None,
            stops: vec![],
        };
        let bg = Background::Gradient(gradient);
        assert!(matches!(bg, Background::Gradient(_)));
    }

    #[test]
    fn test_background_blur() {
        let bg = Background::Blur("10px".to_string());
        match bg {
            Background::Blur(blur) => assert_eq!(blur, "10px"),
            _ => panic!("Expected Blur variant"),
        }
    }

    #[test]
    fn test_border_default() {
        let border = Border::default();
        assert!(border.width.is_none());
        assert!(border.style.is_none());
        assert!(border.color.is_none());
        assert!(border.radius.is_none());
    }

    #[test]
    fn test_border_with_values() {
        let border = Border {
            width: Some("2px".to_string()),
            style: Some("solid".to_string()),
            color: Some("#000000".to_string()),
            radius: Some("8px".to_string()),
        };
        assert_eq!(border.width.as_deref(), Some("2px"));
        assert_eq!(border.style.as_deref(), Some("solid"));
        assert_eq!(border.color.as_deref(), Some("#000000"));
        assert_eq!(border.radius.as_deref(), Some("8px"));
    }

    #[test]
    fn test_border_builder_methods() {
        let border = Border::new()
            .with_width("1px")
            .with_radius("4px");
        assert_eq!(border.width.as_deref(), Some("1px"));
        assert_eq!(border.radius.as_deref(), Some("4px"));
    }

    #[test]
    fn test_border_to_css() {
        let border = Border {
            width: Some("2px".to_string()),
            style: Some("solid".to_string()),
            color: Some("#000".to_string()),
            radius: Some("4px".to_string()),
        };
        let css = border.to_css();
        assert!(css.contains("border-width"));
        assert!(css.contains("border-style"));
    }

    #[test]
    fn test_shadow_default() {
        let shadow = Shadow::default();
        assert!(shadow.box_shadows.is_empty());
        assert!(shadow.text_shadow.is_none());
    }

    #[test]
    fn test_shadow_with_values() {
        let shadow = Shadow {
            box_shadows: vec!["0 2px 4px rgba(0,0,0,0.25)".to_string()],
            text_shadow: Some("1px 1px 2px black".to_string()),
        };
        assert_eq!(shadow.box_shadows.len(), 1);
        assert!(shadow.text_shadow.is_some());
    }

    #[test]
    fn test_shadow_to_css() {
        let shadow = Shadow {
            box_shadows: vec!["0 2px 4px rgba(0,0,0,0.25)".to_string()],
            text_shadow: None,
        };
        let css = shadow.to_css();
        assert!(css.contains("box-shadow"));
    }

    #[test]
    fn test_style_default() {
        let style = Style::default();
        assert!(style.class_name.is_none());
        assert!(style.background.is_none());
        assert!(style.border.is_none());
        assert!(style.shadow.is_none());
        assert!(style.padding.is_none());
        assert!(style.margin.is_none());
        assert!(style.width.is_none());
        assert!(style.height.is_none());
        assert!(style.position.is_none());
        assert!(style.transform.is_none());
        assert!(style.opacity.is_none());
        assert!(style.overflow.is_none());
        assert!(style.cursor.is_none());
        assert!(style.responsive.is_none());
    }

    #[test]
    fn test_style_new() {
        let style = Style::new();
        assert!(style.class_name.is_none());
    }

    #[test]
    fn test_style_builder_methods() {
        let style = Style::new()
            .with_class("my-class")
            .with_padding("16px")
            .with_margin("8px")
            .with_width(Size::px(100))
            .with_height(Size::percent(50));

        assert_eq!(style.class_name.as_deref(), Some("my-class"));
        assert!(style.padding.is_some());
        assert!(style.margin.is_some());
        assert!(style.width.is_some());
        assert!(style.height.is_some());
    }

    #[test]
    fn test_style_to_tailwind_classes() {
        let style = Style::new().with_class("p-4 m-2 bg-blue-500");
        let classes = style.to_tailwind_classes();
        assert_eq!(classes, "p-4 m-2 bg-blue-500");
    }

    #[test]
    fn test_responsive_overrides_default() {
        let overrides = ResponsiveOverrides::default();
        assert!(overrides.sm.is_none());
        assert!(overrides.md.is_none());
        assert!(overrides.lg.is_none());
        assert!(overrides.xl.is_none());
        assert!(overrides.xxl.is_none());
    }

    #[test]
    fn test_breakpoint_style_default() {
        let bp = BreakpointStyle::default();
        assert!(bp.class_name.is_none());
        assert!(bp.display.is_none());
        assert!(bp.hidden.is_none());
    }

    #[test]
    fn test_responsive_overrides_with_breakpoints() {
        let overrides = ResponsiveOverrides {
            sm: Some(BreakpointStyle {
                hidden: Some(true),
                ..Default::default()
            }),
            md: Some(BreakpointStyle {
                hidden: Some(false),
                display: Some("flex".to_string()),
                ..Default::default()
            }),
            lg: None,
            xl: None,
            xxl: None,
        };

        assert!(overrides.sm.is_some());
        assert!(overrides.md.is_some());
        assert!(overrides.lg.is_none());

        if let Some(sm) = &overrides.sm {
            assert_eq!(sm.hidden, Some(true));
        }

        if let Some(md) = &overrides.md {
            assert_eq!(md.hidden, Some(false));
            assert_eq!(md.display.as_deref(), Some("flex"));
        }
    }

    #[test]
    fn test_transform_with_values() {
        let transform = Transform {
            translate: Some("10px 20px".to_string()),
            rotate: Some(45.0),
            scale: Some("1.5".to_string()),
            transform_origin: Some("center".to_string()),
            skew: Some("5deg".to_string()),
        };
        assert_eq!(transform.translate.as_deref(), Some("10px 20px"));
        assert_eq!(transform.rotate, Some(45.0));
        assert_eq!(transform.scale.as_deref(), Some("1.5"));
    }
}
