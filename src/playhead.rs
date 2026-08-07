//! Timeline playhead widgets.

use crate::{ruler::MusicalInfo, PlayheadStyle, TimeAxis};

/// Provides a playhead position and musical scale.
pub trait Info: MusicalInfo {
    /// Returns the playhead offset from the left edge of the visible timeline.
    fn playhead_ticks(&self) -> f32;
}

/// Handles playhead movement.
pub trait Interaction {
    /// Sets the playhead offset from the left edge of the visible timeline.
    fn set_playhead_ticks(&mut self, ticks: f32);
}

/// Combines playhead information and interaction.
pub trait PlayheadApi: Info + Interaction {}

impl<T> PlayheadApi for T where T: Info + Interaction {}

/// Timeline playhead configuration.
pub struct Playhead {
    extend_beyond_last_track: f32,
    extend_to_available_height: bool,
    width: f32,
}

impl Playhead {
    /// Default extension below the final laid-out row.
    pub const DEFAULT_EXTEND_BEYOND_LAST_TRACK: f32 = 0.0;
    /// Whether the default playhead spans all available timeline height.
    pub const DEFAULT_EXTEND_TO_AVAILABLE_HEIGHT: bool = false;
    /// Default painted and interactive width in screen points.
    pub const DEFAULT_WIDTH: f32 = 1.0;

    /// Creates a playhead with the default geometry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Chooses whether the line reaches the timeline's available bottom edge.
    pub fn extend_to_available_height(mut self, extend: bool) -> Self {
        self.extend_to_available_height = extend;
        self
    }

    /// Extends the line below the last row when available-height mode is off.
    pub fn extend_beyond_last_track(mut self, extension: f32) -> Self {
        self.extend_beyond_last_track = extension;
        self
    }

    /// Sets the painted and interactive width in screen points.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Returns the painted playhead rectangle for the current row layout.
    fn rect(&self, x: f32, timeline_rect: egui::Rect, tracks_bottom: f32) -> egui::Rect {
        let half_width = self.width * 0.5;
        let bottom = if self.extend_to_available_height {
            timeline_rect.bottom()
        } else {
            tracks_bottom + self.extend_beyond_last_track
        };
        egui::Rect::from_min_max(
            egui::pos2(x - half_width, timeline_rect.top()),
            egui::pos2(x + half_width, bottom),
        )
    }
}

impl Default for Playhead {
    /// Returns a one-point-wide playhead with no extra extension.
    fn default() -> Self {
        Self {
            extend_beyond_last_track: Self::DEFAULT_EXTEND_BEYOND_LAST_TRACK,
            extend_to_available_height: Self::DEFAULT_EXTEND_TO_AVAILABLE_HEIGHT,
            width: Self::DEFAULT_WIDTH,
        }
    }
}

/// Shows the callback-driven playhead.
pub fn set(
    ui: &mut egui::Ui,
    api: &mut dyn PlayheadApi,
    timeline_rect: egui::Rect,
    tracks_bottom: f32,
    playhead: Playhead,
) -> egui::Response {
    let playhead_x = timeline_rect.left() + api.playhead_ticks() / api.ticks_per_point();
    let rect = playhead.rect(playhead_x, timeline_rect, tracks_bottom);
    let mut response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

    if response.clicked() || response.dragged() {
        if let Some(position) = response.interact_pointer_pos() {
            let visible_ticks = api.ticks_per_point() * timeline_rect.width();
            let tick = (((position.x - timeline_rect.min.x) / timeline_rect.width())
                * visible_ticks)
                .max(0.0);
            api.set_playhead_ticks(tick);
            response.mark_changed();
        }
    }

    if timeline_rect.x_range().contains(playhead_x) {
        let visuals = ui.style().interact(&response);
        let stroke = egui::Stroke {
            width: 0.5,
            ..visuals.fg_stroke
        };
        ui.painter().rect(
            rect,
            egui::CornerRadius::ZERO,
            visuals.bg_fill,
            stroke,
            egui::StrokeKind::Middle,
        );
    }

    response
}

/// Paints a hover-only playhead at an absolute tick.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_at(
    ui: &mut egui::Ui,
    axis: &TimeAxis,
    tick: i64,
    timeline_rect: egui::Rect,
    tracks_bottom: f32,
    playhead: Playhead,
    style: &PlayheadStyle,
) -> egui::Response {
    let x = axis.tick_to_x(timeline_rect, tick);
    let rect = playhead.rect(x, timeline_rect, tracks_bottom);
    let response = ui.allocate_rect(rect, egui::Sense::hover());
    if timeline_rect.x_range().contains(x) {
        ui.painter().rect(
            rect,
            egui::CornerRadius::ZERO,
            style.color,
            egui::Stroke::NONE,
            egui::StrokeKind::Middle,
        );
    }
    response
}
