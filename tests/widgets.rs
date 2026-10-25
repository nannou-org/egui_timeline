//! Headless egui tests for timeline widgets and input.

use egui::{
    Color32, Event, FullOutput, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect,
    Shape, Stroke, TouchPhase, Vec2,
};
use egui_timeline::{
    ruler, GridSpec, Meter, Playhead, TimeAxis, Timeline, TimelineStyle, TRIPLET_DIVISIONS,
};

const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));

/// Wraps synthesized events in a stable screen rectangle.
fn input(events: Vec<Event>, modifiers: Modifiers) -> RawInput {
    RawInput {
        screen_rect: Some(SCREEN),
        events: std::iter::once(Event::ModifiersChanged(modifiers))
            .chain(events)
            .collect(),
        ..RawInput::default()
    }
}

/// Builds a point-unit mouse-wheel movement.
fn wheel(delta: Vec2, modifiers: Modifiers) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta,
        phase: TouchPhase::Move,
        modifiers,
    }
}

/// Builds one primary-pointer transition.
fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    }
}

/// Borrows every leaf shape from egui's nested paint output.
fn flatten_shapes(output: &FullOutput) -> Vec<&Shape> {
    /// Recursively visits nested shape groups.
    fn flatten<'a>(shape: &'a Shape, destination: &mut Vec<&'a Shape>) {
        if let Shape::Vec(shapes) = shape {
            for shape in shapes {
                flatten(shape, destination);
            }
        } else {
            destination.push(shape);
        }
    }

    let mut shapes = Vec::new();
    for clipped in &output.shapes {
        flatten(&clipped.shape, &mut shapes);
    }
    shapes
}

/// Exercises every integer-tick builder stage and both grid chains.
#[test]
fn absolute_timeline_chain_lays_out_and_paints() {
    for divisions in [egui_timeline::STRAIGHT_DIVISIONS, TRIPLET_DIVISIONS] {
        let context = egui::Context::default();
        let mut axis = TimeAxis::new(0, 4.0);
        let mut style = TimelineStyle::default();
        style.grid.bar = Stroke::new(1.0_f32, Color32::RED);
        style.grid.beat = Stroke::new(1.0_f32, Color32::GREEN);
        style.grid.division = Stroke::new(1.0_f32, Color32::BLUE);
        style.playhead.color = Color32::YELLOW;
        let grid = GridSpec::new(Meter::DEFAULT).divisions(divisions);
        let mut background_called = false;
        let mut playhead_rect = Rect::NOTHING;

        let mut output = context.run_ui(input(Vec::new(), Modifiers::default()), |ui| {
            let set_playhead = Timeline::new()
                .header(120.0)
                .show_with_axis(ui, &mut axis, &style)
                .background(|background, _| {
                    background_called = true;
                    assert!(background.header_full_rect.is_some());
                    assert!(background.timeline.full_rect.width() > 0.0);
                })
                .paint_grid_with_axis(&axis, &grid, &style.grid)
                .pinned_tracks(|tracks, ui| {
                    tracks.next(ui).show(|_, ui| {
                        let _ = ruler::musical_with_axis(ui, &axis, &grid, &style.ruler);
                    });
                })
                .tracks(|tracks, _, ui| {
                    for _ in 0..3 {
                        tracks
                            .next(ui)
                            .header(|ui| {
                                ui.allocate_space(Vec2::new(20.0, 24.0));
                            })
                            .show(|_, ui| {
                                ui.allocate_space(Vec2::new(100.0, 32.0));
                            });
                    }
                });
            playhead_rect = set_playhead
                .playhead_at(ui, &axis, 960, Playhead::new().width(8.0), &style.playhead)
                .rect;
        });

        output.textures_delta.clear();
        assert!(background_called);
        assert!((playhead_rect.center().x - 360.0).abs() < 0.001);
        let shapes = flatten_shapes(&output);
        assert!(shapes.iter().any(|shape| {
            matches!(shape, Shape::LineSegment { stroke, .. } if stroke.color == Color32::RED)
        }));
        assert!(shapes.iter().any(|shape| {
            matches!(shape, Shape::LineSegment { stroke, .. } if stroke.color == Color32::GREEN)
        }));
        assert!(shapes.iter().any(|shape| {
            matches!(shape, Shape::LineSegment { stroke, .. } if stroke.color == Color32::BLUE)
        }));
        assert!(shapes
            .iter()
            .any(|shape| { matches!(shape, Shape::Rect(rect) if rect.fill == Color32::YELLOW) }));
    }
}

