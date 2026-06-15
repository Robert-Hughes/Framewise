use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    types::{ClipRect, Color, Layer, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use crate::text::{emit_text_in_rect, measure_text};

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonSpec<'a> {
        pub layer: Layer,
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::ButtonStyle,
        pub clip_rect: ClipRect,
        pub disabled: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonCalcIntrinsicSizeSpec<'a> {
        pub text: &'a str,
        pub style: super::ButtonStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Measure a button's intrinsic size from its measurement spec.
    ///
    /// The preferred width is the label width plus horizontal padding; the
    /// preferred height is the larger of the standard control height and the
    /// padded label height.
    pub fn calc_button_intrinsic_size<T: TextBackend>(
        spec: &ButtonCalcIntrinsicSizeSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let style = &spec.style;
        let t = measure_text(
            text_system,
            spec.text,
            style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let w = t.logical_size.x + 2.0 * style.pad_x;
        let h = (t.logical_size.y + 2.0 * style.pad_y).max(style.min_height);
        crate::layout::IntrinsicSize::preferred(crate::types::Vec2::new(w, h))
    }

    /// Shape the label inside the button content rect and emit it.
    fn emit_placed_text<T: TextBackend>(
        text: &str,
        style: &super::ButtonStyle,
        rect: Rect,
        text_system: &mut T,
        cmds: &mut DrawCommands,
        color: Color,
        z: u32,
    ) -> Rect {
        let content_rect = Rect::new(
            rect.x + style.pad_x,
            rect.y + style.pad_y,
            (rect.w - 2.0 * style.pad_x).max(0.0),
            (rect.h - 2.0 * style.pad_y).max(0.0),
        );
        let m = measure_text(
            text_system,
            text,
            style.text_style,
            crate::text::TextBounds {
                max_width: Some(content_rect.w),
                max_height: Some(content_rect.h),
            },
        );
        let text_rect = style.content_placement.resolve_rect(content_rect, m);
        emit_text_in_rect(
            cmds,
            text_system,
            text,
            style.text_style,
            text_rect,
            color,
            z,
        );
        text_rect
    }

    /// Low-level button widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn button<T: TextBackend>(
        spec: ButtonSpec,
        state: &mut ButtonState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> ButtonResult {
        // Disabled: register_keyboard for layout but skip all interaction.
        if spec.disabled {
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: spec.rect,
                color: tint(spec.style.background),
                z: spec.layer.get_z(),
            });
            if spec.style.border_width > 0.0 {
                cmds.push(DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: spec.rect,
                    color: tint(spec.style.border),
                    width: spec.style.border_width,
                    z: spec.layer.get_z(),
                });
            }
            emit_placed_text(
                spec.text,
                &spec.style,
                spec.rect,
                text_system,
                cmds,
                tint(spec.style.text_color),
                spec.layer.get_z(),
            );
            return ButtonResult {
                content_bounds: spec.rect.inset(spec.style.border_width),
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
                focused: false,
            };
        }

        let interaction = crate::widgets::widget_helpers::handle_press_interaction(
            crate::widgets::widget_helpers::PressInteractionSpec {
                focus_id: state.focus_id,
                rect: spec.rect,
                clip_rect: spec.clip_rect,
                disabled: false,
                traversal_keys: crate::focus::FocusTraversalKeys::all(),
            },
            input,
            focus_system,
            &mut state.is_active,
            &mut state.space_is_active,
        );
        let focused = interaction.focused;
        let input_info = interaction.input;

        // Choose fill colour based on interaction state.
        let fill = crate::widgets::widget_helpers::interaction_color(
            spec.style.background,
            spec.style.hovered,
            spec.style.pressed,
            input_info.hovered,
            input_info.pressed,
        );

        // CSS outline sits outside the border box. StrokeRect draws inside its
        // rect, so expand by both the desired gap and the stroke width.
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: spec
                    .rect
                    .inset(-(spec.style.focus_offset + spec.style.focus_width)),
                color: spec.style.focus,
                width: spec.style.focus_width,
                z: spec.layer.get_focus_z(),
            });
        }

        // Background fill.
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: spec.rect,
            color: fill,
            z: spec.layer.get_z(),
        });

        // Border.
        if spec.style.border_width > 0.0 {
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: spec.rect,
                color: spec.style.border,
                width: spec.style.border_width,
                z: spec.layer.get_z(),
            });
        }

        // Text centered.
        emit_placed_text(
            spec.text,
            &spec.style,
            spec.rect,
            text_system,
            cmds,
            spec.style.text_color,
            spec.layer.get_z(),
        );

        ButtonResult {
            content_bounds: spec.rect.inset(spec.style.border_width),
            input: input_info,
            focused,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    pub background: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub border: Color,
    pub border_width: f32,
    pub focus: Color,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub text_style: crate::text::TextStyle,
    /// Placement of the prepared text block inside the padded button content rect.
    pub content_placement: crate::text::TextContentPlacement,
    pub text_color: Color,
    pub disabled_alpha: f32,
    /// Horizontal padding each side of the label, used for intrinsic width.
    pub pad_x: f32,
    /// Vertical padding above/below the label, used for intrinsic height.
    pub pad_y: f32,
    /// Minimum intrinsic height (the standard control height); the preferred
    /// height is the larger of this and the padded text height.
    pub min_height: f32,
}

