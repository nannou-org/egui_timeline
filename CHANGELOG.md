# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-10-25

### Changed

- Updated `egui` and `eframe` to 0.36 and `egui_plot` to 0.37, raising the minimum Rust version to 1.95.
- `TimelineCtx::plot_ticks` takes `impl egui::AsId` (`Hash + Debug`), matching `egui_plot::Plot::new`.

## [0.4.0] - 2026-08-07

### Added

- `Meter` and `TimeAxis` for uniform musical geometry and a caller-owned absolute integer-tick viewport.
- Opt-in `show_with_axis`, `paint_grid_with_axis`, `ruler::musical_with_axis`, and `playhead_at` APIs.
- Straight and triplet division chains, adaptive ranked grid steps, labelled ruler output, and neutral styles.
- A runnable eframe example and headless integration tests.

### Changed

- Updated `egui` and `eframe` to 0.34 and `egui_plot` to 0.35.
