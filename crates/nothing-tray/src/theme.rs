//! The app's look, inspired by Nothing products: monochrome colours,
//! a monospaced font for the text and a dot-matrix font for big figures.
//!
//! Both fonts are free (SIL Open Font License) and built into the binary:
//! - Space Mono, for all the text;
//! - Doto, a dot-matrix font, for the earbuds name and battery levels.
//!
//! The window follows the desktop's light or dark theme: black on white, or
//! white on black.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, RichText, Shadow, Stroke,
    TextStyle, Theme, Vec2, Visuals,
};

const SPACE_MONO: &[u8] = include_bytes!("../assets/fonts/SpaceMono-Regular.ttf");
const SPACE_MONO_BOLD: &[u8] = include_bytes!("../assets/fonts/SpaceMono-Bold.ttf");
const DOTO: &[u8] = include_bytes!("../assets/fonts/Doto.ttf");

/// Red used only for errors, like the accent of Nothing products.
pub const ERROR_COLOR: Color32 = Color32::from_rgb(215, 25, 33);

/// Corner radius of buttons and other small widgets.
pub const WIDGET_RADIUS: u8 = 6;
/// Corner radius of cards.
pub const CARD_RADIUS: u8 = 12;

/// Applies the fonts, the text sizes and the colours of both themes.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals_of(Theme::Dark, visuals(Palette::DARK));
    ctx.set_visuals_of(Theme::Light, visuals(Palette::LIGHT));
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(8.0, 5.0);
        style.spacing.interact_size.y = 24.0;
        style.spacing.slider_width = 200.0;
        style.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, dot())),
            (TextStyle::Body, FontId::proportional(12.0)),
            (TextStyle::Button, FontId::proportional(11.0)),
            (TextStyle::Monospace, FontId::monospace(12.0)),
            (TextStyle::Small, FontId::proportional(10.0)),
        ]
        .into();
    });
}

/// Bold Space Mono.
pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Doto, the dot-matrix font. Only readable in large sizes.
pub fn dot() -> FontFamily {
    FontFamily::Name("dot".into())
}

/// Text in the dot-matrix font, with round dots like Nothing's own font.
pub fn dot_text(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into().to_uppercase())
        .family(dot())
        .size(size)
        .variation("ROND", 100.0)
}

/// Upper-case text with a little letter spacing, used for labels and buttons.
pub fn caps(text: impl AsRef<str>) -> RichText {
    RichText::new(text.as_ref().to_uppercase()).extra_letter_spacing(0.5)
}

/// Small grey upper-case caption, used for card titles and field names.
pub fn caption(text: impl AsRef<str>) -> RichText {
    caps(text).small().weak()
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    for (name, data) in [
        ("space-mono", SPACE_MONO),
        ("space-mono-bold", SPACE_MONO_BOLD),
        ("doto", DOTO),
    ] {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(data)));
    }

    // egui's own fonts stay as fallbacks for symbols and emoji (⚡, ⚠, 🎧…).
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let with_fallbacks = |first: &[&str]| -> Vec<String> {
        let mut list: Vec<String> = first.iter().map(|name| name.to_string()).collect();
        list.extend(fallbacks.iter().cloned());
        list
    };
    fonts
        .families
        .insert(FontFamily::Proportional, with_fallbacks(&["space-mono"]));
    fonts
        .families
        .insert(FontFamily::Monospace, with_fallbacks(&["space-mono"]));
    fonts
        .families
        .insert(bold(), with_fallbacks(&["space-mono-bold"]));
    fonts
        .families
        .insert(dot(), with_fallbacks(&["doto", "space-mono"]));
    fonts
}

/// The few colours of one theme.
struct Palette {
    dark: bool,
    /// Window background.
    background: Color32,
    /// Card background.
    surface: Color32,
    /// Thin borders.
    border: Color32,
    /// Buttons and switch tracks at rest.
    control: Color32,
    control_hovered: Color32,
    /// Main text.
    text: Color32,
    /// Secondary text.
    weak_text: Color32,
}

impl Palette {
    const DARK: Self = Self {
        dark: true,
        background: Color32::BLACK,
        surface: Color32::from_gray(18),
        border: Color32::from_gray(38),
        control: Color32::from_gray(30),
        control_hovered: Color32::from_gray(44),
        text: Color32::WHITE,
        weak_text: Color32::from_gray(140),
    };

    const LIGHT: Self = Self {
        dark: false,
        background: Color32::WHITE,
        surface: Color32::from_gray(245),
        border: Color32::from_gray(222),
        control: Color32::from_gray(232),
        control_hovered: Color32::from_gray(218),
        text: Color32::BLACK,
        weak_text: Color32::from_gray(110),
    };

    /// Colour of selected elements: the text colour, so selection inverts
    /// black and white.
    fn accent(&self) -> Color32 {
        self.text
    }

    /// Colour drawn on top of the accent.
    fn on_accent(&self) -> Color32 {
        self.background
    }
}

fn visuals(palette: Palette) -> Visuals {
    let mut visuals = if palette.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let radius = CornerRadius::same(WIDGET_RADIUS);

    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.surface;
    visuals.window_stroke = Stroke::new(1.0, palette.border);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.menu_corner_radius = radius;
    visuals.faint_bg_color = palette.surface;
    visuals.extreme_bg_color = palette.background;
    visuals.code_bg_color = palette.surface;
    visuals.weak_text_color = Some(palette.weak_text);
    visuals.hyperlink_color = palette.text;
    visuals.error_fg_color = ERROR_COLOR;
    visuals.warn_fg_color = ERROR_COLOR;
    visuals.slider_trailing_fill = true;

    visuals.selection.bg_fill = palette.accent();
    visuals.selection.stroke = Stroke::new(1.0, palette.on_accent());

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = palette.surface;
    widgets.noninteractive.weak_bg_fill = palette.surface;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);

    for (state, fill) in [
        (&mut widgets.inactive, palette.control),
        (&mut widgets.hovered, palette.control_hovered),
        (&mut widgets.active, palette.control_hovered),
        (&mut widgets.open, palette.control_hovered),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::NONE;
        state.fg_stroke = Stroke::new(1.0, palette.text);
        state.expansion = 0.0;
    }
    widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    widgets.hovered.bg_stroke = Stroke::new(1.0, palette.weak_text);
    widgets.active.bg_stroke = Stroke::new(1.0, palette.text);

    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.corner_radius = radius;
    }
    visuals
}
