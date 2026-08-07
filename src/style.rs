//! Neutral style values for all timeline-owned visuals.

/// Neutral style values for a complete timeline.
pub struct TimelineStyle {
    /// Fill behind the complete widget.
    pub background: egui::Color32,
    /// Separator under the pinned-track region.
    pub separator: egui::Stroke,
    /// Grid-line styling by musical rank.
    pub grid: GridStyle,
    /// Ruler geometry, marks, and labels.
    pub ruler: RulerStyle,
    /// Playhead colour.
    pub playhead: PlayheadStyle,
}

/// Grid-line strokes by musical rank.
pub struct GridStyle {
    /// Bar-line stroke.
    pub bar: egui::Stroke,
    /// Beat-line stroke.
    pub beat: egui::Stroke,
    /// Subdivision-line stroke.
    pub division: egui::Stroke,
}

/// Musical-ruler layout and appearance settings.
pub struct RulerStyle {
    /// Allocated ruler height in screen points.
    pub height: f32,
    /// Bar-mark stroke.
    pub bar: egui::Stroke,
    /// Beat-mark stroke.
    pub beat: egui::Stroke,
    /// Subdivision-mark stroke.
    pub division: egui::Stroke,
    /// Bar-mark height as a fraction of the ruler height.
    pub bar_mark: f32,
    /// Beat-mark height as a fraction of the ruler height.
    pub beat_mark: f32,
    /// Subdivision-mark height as a fraction of the ruler height.
    pub division_mark: f32,
    /// Font used for bar and beat labels.
    pub label_font: egui::FontId,
    /// Label colour.
    pub label_color: egui::Color32,
    /// Offset from each labelled line to the label's top-left corner.
    pub label_offset: egui::Vec2,
    /// Minimum point spacing before labels are thinned.
    pub min_label_gap: f32,
}

/// Playhead appearance settings.
pub struct PlayheadStyle {
    /// Flat playhead fill colour.
    pub color: egui::Color32,
}

impl Default for TimelineStyle {
    /// Returns the complete neutral dark style.
    fn default() -> Self {
        Self {
            background: egui::Color32::from_gray(27),
            separator: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(60)),
            grid: GridStyle::default(),
            ruler: RulerStyle::default(),
            playhead: PlayheadStyle::default(),
        }
    }
}

impl Default for GridStyle {
    /// Returns the neutral bar, beat, and subdivision strokes.
    fn default() -> Self {
        Self {
            bar: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(90)),
            beat: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(60)),
            division: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(45)),
        }
    }
}

impl Default for RulerStyle {
    /// Returns the neutral ruler geometry, strokes, and label appearance.
    fn default() -> Self {
        Self {
            height: 20.0,
            bar: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(160)),
            beat: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(120)),
            division: egui::Stroke::new(1.0_f32, egui::Color32::from_gray(90)),
            bar_mark: 1.0,
            beat_mark: 0.5,
            division_mark: 0.25,
            label_font: egui::FontId::proportional(10.0),
            label_color: egui::Color32::from_gray(160),
            label_offset: egui::vec2(3.0, 1.0),
            min_label_gap: 48.0,
        }
    }
}

impl Default for PlayheadStyle {
    /// Returns the neutral playhead color.
    fn default() -> Self {
        Self {
            color: egui::Color32::from_gray(220),
        }
    }
}
