#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    Full,
    Compact,
    Overview,
    Tiny,
}

impl Detail {
    pub fn for_zoom(enabled: bool, zoom: f32) -> Self {
        if !enabled || zoom >= 0.5 {
            Self::Full
        } else if zoom >= 0.2 {
            Self::Compact
        } else if zoom >= 0.1 {
            Self::Overview
        } else {
            Self::Tiny
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Compact => "compact",
            Self::Overview => "overview",
            Self::Tiny => "tiny",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Full => "All labels, icons, pins and detailed edges",
            Self::Compact => "Titles and pin markers; no icons or pin labels",
            Self::Overview => {
                "Node bodies and headers; no labels or pins; continuous data edges; execution pulses retained"
            }
            Self::Tiny => {
                "Kind-tinted node rectangles; no labels or pins; continuous data edges; execution pulses retained"
            }
        }
    }

    pub fn title(self) -> bool {
        matches!(self, Self::Full | Self::Compact)
    }
    pub fn pins(self) -> bool {
        self.title()
    }
    pub fn coarse_edges(self) -> bool {
        matches!(self, Self::Overview | Self::Tiny)
    }
}

pub fn coarse_curve_allowed(enabled: bool, world_error: f32, zoom: f32) -> bool {
    enabled && world_error * zoom <= 0.75
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_lod_preserves_full_detail_at_every_zoom() {
        for zoom in [0.01, 0.05, 0.099, 0.1, 0.199, 0.2, 0.499, 0.5, 3.0] {
            assert_eq!(Detail::for_zoom(false, zoom), Detail::Full);
        }
    }

    #[test]
    fn thresholds_restore_detail_at_the_boundary() {
        for (zoom, expected) in [
            (0.099, Detail::Tiny),
            (0.1, Detail::Overview),
            (0.199, Detail::Overview),
            (0.2, Detail::Compact),
            (0.499, Detail::Compact),
            (0.5, Detail::Full),
        ] {
            assert_eq!(Detail::for_zoom(true, zoom), expected);
        }
    }

    #[test]
    fn coarse_curves_must_remain_within_screen_error_budget() {
        assert!(coarse_curve_allowed(true, 3.0, 0.2));
        assert!(coarse_curve_allowed(true, 3.75, 0.2));
        assert!(!coarse_curve_allowed(true, 4.0, 0.2));
        assert!(!coarse_curve_allowed(false, 0.0, 0.01));
    }
}
