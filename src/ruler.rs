//! Musical ruler widgets and step iterators.

use crate::{
    grid::{power_of_two_stride, sanitize_gap},
    Bar, GridSpec, RulerStyle, StepRank, TimeAxis,
};

/// Musical information required by the timeline and ruler.
pub trait MusicalInfo {
    /// Returns the number of ticks per quarter-note beat.
    fn ticks_per_beat(&self) -> u32;

    /// Returns the bar at an offset from the left edge of the current view.
    fn bar_at_ticks(&self, tick: f32) -> Bar;

    /// Returns the view zoom in ticks per screen point.
    fn ticks_per_point(&self) -> f32 {
        self.ticks_per_beat() as f32 / 16.0
    }
}

/// Handles seeking on the musical ruler.
pub trait MusicalInteract {
    /// Seeks to an offset from the left edge of the current view.
    fn click_at_tick(&mut self, tick: f32);
}

/// Provides musical-ruler information and interaction.
pub trait MusicalRuler {
    /// Returns the ruler's musical information.
    fn info(&self) -> &dyn MusicalInfo;

    /// Returns the ruler's interaction handler.
    fn interact(&mut self) -> &mut dyn MusicalInteract;
}

/// Shows the view-relative musical ruler.
pub fn musical(ui: &mut egui::Ui, api: &mut dyn MusicalRuler) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    let width = ui.available_width();
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());

    let ticks_per_point = api.info().ticks_per_point();
    let visible_ticks = rect.width() * ticks_per_point;
    if response.clicked() || response.dragged() {
        if let Some(position) = response.interact_pointer_pos() {
            let tick = (((position.x - rect.min.x) / rect.width()) * visible_ticks).max(0.0);
            api.interact().click_at_tick(tick);
            response.mark_changed();
        }
    }

    let visuals = ui.style().noninteractive();
    let mut stroke = visuals.fg_stroke;
    let bar_color = stroke.color.linear_multiply(0.5);
    let step_color = stroke.color.linear_multiply(0.125);
    let bar_y = rect.center().y;
    let step_even_y = rect.top() + rect.height() * 0.25;
    let step_odd_y = rect.top() + rect.height() * 0.125;
    let mut steps = Steps::new(api.info(), rect.width(), crate::MIN_STEP_GAP);
    while let Some(step) = steps.next(api.info()) {
        let (y, color) = match step.index_in_bar {
            0 => (bar_y, bar_color),
            index if index % 2 == 0 => (step_even_y, step_color),
            _ => (step_odd_y, step_color),
        };
        stroke.color = color;
        let x = rect.left() + step.x;
        ui.painter()
            .line_segment([egui::pos2(x, rect.top()), egui::pos2(x, y)], stroke);
    }

    response
}

/// One line emitted by the view-relative ruler iterator.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    /// Index of this step within its bar; zero identifies the bar boundary.
    pub index_in_bar: usize,
    /// Tick offset from the left edge of the visible timeline.
    pub ticks: f32,
    /// Point offset from the left edge of the ruler.
    pub x: f32,
}

/// Iterator state for view-relative ruler steps.
#[derive(Clone, Debug)]
pub struct Steps {
    ticks_per_beat: f32,
    ticks_per_point: f32,
    visible_ticks: f32,
    min_step_ticks: f32,
    index_in_bar: usize,
    step_ticks: f32,
    bar: Bar,
    ticks: f32,
}

impl Steps {
    /// Creates a step iterator for a visible point length.
    pub fn new(api: &dyn MusicalInfo, visible_len: f32, min_step_gap: f32) -> Self {
        let ticks_per_beat = api.ticks_per_beat() as f32;
        let ticks_per_point = api.ticks_per_point();
        Self {
            ticks_per_beat,
            ticks_per_point,
            visible_ticks: ticks_per_point * visible_len,
            min_step_ticks: ticks_per_point * min_step_gap,
            index_in_bar: 0,
            step_ticks: 0.0,
            bar: api.bar_at_ticks(0.0),
            ticks: 0.0,
        }
    }

