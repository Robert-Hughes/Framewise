use crate::{
    text::{FontId, FontRole},
    types::Color,
    widgets::{
        button::ButtonStyle, checkbox::CheckboxStyle, chip::ChipStyle,
        drag_number::DragNumberStyle, frame::FrameStyle, menu::MenuStyle,
        progress_bar::ProgressBarStyle, radio::RadioStyle, segmented::SegmentedStyle,
        select::SelectStyle, slider::SliderStyle, spinner::SpinnerStyle, status::StatusStyle,
        switch::SwitchStyle, tabs::TabsStyle, text_edit::TextEditStyle, tooltip::TooltipStyle,
        tree::TreeStyle, window::WindowStyle,
    },
};

/// The Framewise design-language palette and size constants.
///
/// Three root colours — ink, paper, rust — everything else is derived.
/// Widgets reference this through their `*Style::default()` impls.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Fonts
    pub sans_font: FontId,
    pub sans_bold_font: FontId,
    pub mono_font: FontId,

    // Palette
    pub ink: Color,        // #15130f — text, borders, fills
    pub paper: Color,      // #f4f1ea — window background
    pub paper_elev: Color, // #fbf9f4 — raised surfaces (inputs, cards)
    pub rust: Color,       // #c25a2c — focus, drag, accent action
    pub muted: Color,      // #8a8378 — secondary text, placeholders
    pub rust_soft: Color,  // rust @ 14% α
    pub line: Color,       // ink @ 20% α — dividers
    pub line_soft: Color,  // ink @ 10% α — subtle structure
    pub hover: Color,      // ink @  6% α — button hover tint
    pub press: Color,      // ink @ 14% α — button press tint

    // Height grid
    pub h_sm: f32, // 22 px
    pub h_md: f32, // 28 px
    pub h_lg: f32, // 36 px

    // Border / focus ring
    pub border: f32,       // 1.0 — standard border width
    pub focus_width: f32,  // 2.0 — focus ring width
    pub focus_offset: f32, // 2.0 — focus ring outset gap

    // Type scale
    pub text_sm: f32,   // 11 — mono caption
    pub text_md: f32,   // 13 — sans body
    pub text_mono: f32, // 12 — mono body
}

