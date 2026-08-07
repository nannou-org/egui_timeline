//! Adaptive musical-grid selection and bounded visible-step iteration.

use crate::{Meter, TimeAxis};

/// A candidate grid interval expressed as a fraction of one beat.
pub struct Division {
    /// Numerator of the beat fraction.
    numer: i64,
    /// Denominator of the beat fraction.
    denom: i64,
}

impl Division {
    /// Creates a division. Invalid values are ignored during grid selection.
    pub const fn new(numer: i64, denom: i64) -> Self {
        Self { numer, denom }
    }

    /// Returns the exact tick length, or `None` when it cannot be represented.
    fn ticks(&self, meter: Meter) -> Option<i64> {
        if self.numer <= 0 || self.denom <= 0 {
            return None;
        }
        let mut common = (meter.ticks_per_beat(), self.denom);
        while common.1 != 0 {
            common = (common.1, common.0 % common.1);
        }
        let ticks = meter.ticks_per_beat() / common.0;
        let denominator = self.denom / common.0;
        (self.numer % denominator == 0).then(|| ticks.checked_mul(self.numer / denominator))?
    }
}

/// Straight beat divisions, ordered from coarse to fine.
pub const STRAIGHT_DIVISIONS: &[Division] = &[
    Division::new(1, 1),
    Division::new(1, 2),
    Division::new(1, 4),
    Division::new(1, 8),
    Division::new(1, 16),
    Division::new(1, 32),
    Division::new(1, 64),
];

/// Triplet beat divisions, ordered from coarse to fine.
pub const TRIPLET_DIVISIONS: &[Division] = &[
    Division::new(4, 3),
    Division::new(2, 3),
    Division::new(1, 3),
    Division::new(1, 6),
    Division::new(1, 12),
    Division::new(1, 24),
    Division::new(1, 48),
];

/// Adaptive grid configuration.
pub struct GridSpec<'a> {
    /// Uniform musical geometry for rank and interval calculations.
    pub(crate) meter: Meter,
    /// Candidate sub-bar intervals. Their order does not affect selection.
    pub(crate) divisions: &'a [Division],
    /// Minimum screen-point spacing between adjacent lines.
    pub(crate) min_step_gap: f32,
}

impl<'a> GridSpec<'a> {
    /// Creates a straight grid at the default minimum gap.
    pub fn new(meter: Meter) -> Self {
        Self {
            meter,
            divisions: STRAIGHT_DIVISIONS,
            min_step_gap: crate::MIN_STEP_GAP,
        }
    }
    /// Replaces the candidate division chain.
    pub fn divisions(mut self, divisions: &'a [Division]) -> Self {
        self.divisions = divisions;
        self
    }
    /// Replaces the minimum gap, treating invalid or sub-point values as one.
    pub fn min_step_gap(mut self, gap: f32) -> Self {
        self.min_step_gap = sanitize_gap(gap);
        self
    }
}

/// Musical emphasis of a grid line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepRank {
    /// The start of a bar.
    Bar,
    /// The start of a beat that is not also a bar.
    Beat,
    /// A finer subdivision.
    Division,
}

/// One visible musical grid line.
pub struct Step {
    /// Absolute timeline tick.
    pub tick: i64,
    /// Absolute screen x.
    pub x: f32,
    /// Musical emphasis derived from tick divisibility.
    pub rank: StepRank,
}

/// Grid lines visible in one finite rectangle, bounded by a fixed hard cap.
pub struct Steps {
    meter: Meter,
    axis: TimeAxis,
    rect: egui::Rect,
    next_tick: i64,
    step_ticks: i64,
    remaining: i64,
}

impl Steps {
    /// Maximum number of steps emitted by one iterator.
    const MAX_STEPS: i64 = 100_000;

