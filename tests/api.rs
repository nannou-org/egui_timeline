//! Compile and behavior coverage for the callback-driven timeline API.

use std::ops::RangeInclusive;

use egui_timeline::{
    playhead, ruler, BackgroundCtx, Bar, MusicalRuler, Playhead, PlayheadApi, TimeSig, Timeline,
    TimelineApi, TimelineCtx, TracksCtx, MIN_STEP_GAP,
};

/// Minimal callback owner used by the API probes.
struct State {
    start: f32,
    zoom: f32,
    playhead: f32,
    seeks: usize,
}

impl Default for State {
    /// Returns conventional ruler values.
    fn default() -> Self {
        Self {
            start: 0.0,
            zoom: 4.0,
            playhead: 0.0,
            seeks: 0,
        }
    }
}

impl ruler::MusicalInfo for State {
    /// Uses conventional 960 PPQN.
    fn ticks_per_beat(&self) -> u32 {
        960
    }

    /// Returns one relative 4/4 bar containing the requested tick.
    fn bar_at_ticks(&self, tick: f32) -> Bar {
        let start = (tick / 3_840.0).floor() * 3_840.0;
        Bar {
            tick_range: start..start + 3_840.0,
            time_sig: TimeSig { top: 4, bottom: 4 },
        }
    }

    /// Returns the callback-owned zoom.
    fn ticks_per_point(&self) -> f32 {
        self.zoom
    }
}

impl TimelineApi for State {
    /// Exposes this state as musical information.
    fn musical_ruler_info(&self) -> &dyn ruler::MusicalInfo {
        self
    }

    /// Records relative horizontal scrolling.
    fn shift_timeline_start(&mut self, ticks: f32) {
        self.start += ticks;
    }

    /// Records Ctrl-wheel zoom callbacks.
    fn zoom(&mut self, y_delta: f32) {
        self.zoom += y_delta;
    }
}

impl ruler::MusicalInteract for State {
    /// Records a view-relative ruler seek.
    fn click_at_tick(&mut self, tick: f32) {
        self.playhead = tick;
        self.seeks += 1;
    }
}

impl MusicalRuler for State {
    /// Exposes musical information to the ruler.
    fn info(&self) -> &dyn ruler::MusicalInfo {
        self
    }

    /// Exposes ruler interaction to the ruler.
    fn interact(&mut self) -> &mut dyn ruler::MusicalInteract {
        self
    }
}

impl playhead::Info for State {
    /// Returns the view-relative playhead location.
    fn playhead_ticks(&self) -> f32 {
        self.playhead
    }
}

impl playhead::Interaction for State {
    /// Records a view-relative playhead seek.
    fn set_playhead_ticks(&mut self, ticks: f32) {
        self.playhead = ticks;
        self.seeks += 1;
    }
}

/// Exercises object-safe coercions for every callback trait.
fn object_safety(
    _: &mut dyn TimelineApi,
    _: &dyn ruler::MusicalInfo,
    _: &mut dyn MusicalRuler,
    _: &mut dyn PlayheadApi,
) {
}

/// Exercises every callback API struct literal and root constant.
fn public_struct_literals(rect: egui::Rect) {
    let time_sig = TimeSig { top: 7, bottom: 8 };
    let _beats: f32 = time_sig.beats_per_bar();
    let _bar = Bar {
        tick_range: -10.0..20.0,
        time_sig,
    };
    let timeline = TimelineCtx {
        full_rect: rect,
        visible_ticks: 100.0,
    };
    let _background = BackgroundCtx {
        header_full_rect: Some(rect),
        timeline: &timeline,
    };
    let _tracks = TracksCtx {
        full_rect: rect,
        header_full_rect: Some(rect),
        timeline,
    };
    let _step = ruler::Step {
        index_in_bar: 0,
        ticks: 0.0,
        x: 0.0,
    };
    let _minimum_gap: f32 = MIN_STEP_GAP;
}

/// Compiles the complete 0.3 builder chain and return types unchanged.
fn callback_builder_chain(ui: &mut egui::Ui, state: &mut State) {
    let _: fn(&mut egui::Ui, &mut dyn PlayheadApi, egui::Rect, f32, Playhead) -> egui::Response =
        playhead::set;
    let response: egui::Response = Timeline::new()
        .header(128.0)
        .show(ui, state)
        .background(|context, ui| {
            let _: &TimelineCtx = context.timeline;
            ui.label("background");
        })
        .paint_grid(state)
        .pinned_tracks(|tracks, ui| {
            let _: egui::Response = ruler::musical(ui, state);
            let _: () = tracks
                .next(ui)
                .header(|ui| {
                    ui.label("header");
                })
                .show(|timeline, ui| {
                    let _: f32 = timeline.visible_ticks();
                    let _: egui_plot::Plot<'_> =
                        timeline.plot_ticks("plot", RangeInclusive::new(-1.0, 1.0));
                    ui.label("track");
                });
        })
        .tracks(|tracks, viewport, ui| {
            let _: egui::Rect = viewport;
            tracks.next(ui).show(|_, ui| {
                ui.label("scrolling track");
            });
        })
        .playhead(
            ui,
            state,
            Playhead::new()
                .extend_to_available_height(false)
                .extend_beyond_last_track(2.0)
                .width(3.0),
        );
    let _: bool = response.changed();

    let mut steps = ruler::Steps::new(state, 500.0, MIN_STEP_GAP);
    let _: Option<ruler::Step> = steps.next(state);
}