impl ButtonStyle {
    pub fn secondary_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: theme.hover,
            pressed: theme.press,
            border: theme.ink,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: theme.ink,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn primary_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.ink,
            hovered: Color::BLACK,
            pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: theme.ink,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: theme.paper,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn accent_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.rust,
            hovered: Color::from_srgb_u8(176, 79, 35, 255),
            pressed: Color::from_srgb_u8(156, 69, 32, 255),
            border: theme.rust,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: Color::WHITE,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn ghost_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: theme.hover,
            pressed: theme.press,
            border: Color::TRANSPARENT,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: theme.ink,
            disabled_alpha: 0.32f32,
            pad_x: 10.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ButtonState {
    /// True if the mouse was pressed while hovering this button, until the mouse is released.
    pub is_active: bool,
    /// True if the spacebar was pressed while this button was focused, until space or focus is lost.
    pub space_is_active: bool,
    /// Globally unique ID for tracking keyboard focus.
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonSpec<'a> {
    pub text: &'a str,
    pub style: ButtonStyle,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ButtonSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<ButtonStyle>,
    pub disabled: Option<bool>,
}

impl<'a> ButtonSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(ButtonStyle::secondary_from_theme(theme));
        }
        self
    }
    pub fn build(self) -> ButtonSpec<'a> {
        ButtonSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level button widget function using WidgetContext.
