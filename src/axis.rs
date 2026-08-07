//! Musical geometry and absolute integer-tick viewport transforms.

use std::ops::Range;

/// Uniform musical geometry for one timeline.
#[derive(Clone, Copy)]
pub struct Meter {
    ticks_per_beat: i64,
    beats_per_bar: i64,
}

impl Meter {
    /// Conventional 960 PPQN in 4/4.
    pub const DEFAULT: Self = Self {
        ticks_per_beat: 960,
        beats_per_bar: 4,
    };
    /// Largest accepted number of ticks per beat.
    const MAX_TICKS_PER_BEAT: i64 = 4_294_967_296;
    /// Largest accepted number of beats per bar.
    const MAX_BEATS_PER_BAR: i64 = 65_536;

    /// Creates a meter, clamping both dimensions to their supported ranges.
    pub fn new(ticks_per_beat: i64, beats_per_bar: i64) -> Self {
        Self {
            ticks_per_beat: ticks_per_beat.clamp(1, Self::MAX_TICKS_PER_BEAT),
            beats_per_bar: beats_per_bar.clamp(1, Self::MAX_BEATS_PER_BAR),
        }
    }

    /// Returns the number of ticks in one beat.
    pub fn ticks_per_beat(self) -> i64 {
        self.ticks_per_beat
    }
    /// Returns the number of beats in one bar.
    pub fn beats_per_bar(self) -> i64 {
        self.beats_per_bar
    }
    /// Returns the saturating number of ticks in one bar.
    pub fn ticks_per_bar(self) -> i64 {
        self.ticks_per_beat.saturating_mul(self.beats_per_bar)
    }
}

/// A caller-owned visible time window using absolute integer ticks.
#[derive(Clone, Copy)]
pub struct TimeAxis {
    start: i64,
    ticks_per_point: f64,
}

impl TimeAxis {
    /// Default zoom, in ticks per screen point.
    pub const DEFAULT_TICKS_PER_POINT: f64 = 4.0;
    /// Deepest supported zoom-in, in ticks per point.
    const MIN_TICKS_PER_POINT: f64 = 0.05;
    /// Furthest supported zoom-out, in ticks per point.
    const MAX_TICKS_PER_POINT: f64 = 4096.0;

    /// Creates an axis with a non-negative start and supported zoom value.
    pub fn new(start: i64, ticks_per_point: f64) -> Self {
        let ticks_per_point = sanitize_zoom(ticks_per_point)
            .unwrap_or(Self::DEFAULT_TICKS_PER_POINT)
            .clamp(Self::MIN_TICKS_PER_POINT, Self::MAX_TICKS_PER_POINT);
        Self {
            start: start.max(0),
            ticks_per_point,
        }
    }

    /// Returns the absolute tick at the viewport's left edge.
    pub fn start(&self) -> i64 {
        self.start
    }
    /// Returns the current zoom in ticks per screen point.
    pub fn ticks_per_point(&self) -> f64 {
        self.ticks_per_point
    }
    /// Sets the left-edge tick, clamped to zero.
    pub fn set_start(&mut self, start: i64) {
        self.start = start.max(0);
    }
    /// Sets the zoom, ignoring non-finite and non-positive values.
    pub fn set_ticks_per_point(&mut self, ticks_per_point: f64) {
        if let Some(ticks_per_point) = sanitize_zoom(ticks_per_point) {
            self.ticks_per_point =
                ticks_per_point.clamp(Self::MIN_TICKS_PER_POINT, Self::MAX_TICKS_PER_POINT);
        }
    }
    /// Pans later for positive point deltas and earlier for negative deltas.
    pub fn pan_points(&mut self, delta_points: f32) {
        let delta = (f64::from(delta_points) * self.ticks_per_point).round() as i64;
        self.set_start(self.start.saturating_add(delta));
    }
    /// Maps an absolute tick to screen x.
    pub fn tick_to_x(&self, rect: egui::Rect, tick: i64) -> f32 {
        rect.left() + (tick.saturating_sub(self.start) as f64 / self.ticks_per_point) as f32
    }
    /// Maps screen x to the nearest absolute tick.
    pub fn x_to_tick(&self, rect: egui::Rect, x: f32) -> i64 {
        let delta = (f64::from(x - rect.left()) * self.ticks_per_point).round() as i64;
        self.start.saturating_add(delta)
    }
    /// Returns the tick span of a point width, rounded up.
    pub(crate) fn visible_ticks(&self, width_points: f32) -> i64 {
        if !width_points.is_finite() || width_points <= 0.0 {
            return 0;
        }
        (f64::from(width_points) * self.ticks_per_point).ceil() as i64
    }

    /// Returns the half-open tick range visible inside `rect`.
    pub fn visible_range(&self, rect: egui::Rect) -> Range<i64> {
        self.start..self.start.saturating_add(self.visible_ticks(rect.width()))
    }

    /// Scales about `x`, ignoring invalid factors and preserving its tick unless
    /// the new left edge must clamp to zero.
    pub fn zoom_about_x(&mut self, rect: egui::Rect, x: f32, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let anchor = self.x_to_tick(rect, x);
        let offset = x - rect.left();
        let zoom = (self.ticks_per_point * factor)
            .clamp(Self::MIN_TICKS_PER_POINT, Self::MAX_TICKS_PER_POINT);
        self.set_ticks_per_point(zoom);
        let anchor_offset = (f64::from(offset) * self.ticks_per_point).round() as i64;
        self.set_start(anchor.saturating_sub(anchor_offset));
    }
}

/// Retains a finite positive zoom value and rejects every other value.
fn sanitize_zoom(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}