    /// Chooses the finest exact division meeting the gap, widening narrow bars.
    pub fn new(spec: &GridSpec<'_>, axis: &TimeAxis, rect: egui::Rect) -> Self {
        if !valid_rect(rect) {
            return Self::empty(spec.meter, *axis, rect);
        }
        let gap = f64::from(sanitize_gap(spec.min_step_gap));
        let ticks_per_bar = spec.meter.ticks_per_bar();
        let bar_points = ticks_per_bar as f64 / axis.ticks_per_point();
        let step_ticks = if bar_points < gap {
            match power_of_two_stride(bar_points, gap)
                .and_then(|bars| bars.checked_mul(ticks_per_bar))
            {
                Some(step_ticks) => step_ticks,
                None => return Self::empty(spec.meter, *axis, rect),
            }
        } else {
            spec.divisions
                .iter()
                .filter_map(|division| division.ticks(spec.meter))
                .filter(|step| {
                    *step < ticks_per_bar
                        && ticks_per_bar % *step == 0
                        && *step as f64 / axis.ticks_per_point() >= gap
                })
                .min()
                .unwrap_or(ticks_per_bar)
        };
        Self::with_step(spec.meter, step_ticks, *axis, rect)
    }

    /// Emits a fixed positive interval, or no steps for invalid inputs.
    pub fn fixed(meter: Meter, step_ticks: i64, axis: &TimeAxis, rect: egui::Rect) -> Self {
        if step_ticks <= 0 || !valid_rect(rect) {
            return Self::empty(meter, *axis, rect);
        }
        Self::with_step(meter, step_ticks, *axis, rect)
    }
    /// Returns the emitted interval, or zero for an empty iterator.
    pub fn step_ticks(&self) -> i64 {
        self.step_ticks
    }

    /// Builds a bounded iterator for a validated fixed interval and rectangle.
    fn with_step(meter: Meter, step_ticks: i64, axis: TimeAxis, rect: egui::Rect) -> Self {
        let remainder = axis.start().rem_euclid(step_ticks);
        let offset = if remainder == 0 {
            0
        } else {
            step_ticks - remainder
        };
        let Some(first_tick) = axis.start().checked_add(offset) else {
            return Self::empty(meter, axis, rect);
        };
        Self {
            meter,
            axis,
            rect,
            next_tick: first_tick,
            step_ticks,
            remaining: Self::MAX_STEPS,
        }
    }

    /// Builds an iterator that exposes no interval and emits no steps.
    fn empty(meter: Meter, axis: TimeAxis, rect: egui::Rect) -> Self {
        Self {
            meter,
            axis,
            rect,
            next_tick: 0,
            step_ticks: 0,
            remaining: 0,
        }
    }
}

impl Iterator for Steps {
    type Item = Step;

    /// Emits the next in-bounds tick and its musical rank, within the fixed budget.
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining <= 0 {
            return None;
        }
        let tick = self.next_tick;
        let x = self.axis.tick_to_x(self.rect, tick);
        if x > self.rect.right() {
            self.remaining = 0;
            return None;
        }
        self.remaining -= 1;
        match tick.checked_add(self.step_ticks) {
            Some(next_tick) => self.next_tick = next_tick,
            None => self.remaining = 0,
        }
        let ticks_per_bar = self.meter.ticks_per_bar();
        let tick_in_bar = tick.rem_euclid(ticks_per_bar);
        let rank = if tick_in_bar == 0 {
            StepRank::Bar
        } else if tick_in_bar.rem_euclid(self.meter.ticks_per_beat()) == 0 {
            StepRank::Beat
        } else {
            StepRank::Division
        };
        Some(Step { tick, x, rank })
    }
}

/// Returns the smallest representable power-of-two stride meeting `minimum`.
pub(crate) fn power_of_two_stride(spacing: f64, minimum: f64) -> Option<i64> {
    let mut stride = 1_i64;
    while stride as f64 * spacing < minimum {
        stride = stride.checked_mul(2)?;
    }
    Some(stride)
}

/// Clamps a finite visual gap to at least one point and replaces invalid gaps.
pub(crate) fn sanitize_gap(gap: f32) -> f32 {
    if gap.is_finite() {
        gap.max(1.0)
    } else {
        1.0
    }
}

/// Reports whether a rectangle is finite, ordered, and has positive width.
fn valid_rect(rect: egui::Rect) -> bool {
    rect.is_finite() && rect.min.x <= rect.max.x && rect.min.y <= rect.max.y && rect.width() > 0.0
}
