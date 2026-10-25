//! Generic musical-timeline layout and interactions for egui.

use egui_plot as plot;
use std::ops::{Range, RangeInclusive};

pub use axis::{Meter, TimeAxis};
pub use grid::{Division, GridSpec, StepRank, STRAIGHT_DIVISIONS, TRIPLET_DIVISIONS};
pub use playhead::{Playhead, PlayheadApi};
pub use ruler::MusicalRuler;
pub use style::{GridStyle, PlayheadStyle, RulerStyle, TimelineStyle};

mod axis;
pub mod grid;
pub mod playhead;
pub mod ruler;
mod style;

/// Minimum screen-space gap between ruler steps.
pub const MIN_STEP_GAP: f32 = 4.0;

/// The callbacks required to instantiate a view-relative timeline.
pub trait TimelineApi {
    /// Returns the musical information used by the ruler and grid.
    fn musical_ruler_info(&self) -> &dyn ruler::MusicalInfo;

    /// Shifts the timeline start by the given number of ticks.
    fn shift_timeline_start(&mut self, ticks: f32);

    /// Responds to vertical scrolling while `Ctrl` is held.
    fn zoom(&mut self, y_delta: f32);
}

/// A musical bar expressed relative to the visible timeline start.
#[derive(Clone, Debug)]
pub struct Bar {
    /// The start and end tick offsets of the bar.
    pub tick_range: Range<f32>,
    /// The time signature of this bar.
    pub time_sig: TimeSig,
}

/// A musical time signature.
#[derive(Clone, Debug)]
pub struct TimeSig {
    /// Number of notated beats in each bar.
    pub top: u16,
    /// Note value that represents one notated beat.
    pub bottom: u16,
}

impl TimeSig {
    /// Returns the number of quarter-note beats in one bar.
    pub fn beats_per_bar(&self) -> f32 {
        4.0 * self.top as f32 / self.bottom as f32
    }
}

/// The top-level timeline widget.
pub struct Timeline {
    header: Option<f32>,
}

/// The layout chain returned by [`Timeline::show`].
pub struct Show {
    tracks: TracksCtx,
    separator: Option<egui::Stroke>,
    ui: egui::Ui,
}

/// Context shared while laying out pinned or scrollable tracks.
pub struct TracksCtx {
    /// The full widget rectangle, including the header and timeline areas.
    pub full_rect: egui::Rect,
    /// The full header column, when one was configured.
    pub header_full_rect: Option<egui::Rect>,
    /// Coordinate helpers for the non-header timeline area.
    pub timeline: TimelineCtx,
}

/// Coordinate context for the visible timeline area.
pub struct TimelineCtx {
    /// The full visible timeline rectangle, excluding the header column.
    pub full_rect: egui::Rect,
    /// The total number of view-relative ticks visible across the timeline.
    pub visible_ticks: f32,
}

/// Context passed to a timeline background painter.
pub struct BackgroundCtx<'a> {
    /// The full header column, when one was configured.
    pub header_full_rect: Option<egui::Rect>,
    /// Coordinate helpers for the non-header timeline area.
    pub timeline: &'a TimelineCtx,
}

/// Context for building one track row with an optional header.
pub struct TrackCtx<'a> {
    tracks: &'a TracksCtx,
    ui: &'a mut egui::Ui,
    available_rect: egui::Rect,
    header_height: f32,
}

/// Final timeline layout state used to place the playhead above all rows.
pub struct SetPlayhead {
    timeline_rect: egui::Rect,
    tracks_bottom: f32,
}

#[allow(clippy::new_without_default)]
impl Timeline {
    /// Begins building a timeline with no header column.
    pub fn new() -> Self {
        Self { header: None }
    }

    /// Adds a header column with the given width.
    pub fn header(mut self, width: f32) -> Self {
        self.header = Some(width);
        self
    }

    /// Shows the callback-driven, view-relative timeline.
    pub fn show(self, ui: &mut egui::Ui, timeline: &mut dyn TimelineApi) -> Show {
        let (full_rect, header_rect, timeline_rect) = self.layout_rects(ui);

        let hovered = ui.rect_contains_pointer(timeline_rect);
        let recovered_zoom = recovered_zoom_scroll(ui, timeline_rect);
        if hovered {
            let (delta, ctrl) = ui.input(|input| (input.smooth_scroll_delta, input.modifiers.ctrl));
            if ctrl {
                let y_delta = if delta.x != 0.0 || delta.y != 0.0 {
                    delta.y - delta.x
                } else {
                    recovered_zoom
                };
                if y_delta != 0.0 {
                    timeline.zoom(y_delta);
                }
            } else if delta.x != 0.0 {
                let ticks_per_point = timeline.musical_ruler_info().ticks_per_point();
                timeline.shift_timeline_start(delta.x * ticks_per_point);
            }
        }

        let visuals = ui.style().noninteractive();
        let background_stroke = egui::Stroke {
            width: 0.0,
            ..visuals.bg_stroke
        };
        let visible_ticks = timeline.musical_ruler_info().ticks_per_point() * timeline_rect.width();
        self.finish_show(
            ui,
            full_rect,
            header_rect,
            timeline_rect,
            visible_ticks,
            visuals.bg_fill,
            background_stroke,
            None,
        )
    }