impl Theme {
    pub fn framewise() -> Self {
        Self {
            sans_font: FontId(1),
            sans_bold_font: FontId(2),
            mono_font: FontId(0),
            ink: Color::from_srgb_u8(21, 19, 15, 255),
            paper: Color::from_srgb_u8(244, 241, 234, 255),
            paper_elev: Color::from_srgb_u8(251, 249, 244, 255),
            rust: Color::from_srgb_u8(194, 90, 44, 255),
            muted: Color::from_srgb_u8(138, 131, 120, 255),
            rust_soft: Color::from_srgb_f32(194.0 / 255.0, 90.0 / 255.0, 44.0 / 255.0, 0.14),
            line: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.20),
            line_soft: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.10),
            hover: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.06),
            press: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.14),
            h_sm: 22.0,
            h_md: 28.0,
            h_lg: 36.0,
            border: 1.0,
            focus_width: 2.0,
            focus_offset: 2.0,
            text_sm: 11.0,
            text_md: 13.0,
            text_mono: 12.0,
        }
    }

    pub fn font(&self, role: FontRole) -> FontId {
        match role {
            FontRole::Sans => self.sans_font,
            FontRole::Mono => self.mono_font,
        }
    }

    pub fn frame_style(&self) -> FrameStyle {
        FrameStyle {
            background: self.paper_elev,
            border: self.ink,
            border_width: self.border,
            padding: 4.0,
        }
    }

    pub fn button_secondary_style(&self) -> ButtonStyle {
        ButtonStyle {
            background: Color::TRANSPARENT,
            hovered: self.hover,
            pressed: self.press,
            border: self.ink,
            border_width: self.border,
            focus_border: self.rust,
            text_size: self.text_md,
            font: self.sans_font,
            text_color: self.ink,
        }
    }

    pub fn button_primary_style(&self) -> ButtonStyle {
        ButtonStyle {
            background: self.ink,
            hovered: Color::BLACK,
            pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: self.ink,
            border_width: self.border,
            focus_border: self.rust,
            text_size: self.text_md,
            font: self.sans_font,
            text_color: self.paper,
        }
    }

    pub fn button_accent_style(&self) -> ButtonStyle {
        ButtonStyle {
            background: self.rust,
            hovered: Color::from_srgb_u8(176, 79, 35, 255),
            pressed: Color::from_srgb_u8(156, 69, 32, 255),
            border: self.rust,
            border_width: self.border,
            focus_border: self.rust,
            text_size: self.text_md,
            font: self.sans_font,
            text_color: Color::WHITE,
        }
    }

    pub fn button_ghost_style(&self) -> ButtonStyle {
        ButtonStyle {
            background: Color::TRANSPARENT,
            hovered: self.hover,
            pressed: self.press,
            border: Color::TRANSPARENT,
            border_width: 0.0,
            focus_border: self.rust,
            text_size: self.text_md,
            font: self.sans_font,
            text_color: self.ink,
        }
    }

    pub fn text_edit_style(&self) -> TextEditStyle {
        TextEditStyle {
            background: self.paper_elev,
            border: self.ink,
            focus_border: self.rust,
            border_width: self.border,
            padding: 4.0,
            text_size: self.text_mono,
            font: self.mono_font,
            text_color: self.ink,
            caret_color: self.rust,
            select_color: self.rust_soft,
        }
    }

    pub fn slider_style(&self) -> SliderStyle {
        SliderStyle {
            track_color: self.ink,
            thumb_color: self.paper_elev,
            thumb_border_color: self.ink,
            thumb_border_width: 1.5,
            thumb_hover_color: self.paper_elev,
            thumb_drag_color: self.rust,
            focus_outline_color: self.rust,
            thickness: 1.5,
            thumb_size: 12.0,
            scrollbar_mode: false,
        }
    }

    pub fn scrollbar_style(&self) -> SliderStyle {
        SliderStyle {
            track_color: Color::linear_rgba(self.ink.r, self.ink.g, self.ink.b, 0.04),
            thumb_color: self.ink,
            thumb_border_color: Color::TRANSPARENT,
            thumb_border_width: 0.0,
            thumb_hover_color: self.rust,
            thumb_drag_color: self.rust,
            focus_outline_color: self.rust,
            thickness: 1.5,
            thumb_size: 12.0,
            scrollbar_mode: true,
        }
    }

    pub fn checkbox_style(&self) -> CheckboxStyle {
        CheckboxStyle {
            size: 14.0,
            background: self.paper_elev,
            selected_fill: self.ink,
            border: self.ink,
            mark: self.paper,
            focus: self.rust,
            border_width: 1.5,
            mark_width: 1.5,
            focus_width: self.focus_width,
            focus_offset: self.focus_offset,
            disabled_alpha: 0.35,
        }
    }

    pub fn radio_style(&self) -> RadioStyle {
        RadioStyle {
            radius: 7.0,
            dot_radius: 3.0,
            background: self.paper_elev,
            border: self.ink,
            dot: self.ink,
            focus: self.rust,
            border_width: 1.5,
            focus_width: self.focus_width,
            focus_offset: self.focus_offset,
            disabled_alpha: 0.35,
        }
    }

    pub fn switch_style(&self) -> SwitchStyle {
        SwitchStyle {
            size: (30.0, 16.0),
            thumb_size: 10.0,
            off_fill: self.paper_elev,
            on_fill: self.ink,
            border: self.ink,
            off_thumb: self.ink,
            on_thumb: self.paper,
            focus: self.rust,
            border_width: 1.5,
            focus_width: self.focus_width,
            focus_offset: self.focus_offset,
            disabled_alpha: 0.35,
        }
    }

    pub fn progress_bar_style(&self) -> ProgressBarStyle {
        ProgressBarStyle {
            track_color: self.line_soft,
            fill_color: self.ink,
            active_fill_color: self.rust,
            track_height: 3.0,
            indeterminate_fraction: 0.3,
        }
    }

    pub fn spinner_style(&self) -> SpinnerStyle {
        SpinnerStyle {
            color: self.ink,
            highlight: self.rust,
            small_size: 16.0,
            large_size: 24.0,
            small_arm: 5.0,
            large_arm: 7.0,
            width: 1.5,
            highlight_fraction: 0.4,
        }
    }

    pub fn chip_style(&self) -> ChipStyle {
        ChipStyle {
            height: self.h_sm,
            pad_x: 8.0,
            text_size: self.text_sm,
            background: self.paper_elev,
            active_bg: self.ink,
            border: self.ink,
            text: self.ink,
            active_text: self.paper,
            focus: self.rust,
            border_width: self.border,
            focus_width: self.focus_width,
            focus_offset: self.focus_offset,
            disabled_alpha: 0.35,
        }
    }

    pub fn segmented_style(&self) -> SegmentedStyle {
        SegmentedStyle {
            height: self.h_md,
            pad_x: 14.0,
            text_size: self.text_md,
            background: self.paper_elev,
            border: self.ink,
            active_bg: self.ink,
            text: self.ink,
            active_text: self.paper,
            focus: self.rust,
            border_width: self.border,
            focus_width: self.focus_width,
            focus_inset: 2.0,
            disabled_alpha: 0.35,
        }
    }

    pub fn tabs_style(&self) -> TabsStyle {
        TabsStyle {
            height: 36.0,
            pad_x: 18.0,
            underbar_height: 3.0,
            text_size: self.text_md,
            border: self.ink,
            text: self.ink,
            inactive_text: self.muted,
            accent: self.rust,
            focus: self.rust,
            border_width: self.border,
            focus_width: self.focus_width,
            focus_offset: self.focus_offset,
            disabled_alpha: 0.35,
        }
    }

    pub fn select_style(&self) -> SelectStyle {
        SelectStyle {
            min_width: 180.0,
            height: self.h_md,
            row_height: 26.0,
            popup_gap: 2.0,
            popup_pad_y: 4.0,
            pad_x: 10.0,
            chevron_right: 18.0,
            text_size: self.text_md,
            chevron_size: self.text_sm,
            background: self.paper_elev,
            border: self.ink,
            text: self.ink,
            selected_bg: self.ink,
            selected_text: self.paper,
            hover: self.hover,
            muted: self.muted,
            accent: self.rust,
            border_width: self.border,
            focus_width: self.focus_width,
            focus_offset: 1.0,
            disabled_alpha: 0.35,
        }
    }

    pub fn status_style(&self) -> StatusStyle {
        StatusStyle {
            dot_size: 6.0,
            gap: 8.0,
            text_size: self.text_sm,
            neutral: self.muted,
            ok: Color::from_srgb_f32(0.302, 0.541, 0.227, 1.0),
            warn: self.rust,
            err: Color::from_srgb_f32(0.702, 0.145, 0.122, 1.0),
            live: self.rust,
            text: self.muted,
        }
    }

    pub fn tooltip_style(&self) -> TooltipStyle {
        TooltipStyle {
            text_size: self.text_sm,
            pad_x: 8.0,
            pad_y_top: 5.0,
            pad_y_bot: 6.0,
            arrow_h: 4.0,
            arrow_w: 8.0,
            arrow_x: 14.0,
            max_width: 240.0,
            dark_bg: self.ink,
            dark_text: self.paper,
            rust_bg: self.rust,
            rust_text: Color::WHITE,
            arrow_width: 1.5,
        }
    }

    pub fn tree_style(&self) -> TreeStyle {
        TreeStyle {
            row_height: 20.0,
            indent_width: 14.0,
            caret_width: 12.0,
            pad_x: 10.0,
            pad_y: 4.0,
            min_width: 280.0,
            text_size: self.text_sm,
            background: self.paper_elev,
            border: self.ink,
            selected_bg: self.ink,
            text: self.ink,
            selected_text: self.paper,
            muted: self.muted,
            selected_meta_alpha: 0.7,
            border_width: self.border,
        }
    }

    pub fn drag_number_style(&self) -> DragNumberStyle {
        DragNumberStyle {
            text_size: self.text_md,
            label_pad_x: 10.0,
            background: self.paper_elev,
            border: self.ink,
            focus: self.rust,
            label_bg: self.ink,
            active_label_bg: self.rust,
            label_text: self.paper,
            value_text: self.ink,
            value_fill: self.rust_soft,
            border_width: self.border,
            focus_width: self.focus_width,
            focus_offset: 1.0,
            disabled_alpha: 0.35,
        }
    }

    pub fn menu_style(&self) -> MenuStyle {
        MenuStyle {
            row_height: 26.0,
            separator_height: 9.0,
            group_height: 22.0,
            pad_x: 12.0,
            pad_y: 4.0,
            group_text_y: 8.0,
            separator_y: 4.0,
            min_width: 200.0,
            label_size: self.text_md,
            meta_size: self.text_sm,
            background: self.paper_elev,
            border: self.ink,
            separator: self.line,
            selected_bg: self.ink,
            selected_text: self.paper,
            text: self.ink,
            muted: self.muted,
            shortcut_selected_alpha: 0.6,
            disabled_alpha: 0.4,
            border_width: self.border,
        }
    }

    pub fn window_style(&self) -> WindowStyle {
        WindowStyle {
            title_height: 26.0,
            button_size: 16.0,
            button_gap: 2.0,
            button_right_pad: 4.0,
            status_height: 22.0,
            content_pad_x: 16.0,
            content_pad_y: 16.0,
            text_pad_x: 10.0,
            text_size: self.text_sm,
            background: self.paper_elev,
            border: self.ink,
            title_bg: self.ink,
            title_text: self.paper,
            status_text: self.muted,
            status_border: self.line,
            border_width: self.border,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::framewise()
    }
}
