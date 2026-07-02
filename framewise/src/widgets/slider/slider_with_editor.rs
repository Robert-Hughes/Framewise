use crate::{
    layout::{LayoutState, SizeRequest},
    text::TextBackend,
    types::{Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
    widgets::number_edit::{
        raw as number_edit_raw, run_number_edit_at_rect, NumberEditResult, NumberEditSpec,
        NumberEditState, NumberEditStyle, NumberEditTextEntryMode,
    },
};

use super::{raw as slider_raw, Orientation, SliderResult, SliderSpec, SliderState, SliderValue};

#[derive(Debug, Clone, PartialEq)]
pub struct SliderWithEditorSpec {
    pub slider: SliderSpec,
    pub editor_style: NumberEditStyle,
    pub editor_width: f32,
    pub gap: f32,
}

impl Default for SliderWithEditorSpec {
    fn default() -> Self {
        Self {
            slider: SliderSpec::default(),
            editor_style: NumberEditStyle::default(),
            editor_width: 72.0,
            gap: 8.0,
        }
    }
}

impl SliderWithEditorSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            slider: SliderSpec::default_from_theme(theme),
            editor_style: NumberEditStyle::from_theme(theme),
            ..Self::default()
        }
    }

    pub fn slider(mut self, slider: SliderSpec) -> Self {
        self.slider = slider;
        self
    }

    pub fn editor_style(mut self, style: NumberEditStyle) -> Self {
        self.editor_style = style;
        self
    }

    pub fn editor_width(mut self, width: f32) -> Self {
        self.editor_width = width;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SliderWithEditorState {
    pub slider: SliderState,
    pub editor: NumberEditState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliderWithEditorResult {
    pub slider: SliderResult,
    pub editor: NumberEditResult,
}

pub fn slider_with_editor<T: TextBackend, S: LayoutState, CF>(
    spec: SliderWithEditorSpec,
    layout_params: S::Params,
    state: &mut SliderWithEditorState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SliderWithEditorResult {
    coerce_slider_to_single(&mut state.slider);

    let editor_spec = number_edit_spec_from_slider(&spec, &ctx.theme);
    let slider_spec = spec.slider;
    let editor_has_keyboard_focus =
        ctx.focus_system.current_keyboard_focus() == Some(state.editor.focus_id);

    let offer = ctx.peek_offer(layout_params.clone());
    let slider_pre_layout_spec = slider_raw::SliderPreLayoutSpec {
        orientation: slider_spec.orientation,
        style: slider_spec.style,
    };
    let slider_pre_layout = slider_raw::pre_layout_slider(&slider_pre_layout_spec, offer);
    let slider_size = slider_pre_layout
        .size_request
        .preferred
        .unwrap_or(Vec2::ZERO);

    let editor_pre_layout_spec = number_edit_raw::NumberEditPreLayoutSpec {
        style: editor_spec.style,
        value: state.editor.value,
        step_buttons_enabled: editor_spec.step_buttons_enabled,
        text_converter: &editor_spec.text_converter,
    };
    let editor_pre_layout =
        number_edit_raw::pre_layout_number_edit(&editor_pre_layout_spec, offer, ctx.text_backend);
    let editor_preferred = editor_pre_layout
        .size_request
        .preferred
        .unwrap_or(Vec2::ZERO);

    let editor_size = Vec2::new(spec.editor_width.max(0.0), editor_preferred.y);
    let gap = spec.gap.max(0.0);
    let outer_size = Vec2::new(
        slider_size.x + gap + editor_size.x,
        slider_size.y.max(editor_size.y),
    );
    let outer_rect = ctx.layout(layout_params, SizeRequest::preferred(outer_size));
    let layout = layout_slider_with_editor(
        outer_rect,
        slider_spec.orientation,
        slider_size,
        editor_size,
        gap,
    );

    // Run the focused editor before the slider so committed text is repaired by
    // the slider's own clamp/snap logic in this frame. When the editor is not
    // focused, run the slider first so the visible editor mirrors the canonical
    // slider value without disturbing an active text draft.
    let (slider_result, editor_result) = if editor_has_keyboard_focus {
        if editor_value_can_follow_slider(&state.editor) {
            state.editor.value = state.slider.value.lower();
        }
        let editor_result = run_number_edit_at_rect(
            editor_spec,
            layout.editor_rect,
            editor_pre_layout,
            &mut state.editor,
            ctx,
        );
        state.slider.value = SliderValue::Single(state.editor.value);
        let slider_result = run_slider_at_rect(
            slider_spec,
            layout.slider_rect,
            slider_pre_layout,
            &mut state.slider,
            ctx,
        );
        state.editor.value = state.slider.value.lower();
        (slider_result, editor_result)
    } else {
        let slider_result = run_slider_at_rect(
            slider_spec,
            layout.slider_rect,
            slider_pre_layout,
            &mut state.slider,
            ctx,
        );
        state.editor.value = state.slider.value.lower();
        let editor_result = run_number_edit_at_rect(
            editor_spec,
            layout.editor_rect,
            editor_pre_layout,
            &mut state.editor,
            ctx,
        );
        (slider_result, editor_result)
    };

    // The composite's visual tab order is slider -> editor. The internal run
    // order may vary so editor commits can be processed before slider
    // repair/snap, so reorder the registered focus IDs after both children have
    // registered.
    ctx.focus_system
        .override_keyboard_order(&[state.slider.focus_id, state.editor.focus_id]);

    SliderWithEditorResult {
        slider: slider_result,
        editor: editor_result,
    }
}

fn coerce_slider_to_single(state: &mut SliderState) {
    if let SliderValue::Range { lower, .. } = state.value {
        debug_assert!(
            !state.value.is_range(),
            "slider_with_editor only supports SliderValue::Single"
        );
        state.value = SliderValue::Single(lower);
    }
}

fn number_edit_spec_from_slider(
    spec: &SliderWithEditorSpec,
    theme: &crate::theme::Theme,
) -> NumberEditSpec {
    NumberEditSpec::new_from_theme(theme)
        .range(spec.slider.min, spec.slider.max)
        .step(spec.slider.step)
        .page_step(spec.slider.page_step)
        .text_entry_mode(NumberEditTextEntryMode::Always)
        .without_step_buttons()
        .drag_enabled(false)
        .value_fill_enabled(false)
        .disabled(spec.slider.disabled)
        .style(spec.editor_style)
}

fn editor_value_can_follow_slider(state: &NumberEditState) -> bool {
    !matches!(
        &state.edit,
        crate::widgets::number_edit::NumberEditEditState::Editing { dirty: true, .. }
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SliderWithEditorLayout {
    slider_rect: Rect,
    editor_rect: Rect,
}

fn layout_slider_with_editor(
    outer_rect: Rect,
    orientation: Orientation,
    slider_size: Vec2,
    editor_size: Vec2,
    gap: f32,
) -> SliderWithEditorLayout {
    let editor_w = editor_size.x.clamp(0.0, outer_rect.w);
    let editor_h = editor_size.y.min(outer_rect.h).max(0.0);
    let editor_x = (outer_rect.right() - editor_w).max(outer_rect.x);
    let editor_rect = Rect::new(
        editor_x,
        outer_rect.y + (outer_rect.h - editor_h).max(0.0) * 0.5,
        editor_w,
        editor_h,
    );
    let available_slider_w = (editor_x - gap - outer_rect.x).max(0.0);
    let slider_h = if orientation == Orientation::Vertical {
        outer_rect.h
    } else {
        slider_size.y.min(outer_rect.h).max(0.0)
    };
    let slider_w = if orientation == Orientation::Vertical {
        slider_size.x.min(available_slider_w).max(0.0)
    } else {
        available_slider_w
    };
    let slider_rect = Rect::new(
        outer_rect.x,
        outer_rect.y + (outer_rect.h - slider_h).max(0.0) * 0.5,
        slider_w,
        slider_h,
    );

    SliderWithEditorLayout {
        slider_rect,
        editor_rect,
    }
}

fn run_slider_at_rect<T: TextBackend, S: LayoutState, CF>(
    spec: SliderSpec,
    rect: Rect,
    pre_layout: slider_raw::SliderPreLayoutResult,
    state: &mut SliderState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SliderResult {
    let raw_spec = slider_raw::SliderSpec {
        rect,
        min: spec.min,
        max: spec.max,
        min_gap: spec.min_gap,
        max_gap: spec.max_gap,
        value_snap: spec.value_snap,
        page_step: spec.page_step,
        step: spec.step,
        orientation: spec.orientation,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        scroll_claim: spec.scroll_claim,
        time: ctx.time,
        disabled: spec.disabled,
        keyboard_focusable: spec.keyboard_focusable,
        layer: ctx.layer,
    };

    let result = slider_raw::post_layout_slider(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );
    ctx.request_cursor(result.cursor_icon);

    SliderResult {
        layout: LayoutInfo::tight(rect),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        draw::DrawCommands,
        focus::{FocusDirection, FocusId, FocusSystem},
        input::{Input, Key},
        layouts::ManualLayout,
        test_utils::TestTextBackend,
        types::Rect,
        widgets::number_edit::{NumberEditEditState, NumberEditState},
        Output,
    };

    fn run_once(
        spec: SliderWithEditorSpec,
        state: &mut SliderWithEditorState,
        input: &Input,
        focus: &mut FocusSystem,
    ) -> SliderWithEditorResult {
        let mut text_backend = TestTextBackend::default();
        let mut output = Output::default();
        let mut cmds = DrawCommands::new(1.0);
        let mut ctx = WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            focus,
            input,
            &mut output,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        slider_with_editor(spec, Rect::new(0.0, 0.0, 240.0, 28.0), state, &mut ctx)
    }

    fn run_slider_with_editor_then_register_outside(
        state: &mut SliderWithEditorState,
        input: &Input,
        focus: &mut FocusSystem,
        outside_id: FocusId,
    ) {
        run_once(
            SliderWithEditorSpec::default().slider(SliderSpec::default().max(1.0)),
            state,
            input,
            focus,
        );

        focus.register_keyboard(outside_id, Rect::new(300.0, 0.0, 80.0, 28.0), None);
    }

    fn editing_state(text: &str, dirty: bool) -> NumberEditEditState {
        NumberEditEditState::Editing {
            text_edit: crate::widgets::TextEditState::new(text),
            error: false,
            dirty,
        }
    }

    #[test]
    fn test_slider_state_syncs_to_editor_value() {
        let mut state = SliderWithEditorState {
            slider: SliderState {
                value: SliderValue::Single(0.42),
                ..Default::default()
            },
            editor: NumberEditState {
                value: 0.0,
                ..Default::default()
            },
        };

        let mut focus = FocusSystem::new();
        run_once(
            SliderWithEditorSpec::default().slider(SliderSpec::default().max(1.0)),
            &mut state,
            &Input::default(),
            &mut focus,
        );

        assert_eq!(state.editor.value, state.slider.value.lower());
    }

    #[test]
    fn test_slider_with_editor_overrides_forward_tab_from_slider_to_editor() {
        let mut state = SliderWithEditorState {
            slider: SliderState {
                value: SliderValue::Single(0.5),
                ..Default::default()
            },
            editor: NumberEditState {
                value: 0.5,
                ..Default::default()
            },
        };
        let mut focus = FocusSystem::new();
        let outside_id = FocusId::new();

        focus.take_keyboard_focus(state.editor.focus_id);
        focus.begin_frame();
        run_slider_with_editor_then_register_outside(
            &mut state,
            &Input::default(),
            &mut focus,
            outside_id,
        );
        focus.take_keyboard_focus(state.slider.focus_id);
        focus.request_keyboard_shift(FocusDirection::Next);
        focus.end_frame();

        assert_eq!(focus.current_keyboard_focus(), Some(state.editor.focus_id));
    }

    #[test]
    fn test_slider_with_editor_overrides_shift_tab_from_editor_to_slider() {
        let mut state = SliderWithEditorState {
            slider: SliderState {
                value: SliderValue::Single(0.5),
                ..Default::default()
            },
            editor: NumberEditState {
                value: 0.5,
                ..Default::default()
            },
        };
        let mut focus = FocusSystem::new();
        let outside_id = FocusId::new();

        focus.take_keyboard_focus(state.editor.focus_id);
        focus.begin_frame();
        run_slider_with_editor_then_register_outside(
            &mut state,
            &Input::default(),
            &mut focus,
            outside_id,
        );
        focus.request_keyboard_shift(FocusDirection::Prev);
        focus.end_frame();

        assert_eq!(focus.current_keyboard_focus(), Some(state.slider.focus_id));
    }

    #[test]
    fn test_editor_commit_updates_slider_value() {
        let mut state = SliderWithEditorState {
            slider: SliderState {
                value: SliderValue::Single(0.1),
                ..Default::default()
            },
            editor: NumberEditState {
                value: 0.1,
                edit: editing_state("0.75", true),
                ..Default::default()
            },
        };
        let mut focus = FocusSystem::new();
        focus.take_keyboard_focus(state.editor.focus_id);
        let mut input = Input::default();
        input.keys_pressed.insert(Key::Enter);

        run_once(
            SliderWithEditorSpec::default().slider(SliderSpec::default().max(1.0)),
            &mut state,
            &input,
            &mut focus,
        );

        assert_eq!(state.slider.value.lower(), 0.75);
        assert_eq!(state.editor.value, 0.75);
    }

    #[test]
    fn test_editor_commit_obeys_slider_bounds() {
        let mut state = SliderWithEditorState {
            slider: SliderState {
                value: SliderValue::Single(0.5),
                ..Default::default()
            },
            editor: NumberEditState {
                value: 0.5,
                edit: editing_state("-10", true),
                ..Default::default()
            },
        };
        let mut focus = FocusSystem::new();
        focus.take_keyboard_focus(state.editor.focus_id);
        let mut input = Input::default();
        input.keys_pressed.insert(Key::Enter);

        run_once(
            SliderWithEditorSpec::default().slider(SliderSpec::default().min(0.0).max(1.0)),
            &mut state,
            &input,
            &mut focus,
        );

        assert_eq!(state.slider.value.lower(), 0.0);
        assert_eq!(state.editor.value, 0.0);
    }

    #[test]
    fn test_editor_commit_obeys_slider_value_snap() {
        let mut state = SliderWithEditorState {
            slider: SliderState {
                value: SliderValue::Single(0.5),
                ..Default::default()
            },
            editor: NumberEditState {
                value: 0.5,
                edit: editing_state("0.62", true),
                ..Default::default()
            },
        };
        let mut focus = FocusSystem::new();
        focus.take_keyboard_focus(state.editor.focus_id);
        let mut input = Input::default();
        input.keys_pressed.insert(Key::Enter);

        run_once(
            SliderWithEditorSpec::default()
                .slider(SliderSpec::default().max(1.0).value_snap(Some(0.25))),
            &mut state,
            &input,
            &mut focus,
        );

        assert_eq!(state.slider.value.lower(), 0.5);
        assert_eq!(state.editor.value, 0.5);
    }

    #[test]
    fn test_disabled_propagates_to_derived_editor_spec() {
        let spec = SliderWithEditorSpec::default()
            .slider(SliderSpec::default().disabled(true))
            .editor_style(NumberEditStyle::default());

        let editor_spec = number_edit_spec_from_slider(&spec, &crate::theme::Theme::framewise());

        assert!(editor_spec.disabled);
        assert_eq!(editor_spec.text_entry_mode, NumberEditTextEntryMode::Always);
        assert!(!editor_spec.step_buttons_enabled);
        assert!(!editor_spec.drag_enabled);
        assert!(!editor_spec.value_fill_enabled);
    }
}
