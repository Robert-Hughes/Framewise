use crate::text::{FontId, FontRole, LineHeight, TextFlow, TextStyle};
use crate::types::Color;

/// The Framewise design-language palette and size constants.
///
/// Three root colours — ink, paper, rust — everything else is derived.
/// Widgets reference this through their `*Style::from_theme` impls.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Fonts
    pub sans_font: FontId,
    pub sans_weight_regular: u16, // Default sans weight (typically 400)
    pub sans_weight_bold: u16,    // Bold sans weight (typically 700)
    pub heading_font: FontId,     // For hero headings and similar elements
    pub heading_weight: u16,
    pub mono_font: FontId,
    pub overline_weight: u16,

    // Palette
    pub ink: Color,        // #15130f — text, borders, fills
    pub paper: Color,      // #f4f1ea — window background
    pub paper_elev: Color, // #fbf9f4 — raised surfaces (inputs, cards)
    pub rust: Color,       // #c25a2c — focus, drag, accent action
    pub ok: Color,         // #4d8a3a — success / online status
    pub err: Color,        // #b3251f — error / failure status
    pub muted: Color,      // #8a8378 — secondary text, placeholders
    pub rust_soft: Color,  // rust @ 14% α
    pub line: Color,       // ink @ 20% α — dividers
    pub line_soft: Color,  // ink @ 10% α — subtle structure
    pub hover: Color,      // ink @  6% α — button hover tint
    pub press: Color,      // ink @ 14% α — button press tint

    // Height grid
    pub h_sm: f32,       // 22 px
    pub h_md: f32,       // 28 px
    pub h_lg: f32,       // 36 px
    pub row_height: f32, // 26 px — menu / dropdown rows

    // Border / focus ring
    pub border: f32,             // 1.0 — standard border width
    pub focus_width: f32,        // 2.0 — focus ring width
    pub focus_offset: f32,       // 2.0 — focus ring outset gap
    pub focus_offset_tight: f32, // 1.0 — compact controls (drag_number, select)

    // Type scale
    pub text_sm: f32,   // 11 — mono caption
    pub text_md: f32,   // 13 — sans body
    pub text_mono: f32, // 12 — mono body

    pub scrollbar_width: f32,

    // Semantic Letter Spacing (Tracking)
    pub heading_letter_spacing: f32,
    pub overline_letter_spacing: f32,
    pub body_letter_spacing: f32,

    // Semantic Line Heights
    pub heading_line_height: LineHeight,
    pub body_line_height: LineHeight,
}

impl Theme {
    pub fn framewise() -> Self {
        Self {
            // Body text, UI labels, forms, documentation: use Inter.
            // Hero headings, landing pages, article titles: Inter Tight often looks more compact and polished.
            sans_font: FontId(1),
            sans_weight_regular: 400,
            sans_weight_bold: 700,
            heading_font: FontId(2),
            heading_weight: 600,
            mono_font: FontId(0),
            overline_weight: 500,
            ink: Color::from_srgb_u8(21, 19, 15, 255),
            paper: Color::from_srgb_u8(244, 241, 234, 255),
            paper_elev: Color::from_srgb_u8(251, 249, 244, 255),
            rust: Color::from_srgb_u8(194, 90, 44, 255),
            ok: Color::from_srgb_u8(77, 138, 58, 255),
            err: Color::from_srgb_u8(179, 37, 31, 255),
            muted: Color::from_srgb_u8(138, 131, 120, 255),
            rust_soft: Color::from_srgb_f32(194.0 / 255.0, 90.0 / 255.0, 44.0 / 255.0, 0.14),
            line: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.20),
            line_soft: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.10),
            hover: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.06),
            press: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.14),
            h_sm: 22.0,
            h_md: 28.0,
            h_lg: 36.0,
            row_height: 26.0,
            border: 1.0,
            focus_width: 2.0,
            focus_offset: 2.0,
            focus_offset_tight: 1.0,
            text_sm: 11.0,
            text_md: 13.0,
            text_mono: 12.0,
            scrollbar_width: 12.0,

            heading_letter_spacing: -0.035,
            overline_letter_spacing: 0.16,
            body_letter_spacing: 0.0,

            heading_line_height: LineHeight::Relative(0.95),
            body_line_height: LineHeight::Relative(1.55),
        }
    }

    pub fn font(&self, role: FontRole) -> FontId {
        match role {
            FontRole::Sans => self.sans_font,
            FontRole::Mono => self.mono_font,
        }
    }

    pub fn heading_text_style(&self, size: f32) -> TextStyle {
        TextStyle::new(
            self.heading_font,
            size,
            self.heading_weight,
            TextFlow::wrapped(),
        )
        .with_letter_spacing(self.heading_letter_spacing)
        .with_line_height(self.heading_line_height)
    }

    pub fn body_text_style(&self, size: f32) -> TextStyle {
        TextStyle::new(
            self.sans_font,
            size,
            self.sans_weight_regular,
            TextFlow::wrapped(),
        )
        .with_letter_spacing(self.body_letter_spacing)
        .with_line_height(self.body_line_height)
    }

    pub fn overline_text_style(&self, size: f32) -> TextStyle {
        TextStyle::new(
            self.mono_font,
            size,
            self.overline_weight,
            TextFlow::single_line(),
        )
        .with_letter_spacing(self.overline_letter_spacing)
        .with_line_height(LineHeight::Normal)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::framewise()
    }
}