/// Proves that downstream 0.3 source continues to compile without migration.
#[test]
fn complete_v03_surface_compiles() {
    let _ = (
        object_safety,
        public_struct_literals,
        callback_builder_chain,
        Playhead::DEFAULT_EXTEND_BEYOND_LAST_TRACK,
        Playhead::DEFAULT_EXTEND_TO_AVAILABLE_HEIGHT,
        Playhead::DEFAULT_WIDTH,
    );
}

/// Builds a point-unit wheel event for callback behavior coverage.
fn wheel(delta: egui::Vec2, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta,
        phase: egui::TouchPhase::Move,
        modifiers,
    }
}

/// Wraps synthesized events in stable timeline geometry.
fn input(events: Vec<egui::Event>, modifiers: egui::Modifiers) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        events: std::iter::once(egui::Event::ModifiersChanged(modifiers))
            .chain(events)
            .collect(),
        ..egui::RawInput::default()
    }
}

/// Runs one timeline frame against stable geometry.
fn timeline_frame(
    context: &egui::Context,
    state: &mut State,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) {
    timeline_frame_in_rect(
        context,
        state,
        events,
        modifiers,
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
    );
}

/// Runs one timeline frame inside a caller-selected widget rectangle.
fn timeline_frame_in_rect(
    context: &egui::Context,
    state: &mut State,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    rect: egui::Rect,
) {
    context
        .run_ui(input(events, modifiers), |ui| {
            let mut child =
                ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
            Timeline::new().show(&mut child, state);
        })
        .textures_delta
        .clear();
}

/// Preserves horizontal-pan and Ctrl-wheel callback behavior after the egui update.
#[test]
fn scroll_callbacks_still_fire() {
    let context = egui::Context::default();
    let mut state = State::default();
    let pointer = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(pointer)],
        egui::Modifiers::default(),
    );

    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(7.0, 0.0), egui::Modifiers::default())],
        egui::Modifiers::default(),
    );
    assert_eq!(state.start, 28.0);

    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    let zoom_before = state.zoom;
    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(0.0, 7.0), ctrl)],
        ctrl,
    );
    assert!(state.zoom > zoom_before);
}

/// Runs one Ctrl-wheel gesture and returns the callback delta.
fn zoom_delta(delta: egui::Vec2) -> f32 {
    let context = egui::Context::default();
    let mut state = State::default();
    let pointer = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(pointer)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    let before = state.zoom;
    timeline_frame(&context, &mut state, vec![wheel(delta, ctrl)], ctrl);
    state.zoom - before
}

/// Preserves the subtraction of horizontal Ctrl-wheel movement.
#[test]
fn ctrl_wheel_preserves_axis_signs() {
    assert!((zoom_delta(egui::vec2(7.0, 0.0)) + 7.0).abs() < 0.001);
    assert!((zoom_delta(egui::vec2(2.0, 7.0)) - 5.0).abs() < 0.001);
}

/// Ignores native zoom input even while the Ctrl modifier is held.
#[test]
fn zoom_callback_ignores_native_zoom_events() {
    let context = egui::Context::default();
    let mut state = State::default();
    let pointer = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(pointer)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    let before = state.zoom;
    timeline_frame(&context, &mut state, vec![egui::Event::Zoom(2.0)], ctrl);
    assert_eq!(state.zoom, before);
}

/// Retains the wheel axis while egui smooths a large delta across frames.
#[test]
fn ctrl_wheel_smoothing_retains_horizontal_sign() {
    let context = egui::Context::default();
    let mut state = State::default();
    let pointer = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(pointer)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(20.0, 0.0), ctrl)],
        ctrl,
    );
    let after_event = state.zoom;
    timeline_frame(&context, &mut state, Vec::new(), ctrl);
    assert!(after_event < 4.0);
    assert!(state.zoom < after_event);
}

/// Keeps pending horizontal smoothing when vertical wheel input arrives.
#[test]
fn ctrl_wheel_smoothing_combines_axes_before_subtraction() {
    let context = egui::Context::default();
    let mut state = State::default();
    let pointer = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(pointer)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(20.0, 0.0), ctrl)],
        ctrl,
    );
    let after_horizontal = state.zoom;
    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(0.0, 3.0), ctrl)],
        ctrl,
    );
    assert!(state.zoom < after_horizontal);
}

