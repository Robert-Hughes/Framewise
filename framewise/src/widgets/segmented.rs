use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    text::TextSystem,
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedSpec<'a> {
        pub layer: Layer,
        /// Top-left origin. Height is fixed at h_md (28).
        pub rect: Rect,
        pub items: &'a [&'a str],
        pub style: super::SegmentedStyle,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedCalcIntrinsicSizeSpec<'a> {
        pub items: &'a [&'a str],
        pub style: super::SegmentedStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Measure a segmented control's intrinsic size from its measurement spec.
    pub fn calc_segmented_intrinsic_size<T: TextSystem>(
        spec: &SegmentedCalcIntrinsicSizeSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let s = spec.style;
        let total_w = spec
            .items
            .iter()
            .map(|text| {
                text_system
                    .measure(text, s.text_style, crate::text::TextBounds::UNBOUNDED)
                    .logical_size
                    .x
                    + s.pad_x * 2.0
            })
            .sum();
        crate::layout::IntrinsicSize::preferred(Vec2::new(total_w, s.height))
    }

    /// Low-level segmented widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn segmented<'a, T: TextSystem>(
        spec: SegmentedSpec<'a>,
        state: &mut SegmentedState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> SegmentedResult {
        let s = spec.style;

        if spec.items.is_empty() {
            return SegmentedResult {
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
                focused: false,
                content_bounds: spec.rect,
            };
        }

        let h = s.height;
        let pad_x = s.pad_x;

        // Pre-measure all labels to get their widths.
        let metrics: Vec<_> = spec
            .items
            .iter()
            .map(|text| text_system.measure(text, s.text_style, crate::text::TextBounds::UNBOUNDED))
            .collect();
        let widths: Vec<f32> = metrics
            .iter()
            .map(|m| m.logical_size.x + pad_x * 2.0)
            .collect();
        let total_w: f32 = widths.iter().sum();

        let outer = Rect::new(spec.rect.x, spec.rect.y, total_w, h);

        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_focus(
                state.focus_id,
                outer,
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::all(),
                spec.disabled,
            )
        };

        let mut is_clicked = clicked;

        // Left/Right keyboard navigation
        if focused && !spec.disabled && !spec.items.is_empty() {
            if input.key_pressed_left && state.active_index > 0 {
                state.active_index -= 1;
                is_clicked = true;
            }
            if input.key_pressed_right && state.active_index + 1 < spec.items.len() {
                state.active_index += 1;
                is_clicked = true;
            }
        }

        // Mouse click segment detection
        if clicked && !spec.disabled && !spec.items.is_empty() {
            let mut x = spec.rect.x;
            for (i, &w) in widths.iter().enumerate() {
                let seg_rect = Rect::new(x, spec.rect.y, w, h);
                let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
                if seg_rect.contains(input.mouse_pos) && is_visible {
                    state.active_index = i;
                    break;
                }
                x += w;
            }
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        cmds.push(DrawCmd::FillRect {
            rect: outer,
            color: tint(s.background),
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: outer,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        let mut x = spec.rect.x;
        for (i, ((label, metric), &w)) in spec
            .items
            .iter()
            .zip(metrics.iter())
            .zip(widths.iter())
            .enumerate()
        {
            let is_active = i == state.active_index;
            let seg_rect = Rect::new(x, spec.rect.y, w, h);

            if is_active {
                cmds.push(DrawCmd::FillRect {
                    rect: seg_rect,
                    color: tint(s.active_bg),
                    z: spec.layer.get_z(),
                });
            }

            // Focus ring (inset to stay within bounds).
            let visually_focused = focused && i == state.active_index;
            if visually_focused && !spec.disabled {
                cmds.push(DrawCmd::StrokeRect {
                    rect: seg_rect.inset(s.focus_offset),
                    color: tint(s.focus),
                    width: s.focus_width,
                    z: spec.layer.get_z(),
                });
            }

            // Divider between segments (right edge, except last).
            if i + 1 < spec.items.len() {
                let div_x = x + w;
                cmds.push(DrawCmd::StrokeLine {
                    p0: Vec2::new(div_x, spec.rect.y),
                    p1: Vec2::new(div_x, spec.rect.y + h),
                    color: tint(s.border),
                    width: s.border_width,
                    z: spec.layer.get_z(),
                });
            }

            let text_color = if is_active { s.active_text } else { s.text };
            let ty = spec.rect.y + (h - metric.logical_size.y) * 0.5;
            let text_rect = Rect::new(x + pad_x, ty, metric.logical_size.x, metric.logical_size.y);
            let layout = text_system.prepare(label, s.text_style, text_rect);
            cmds.push(DrawCmd::Text {
                rect: text_rect,
                color: tint(text_color),
                handle: layout.handle,
                z: spec.layer.get_z(),
            });

            x += w;
        }

        SegmentedResult {
            input: InputInfo {
                hovered: outer.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            focused,
            content_bounds: outer.inset(s.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentedStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_style: crate::text::TextStyle,
    pub background: Color,
    pub border: Color,
    pub active_bg: Color,
    pub text: Color,
    pub active_text: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl SegmentedStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: theme.h_md,
            pad_x: 14.0,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: theme.ink,
            active_bg: theme.ink,
            text: theme.ink,
            active_text: theme.paper,
            focus: theme.rust,
            border_width: theme.border,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SegmentedState {
    pub active_index: usize,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentedResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentedSpec<'a> {
    pub items: &'a [&'a str],
    pub style: SegmentedStyle,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegmentedSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub style: Option<SegmentedStyle>,
    pub disabled: Option<bool>,
}

impl<'a> SegmentedSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn style(mut self, style: SegmentedStyle) -> Self {
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
            self.style = Some(SegmentedStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SegmentedSpec<'a> {
        SegmentedSpec {
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level segmented widget function using WidgetContext.
///
/// This function accepts a SegmentedSpecBuilder and calls the low-level raw::segmented function.
pub fn segmented<'a, T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SegmentedSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut SegmentedState,
) -> SegmentedResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::SegmentedCalcIntrinsicSizeSpec {
        items: spec.items,
        style: spec.style,
    };
    let intrinsic = raw::calc_segmented_intrinsic_size(&calc_spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::SegmentedSpec {
        layer: ctx.layer,
        rect,
        items: spec.items,
        style: spec.style,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::segmented(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_system,
        ctx.cmds,
    );

    SegmentedResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SegmentedSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

    fn segmented_dummy<'a>(
        spec: SegmentedSpec<'a>,
        active_index: usize,
    ) -> (raw::SegmentedResult, DrawCommands) {
        let mut cmds = DrawCommands::new();
        let res = raw::segmented(
            spec,
            &mut SegmentedState {
                active_index,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut DummyTextSys,
            &mut cmds,
        );
        (res, cmds)
    }

    #[test]
    fn test_segmented_visual_normal() {
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            disabled: false,
            style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let (_res, cmds) = segmented_dummy(spec, 0);

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 36.0, 28.0),
                    color: style.active_bg,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(36.0, 0.0),
                    p1: Vec2::new(36.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, 6.0, 8.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(50.0, 6.0, 8.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_segmented_visual_focused() {
        let mut state = SegmentedState {
            active_index: 1,
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let mut text_system = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            disabled: false,
            style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        let _res = raw::segmented(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(36.0, 0.0),
                    p1: Vec2::new(36.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, 6.0, 8.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(36.0, 0.0, 36.0, 28.0),
                    color: style.active_bg,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(38.0, 2.0, 32.0, 24.0),
                    color: style.focus,
                    width: style.focus_width,
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(50.0, 6.0, 8.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_segmented_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = SegmentedState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            disabled: false,
            style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::segmented(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking segmented must request focus"
        );
    }

    #[test]
    fn test_segmented_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = SegmentedState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            disabled: false,
            style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 200.0, 28.0)),
        };

        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::segmented(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away segmented control must not take focus"
        );
    }

    #[test]
    fn test_segmented_keyboard_navigation() {
        let mut focus_system = FocusSystem::new();
        let mut state = SegmentedState::default();
        let mut input = Input::default();
        let mut text_system = DummyTextSys;
        let items = ["A", "B"];

        // Focus the widget
        focus_system.take_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> changes active index to 1
        input.key_pressed_right = true;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::segmented(
            SegmentedSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 200.0, 28.0),
                items: &items,
                disabled: false,
                style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.active_index, 1);

        // Frame 2: Press Arrow Left -> changes active index back to 0
        input.key_pressed_left = true;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::segmented(
            SegmentedSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 200.0, 28.0),
                items: &items,
                disabled: false,
                style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.active_index, 0);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = SegmentedSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(SegmentedStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = SegmentedStyle::from_theme(&theme);
        custom_style.text_style.size = 99.0;
        let builder = SegmentedSpecBuilder::new().style(custom_style);
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
        let mut seg_state = SegmentedState::default();
        let result = super::segmented(
            &mut ctx,
            SegmentedSpecBuilder::new().items(&[]),
            placement,
            &mut seg_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }
}