    /// Produces the next visible ruler step.
    pub fn next(&mut self, api: &dyn MusicalInfo) -> Option<Step> {
        'bars: loop {
            if self.index_in_bar == 0 {
                self.ticks = self.bar.tick_range.start;
                let mut beat_subdivisions = self.bar.time_sig.bottom / 4;
                self.step_ticks = self.ticks_per_beat / beat_subdivisions as f32;
                if self.step_ticks >= self.min_step_ticks {
                    loop {
                        let next_subdivisions = beat_subdivisions * 2;
                        let next_step_ticks = self.ticks_per_beat / next_subdivisions as f32;
                        if next_step_ticks <= self.min_step_ticks {
                            break;
                        }
                        beat_subdivisions = next_subdivisions;
                        self.step_ticks = next_step_ticks;
                    }
                } else {
                    self.step_ticks = self.bar.tick_range.end - self.bar.tick_range.start;
                }
            }

            loop {
                if self.ticks > self.visible_ticks {
                    return None;
                }
                if self.ticks >= self.bar.tick_range.end {
                    self.index_in_bar = 0;
                    self.bar = api.bar_at_ticks(self.bar.tick_range.end + 0.5);
                    continue 'bars;
                }
                let index_in_bar = self.index_in_bar;
                let ticks = self.ticks;
                self.index_in_bar += 1;
                self.ticks += self.step_ticks;
                if ticks < 0.0 {
                    continue;
                }
                return Some(Step {
                    index_in_bar,
                    ticks,
                    x: ticks / self.ticks_per_point,
                });
            }
        }
    }
}

/// Result of showing a musical ruler.
pub struct RulerResponse {
    /// Unsnapped absolute seek tick, clamped to the axis's minimum start.
    pub seek: Option<i64>,
}

/// Shows a labelled musical ruler and reports click-or-drag seeking.
///
/// Marks and thinned bar or bar-beat labels follow the visible grid. Seeking is
/// unsnapped and non-negative; a ruler with no area cannot paint or seek.
pub fn musical_with_axis(
    ui: &mut egui::Ui,
    axis: &TimeAxis,
    spec: &GridSpec<'_>,
    style: &RulerStyle,
) -> RulerResponse {
    let width = ui.available_width().max(0.0);
    let height = if style.height.is_finite() {
        style.height.max(0.0)
    } else {
        0.0
    };
    let has_area = width > 0.0 && height > 0.0;
    let sense = if has_area {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);

    if !has_area {
        return RulerResponse { seek: None };
    }

    let seek = if response.clicked() || response.dragged() {
        response
            .interact_pointer_pos()
            .map(|position| axis.x_to_tick(rect, position.x).max(0))
    } else {
        None
    };

    let steps = crate::grid::Steps::new(spec, axis, rect);
    let step_ticks = steps.step_ticks();
    let meter = spec.meter;
    let beat_points = meter.ticks_per_beat() as f64 / axis.ticks_per_point();
    let bar_points = meter.ticks_per_bar() as f64 / axis.ticks_per_point();
    let label_gap = f64::from(sanitize_gap(style.min_label_gap));
    let bar_beat_labels =
        step_ticks > 0 && step_ticks < meter.ticks_per_beat() && beat_points >= label_gap;
    let bar_stride = if bar_beat_labels {
        Some(1)
    } else {
        power_of_two_stride(bar_points, label_gap)
    };
    let bottom = rect.bottom();
    let height = rect.height();
    let bar_top = bottom - height * sanitize_proportion(style.bar_mark);
    let beat_top = bottom - height * sanitize_proportion(style.beat_mark);
    let division_top = bottom - height * sanitize_proportion(style.division_mark);

    for step in steps {
        let bar = step.tick.div_euclid(meter.ticks_per_bar());
        let beat_in_bar = step
            .tick
            .rem_euclid(meter.ticks_per_bar())
            .div_euclid(meter.ticks_per_beat());
        let (stroke, top) = match step.rank {
            StepRank::Bar => (style.bar, bar_top),
            StepRank::Beat => (style.beat, beat_top),
            StepRank::Division => (style.division, division_top),
        };
        ui.painter().line_segment(
            [egui::pos2(step.x, top), egui::pos2(step.x, bottom)],
            stroke,
        );

        let label = if bar_beat_labels && matches!(step.rank, StepRank::Bar | StepRank::Beat) {
            Some(format!(
                "{}.{}",
                bar.saturating_add(1),
                beat_in_bar.saturating_add(1)
            ))
        } else if step.rank == StepRank::Bar
            && bar_stride.is_some_and(|stride| bar.rem_euclid(stride) == 0)
        {
            Some(bar.saturating_add(1).to_string())
        } else {
            None
        };

        if let Some(label) = label {
            let galley =
                ui.painter()
                    .layout_no_wrap(label, style.label_font.clone(), style.label_color);
            let position = egui::pos2(
                step.x + style.label_offset.x,
                rect.top() + style.label_offset.y,
            );
            if position.x + galley.size().x <= rect.right() {
                ui.painter().galley(position, galley, style.label_color);
            }
        }
    }

    RulerResponse { seek }
}

/// Retains finite mark-height multipliers and replaces invalid values with zero.
fn sanitize_proportion(proportion: f32) -> f32 {
    if proportion.is_finite() {
        proportion
    } else {
        0.0
    }
}