/// Classifies all same-frame wheel movement by the final modifier state.
#[test]
fn ctrl_wheel_includes_movement_before_modifier_transition() {
    let context = egui::Context::default();
    let mut state = State::default();
    let pointer = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(pointer)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    let before = state.zoom;
    timeline_frame(
        &context,
        &mut state,
        vec![
            wheel(egui::vec2(3.0, 0.0), egui::Modifiers::default()),
            wheel(egui::vec2(0.0, 2.0), ctrl),
        ],
        ctrl,
    );
    assert!((state.zoom - before + 1.0).abs() < 0.001);
}

/// Converts page-wheel input using the viewport rather than widget height.
#[test]
fn ctrl_page_wheel_matches_equivalent_point_movement() {
    let delta_for = |unit, delta| {
        let context = egui::Context::default();
        let mut state = State::default();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 100.0));
        let pointer = egui::pos2(200.0, 50.0);
        timeline_frame_in_rect(
            &context,
            &mut state,
            vec![egui::Event::PointerMoved(pointer)],
            egui::Modifiers::default(),
            rect,
        );
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::default()
        };
        let before = state.zoom;
        timeline_frame_in_rect(
            &context,
            &mut state,
            vec![egui::Event::MouseWheel {
                unit,
                delta,
                phase: egui::TouchPhase::Move,
                modifiers: ctrl,
            }],
            ctrl,
            rect,
        );
        state.zoom - before
    };
    let page = delta_for(egui::MouseWheelUnit::Page, egui::vec2(0.0, 1.0));
    let points = delta_for(egui::MouseWheelUnit::Point, egui::vec2(0.0, 600.0));
    assert!((page - points).abs() < 0.001);
}

/// Drains pending wheel smoothing while the pointer is outside the timeline.
#[test]
fn ctrl_wheel_smoothing_does_not_replay_after_pointer_returns() {
    let context = egui::Context::default();
    let mut state = State::default();
    let inside = egui::pos2(200.0, 200.0);
    let outside = egui::pos2(900.0, 700.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(inside)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(20.0, 0.0), ctrl)],
        ctrl,
    );
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(outside)],
        ctrl,
    );
    for _ in 0..16 {
        timeline_frame(&context, &mut state, Vec::new(), ctrl);
    }
    let before_return = state.zoom;
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(inside)],
        ctrl,
    );
    assert_eq!(state.zoom, before_return);
}

/// Discards pending smoothing when the timeline skips rendering passes.
#[test]
fn ctrl_wheel_smoothing_does_not_replay_after_timeline_returns() {
    let context = egui::Context::default();
    let mut state = State::default();
    let inside = egui::pos2(200.0, 200.0);
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(inside)],
        egui::Modifiers::default(),
    );
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::default()
    };
    timeline_frame(
        &context,
        &mut state,
        vec![wheel(egui::vec2(20.0, 0.0), ctrl)],
        ctrl,
    );
    for _ in 0..16 {
        context
            .run_ui(input(Vec::new(), ctrl), |_| {})
            .textures_delta
            .clear();
    }
    let before_return = state.zoom;
    timeline_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerMoved(inside)],
        ctrl,
    );
    assert_eq!(state.zoom, before_return);
}

/// Runs one callback-driven ruler frame and reports response changes.
fn ruler_frame(context: &egui::Context, state: &mut State, events: Vec<egui::Event>) -> bool {
    let mut changed = false;
    context
        .run_ui(input(events, egui::Modifiers::default()), |ui| {
            changed = ruler::musical(ui, state).changed();
        })
        .textures_delta
        .clear();
    changed
}

/// Runs one callback-driven playhead frame and reports response changes.
fn playhead_frame(context: &egui::Context, state: &mut State, events: Vec<egui::Event>) -> bool {
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 40.0), egui::vec2(800.0, 100.0));
    let mut changed = false;
    context
        .run_ui(input(events, egui::Modifiers::default()), |ui| {
            changed =
                playhead::set(ui, state, rect, rect.bottom(), Playhead::new().width(8.0)).changed();
        })
        .textures_delta
        .clear();
    changed
}

/// Routes ruler and playhead clicks through their callback traits.
#[test]
fn ruler_and_playhead_interactions_call_callbacks() {
    let context = egui::Context::default();
    let mut state = State::default();
    let ruler_click = egui::pos2(101.0, 10.0);
    ruler_frame(&context, &mut state, Vec::new());
    ruler_frame(
        &context,
        &mut state,
        vec![
            egui::Event::PointerMoved(ruler_click),
            egui::Event::PointerButton {
                pos: ruler_click,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ],
    );
    assert!(ruler_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerButton {
            pos: ruler_click,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }],
    ));
    assert_eq!(state.seeks, 1);

    let playhead_click = egui::pos2(state.playhead / state.zoom, 60.0);
    playhead_frame(&context, &mut state, Vec::new());
    playhead_frame(
        &context,
        &mut state,
        vec![
            egui::Event::PointerMoved(playhead_click),
            egui::Event::PointerButton {
                pos: playhead_click,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ],
    );
    assert!(playhead_frame(
        &context,
        &mut state,
        vec![egui::Event::PointerButton {
            pos: playhead_click,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }],
    ));
    assert_eq!(state.seeks, 2);
}
