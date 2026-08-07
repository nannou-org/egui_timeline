//! Run with `cargo run --example timeline`.
//!
//! Demonstrates the integer-tick axis, labelled ruler, adaptive straight
//! and triplet grids, track layout, transport, pan, zoom, and playhead.

use std::time::Duration;

use eframe::egui;
use egui_timeline::{
    ruler, GridSpec, Meter, Playhead, TimeAxis, Timeline, TimelineStyle, STRAIGHT_DIVISIONS,
    TRIPLET_DIVISIONS,
};

/// Selectable grid chain used by the demonstration controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chain {
    /// Binary subdivisions of one beat.
    Straight,
    /// Triplet subdivisions of one beat.
    Triplet,
}

/// Interactive state owned by the demonstration host.
struct TimelineExample {
    axis: TimeAxis,
    meter: Meter,
    style: TimelineStyle,
    chain: Chain,
    min_step_gap: f32,
    playing: bool,
    bpm: f32,
    playhead_tick: i64,
    fractional_tick: f64,
}

impl Default for TimelineExample {
    /// Builds the initial stopped demonstration state.
    fn default() -> Self {
        Self {
            axis: TimeAxis::new(0, TimeAxis::DEFAULT_TICKS_PER_POINT),
            meter: Meter::DEFAULT,
            style: TimelineStyle::default(),
            chain: Chain::Straight,
            min_step_gap: egui_timeline::MIN_STEP_GAP,
            playing: false,
            bpm: 120.0,
            playhead_tick: 0,
            fractional_tick: 0.0,
        }
    }
}

impl TimelineExample {
    /// Advances transport time in the beat domain while playing.
    fn advance(&mut self, delta_seconds: f32) {
        if !self.playing {
            return;
        }
        let ticks_per_second = self.meter.ticks_per_beat() as f64 * f64::from(self.bpm) / 60.0;
        self.fractional_tick += f64::from(delta_seconds) * ticks_per_second;
        let whole_ticks = self.fractional_tick.trunc() as i64;
        self.fractional_tick -= whole_ticks as f64;
        self.playhead_tick = self.playhead_tick.saturating_add(whole_ticks);
    }
}

impl eframe::App for TimelineExample {
    /// Draws one frame and applies transport and ruler interaction.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.advance(ui.ctx().input(|input| input.stable_dt));
        if self.playing {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }

        egui::Panel::top("controls").show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.chain, Chain::Straight, "Straight");
                ui.selectable_value(&mut self.chain, Chain::Triplet, "Triplet");
                ui.label("min gap");
                ui.add(egui::Slider::new(&mut self.min_step_gap, 1.0..=32.0));
                if ui
                    .button(if self.playing { "pause" } else { "play" })
                    .clicked()
                {
                    self.playing = !self.playing;
                }
                ui.label("bpm");
                ui.add(egui::DragValue::new(&mut self.bpm).range(30.0..=300.0));
                ui.monospace(format!(
                    "start {}  {:.3} ticks/point  playhead {}",
                    self.axis.start(),
                    self.axis.ticks_per_point(),
                    self.playhead_tick,
                ));
            });
            ui.weak("Drag the ruler to seek. Scroll sideways to pan; Ctrl+scroll to zoom.");
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let divisions = match self.chain {
                Chain::Straight => STRAIGHT_DIVISIONS,
                Chain::Triplet => TRIPLET_DIVISIONS,
            };
            let grid = GridSpec::new(self.meter)
                .divisions(divisions)
                .min_step_gap(self.min_step_gap);
            let mut seek = None;

            let set_playhead = Timeline::new()
                .header(120.0)
                .show_with_axis(ui, &mut self.axis, &self.style)
                .paint_grid_with_axis(&self.axis, &grid, &self.style.grid)
                .pinned_tracks(|tracks, ui| {
                    tracks
                        .next(ui)
                        .header(|ui| {
                            ui.label("ruler");
                        })
                        .show(|_, ui| {
                            seek =
                                ruler::musical_with_axis(ui, &self.axis, &grid, &self.style.ruler)
                                    .seek;
                        });
                })
                .tracks(|tracks, _viewport, ui| {
                    for (index, name) in ["Track A", "Track B", "Track C"].into_iter().enumerate() {
                        tracks
                            .next(ui)
                            .header(|ui| {
                                ui.add_space(8.0);
                                ui.label(name);
                            })
                            .show(|_, ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width().max(0.0), 54.0),
                                    egui::Sense::hover(),
                                );
                                let stroke = egui::Stroke::new(
                                    1.0_f32,
                                    egui::Color32::from_gray(70 + index as u8 * 12),
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    egui::CornerRadius::ZERO,
                                    stroke,
                                    egui::StrokeKind::Inside,
                                );
                            });
                    }
                });

            if let Some(tick) = seek {
                self.playhead_tick = tick;
                self.fractional_tick = 0.0;
            }
            set_playhead.playhead_at(
                ui,
                &self.axis,
                self.playhead_tick,
                Playhead::new(),
                &self.style.playhead,
            );
        });
    }
}

/// Starts the native demonstration window.
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "egui timeline 0.4",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<TimelineExample>::default())),
    )
}