///
/// This function accepts a ButtonSpecBuilder and layout parameters, resolves geometry and styles internally,
/// and calls the low-level raw::button function.
pub fn button<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::ButtonCalcIntrinsicSizeSpec {
        text: spec.text,
        style: spec.style,
    };
    let intrinsic = raw::calc_button_intrinsic_size(&calc_spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::ButtonSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        disabled: spec.disabled,
    };

    let r = raw::button(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_system,
        ctx.cmds,
    );

    ButtonResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
        input: r.input,
        focused: r.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::ButtonSpec;
    use super::*;

    use crate::test_utils::DummyTextSys;
    use crate::text::TextHandle;
    use crate::text::{CaretPosition, FontId};
    use crate::text::{PrepareGlyphRequest, ShapedCluster, ShapedGlyph, ShapedText};
    use crate::theme;
    use crate::types::Vec2;
    use crate::TextSystem;
    use crate::{DrawGlyph, PreparedGlyphHandle};

    struct PlacementTextSys {
        metrics: crate::text::TextMetrics,
        prepared_rect: Option<Rect>,
    }

    impl TextBackend for PlacementTextSys {
        type ShapedGlyphId = u32;

        fn line_height(&mut self, _style: crate::text::TextStyle) -> f32 {
            self.metrics.logical_size.y.max(1.0)
        }

        fn shape_text(
            &mut self,
            text: &str,
            style: crate::text::TextStyle,
        ) -> ShapedText<Self::ShapedGlyphId> {
            if text.is_empty() {
                return ShapedText {
                    clusters: Vec::new(),
                };
            }
            ShapedText {
                clusters: vec![ShapedCluster {
                    byte_start: 0,
                    byte_end: text.len(),
                    advance: self.metrics.logical_size.x,
                    is_whitespace: false,
                    glyphs: vec![ShapedGlyph {
                        id: 1,
                        x: 0.0,
                        y: -style.size.round(),
                        advance: self.metrics.logical_size.x,
                    }],
                }],
            }
        }

        fn shape_ellipsis(
            &mut self,
            style: crate::text::TextStyle,
        ) -> ShapedText<Self::ShapedGlyphId> {
            self.shape_text(".", style)
        }

        fn prepare_glyph(
            &mut self,
            request: PrepareGlyphRequest<Self::ShapedGlyphId>,
        ) -> Option<DrawGlyph> {
            self.prepared_rect = Some(Rect::new(
                request.glyph_origin.x,
                request.glyph_origin.y,
                self.metrics.logical_size.x,
                self.metrics.logical_size.y,
            ));
            Some(DrawGlyph {
                handle: PreparedGlyphHandle(request.glyph),
                top_left: request.glyph_origin,
            })
        }
    }

    impl TextSystem for PlacementTextSys {
        fn measure(
            &mut self,
            _text: &str,
            _style: crate::text::TextStyle,
            _bounds: crate::text::TextBounds,
        ) -> crate::text::TextMetrics {
            self.metrics.clone()
        }

        fn prepare(
            &mut self,
            _text: &str,
            _style: crate::text::TextStyle,
            rect: Rect,
        ) -> crate::text::TextLayout {
            self.prepared_rect = Some(rect);
            crate::text::TextLayout {
                handle: TextHandle(7),
                metrics: self.metrics.clone(),
                lines: Vec::new(),
                clusters: Vec::new(),
                glyphs: Vec::new(),
            }
        }

        fn caret_geom(
            &self,
            _handle: TextHandle,
            _position: CaretPosition,
        ) -> crate::text::CaretGeom {
            crate::text::CaretGeom {
                x: 0.0,
                y_top: 0.0,
                height: 0.0,
            }
        }

        fn hit_test_caret(&self, _handle: TextHandle, _pos: Vec2) -> CaretPosition {
            CaretPosition::EmptyText
        }

        fn caret_insertion_byte(&self, _handle: TextHandle, _position: CaretPosition) -> usize {
            0
        }

        fn caret_position_at_insertion_byte(
            &self,
            _handle: TextHandle,
            _byte_index: usize,
        ) -> CaretPosition {
            CaretPosition::EmptyText
        }

        fn hit_test_cluster(&self, _handle: TextHandle, _pos: Vec2) -> usize {
            0
        }
    }

    fn btn_spec(rect: Rect) -> ButtonSpec<'static> {
        ButtonSpec {
            layer: Layer::default(),
            rect,
            text: "Btn",
            style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
            clip_rect: None,
            disabled: false,
        }
    }

    fn draw_two_buttons(
        focus_system: &mut FocusSystem,
        s1: &mut ButtonState,
        s2: &mut ButtonState,
        input: &Input,
        text_system: &mut DummyTextSys,
        cmds: &mut DrawCommands,
    ) {
        raw::button(
            btn_spec(Rect::new(0.0, 0.0, 100.0, 30.0)),
            s1,
            input,
            focus_system,
            text_system,
            cmds,
        );
        raw::button(
            btn_spec(Rect::new(0.0, 40.0, 100.0, 30.0)),
            s2,
            input,
            focus_system,
            text_system,
            cmds,
        );
    }

    #[test]
    fn test_button_tab_moves_focus_next() {
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        let focus1 = s1.focus_id;
        let focus2 = s2.focus_id;
        let mut text_system = DummyTextSys;

        crate::widgets::test_helpers::assert_tab_moves_focus_next(
            &mut s1,
            focus1,
            &mut s2,
            focus2,
            |s1, s2, input, focus_system, cmds| {
                draw_two_buttons(focus_system, s1, s2, input, &mut text_system, cmds);
            },
        );
    }

    #[test]
    fn test_button_right_arrow_moves_focus_next() {
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        let focus1 = s1.focus_id;
        let focus2 = s2.focus_id;
        let mut text_system = DummyTextSys;

        crate::widgets::test_helpers::assert_right_arrow_moves_focus_next(
            &mut s1,
            focus1,
            &mut s2,
            focus2,
            |s1, s2, input, focus_system, cmds| {
                draw_two_buttons(focus_system, s1, s2, input, &mut text_system, cmds);
            },
        );
    }

    #[test]
    fn test_button_down_arrow_moves_focus_next() {
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        let focus1 = s1.focus_id;
        let focus2 = s2.focus_id;
        let mut text_system = DummyTextSys;

        crate::widgets::test_helpers::assert_down_arrow_moves_focus_next(
            &mut s1,
            focus1,
            &mut s2,
            focus2,
            |s1, s2, input, focus_system, cmds| {
                draw_two_buttons(focus_system, s1, s2, input, &mut text_system, cmds);
            },
        );
    }

    #[test]
    fn test_button_shift_tab_moves_focus_prev() {
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        let focus1 = s1.focus_id;
        let focus2 = s2.focus_id;
        let mut text_system = DummyTextSys;

        crate::widgets::test_helpers::assert_shift_tab_moves_focus_prev(
            &mut s1,
            focus1,
            &mut s2,
            focus2,
            |s1, s2, input, focus_system, cmds| {
                draw_two_buttons(focus_system, s1, s2, input, &mut text_system, cmds);
            },
        );
    }

    #[test]
    fn test_drag_off_and_release_does_not_click_other_button() {
        let mut text_system = DummyTextSys;
        let mut state1 = ButtonState::default();
        let mut state2 = ButtonState::default();

        crate::widgets::test_helpers::assert_drag_off_and_release_does_not_click_other(
            &mut state1,
            &mut state2,
            Vec2::new(50.0, 25.0),
            Vec2::new(50.0, 125.0),
            false,
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::button(
                    ButtonSpec {
                        layer: Layer::default(),
                        text: "Click Me",
                        ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
                    },
                    state1,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                );
                let res2 = raw::button(
                    ButtonSpec {
                        layer: Layer::default(),
                        text: "Btn2",
                        ..btn_spec(Rect::new(0.0, 100.0, 100.0, 50.0))
                    },
                    state2,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_click_triggers_clicked_state() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();

        crate::widgets::test_helpers::assert_mouse_click_on_release(
            &mut state,
            Vec2::new(50.0, 25.0),
            |state, input, focus_system, cmds| {
                raw::button(
                    btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_button_overlapping_hover() {
        let mut text_system = DummyTextSys;
        let mut state1 = ButtonState::default();
        let mut state2 = ButtonState::default();

        crate::widgets::test_helpers::assert_overlapping_hover(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::button(
                    btn_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                    state1,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                );
                let res2 = raw::button(
                    btn_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                    state2,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_button_overlapping_click() {
        let mut text_system = DummyTextSys;
        let mut state1 = ButtonState::default();
        let mut state2 = ButtonState::default();

        crate::widgets::test_helpers::assert_overlapping_click(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            true,
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::button(
                    btn_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                    state1,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                );
                let res2 = raw::button(
                    btn_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                    state2,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_button_click_takes_focus() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_mouse_press_takes_focus(
            &mut state,
            focus_id,
            Vec2::new(50.0, 25.0),
            |state, input, focus_system, cmds| {
                raw::button(
                    btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_button_clipped_click_does_not_take_focus() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();

        crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
            &mut state,
            Vec2::new(50.0, 25.0),
            |state, input, focus_system, cmds| {
                raw::button(
                    ButtonSpec {
                        layer: Layer::default(),
                        rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                        text: "Btn",
                        style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
                        clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 30.0)),
                        disabled: false,
                    },
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_button_disabled_ignores_interaction() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
            &mut state,
            focus_id,
            Vec2::new(50.0, 25.0),
            |state, input, focus_system, cmds| {
                raw::button(
                    ButtonSpec {
                        layer: Layer::default(),
                        disabled: true,
                        ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
                    },
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_enter_clicks_raw_button() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_system = FocusSystem::new();

        let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Frame 1: Register and take focus explicitly
        let mut input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        // Frame 2: Press Enter
        input.key_pressed_enter = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.clicked, "Button should be clicked by Enter key");
    }

    #[test]
    fn test_hover_and_press_state() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();

        crate::widgets::test_helpers::assert_hover_and_press_state(
            &mut state,
            Vec2::new(50.0, 25.0),
            Vec2::new(150.0, 150.0),
            |state, input, focus_system, cmds| {
                raw::button(
                    btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_spacebar_click() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_spacebar_click(
            &mut state,
            focus_id,
            |state, input, focus_system, cmds| {
                raw::button(
                    btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_spacebar_loses_focus_does_not_click() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_spacebar_loses_focus_does_not_click(
            &mut state,
            focus_id,
            |state, input, focus_system, cmds| {
                raw::button(
                    btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                    state,
                    input,
                    focus_system,
                    &mut text_system,
                    cmds,
                )
                .input
            },
        );
    }

    // ── Visual Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_button_visual_normal() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let input = Input::default();
        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            background,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: text_color,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(66),
                    top_left: Vec2 { x: 48.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 56.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(110),
                    top_left: Vec2 { x: 64.0, y: 30.0 },
                },
            ]
        );
    }

    #[test]
    fn test_button_visual_hovered() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0); // Inside bounds

        let mut state = state;
        // Warmup frame to establish hover claim
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            hovered,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: hovered,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: text_color,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(66),
                    top_left: Vec2 { x: 48.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 56.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(110),
                    top_left: Vec2 { x: 64.0, y: 30.0 },
                },
            ]
        );
    }

    #[test]
    fn test_button_visual_pressed() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0);

        let mut state = state;
        // Warmup frame to establish hover claim
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame with mouse pressed
        input.mouse_down = true;
        input.mouse_pressed = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            pressed,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: pressed,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: text_color,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(66),
                    top_left: Vec2 { x: 48.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 56.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(110),
                    top_left: Vec2 { x: 64.0, y: 30.0 },
                },
            ]
        );
    }

    #[test]
    fn test_button_visual_focused() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        focus_system.take_keyboard_focus(state.focus_id);

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            background,
            border,
            border_width,
            text_color,
            focus,
            focus_offset,
            focus_width,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        let expected_focus_rect =
            Rect::new(10.0, 10.0, 100.0, 30.0).inset(-(focus_offset + focus_width));

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: expected_focus_rect,
                    color: focus,
                    width: focus_width,
                    z: 1,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: text_color,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(66),
                    top_left: Vec2 { x: 48.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 56.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(110),
                    top_left: Vec2 { x: 64.0, y: 30.0 },
                },
            ]
        );
    }

    #[test]
    fn test_button_visual_disabled() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let spec = ButtonSpec {
            layer: Layer::default(),
            disabled: true,
            ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let alpha = 0.32_f32;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let primary_style = ButtonStyle::primary_from_theme(&theme::Theme::default());
        let expected_bg = tint(primary_style.background);
        let expected_border = tint(primary_style.border);
        let expected_text = tint(primary_style.text_color);
        let border_width = primary_style.border_width;

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: expected_bg,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: expected_border,
                    width: border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: expected_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(66),
                    top_left: Vec2 { x: 48.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 56.0, y: 30.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(110),
                    top_left: Vec2 { x: 64.0, y: 30.0 },
                },
            ]
        );
    }

    #[test]
    fn test_button_logical_content_placement_respects_padding() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = ButtonState::default();
        let spec = ButtonSpec {
            layer: Layer::default(),
            style: ButtonStyle {
                content_placement: crate::text::TextContentPlacement::logical(
                    crate::text::ContentPlacement::Align(crate::Align::End),
                    crate::text::ContentPlacement::Align(crate::Align::End),
                ),
                ..ButtonStyle::primary_from_theme(&theme::Theme::default())
            },
            ..btn_spec(Rect::new(10.0, 20.0, 100.0, 50.0))
        };

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _ = raw::button(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert!(
            cmds.glyphs()
                .first()
                .is_some_and(|glyph| glyph.top_left == Vec2::new(72.0, 61.0)),
            "button text should be bottom-right aligned inside the padded content rect"
        );
    }

    #[test]
    fn test_button_ink_content_placement_uses_ink_bounds_when_disabled() {
        let metrics = crate::text::TextMetrics {
            logical_size: Vec2::new(30.0, 20.0),
            ink_bounds: Rect::new(-4.0, 3.0, 18.0, 10.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines: Vec::new(),
        };
        let mut text_system = PlacementTextSys {
            metrics,
            prepared_rect: None,
        };
        let mut focus_system = FocusSystem::new();
        let mut state = ButtonState::default();
        let spec = ButtonSpec {
            layer: Layer::default(),
            disabled: true,
            style: ButtonStyle {
                content_placement: crate::text::TextContentPlacement::INK_CENTER,
                ..ButtonStyle::primary_from_theme(&theme::Theme::default())
            },
            ..btn_spec(Rect::new(10.0, 20.0, 100.0, 50.0))
        };

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _ = raw::button(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            text_system.prepared_rect,
            Some(Rect::new(55.0, 37.0, 30.0, 20.0))
        );
    }

    #[test]
    fn test_regression_custom_style_no_theme_lookup() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let input = Input::default();

        let custom_style = ButtonStyle {
            background: Color::from_srgb_u8(100, 150, 200, 255),
            hovered: Color::from_srgb_u8(110, 160, 210, 255),
            pressed: Color::from_srgb_u8(120, 170, 220, 255),
            border: Color::from_srgb_u8(220, 230, 240, 255),
            border_width: 4.5,
            focus: Color::from_srgb_u8(255, 0, 0, 255),
            focus_width: 2.0,
            focus_offset: 2.0,
            text_style: crate::text::TextStyle::new(
                FontId(0),
                19.5,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: Color::from_srgb_u8(50, 60, 70, 255),
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: 28.0,
        };

        let spec = ButtonSpec {
            layer: Layer::default(),
            rect: Rect::new(5.0, 15.0, 120.0, 45.0),
            text: "Explicit Spec",
            style: custom_style,
            clip_rect: None,
            disabled: false,
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(5.0, 15.0, 120.0, 45.0),
                    color: custom_style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(5.0, 15.0, 120.0, 45.0),
                    color: custom_style.border,
                    width: custom_style.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..10,
                    color: custom_style.text_color,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(69),
                    top_left: Vec2 { x: 21.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(120),
                    top_left: Vec2 { x: 29.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(112),
                    top_left: Vec2 { x: 37.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(108),
                    top_left: Vec2 { x: 45.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(105),
                    top_left: Vec2 { x: 53.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(99),
                    top_left: Vec2 { x: 61.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(105),
                    top_left: Vec2 { x: 69.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 77.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(83),
                    top_left: Vec2 { x: 93.0, y: 49.5 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(112),
                    top_left: Vec2 { x: 101.0, y: 49.5 },
                },
            ]
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = ButtonSpecBuilder::new().text("test");
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        let expected = ButtonStyle::secondary_from_theme(&theme);
        assert_eq!(
            builder.style.unwrap().text_style.font,
            expected.text_style.font
        );
        assert_eq!(
            builder.style.unwrap().text_style.size,
            expected.text_style.size
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let default_primary = ButtonStyle::primary_from_theme(&theme::Theme::default());
        let custom_style = ButtonStyle {
            text_style: crate::text::TextStyle {
                size: 99.0,
                ..default_primary.text_style
            },
            ..default_primary
        };
        let builder = ButtonSpecBuilder::new().text("test").style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut btn_state = ButtonState::default();
        // Under ManualLayout the layout param *is* the rect — the sanctioned way
        // to place a high-level widget explicitly.
        let result = super::button(
            &mut ctx,
            ButtonSpecBuilder::new().text("X"),
            placement,
            &mut btn_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_high_level_honors_user_style() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        // A user-set builder field (style) must be honored, not overwritten by
        // theme defaults.
        let custom = ButtonStyle {
            background: Color::from_srgb_u8(1, 2, 3, 255),
            ..ButtonStyle::accent_from_theme(&theme::Theme::default())
        };
        let mut btn_state = ButtonState::default();
        // Placed away from the default mouse position (0,0) so it isn't hovered.
        super::button(
            &mut ctx,
            ButtonSpecBuilder::new().text("X").style(custom),
            Rect::new(100.0, 100.0, 40.0, 28.0),
            &mut btn_state,
        );
        let has_custom_fill = cmds
            .iter()
            .any(|c| matches!(c, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == custom.background));
        assert!(
            has_custom_fill,
            "high-level button must honor user-set style"
        );
    }

    #[test]
    fn test_calc_button_intrinsic_size() {
        let mut ts = DummyTextSys;
        let spec = raw::ButtonCalcIntrinsicSizeSpec {
            text: "Btn",
            style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
        };
        // "Btn" = 3 chars * 8px = 24 wide, 16 tall (DummyTextSys).
        // width = 24 + 2*pad_x(14) = 52; height = max(16 + 2*pad_y(6), min_height 28) = 28.
        let i = raw::calc_button_intrinsic_size(&spec, &mut ts);
        assert_eq!(i.preferred, Some(Vec2::new(52.0, 28.0)));
    }

    #[test]
    fn test_button_auto_layout_uses_intrinsic_size() {
        use crate::layouts::{ColumnLayout, ColumnLayoutParams, ManualLayout};
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut ctx = WidgetContext::root(
            theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut col = ctx.child_with_layout(Rect::new(10.0, 10.0, 300.0, 400.0), ColumnLayout);
        let mut st = ButtonState::default();
        // Auto on both axes → the button sizes to its label intrinsic.
        // "Save" = 4*8 = 32 wide; width = 32 + 28 = 60; height = 28.
        let r = super::button(
            &mut col,
            ButtonSpecBuilder::new().text("Save"),
            ColumnLayoutParams::auto(),
            &mut st,
        );
        assert_eq!(r.layout.bounds, Rect::new(10.0, 10.0, 60.0, 28.0));
    }
}
