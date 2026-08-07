//! Numeric contracts for the integer-tick timeline axis.

use egui_timeline::TimeAxis;

/// Builds a stable screen rectangle with a nonzero origin.
fn rect() -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(20.0, 10.0), egui::vec2(400.0, 80.0))
}

/// Keeps absolute coordinate conversion within one tick-per-point interval.
#[test]
fn screen_and_tick_conversion_round_trip() {
    for ticks_per_point in [0.05, 0.5, 1.0, 4.0, 128.0, 4096.0] {
        let axis = TimeAxis::new(12_345, ticks_per_point);
        let visible = axis.visible_range(rect());
        for sample in 0..=32 {
            let tick = visible.start + (visible.end - visible.start) * sample / 32;
            let round_trip = axis.x_to_tick(rect(), axis.tick_to_x(rect(), tick));
            assert!((round_trip - tick).unsigned_abs() as f64 <= ticks_per_point.ceil());
        }
    }
}

/// Preserves the cursor anchor while zooming away from the zero boundary.
#[test]
fn zoom_about_x_preserves_the_anchor() {
    let mut axis = TimeAxis::new(10_000, 4.0);
    let x = rect().left() + 137.0;
    let anchor = axis.x_to_tick(rect(), x);
    axis.zoom_about_x(rect(), x, 0.5);
    assert_eq!(axis.ticks_per_point(), 2.0);
    assert_eq!(axis.x_to_tick(rect(), x), anchor);
}

/// Clamps finite zoom factors whose multiplication reaches a numeric rail.
#[test]
fn zoom_about_x_clamps_extreme_finite_factors() {
    let x = rect().left() + 137.0;
    for (factor, expected) in [(f64::MAX, 4096.0), (f64::MIN_POSITIVE, 0.05)] {
        let mut axis = TimeAxis::new(10_000, 4.0);
        let anchor = axis.x_to_tick(rect(), x);
        axis.zoom_about_x(rect(), x, factor);
        assert_eq!(axis.ticks_per_point(), expected);
        if expected == 4096.0 {
            assert_eq!(axis.start(), 0);
        } else {
            assert_eq!(axis.x_to_tick(rect(), x), anchor);
        }
    }
}

/// Clamps the caller-owned viewport to supported non-negative values.
#[test]
fn setters_and_pan_sanitize_viewport_state() {
    let mut axis = TimeAxis::new(-100, f64::NAN);
    assert_eq!(axis.start(), 0);
    assert_eq!(axis.ticks_per_point(), TimeAxis::DEFAULT_TICKS_PER_POINT);

    axis.set_start(100);
    axis.pan_points(3.0);
    assert_eq!(axis.start(), 112);
    axis.pan_points(-1000.0);
    assert_eq!(axis.start(), 0);

    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        axis.set_ticks_per_point(invalid);
        assert_eq!(axis.ticks_per_point(), TimeAxis::DEFAULT_TICKS_PER_POINT);
    }
}

/// Returns a half-open absolute range covering the complete screen rectangle.
#[test]
fn visible_range_rounds_its_end_up() {
    let axis = TimeAxis::new(100, 1.5);
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(101.0, 20.0));
    assert_eq!(axis.visible_range(rect), 100..252);
}