    /// Shows an absolute integer-tick timeline backed by a caller-owned axis.
    pub fn show_with_axis(
        self,
        ui: &mut egui::Ui,
        axis: &mut TimeAxis,
        style: &TimelineStyle,
    ) -> Show {
        let (full_rect, header_rect, timeline_rect) = self.layout_rects(ui);

        if timeline_rect.is_finite()
            && timeline_rect.width() > 0.0
            && ui.rect_contains_pointer(timeline_rect)
        {
            let (zoom_delta, pointer_x, scroll_delta) = ui.input(|input| {
                (
                    input.zoom_delta(),
                    input
                        .pointer
                        .hover_pos()
                        .map_or(timeline_rect.center().x, |position| position.x),
                    input.smooth_scroll_delta,
                )
            });
            if zoom_delta != 1.0 && zoom_delta.is_finite() && zoom_delta > 0.0 {
                axis.zoom_about_x(timeline_rect, pointer_x, 1.0 / f64::from(zoom_delta));
            }
            if scroll_delta.x != 0.0 {
                axis.pan_points(scroll_delta.x);
            }
        }

        let visible_ticks = axis.visible_ticks(timeline_rect.width()) as f32;
        self.finish_show(
            ui,
            full_rect,
            header_rect,
            timeline_rect,
            visible_ticks,
            style.background,
            egui::Stroke::NONE,
            Some(style.separator),
        )
    }

    /// Splits the available widget rectangle into its header and timeline areas.
    fn layout_rects(&self, ui: &egui::Ui) -> (egui::Rect, Option<egui::Rect>, egui::Rect) {
        let full_rect = ui.available_rect_before_wrap();
        let mut timeline_rect = full_rect;
        let header_rect = self.header.map(|width| {
            let mut rect = full_rect;
            rect.set_width(width);
            timeline_rect.min.x = rect.right();
            rect
        });
        (full_rect, header_rect, timeline_rect)
    }

    /// Paints the shared layout and returns the first builder stage.
    #[allow(clippy::too_many_arguments)]
    fn finish_show(
        self,
        ui: &mut egui::Ui,
        full_rect: egui::Rect,
        header_rect: Option<egui::Rect>,
        timeline_rect: egui::Rect,
        visible_ticks: f32,
        background: egui::Color32,
        background_stroke: egui::Stroke,
        separator: Option<egui::Stroke>,
    ) -> Show {
        ui.painter().rect(
            full_rect,
            egui::CornerRadius::ZERO,
            background,
            background_stroke,
            egui::StrokeKind::Middle,
        );

        let timeline = TimelineCtx {
            full_rect: timeline_rect,
            visible_ticks,
        };
        let tracks = TracksCtx {
            full_rect,
            header_full_rect: header_rect,
            timeline,
        };
        let layout = egui::Layout::top_down(egui::Align::Min);
        let ui = ui.new_child(egui::UiBuilder::new().max_rect(full_rect).layout(layout));
        Show {
            tracks,
            separator,
            ui,
        }
    }
}

