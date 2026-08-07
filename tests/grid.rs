//! Adaptive absolute-grid selection, rank, and termination contracts.

use egui_timeline::{
    grid::Steps, Division, GridSpec, Meter, StepRank, TimeAxis, MIN_STEP_GAP, STRAIGHT_DIVISIONS,
    TRIPLET_DIVISIONS,
};

/// Builds a finite positive-width test rectangle.
fn rect(width: f32) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(37.0, 5.0), egui::vec2(width, 40.0))
}

/// Selects the finest straight interval that clears the configured gap.
#[test]
fn straight_grid_adapts_to_zoom() {
    let meter = Meter::DEFAULT;
    for (ticks_per_point, expected) in [(0.05, 15), (4.0, 30), (30.0, 120), (300.0, 3840)] {
        let axis = TimeAxis::new(0, ticks_per_point);
        let steps = Steps::new(&GridSpec::new(meter), &axis, rect(300.0));
        assert_eq!(steps.step_ticks(), expected);
        assert!(steps.step_ticks() as f64 / axis.ticks_per_point() >= 4.0);
    }
}

/// Widens beyond one bar by powers of two when a bar is narrower than the gap.
#[test]
fn straight_grid_widens_to_multiple_bars() {
    let axis = TimeAxis::new(0, 4096.0);
    let steps = Steps::new(&GridSpec::new(Meter::DEFAULT), &axis, rect(300.0));
    assert_eq!(steps.step_ticks(), Meter::DEFAULT.ticks_per_bar() * 8);
}

/// Continues widening beyond the old arbitrary `2^20` stride.
#[test]
fn straight_grid_uses_every_representable_power_of_two_stride() {
    let meter = Meter::new(1, 1);
    let axis = TimeAxis::new(0, 4096.0);
    let spec = GridSpec::new(meter).min_step_gap(300.0);
    assert_eq!(Steps::new(&spec, &axis, rect(300.0)).step_ticks(), 1 << 21);
}

/// Rejects a multi-bar interval whose tick length cannot be represented.
#[test]
fn straight_grid_rejects_overflowing_bar_strides() {
    let meter = Meter::new(1 << 32, 1);
    let axis = TimeAxis::new(0, 4096.0);
    let spec = GridSpec::new(meter).min_step_gap(4.0e18_f32);
    assert_eq!(Steps::new(&spec, &axis, rect(300.0)).step_ticks(), 0);
}

/// Emits exact integer triplet intervals at conventional resolution.
#[test]
fn triplet_grid_uses_exact_tick_intervals() {
    let meter = Meter::DEFAULT;
    let axis = TimeAxis::new(0, 50.0);
    let spec = GridSpec::new(meter).divisions(TRIPLET_DIVISIONS);
    let steps: Vec<_> = Steps::new(&spec, &axis, rect(300.0)).collect();
    assert!(steps
        .windows(2)
        .all(|pair| pair[1].tick - pair[0].tick == 320));
    assert!(steps.iter().all(|step| step.tick % 320 == 0));
}

/// Ignores candidate ordering, duplicates, invalid values, and inexact fractions.
#[test]
fn division_selection_uses_only_valid_exact_candidates() {
    let meter = Meter::new(960, 4);
    let axis = TimeAxis::new(0, 20.0);
    let baseline = Steps::new(&GridSpec::new(meter), &axis, rect(500.0)).step_ticks();
    let shuffled = [
        Division::new(1, 0),
        Division::new(1, 64),
        Division::new(1, 2),
        Division::new(1, 8),
        Division::new(1, 1),
        Division::new(1, 4),
        Division::new(1, 7),
        Division::new(1, 8),
    ];
    assert_eq!(
        Steps::new(
            &GridSpec::new(meter).divisions(&shuffled),
            &axis,
            rect(500.0),
        )
        .step_ticks(),
        baseline
    );
}

/// Reduces exact fractions before multiplying their large components.
#[test]
fn division_selection_accepts_large_reducible_fractions() {
    let candidate = [Division::new(i64::MAX, i64::MAX)];
    let axis = TimeAxis::new(0, 100.0);
    let spec = GridSpec::new(Meter::DEFAULT).divisions(&candidate);
    assert_eq!(Steps::new(&spec, &axis, rect(500.0)).step_ticks(), 960);
}

/// Derives bar, beat, and division emphasis from absolute tick divisibility.
#[test]
fn fixed_steps_report_musical_rank() {
    let meter = Meter::DEFAULT;
    let axis = TimeAxis::new(0, 2.0);
    let emitted: Vec<_> = Steps::fixed(meter, 240, &axis, rect(2000.0)).collect();
    let rank = |tick| emitted.iter().find(|step| step.tick == tick).unwrap().rank;
    assert_eq!(rank(0), StepRank::Bar);
    assert_eq!(rank(960), StepRank::Beat);
    assert_eq!(rank(480), StepRank::Division);
    assert_eq!(rank(3840), StepRank::Bar);
}

/// Anchors fixed intervals to tick zero after a non-aligned viewport pan.
#[test]
fn fixed_steps_remain_zero_anchored() {
    let meter = Meter::DEFAULT;
    let axis = TimeAxis::new(110, 1.0);
    let ticks: Vec<_> = Steps::fixed(meter, 60, &axis, rect(300.0))
        .map(|step| step.tick)
        .collect();
    assert_eq!(ticks[0], 120);
    assert!(ticks.iter().all(|tick| tick % 60 == 0));
}

/// Bounds all iterators and rejects unusable geometry or fixed intervals.
#[test]
fn step_iteration_is_bounded() {
    let meter = Meter::DEFAULT;
    let axis = TimeAxis::new(0, TimeAxis::DEFAULT_TICKS_PER_POINT);
    for invalid in [egui::Rect::NOTHING, egui::Rect::EVERYTHING, rect(0.0)] {
        assert_eq!(Steps::new(&GridSpec::new(meter), &axis, invalid).count(), 0);
    }
    assert_eq!(Steps::fixed(meter, 0, &axis, rect(100.0)).count(), 0);
    let absurd = rect(1.0e9);
    assert_eq!(Steps::fixed(meter, 1, &axis, absurd).count(), 100_000);
}

/// Emits at most one aligned rail tick and never repeats a saturated value.
#[test]
fn fixed_steps_terminate_at_the_integer_rail() {
    let axis = TimeAxis::new(i64::MAX, 1.0);
    assert_eq!(
        Steps::fixed(Meter::DEFAULT, 60, &axis, rect(100.0)).count(),
        0
    );
    assert_eq!(
        Steps::new(&GridSpec::new(Meter::DEFAULT), &axis, rect(100.0)).count(),
        0
    );

    let steps: Vec<_> = Steps::fixed(Meter::DEFAULT, 1, &axis, rect(100.0)).collect();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].tick, i64::MAX);
}

/// Pins public defaults for meter and both supported division chains.
#[test]
fn public_grid_defaults_are_stable() {
    let meter = Meter::new(0, 0);
    assert_eq!((meter.ticks_per_beat(), meter.beats_per_bar()), (1, 1));
    assert_eq!(Meter::DEFAULT.ticks_per_bar(), 3840);
    assert_eq!(MIN_STEP_GAP, 4.0);
    assert!(!STRAIGHT_DIVISIONS.is_empty());
    assert!(!TRIPLET_DIVISIONS.is_empty());
}
