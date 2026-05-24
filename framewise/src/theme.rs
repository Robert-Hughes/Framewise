use crate::types::Color;

/// The Framewise design-language palette and size constants.
///
/// Three root colours — ink, paper, rust — everything else is derived.
/// Widgets reference this through their `*Style::default()` impls.
pub struct Theme {
    // Palette
    pub ink:        Color,  // #15130f — text, borders, fills
    pub paper:      Color,  // #f4f1ea — window background
    pub paper_elev: Color,  // #fbf9f4 — raised surfaces (inputs, cards)
    pub rust:       Color,  // #c25a2c — focus, drag, accent action
    pub muted:      Color,  // #8a8378 — secondary text, placeholders
    pub rust_soft:  Color,  // rust @ 14% α
    pub line:       Color,  // ink @ 20% α — dividers
    pub line_soft:  Color,  // ink @ 10% α — subtle structure
    pub hover:      Color,  // ink @  6% α — button hover tint
    pub press:      Color,  // ink @ 14% α — button press tint

    // Height grid
    pub h_sm: f32,   // 22 px
    pub h_md: f32,   // 28 px
    pub h_lg: f32,   // 36 px

    // Border / focus ring
    pub border:       f32,   // 1.0 — standard border width
    pub focus_width:  f32,   // 2.0 — focus ring width
    pub focus_offset: f32,   // 2.0 — focus ring outset gap

    // Type scale
    pub text_sm:   f32,  // 11 — mono caption
    pub text_md:   f32,  // 13 — sans body
    pub text_mono: f32,  // 12 — mono body
}

impl Theme {
    pub fn framewise() -> Self {
        let ink_r = 21.0_f32  / 255.0;
        let ink_g = 19.0_f32  / 255.0;
        let ink_b = 15.0_f32  / 255.0;
        Self {
            ink:        Color::from_u8(21,  19,  15,  255),
            paper:      Color::from_u8(244, 241, 234, 255),
            paper_elev: Color::from_u8(251, 249, 244, 255),
            rust:       Color::from_u8(194, 90,  44,  255),
            muted:      Color::from_u8(138, 131, 120, 255),
            rust_soft:  Color::new(194.0/255.0, 90.0/255.0, 44.0/255.0, 0.14),
            line:       Color::new(ink_r, ink_g, ink_b, 0.20),
            line_soft:  Color::new(ink_r, ink_g, ink_b, 0.10),
            hover:      Color::new(ink_r, ink_g, ink_b, 0.06),
            press:      Color::new(ink_r, ink_g, ink_b, 0.14),
            h_sm:       22.0,
            h_md:       28.0,
            h_lg:       36.0,
            border:     1.0,
            focus_width:  2.0,
            focus_offset: 2.0,
            text_sm:    11.0,
            text_md:    13.0,
            text_mono:  12.0,
        }
    }
}

impl Default for Theme {
    fn default() -> Self { Self::framewise() }
}