/// Reconstructs egui's signed, axis-aware Ctrl-wheel smoothing.
fn recovered_zoom_scroll(ui: &egui::Ui, rect: egui::Rect) -> f32 {
    let options = ui.ctx().options(|options| options.input_options);
    let state_id = ui.id().with([rect.min.x.to_bits(), rect.min.y.to_bits()]);
    let (mut pending, mut in_touch, mut modifiers): (egui::Vec2, bool, egui::Modifiers) = ui
        .ctx()
        .data(|data| data.get_temp(state_id))
        .unwrap_or_default();
    let pass = ui.ctx().cumulative_pass_nr();
    let previous_pass = ui
        .ctx()
        .data(|data| data.get_temp::<u64>(state_id.with("pass")));
    if previous_pass != pass.checked_sub(1) {
        (pending, in_touch, modifiers) = Default::default();
    }
    let delta = ui.input(|input| {
        let mut smooth = egui::Vec2::ZERO;
        for event in &input.raw.events {
            let egui::Event::MouseWheel {
                unit,
                delta: raw_delta,
                phase,
                modifiers: event_modifiers,
            } = event
            else {
                continue;
            };
            match phase {
                egui::TouchPhase::Start => {
                    in_touch = true;
                    modifiers = *event_modifiers;
                }
                egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                    (pending, smooth, in_touch, modifiers) = Default::default();
                }
                egui::TouchPhase::Move => {
                    modifiers = if in_touch {
                        modifiers | *event_modifiers
                    } else {
                        *event_modifiers
                    };
                    let mut movement = match unit {
                        egui::MouseWheelUnit::Point => *raw_delta,
                        egui::MouseWheelUnit::Line => options.line_scroll_speed * *raw_delta,
                        egui::MouseWheelUnit::Page => input.viewport_rect().height() * *raw_delta,
                    };
                    let horizontal = modifiers.matches_any(options.horizontal_scroll_modifier);
                    let vertical = modifiers.matches_any(options.vertical_scroll_modifier);
                    if horizontal && !vertical {
                        movement = egui::vec2(movement.x + movement.y, 0.0);
                    } else if !horizontal && vertical {
                        movement = egui::vec2(0.0, movement.x + movement.y);
                    }
                    if in_touch
                        || matches!(unit, egui::MouseWheelUnit::Point) && movement.length() < 8.0
                    {
                        smooth += movement;
                    } else {
                        pending += movement;
                    }
                }
            }
        }
        let factor = egui::emath::exponential_smooth_factor(0.90, 0.1, input.stable_dt.min(0.1));
        for dimension in 0..2 {
            if pending[dimension].abs() < 1.0 {
                smooth[dimension] += pending[dimension];
                pending[dimension] = 0.0;
            } else {
                let applied = factor * pending[dimension];
                smooth[dimension] += applied;
                pending[dimension] -= applied;
            }
        }
        smooth.y - smooth.x
    });
    ui.ctx().data_mut(|data| {
        data.insert_temp(state_id, (pending, in_touch, modifiers));
        data.insert_temp(state_id.with("pass"), pass);
    });
    delta
}

impl Show {
    /// Runs a painter before the grid and rows are added.
    pub fn background(
        mut self,
        background: impl FnOnce(&BackgroundCtx<'_>, &mut egui::Ui),
    ) -> Self {
        let context = BackgroundCtx {
            header_full_rect: self.tracks.header_full_rect,
            timeline: &self.tracks.timeline,
        };
        background(&context, &mut self.ui);
        self
    }

    /// Paints the view-relative musical grid.
    pub fn paint_grid(self, info: &dyn ruler::MusicalInfo) -> Self {
        let visuals = self.ui.style().noninteractive();
        let mut stroke = visuals.bg_stroke;
        let bar_color = stroke.color.linear_multiply(0.5);
        let step_even_color = stroke.color.linear_multiply(0.25);
        let step_odd_color = stroke.color.linear_multiply(0.125);
        let rect = self.tracks.timeline.full_rect;
        let mut steps = ruler::Steps::new(info, rect.width(), MIN_STEP_GAP);
        while let Some(step) = steps.next(info) {
            stroke.color = match step.index_in_bar {
                0 => bar_color,
                index if index % 2 == 0 => step_even_color,
                _ => step_odd_color,
            };
            let x = rect.left() + step.x;
            self.ui.painter().line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                stroke,
            );
        }
        self
    }

    /// Paints an absolute integer-tick grid using its musical rank.
    pub fn paint_grid_with_axis(
        self,
        axis: &TimeAxis,
        spec: &GridSpec<'_>,
        style: &GridStyle,
    ) -> Self {
        let rect = self.tracks.timeline.full_rect;
        for step in grid::Steps::new(spec, axis, rect) {
            let stroke = match step.rank {
                StepRank::Bar => style.bar,
                StepRank::Beat => style.beat,
                StepRank::Division => style.division,
            };
            self.ui.painter().line_segment(
                [
                    egui::pos2(step.x, rect.top()),
                    egui::pos2(step.x, rect.bottom()),
                ],
                stroke,
            );
        }
        self
    }

    /// Lays out rows that remain pinned above the scrollable track region.
    pub fn pinned_tracks(mut self, tracks_fn: impl FnOnce(&TracksCtx, &mut egui::Ui)) -> Self {
        self.ui.scope(|ui| tracks_fn(&self.tracks, ui));
        let remaining = self.ui.available_rect_before_wrap();
        let separator = self
            .separator
            .unwrap_or_else(|| self.ui.style().visuals.noninteractive().bg_stroke);
        self.ui
            .painter()
            .line_segment([remaining.left_top(), remaining.right_top()], separator);
        self.ui.add_space(separator.width);
        self.ui.set_clip_rect(self.ui.available_rect_before_wrap());
        self
    }