/// Runs one integer-tick input frame against a caller-owned axis.
fn show_axis_frame(
    context: &egui::Context,
    axis: &mut TimeAxis,
    events: Vec<Event>,
    modifiers: Modifiers,
) {
    let style = TimelineStyle::default();
    context
        .run_ui(input(events, modifiers), |ui| {
            Timeline::new().show_with_axis(ui, axis, &style);
        })
        .textures_delta
        .clear();
}

/// Applies horizontal pan and cursor-anchored Ctrl-wheel zoom to the owned axis.
#[test]
fn absolute_timeline_input_mutates_the_axis() {
    let context = egui::Context::default();
    let pointer = Pos2::new(300.0, 200.0);
    let mut axis = TimeAxis::new(10_000, 4.0);
    show_axis_frame(
        &context,
        &mut axis,
        vec![Event::PointerMoved(pointer)],
        Modifiers::default(),
    );

    let zoom_modifiers = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::default()
    };
    let anchor = axis.x_to_tick(SCREEN, pointer.x);
    show_axis_frame(
        &context,
        &mut axis,
        vec![wheel(Vec2::new(0.0, 7.0), zoom_modifiers)],
        zoom_modifiers,
    );
    assert!(axis.ticks_per_point() < 4.0);
    assert_eq!(axis.x_to_tick(SCREEN, pointer.x), anchor);

    let start = axis.start();
    show_axis_frame(
        &context,
        &mut axis,
        vec![wheel(Vec2::new(7.0, 0.0), Modifiers::default())],
        Modifiers::default(),
    );
    assert!(axis.start() > start);
}

/// Shows one absolute ruler frame and captures its raw seek result.
fn ruler_frame(context: &egui::Context, events: Vec<Event>) -> Option<i64> {
    let axis = TimeAxis::new(100, 4.0);
    let style = TimelineStyle::default();
    let grid = GridSpec::new(Meter::DEFAULT);
    let mut seek = None;
    context
        .run_ui(input(events, Modifiers::default()), |ui| {
            seek = ruler::musical_with_axis(ui, &axis, &grid, &style.ruler).seek;
        })
        .textures_delta
        .clear();
    seek
}

/// Reports unsnapped absolute ruler seeks after egui's warmed hit-testing pass.
#[test]
fn absolute_ruler_reports_raw_seek_ticks() {
    let context = egui::Context::default();
    let click = Pos2::new(101.0, 10.0);
    ruler_frame(&context, Vec::new());
    ruler_frame(
        &context,
        vec![Event::PointerMoved(click), pointer_button(click, true)],
    );
    let seek = ruler_frame(&context, vec![pointer_button(click, false)]);
    assert_eq!(seek, Some(504));
}

/// Keeps the absolute playhead hover-only so editor-owned gestures remain authoritative.
#[test]
fn absolute_playhead_is_non_interactive() {
    let context = egui::Context::default();
    let mut axis = TimeAxis::new(0, 4.0);
    let style = TimelineStyle::default();
    let mut sense = egui::Sense::click();
    context
        .run_ui(input(Vec::new(), Modifiers::default()), |ui| {
            sense = Timeline::new()
                .show_with_axis(ui, &mut axis, &style)
                .tracks(|_, _, _| {})
                .playhead_at(ui, &axis, 100, Playhead::new(), &style.playhead)
                .sense;
        })
        .textures_delta
        .clear();
    assert_eq!(sense, egui::Sense::hover());
}