    /// Lays out scrollable rows, passing their content-relative viewport.
    pub fn tracks(
        mut self,
        tracks_fn: impl FnOnce(&TracksCtx, egui::Rect, &mut egui::Ui),
    ) -> SetPlayhead {
        let rect = self.ui.available_rect_before_wrap();
        let scroll_source = if self.ui.input(|input| input.modifiers.ctrl) {
            egui::containers::scroll_area::ScrollSource::NONE
        } else {
            egui::containers::scroll_area::ScrollSource::ALL
        };
        let result = egui::ScrollArea::vertical()
            .max_height(rect.height())
            .scroll_source(scroll_source)
            .animated(true)
            .stick_to_bottom(true)
            .show_viewport(&mut self.ui, |ui, viewport| {
                tracks_fn(&self.tracks, viewport, ui)
            });
        let tracks_bottom = result
            .inner_rect
            .bottom()
            .min(result.inner_rect.top() + result.content_size.y);
        SetPlayhead {
            timeline_rect: self.tracks.timeline.full_rect,
            tracks_bottom,
        }
    }
}

impl SetPlayhead {
    /// Shows the callback-driven playhead above the timeline rows.
    pub fn playhead(
        &self,
        ui: &mut egui::Ui,
        info: &mut dyn PlayheadApi,
        playhead: Playhead,
    ) -> egui::Response {
        playhead::set(ui, info, self.timeline_rect, self.tracks_bottom, playhead)
    }

    /// Paints a non-interactive playhead at an absolute integer tick.
    pub fn playhead_at(
        &self,
        ui: &mut egui::Ui,
        axis: &TimeAxis,
        tick: i64,
        playhead: Playhead,
        style: &PlayheadStyle,
    ) -> egui::Response {
        playhead::paint_at(
            ui,
            axis,
            tick,
            self.timeline_rect,
            self.tracks_bottom,
            playhead,
            style,
        )
    }
}

impl TracksCtx {
    /// Begins the next track row at the current vertical cursor.
    pub fn next<'a>(&'a self, ui: &'a mut egui::Ui) -> TrackCtx<'a> {
        TrackCtx {
            tracks: self,
            available_rect: ui.available_rect_before_wrap(),
            ui,
            header_height: 0.0,
        }
    }
}

impl TrackCtx<'_> {
    /// Builds the optional header portion of this row.
    pub fn header(mut self, header: impl FnOnce(&mut egui::Ui)) -> Self {
        self.header_height = self
            .tracks
            .header_full_rect
            .map(|mut rect| {
                rect.min.y = self.available_rect.min.y;
                let mut child = self.ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(*self.ui.layout()),
                );
                header(&mut child);
                child.min_rect().height()
            })
            .unwrap_or(0.0);
        self
    }

    /// Builds the timeline portion of this row.
    pub fn show(self, track: impl FnOnce(&TimelineCtx, &mut egui::Ui)) {
        let track_height = {
            let mut rect = self.tracks.timeline.full_rect;
            rect.min.y = self.available_rect.min.y;
            let mut child = self.ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(*self.ui.layout()),
            );
            track(&self.tracks.timeline, &mut child);
            child.min_rect().height()
        };
        let row_height = self.header_height.max(track_height);
        let width = self.tracks.full_rect.width();
        self.ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.spacing_mut().interact_size.y = 0.0;
            ui.horizontal(|ui| ui.add_space(width));
            ui.add_space(row_height);
        });
    }
}

impl TimelineCtx {
    /// Returns the number of view-relative ticks visible across the timeline.
    pub fn visible_ticks(&self) -> f32 {
        self.visible_ticks
    }

    /// Creates an egui plot configured for view-relative timeline ticks.
    pub fn plot_ticks(&self, id_source: impl egui::AsId, y: RangeInclusive<f32>) -> plot::Plot<'_> {
        plot::Plot::new(id_source)
            .set_margin_fraction(egui::Vec2::ZERO)
            .show_grid(egui::Vec2b::FALSE)
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .include_x(0.0)
            .include_x(self.visible_ticks)
            .include_y(*y.start())
            .include_y(*y.end())
            .show_x(false)
            .show_y(false)
            .legend(plot::Legend::default().position(plot::Corner::LeftTop))
            .show_background(false)
            .show_axes([false; 2])
            .height(72.0)
    }
}
