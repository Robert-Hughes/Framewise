#![allow(clippy::too_many_arguments)]
//! Interactive widget specification page — mirrors mockups/Framewise Widgets.html.
//!
//! The page is feature-gated section by section: a small core (label, divider,
//! scroll_area) renders the scaffolding, and each `section_NN_*` fn below is
//! compiled in only when the widgets it demonstrates are enabled.

use crate::text::SampleTextBackend;
#[allow(unused_imports)]
use framewise::text::{layout_text, TextBackend};
#[cfg(feature = "radio")]
use framewise::RowState;
use framewise::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    layout::{LayoutState, SizeRequest, SpacerLayoutState},
    layouts::ManualLayout,
    text::{TextFlow, TextStyle},
    theme::Theme,
    types::{Color, Rect, Vec2},
    widget::WidgetContext,
    Align, ColumnLayout, ColumnLayoutParams, ColumnState, LayoutViolationPolicy, LinearSpacer,
    ManualState, RowLayout, RowLayoutParams, TextLineAlign,
};

// Core widgets — required by the page scaffolding (section headers, captions,
// hero, footer, and the page-level scroll wrapper).
use framewise::widgets::divider::divider;
use framewise::widgets::label::label;
use framewise::widgets::scroll_area::{
    begin_scroll_area, ScrollAreaSpec, ScrollAxis, ScrollExtent, ScrollState, ScrollbarVisibility,
};
use framewise::widgets::{DividerSpec, LabelSpec, LabelStyle};

// Per-widget imports — present only when the owning feature is enabled. Marked
// `unused_imports`-allowed because a widget feature can be on while the (possibly
// multi-widget) section that consumes it is off.
#[cfg(feature = "button")]
#[allow(unused_imports)]
use framewise::widgets::button::{button, ButtonSpec, ButtonState, ButtonStyle};
#[cfg(feature = "checkbox")]
#[allow(unused_imports)]
use framewise::widgets::checkbox::{
    checkbox, labelled_checkbox, CheckboxSpec, CheckboxState, CheckboxStyle, CheckedState,
};
#[cfg(feature = "chip")]
#[allow(unused_imports)]
use framewise::widgets::chip::{chip, ChipSpec, ChipState, ChipStyle};
#[cfg(feature = "color_swatch")]
#[allow(unused_imports)]
use framewise::widgets::color_swatch::{color_swatch, ColorSwatchSpec};
#[cfg(feature = "keycap")]
#[allow(unused_imports)]
use framewise::widgets::keycap::{keycap, KeycapSpec};
#[cfg(feature = "menu")]
#[allow(unused_imports)]
use framewise::widgets::menu::{menu, MenuItem, MenuSpec};
#[cfg(feature = "meter")]
#[allow(unused_imports)]
use framewise::widgets::meter::{meter, MeterSpec};
#[cfg(feature = "number_edit")]
#[allow(unused_imports)]
use framewise::widgets::number_edit::{
    number_edit, prefixed_number_edit, NumberEditSpec, NumberEditState, NumberEditStyle,
};
#[cfg(feature = "progress_bar")]
#[allow(unused_imports)]
use framewise::widgets::progress_bar::{progress_bar, ProgressBarSpec};
#[cfg(feature = "radio")]
#[allow(unused_imports)]
use framewise::widgets::radio::{labelled_radio, radio, RadioSpec, RadioState, RadioStyle};
#[cfg(feature = "segmented")]
#[allow(unused_imports)]
use framewise::widgets::segmented::{segmented, SegmentedSpec, SegmentedState};
#[cfg(feature = "select")]
#[allow(unused_imports)]
use framewise::widgets::select::{select, SelectSpec, SelectState, SelectStyle};
#[cfg(feature = "slider")]
#[allow(unused_imports)]
use framewise::widgets::slider::{
    slider, Orientation, ScrollClaimPolicy, SliderPart, SliderSpec, SliderState, SliderStyle,
    SliderValue, TrackMarksStyle,
};
#[cfg(feature = "spinner")]
#[allow(unused_imports)]
use framewise::widgets::spinner::{spinner, SpinnerSpec};
#[cfg(feature = "status")]
#[allow(unused_imports)]
use framewise::widgets::status::{status, StatusSpec, StatusVariant};
#[cfg(feature = "switch")]
#[allow(unused_imports)]
use framewise::widgets::switch::{labelled_switch, switch, SwitchSpec, SwitchState, SwitchStyle};
#[cfg(feature = "tabs")]
#[allow(unused_imports)]
use framewise::widgets::tabs::{tabs, TabsSpec, TabsState};
#[cfg(feature = "text_edit")]
#[allow(unused_imports)]
use framewise::widgets::text_edit::{
    prefixed_text_edit, text_edit, NewlinePolicy, TextEditSpec, TextEditState, TextEditStyle,
};
#[cfg(feature = "tooltip")]
#[allow(unused_imports)]
use framewise::widgets::tooltip::{tooltip, TooltipSpec, TooltipVariant};
#[cfg(feature = "tree")]
#[allow(unused_imports)]
use framewise::widgets::tree::{tree, TreeRow, TreeSpec};
#[cfg(feature = "window")]
#[allow(unused_imports)]
use framewise::widgets::window::{begin_window, WindowButton, WindowSpec};

// ── Fake State Helpers ────────────────────────────────────────────────────────

#[cfg(feature = "checkbox")]
fn draw_checkbox_fake_state<T: TextBackend, CF>(
    b: &mut WidgetContext<T, ManualState, CF>,
    rect: Rect,
    state_val: CheckedState,
    is_focused: bool,
    is_disabled: bool,
) {
    let mut state = CheckboxState {
        checked: state_val,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_keyboard_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = framewise::widgets::checkbox::raw::CheckboxSpec {
        rect: b.layout(rect, SizeRequest::UNKNOWN),
        disabled: is_disabled,
        allowed_checked_states: vec![
            CheckedState::Unchecked,
            CheckedState::Checked,
            CheckedState::Indeterminate,
        ],
        style: CheckboxStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
        layer: b.layer,
    };

    let pre_layout = framewise::widgets::checkbox::raw::pre_layout_checkbox(
        &framewise::widgets::checkbox::raw::CheckboxPreLayoutSpec { style: spec.style },
        framewise::layout::SizeOffer::UNBOUNDED,
    );

    framewise::widgets::checkbox::raw::post_layout_checkbox(
        spec,
        pre_layout,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.cmds,
    );
}

#[cfg(feature = "radio")]
fn draw_radio_fake_state<T: TextBackend, CF>(
    b: &mut WidgetContext<T, RowState, CF>,
    size: Vec2,
    checked: bool,
    is_focused: bool,
    is_disabled: bool,
) -> framewise::widgets::radio::raw::RadioResult {
    let mut state = RadioState {
        checked,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_keyboard_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = framewise::widgets::radio::raw::RadioSpec {
        rect: b.layout(RowLayoutParams::fixed(size.x, size.y), SizeRequest::UNKNOWN),
        disabled: is_disabled,
        style: RadioStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
        layer: b.layer,
    };

    let pre_layout = framewise::widgets::radio::raw::pre_layout_radio(
        &framewise::widgets::radio::raw::RadioPreLayoutSpec { style: spec.style },
        framewise::layout::SizeOffer::UNBOUNDED,
    );

    framewise::widgets::radio::raw::post_layout_radio(
        spec,
        pre_layout,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.cmds,
    )
}

#[cfg(feature = "switch")]
fn draw_switch_fake_state<T: TextBackend, CF>(
    b: &mut WidgetContext<T, RowState, CF>,
    layout_params: RowLayoutParams,
    checked: bool,
    is_focused: bool,
    is_disabled: bool,
) -> framewise::widgets::switch::raw::SwitchResult {
    let mut state = SwitchState {
        checked,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_keyboard_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = framewise::widgets::switch::raw::SwitchSpec {
        rect: b.layout(layout_params, SizeRequest::UNKNOWN),
        disabled: is_disabled,
        style: SwitchStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
        layer: b.layer,
    };

    let pre_layout = framewise::widgets::switch::raw::pre_layout_switch(
        &framewise::widgets::switch::raw::SwitchPreLayoutSpec { style: spec.style },
        framewise::layout::SizeOffer::UNBOUNDED,
    );

    framewise::widgets::switch::raw::post_layout_switch(
        spec,
        pre_layout,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.cmds,
    )
}

#[cfg(feature = "select")]
fn draw_select_fake_state<'s, T: TextBackend, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, LS, CF>,
    layout_params: LS::Params,
    value: &'s str,
    options: &'s [&'s str],
    is_open: bool,
    is_focused: bool,
    hovered_row: Option<usize>,
    is_disabled: bool,
) {
    let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
    let mut state = SelectState {
        open: is_open,
        hovered: hovered_row,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_keyboard_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = framewise::widgets::select::raw::SelectSpec {
        rect,
        value,
        items: options,
        disabled: is_disabled,
        style: SelectStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
        layer: b.layer,
    };

    let pre_layout_spec = framewise::widgets::select::raw::SelectPreLayoutSpec {
        value,
        style: SelectStyle::from_theme(&b.theme),
        items: options,
    };
    let pre_layout = framewise::widgets::select::raw::pre_layout_select(
        &pre_layout_spec,
        framewise::SizeOffer::UNBOUNDED,
        b.text_backend,
    );

    framewise::widgets::select::raw::post_layout_select(
        spec,
        pre_layout,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.text_backend,
        b.cmds,
    );
}

#[cfg(feature = "number_edit")]
fn draw_prefixed_number_edit_fake_state<T: TextBackend, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, LS, CF>,
    layout_params: LS::Params,
    prefix: &str,
    val: f32,
    min: f32,
    max: f32,
    is_focused: bool,
    is_dragging: bool,
    disabled: bool,
) {
    let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
    let mut state = NumberEditState {
        value: val,
        is_dragging,
        drag_start_value: val,
        ..Default::default()
    };

    let mut dummy_input = Input::default();
    if is_dragging {
        dummy_input.mouse_down = true;
    }

    let mut dummy_focus_sys = if is_focused {
        FocusSystem::new_mocked(Some(state.focus_id), None)
    } else {
        FocusSystem::new()
    };
    let mut dummy_output = framewise::Output::default();

    let mut fake_ctx = WidgetContext::root(
        b.theme,
        b.text_backend,
        &mut dummy_focus_sys,
        &dummy_input,
        &mut dummy_output,
        ManualLayout,
        rect,
        b.cmds,
    );
    fake_ctx.time = b.time;
    fake_ctx.clip_rect = b.clip_rect;
    fake_ctx.layer = b.layer;
    fake_ctx.debug_layout = b.debug_layout;
    fake_ctx.layout_policy = b.layout_policy;

    prefixed_number_edit(
        prefix,
        NumberEditSpec::new_from_theme(&b.theme)
            .min(min)
            .max(max)
            .disabled(disabled),
        Rect::new(0.0, 0.0, rect.w, rect.h),
        &mut state,
        &mut fake_ctx,
    );
    fake_ctx.finish();
}

#[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
fn draw_slider_fake_state<T: TextBackend, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, LS, CF>,
    layout_params: LS::Params,
    val: f32,
    is_dragging: bool,
    focused: bool,
) {
    let size_offer = b.peek_offer(layout_params.clone());
    let pre_layout_spec = framewise::widgets::slider::raw::SliderPreLayoutSpec {
        orientation: Orientation::Horizontal,
        style: SliderStyle::from_theme(&b.theme),
    };
    let pre_layout =
        framewise::widgets::slider::raw::pre_layout_slider(&pre_layout_spec, size_offer);
    let rect = b.layout(layout_params, pre_layout.size_request);
    let mut state = SliderState {
        value: SliderValue::Single(val),
        active_part: is_dragging.then_some(SliderPart::LowerThumb),
        drag_start_value: SliderValue::Single(val),
        press_drag: framewise::widgets::PressDragState {
            dragging: is_dragging,
            drag_start_pos: Vec2::ZERO,
            ..Default::default()
        },
        ..Default::default()
    };
    let dummy_input = Input {
        mouse_down: is_dragging,
        ..Input::default()
    };
    let mock_focus = if focused { Some(state.focus_id) } else { None };
    let mut dummy_focus_sys = FocusSystem::new_mocked(mock_focus, None);
    let spec = framewise::widgets::slider::raw::SliderSpec {
        rect,
        min: 0.0,
        max: 1.0,
        page_step: 0.1,
        step: 0.1,
        orientation: Orientation::Horizontal,
        min_gap: None,
        max_gap: None,
        value_snap: None,
        style: SliderStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: b.time,
        disabled: false,
        keyboard_focusable: true,
        layer: b.layer,
    };
    framewise::widgets::slider::raw::post_layout_slider(
        spec,
        pre_layout,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.cmds,
    );
}

#[cfg(feature = "button")]
fn draw_button_fake_state<T: TextBackend, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, LS, CF>,
    layout_params: LS::Params,
    text: &str,
    style: ButtonStyle,
    hover: bool,
    pressed: bool,
    focused: bool,
) {
    let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
    let mut state = ButtonState::default();

    let fake_input = if pressed {
        state.is_active = true;
        Input {
            mouse_pos: Vec2::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5),
            mouse_down: true,
            ..Input::default()
        }
    } else if hover {
        Input {
            mouse_pos: Vec2::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5),
            ..Input::default()
        }
    } else {
        Input::default()
    };

    let mock_focus = if focused { Some(state.focus_id) } else { None };
    let mock_hover = if hover || pressed {
        Some(state.focus_id)
    } else {
        None
    };
    let mut dummy_focus_sys = FocusSystem::new_mocked(mock_focus, mock_hover);

    let button_spec = framewise::widgets::button::raw::ButtonSpec {
        rect,
        text,
        style,
        clip_rect: None,
        disabled: false,
        layer: b.layer,
    };
    let pre_layout = framewise::widgets::button::raw::pre_layout_button(
        &framewise::widgets::button::raw::ButtonPreLayoutSpec {
            text: button_spec.text,
            style: button_spec.style,
        },
        framewise::layout::SizeOffer::UNBOUNDED,
        b.text_backend,
    );
    framewise::widgets::button::raw::post_layout_button(
        button_spec,
        pre_layout,
        &mut state,
        &fake_input,
        &mut dummy_focus_sys,
        b.text_backend,
        b.cmds,
    );
}

#[cfg(feature = "button")]
fn button_preferred_width<T: TextBackend>(
    text: &str,
    style: ButtonStyle,
    text_backend: &mut T,
) -> f32 {
    let spec = ButtonSpec::new(text).style(style);
    let spec = framewise::widgets::button::raw::ButtonPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };

    framewise::widgets::button::raw::pre_layout_button(
        &spec,
        framewise::layout::SizeOffer::UNBOUNDED,
        text_backend,
    )
    .size_request
    .preferred
    .expect("button size request should report preferred size")
    .x
}

#[cfg(feature = "button")]
fn spec_button_text_left(mut style: ButtonStyle) -> ButtonStyle {
    style.content_placement = framewise::TextContentPlacement::logical(
        framewise::ContentPlacement::Align(Align::Start),
        framewise::ContentPlacement::Align(Align::Center),
    );
    style
}

#[cfg(feature = "text_edit")]
fn draw_text_edit_fake_state<T: TextBackend, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, LS, CF>,
    layout_params: LS::Params,
    value: &str,
    placeholder: Option<&str>,
    hovered: bool,
    focused: bool,
) {
    let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
    let mut state = TextEditState::new(value);
    let style = {
        let mut s = TextEditStyle::from_theme(&b.theme);
        s.size = b.theme.text_md;
        s
    };

    let fake_input = if hovered {
        Input {
            mouse_pos: rect.center(),
            ..Input::default()
        }
    } else {
        Input::default()
    };
    let mock_focus = if focused { Some(state.focus_id) } else { None };
    let mock_hover = if hovered { Some(state.focus_id) } else { None };
    let mut dummy_focus_sys = FocusSystem::new_mocked(mock_focus, mock_hover);

    let spec = framewise::widgets::text_edit::raw::TextEditSpec {
        rect,
        style,
        placeholder: placeholder.map(str::to_string),
        clip_rect: b.clip_rect,
        error: false,
        disabled: false,
        time: 0.0,
        layer: b.layer,
        newline_policy: NewlinePolicy::ReplaceWithSpace,
        wrap: false,
        vertical_align: Align::Center,
        line_align: TextLineAlign::Start,
    };
    let pre_layout = framewise::widgets::text_edit::raw::pre_layout_text_edit(
        &framewise::widgets::text_edit::raw::TextEditPreLayoutSpec {
            style: spec.style,
            wrap: spec.wrap,
            line_align: spec.line_align,
            error: spec.error,
            disabled: spec.disabled,
            newline_policy: spec.newline_policy,
        },
        framewise::layout::SizeOffer::UNBOUNDED,
        &mut state,
        &fake_input,
        &dummy_focus_sys,
        b.text_backend,
    );
    framewise::widgets::text_edit::raw::post_layout_text_edit(
        spec,
        pre_layout,
        &mut state,
        &fake_input,
        &mut dummy_focus_sys,
        b.text_backend,
        b.cmds,
    );
}

// ── Page state ────────────────────────────────────────────────────────────────

/// Top-level state for the spec page.
///
/// `page_scroll` drives the page-level scroll wrapper and is borrowed for the
/// whole frame; the per-section widget state lives in `w` so sections can take a
/// `&mut SpecWidgetsState` that is disjoint from that borrow.
#[derive(Default)]
pub struct SpecPageState {
    pub page_scroll: ScrollState,
    pub widgets: SpecWidgetsState,
}

/// Per-section widget state. Each field is gated by the feature(s) of the
/// section that owns it, mirroring the `section_NN_*` dispatch.
pub struct SpecWidgetsState {
    // 01 Buttons
    #[cfg(feature = "button")]
    pub btn_variants: Vec<ButtonState>, // [primary, secondary, ghost, accent, icon, icon]
    #[cfg(feature = "button")]
    pub btn_matrix: Vec<ButtonState>, // 4 variants × 2 real states (default + disabled) = 8
    #[cfg(feature = "button")]
    pub btn_sizes: Vec<ButtonState>, // [sm, md, lg]
    #[cfg(all(feature = "button", feature = "number_edit"))]
    pub btn_frame_stepper: NumberEditState,
    #[cfg(all(feature = "button", not(feature = "number_edit")))]
    pub btn_grp1: Vec<ButtonState>, // fallback [←, Frame 248, →]
    #[cfg(all(feature = "button", not(feature = "number_edit")))]
    pub btn_frame: i32,
    #[cfg(feature = "button")]
    pub btn_grp2: Vec<ButtonState>, // [Build, Run, Ship]

    // 02 Text Inputs
    #[cfg(feature = "text_edit")]
    pub te_matrix: Vec<TextEditState>, // 2 rows × 5 cols = 10
    #[cfg(feature = "text_edit")]
    pub te_labelled: TextEditState,
    #[cfg(feature = "text_edit")]
    pub te_prefixed: TextEditState,
    #[cfg(feature = "text_edit")]
    pub te_multiline: TextEditState,

    // 03 Checkboxes, radios & switches
    #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
    pub cb_matrix: Vec<CheckboxState>, // 2 rows × 4 real widget-backed cols (off, on, mixed, disabled) = 8
    #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
    pub radio_states: Vec<RadioState>, // items 0,1,3 — item 2 (focused/static) stays fake
    #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
    pub switch_states: Vec<SwitchState>, // items 0,1,3 — item 2 (focused) stays fake

    // 04 Sliders & numeric drags
    #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
    pub slider1_state: SliderState,
    #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
    pub slider2_state: SliderState,
    #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
    pub slider3_state: SliderState,
    #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
    pub slider4_state: SliderState, // stepped 0–9
    #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
    pub slider_range_state: SliderState,
    #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
    pub number_edit_state: Vec<NumberEditState>, // X(320), Y(144), H(400, disabled) — W stays fake

    // 05 Selection
    #[cfg(all(
        feature = "select",
        feature = "segmented",
        feature = "chip",
        feature = "menu"
    ))]
    pub sel_state: SelectState,
    #[cfg(all(
        feature = "select",
        feature = "segmented",
        feature = "chip",
        feature = "menu"
    ))]
    pub seg1_state: SegmentedState,
    #[cfg(all(
        feature = "select",
        feature = "segmented",
        feature = "chip",
        feature = "menu"
    ))]
    pub seg2_state: SegmentedState,
    #[cfg(all(
        feature = "select",
        feature = "segmented",
        feature = "chip",
        feature = "menu"
    ))]
    pub chip_states: Vec<ChipState>, // opengl, vulkan, metal, wgpu, + add backend

    // 07 Tabs
    #[cfg(feature = "tabs")]
    pub tabs1_state: TabsState,
    #[cfg(feature = "tabs")]
    pub tabs2_state: TabsState,

    // 11 Window chrome (Inspector inner content)
    #[cfg(all(feature = "window", feature = "number_edit", feature = "checkbox"))]
    pub win11_number_edits: Vec<NumberEditState>, // X(320), Y(144), W(576), H(400)
    #[cfg(all(feature = "window", feature = "number_edit", feature = "checkbox"))]
    pub win11_cbs: Vec<CheckboxState>, // clip to parent (On), debug overlay (Off)

    // 06 Scroll areas
    #[cfg(feature = "scroll_area")]
    pub scroll_vert: ScrollState,
    #[cfg(feature = "scroll_area")]
    pub scroll_horiz: ScrollState,
    #[cfg(feature = "scroll_area")]
    pub scroll_both: ScrollState,
    #[cfg(feature = "scroll_area")]
    pub scroll_both_axes: ScrollState,

    // 12 In Use
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_backend: SegmentedState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_tabs: TabsState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_fps_slider: SliderState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_btns: Vec<ButtonState>, // [Reset, Cancel, Apply]
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_log_scroll: ScrollState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_vsync: SwitchState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_msaa: SegmentedState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_vp_w: NumberEditState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_vp_h: NumberEditState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "number_edit",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_options: Vec<CheckboxState>,
}

impl Default for SpecWidgetsState {
    fn default() -> Self {
        #[cfg(feature = "text_edit")]
        let mut te_matrix: Vec<TextEditState> = Vec::with_capacity(10);
        #[cfg(feature = "text_edit")]
        for i in 0..10 {
            te_matrix.push(match i {
                3 => TextEditState::new("§ invalid"),
                5 => TextEditState::new("render_pass"),
                6 => TextEditState::new("render_pass"),
                7 => TextEditState::new("render_pass"),
                8 => TextEditState::new("render pass"),
                9 => TextEditState::new("render_pass"),
                _ => TextEditState::new(""),
            });
        }
        Self {
            #[cfg(feature = "button")]
            btn_variants: (0..6).map(|_| ButtonState::default()).collect(),
            #[cfg(feature = "button")]
            btn_matrix: (0..8).map(|_| ButtonState::default()).collect(),
            #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
            cb_matrix: vec![
                CheckboxState {
                    checked: CheckedState::Unchecked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Checked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Indeterminate,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Checked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Unchecked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Checked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Indeterminate,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Checked,
                    ..Default::default()
                },
            ],
            #[cfg(feature = "button")]
            btn_sizes: (0..3).map(|_| ButtonState::default()).collect(),
            #[cfg(all(feature = "button", feature = "number_edit"))]
            btn_frame_stepper: NumberEditState {
                value: 248.0,
                ..Default::default()
            },
            #[cfg(all(feature = "button", not(feature = "number_edit")))]
            btn_grp1: (0..3).map(|_| ButtonState::default()).collect(),
            #[cfg(all(feature = "button", not(feature = "number_edit")))]
            btn_frame: 248,
            #[cfg(feature = "button")]
            btn_grp2: (0..3).map(|_| ButtonState::default()).collect(),
            #[cfg(feature = "text_edit")]
            te_matrix,
            #[cfg(feature = "text_edit")]
            te_labelled: TextEditState::new("framewise"),
            #[cfg(feature = "text_edit")]
            te_prefixed: TextEditState::new("0.1.0"),
            #[cfg(feature = "text_edit")]
            te_multiline: TextEditState::new(
                "A small, procedural Rust library that helps an application describe and draw GUI elements for the current frame.",
            ),
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            slider1_state: SliderState {
                value: SliderValue::Single(0.14),
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            slider2_state: SliderState {
                value: SliderValue::Single(0.62),
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            slider3_state: SliderState {
                value: SliderValue::Single(0.88),
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            slider4_state: SliderState {
                value: SliderValue::Single(3.0),
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            slider_range_state: SliderState {
                value: SliderValue::Range {
                    lower: 0.24,
                    upper: 0.76,
                },
                ..Default::default()
            },
            #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
            radio_states: vec![
                RadioState {
                    checked: true,
                    ..Default::default()
                },
                RadioState {
                    checked: false,
                    ..Default::default()
                },
                RadioState {
                    checked: false,
                    ..Default::default()
                },
            ],
            #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
            switch_states: vec![
                SwitchState {
                    checked: false,
                    ..Default::default()
                },
                SwitchState {
                    checked: true,
                    ..Default::default()
                },
                SwitchState {
                    checked: false, // multisampling disabled
                    ..Default::default()
                },
            ],
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            number_edit_state: vec![
                NumberEditState {
                    value: 320.0,
                    ..Default::default()
                },
                NumberEditState {
                    value: 144.0,
                    ..Default::default()
                },
                NumberEditState {
                    value: 400.0,
                    ..Default::default()
                },
                NumberEditState {
                    value: 12.0,
                    ..Default::default()
                },
            ],
            #[cfg(all(
                feature = "select",
                feature = "segmented",
                feature = "chip",
                feature = "menu"
            ))]
            sel_state: SelectState::default(),
            #[cfg(all(
                feature = "select",
                feature = "segmented",
                feature = "chip",
                feature = "menu"
            ))]
            seg1_state: SegmentedState {
                active_index: 0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "select",
                feature = "segmented",
                feature = "chip",
                feature = "menu"
            ))]
            seg2_state: SegmentedState {
                active_index: 1,
                ..Default::default()
            },
            #[cfg(all(
                feature = "select",
                feature = "segmented",
                feature = "chip",
                feature = "menu"
            ))]
            chip_states: vec![
                ChipState {
                    checked: true,
                    ..Default::default()
                },
                ChipState {
                    checked: false,
                    ..Default::default()
                },
                ChipState {
                    checked: false,
                    ..Default::default()
                },
                ChipState {
                    checked: false,
                    ..Default::default()
                },
                ChipState {
                    checked: false,
                    ..Default::default()
                },
            ],
            #[cfg(feature = "tabs")]
            tabs1_state: TabsState {
                active_index: 0,
                ..Default::default()
            },
            #[cfg(feature = "tabs")]
            tabs2_state: TabsState {
                active_index: 1,
                ..Default::default()
            },
            #[cfg(all(feature = "window", feature = "number_edit", feature = "checkbox"))]
            win11_number_edits: vec![
                NumberEditState {
                    value: 320.0,
                    ..Default::default()
                },
                NumberEditState {
                    value: 144.0,
                    ..Default::default()
                },
                NumberEditState {
                    value: 576.0,
                    ..Default::default()
                },
                NumberEditState {
                    value: 400.0,
                    ..Default::default()
                },
            ],
            #[cfg(all(feature = "window", feature = "number_edit", feature = "checkbox"))]
            win11_cbs: vec![
                CheckboxState {
                    checked: CheckedState::Checked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Unchecked,
                    ..Default::default()
                },
            ],
            #[cfg(feature = "scroll_area")]
            scroll_vert: ScrollState::default(),
            #[cfg(feature = "scroll_area")]
            scroll_horiz: ScrollState::default(),
            #[cfg(feature = "scroll_area")]
            scroll_both: ScrollState::default(),
            #[cfg(feature = "scroll_area")]
            scroll_both_axes: ScrollState::default(),
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_backend: SegmentedState {
                active_index: 1,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_tabs: TabsState {
                active_index: 0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_fps_slider: SliderState {
                value: SliderValue::Single(60.0),
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_btns: (0..3).map(|_| ButtonState::default()).collect(),
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_log_scroll: ScrollState::default(),
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_vsync: SwitchState {
                checked: true,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_msaa: SegmentedState {
                active_index: 2,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_vp_w: NumberEditState {
                value: 1920.0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_vp_h: NumberEditState {
                value: 1080.0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_options: vec![
                CheckboxState {
                    checked: CheckedState::Checked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Unchecked,
                    ..Default::default()
                },
                CheckboxState {
                    checked: CheckedState::Indeterminate,
                    ..Default::default()
                },
            ],
        }
    }
}

// ── Draw helpers ──────────────────────────────────────────────────────────────

// Used only by sections that show fake/static states; may be unused in minimal builds.
#[allow(dead_code)]
fn static_badge<CF, LS: LayoutState>(b: &mut WidgetContext<SampleTextBackend, LS, CF>, rect: Rect) {
    let size = 9.0;
    let color = b.theme.muted;
    let spec = LabelSpec::new("(STATIC)").style(LabelStyle {
        text_style: framewise::TextStyle {
            size,
            ..(LabelStyle::from_theme(&b.theme)).text_style
        },
        text_color: color,
        ..LabelStyle::from_theme(&b.theme)
    });
    let label_spec = framewise::widgets::label::raw::LabelSpec {
        rect,
        text: spec.text,
        style: spec.style,
        layer: b.layer,
    };
    let pre_layout = framewise::widgets::label::raw::pre_layout_label(
        &framewise::widgets::label::raw::LabelPreLayoutSpec {
            text: label_spec.text,
            style: label_spec.style,
        },
        framewise::layout::SizeOffer::UNBOUNDED,
        b.text_backend,
    );
    framewise::widgets::label::raw::post_layout_label(
        label_spec,
        pre_layout,
        b.text_backend,
        b.cmds,
    );
}

fn sec_y<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    w: f32,
    num: &str,
    title: &str,
    detail_text: &str,
) {
    b.spacer(LinearSpacer::always(112.0)); // 16.0 + 80.0 + 16.0
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);
        {
            let color = b.theme.muted;
            let spec = LabelSpec::new(num).style(LabelStyle {
                text_style: framewise::TextStyle {
                    font: b.theme.mono_font,
                    size: b.theme.text_sm,
                    letter_spacing: 0.16,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, RowLayoutParams::auto(), &mut b)
        };
        b.spacer(16.0);
        {
            let color = b.theme.ink;
            let font = b.theme.sans_font;
            let spec = LabelSpec::new(title).style(LabelStyle {
                text_style: framewise::TextStyle {
                    font,
                    size: 22.0,
                    weight: 500,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, RowLayoutParams::auto(), &mut b)
        };
        b.spacer(16.0);
        {
            let mut b = b.child_with_layout(RowLayoutParams::auto().fill_x(), ColumnLayout);
            let size = b.theme.text_mono;
            let color = b.theme.muted;
            let font = b.theme.mono_font;
            let spec = LabelSpec::new(detail_text).style(LabelStyle {
                text_style: TextStyle {
                    font,
                    size,
                    flow: {
                        let mut tf = TextFlow::wrapped();
                        tf.line_align = TextLineAlign::End;
                        tf
                    },
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(
                spec,
                ColumnLayoutParams::fixed(330.0, 48.0).align_x(Align::End),
                &mut b,
            );
            b.finish();
        };
        b.finish();
    }
    b.spacer(16.0);
    {
        let spec = DividerSpec::default_from_theme(&b.theme);
        divider(spec, ColumnLayoutParams::fixed(w, 36.0), b)
    };
}

#[allow(dead_code)]
fn group_y<CF>(b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>, text: &str) {
    b.spacer(32.0);
    {
        let text: &str = &text.to_uppercase();
        let color = b.theme.muted;
        let spec = LabelSpec::new(text).style(LabelStyle {
            text_style: b.theme.overline_text_style(b.theme.text_sm),
            text_color: color,
            ..LabelStyle::from_theme(&b.theme)
        });
        label(spec, ColumnLayoutParams::fixed(400.0, 16.0), b)
    };
    b.spacer(16.0);
}

// ── Main function ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_spec_page(
    ts: &mut SampleTextBackend,
    focus_system: &mut FocusSystem,
    state: &mut SpecPageState,
    input: &Input,
    output: &mut framewise::Output,
    time: f64,
    win_w: f32,
    win_h: f32,
    physical_pixels_per_logical_pixel: f32,
    debug_layout: bool,
) -> DrawCommands {
    let t = Theme::framewise();

    let win_rect = Rect::new(0.0, 0.0, win_w, win_h);
    let mut cmds = DrawCommands::new(physical_pixels_per_logical_pixel);
    let mut b = WidgetContext::root(
        t,
        ts,
        focus_system,
        input,
        output,
        ManualLayout,
        win_rect,
        &mut cmds,
    );
    b.time = time;

    // Background fill (outside clip so it covers the whole viewport).
    b.cmds.push(DrawCmd::FillRect {
        rect: win_rect,
        color: b.theme.paper,
        z: 0,
    });

    // Scroll area provides clip + scroll offset for all page content.
    #[cfg(feature = "button")]
    let mut page = begin_scroll_area(
        ScrollAreaSpec::default_from_theme(&b.theme).vertical(ScrollAxis {
            extent: ScrollExtent::Unbounded,
            vis: ScrollbarVisibility::Auto,
        }),
        win_rect,
        &mut state.page_scroll,
        RowLayout,
        &mut b,
    )
    .ctx;

    draw_spec_page_inner(&mut state.widgets, &mut page, debug_layout, win_rect.w);

    page.finish();

    cmds
}

pub fn draw_spec_page_inner<LS, CF>(
    state: &mut SpecWidgetsState,
    page: &mut WidgetContext<SampleTextBackend, LS, CF>,
    debug_layout: bool,
    w: f32,
) where
    <LS as SpacerLayoutState>::SpacerParams: From<LinearSpacer>,
    LS: SpacerLayoutState<Params = RowLayoutParams>,
    CF: FnOnce(
        &mut FocusSystem,
        &mut SampleTextBackend,
        &mut DrawCommands,
        &mut framewise::Output,
        framewise::Rect,
    ),
{
    let content_w = w.min(1100.0);

    page.debug_layout = debug_layout;
    // Highlight unsatisfiable layout requests in red rather than panicking (Panic is
    // the default, kept for tests).
    page.layout_policy = LayoutViolationPolicy::Highlight;

    // Scroll area provides clip + scroll offset for all page content.
    #[cfg(feature = "button")]
    let mut should_reset = false;
    {
        page.spacer(LinearSpacer::always((w - content_w) / 2.0));
        let mut content_column =
            page.child_with_layout(RowLayoutParams::auto().fixed_x(content_w), ColumnLayout);
        {
            let b = &mut content_column;

            // ── HERO ─────────────────────────────────────────────────────────────────
            header_section(b, content_w);

            // Sections are feature-gated: each draws its block, advances `y`, and is
            // skipped entirely when its widgets aren't in the build.
            #[cfg(feature = "button")]
            {
                section_01_buttons(b, content_w, state, &mut should_reset);
            }
            #[cfg(feature = "text_edit")]
            {
                section_02_text_inputs(b, content_w, state);
            }
            #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
            {
                section_03_toggles(b, content_w, state);
            }
            #[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
            {
                section_04_sliders(b, content_w, state);
            }
            #[cfg(all(
                feature = "select",
                feature = "segmented",
                feature = "chip",
                feature = "menu"
            ))]
            {
                section_05_selection(b, content_w, state);
            }
            #[cfg(feature = "scroll_area")]
            {
                section_06_scrollbars(b, content_w, state);
            }
            #[cfg(feature = "tabs")]
            {
                section_07_tabs(b, content_w, state);
            }
            #[cfg(all(
                feature = "progress_bar",
                feature = "meter",
                feature = "spinner",
                feature = "status"
            ))]
            {
                section_08_progress(b, content_w, b.time);
            }
            #[cfg(feature = "tree")]
            {
                section_09_tree(b, content_w);
            }
            #[cfg(all(feature = "tooltip", feature = "keycap"))]
            {
                section_10_tooltips(b, content_w);
            }
            #[cfg(all(feature = "window", feature = "number_edit", feature = "checkbox"))]
            {
                section_11_window(b, content_w, state);
            }
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "number_edit",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            {
                section_12_in_use(b, content_w, state);
            }

            // ── FOOTER ───────────────────────────────────────────────────────────────
            footer_section(b, content_w);
        }
        content_column.finish();
    };
    #[cfg(feature = "button")]
    if should_reset {
        *state = SpecWidgetsState::default();
    }
}

fn header_section<CF>(b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>, content_w: f32) {
    b.spacer(LinearSpacer::always(64.0));
    let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 320.0), ManualLayout);
    let logo_rect = b.layout(Rect::new(0.0, 0.0, 96.0, 96.0), SizeRequest::UNKNOWN);
    hero_logo(&b.theme, logo_rect.x, logo_rect.y, b.cmds);
    let tx = 124.0;
    // 28px gap + 96px logo = 124px
    let hero_w = content_w - 124.0;
    // Overline
    {
        let layout_params = Rect::new(tx, 0.0, hero_w, 16.0);
        let size = b.theme.text_sm;
        let color = b.theme.muted;
        let spec = LabelSpec::new("FRAMEWISE · WIDGET SPECIFICATION · V0.1").style(LabelStyle {
            text_style: b.theme.overline_text_style(size),
            text_color: color,
            ..LabelStyle::from_theme(&b.theme)
        });
        label(spec, layout_params, &mut b)
    };
    // Two-line Title (56px size, Bold, line-height 0.95)
    {
        let layout_params = Rect::new(tx, 22.0, hero_w.min(540.0), 140.0);
        let color = b.theme.ink;
        let spec = LabelSpec::new("A widget set that explains itself.").style(LabelStyle {
            text_style: b.theme.heading_text_style(56.0),
            text_color: color,
            ..LabelStyle::from_theme(&b.theme)
        });
        label(spec, layout_params, &mut b)
    };
    // Description (15px size, regular, line-height 1.55)
    {
        let layout_params = Rect::new(tx, 168.0, hero_w.min(520.0), 80.0);
        let color = Color::from_srgb_u8(58, 53, 45, 255);
        let spec = LabelSpec::new("Sharp corners, hairline borders, monospaced numerics. One accent — rust — reserved for focus, drag, and primary action. Every widget describes its state explicitly; nothing is hidden behind animation or chrome.").style(LabelStyle {
                text_style: { let mut ts = b.theme.body_text_style(15.0); ts.font = b.theme.heading_font; ts },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
        label(spec, layout_params, &mut b)
    };
    // Color Meta Row
    {
        let mut b = b.child_with_layout(Rect::new(tx, 258.0, content_w, 100.0), RowLayout);
        let meta_items: &[(&str, &str)] = &[
            ("INK", "#15130F"), //TODO: actually show these as colour swatches!
            ("PAPER", "#F4F1EA"),
            ("RUST", "#C25A2C"),
            ("TYPE", "INTER TIGHT · JETBRAINS MONO"),
        ];
        for (key, val) in meta_items {
            // key in ink, bold / medium
            {
                let size = b.theme.text_sm;
                let color = b.theme.ink;
                let spec = LabelSpec::new(key).style(LabelStyle {
                    text_style: b.theme.overline_text_style(size).with_letter_spacing(0.12),
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, RowLayoutParams::auto(), &mut b)
            };
            b.spacer(16.0);
            {
                let size = b.theme.text_sm;
                let color = b.theme.muted;
                let spec = LabelSpec::new(val).style(LabelStyle {
                    text_style: b.theme.overline_text_style(size).with_letter_spacing(0.12),
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, RowLayoutParams::auto(), &mut b)
            };
            b.spacer(40.0);
        }
    }
    {
        let spec = DividerSpec::default_from_theme(&b.theme);
        divider(spec, Rect::new(0.0, 320.0, content_w, 1.0), &mut b)
    };

    b.finish();
}
fn hero_logo(t: &Theme, x0: f32, y0: f32, cmds: &mut DrawCommands) {
    // Logo (Framewise mark), rendered as pixel-aligned filled rectangles.
    //
    // Derived from the SVG mockup: viewBox 200×200, rendered at 96×96 px (scale 0.48),
    // stroke-width 10 with square linecaps. Each stroke segment is equivalent to a filled
    // rectangle whose edges extend ±5 viewBox units past the endpoint in the stroke direction.
    // Coordinates are rounded to the nearest integer pixel for crisp, AA-free rendering.
    //
    //   Bracket V  : viewBox x 35..45, y 35..165  →  screen x+17, y+17, w 5, h 62
    //   Bracket top: viewBox x 35..61, y 35..45   →  screen x+17, y+17, w 12, h 5
    //   Bracket bot: viewBox x 35..61, y 155..165 →  screen x+17, y+74, w 12, h 5
    //   Top horiz  : viewBox x 73..145, y 35..45  →  screen x+35, y+17, w 35, h 5
    //   Middle rust: viewBox x 73..125, y 91..101 →  screen x+35, y+44, w 25, h 5
    //   Vertical   : viewBox x 73..83, y 35..165  →  screen x+35, y+17, w 5, h 62

    let fill = |cmds: &mut DrawCommands, dx: f32, dy: f32, w: f32, h: f32, color: Color| {
        cmds.push(DrawCmd::FillRect {
            rect: Rect::new(x0 + dx, y0 + dy, w, h),
            color,
            z: 0,
        });
    };

    // left bracket — vertical bar
    fill(cmds, 17., 17., 5., 62., t.ink);
    // left bracket — top arm
    fill(cmds, 17., 17., 12., 5., t.ink);
    // left bracket — bottom arm
    fill(cmds, 17., 74., 12., 5., t.ink);
    // top horizontal (connects bracket to vertical)
    fill(cmds, 35., 17., 35., 5., t.ink);
    // middle horizontal — rust accent
    fill(cmds, 35., 44., 25., 5., t.rust);
    // vertical bar
    fill(cmds, 35., 17., 5., 62., t.ink);
}

#[cfg(feature = "button")]
fn section_01_buttons<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
    should_reset: &mut bool,
) {
    // ── 01 · BUTTONS ─────────────────────────────────────────────────────────
    sec_y(b, content_w, "01", "Buttons", "primary fills with ink, accent with rust, ghost stays transparent until hovered. focus = 2px rust ring, outset.");

    // variants row
    group_y(b, "variants");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), ManualLayout {});

        let mut circle_icon_style = ButtonStyle::secondary_from_theme(&b.theme);
        circle_icon_style.text_style.font = b.theme.mono_font;
        circle_icon_style.text_style.size = 18.0;
        circle_icon_style.text_style.weight = b.theme.sans_weight_bold;
        circle_icon_style.pad_x = 0.0;
        circle_icon_style.pad_y = 0.0;
        circle_icon_style.content_placement = framewise::TextContentPlacement::INK_CENTER;
        let mut close_icon_style = ButtonStyle::secondary_from_theme(&b.theme);
        close_icon_style.text_style.size = 18.0;
        close_icon_style.text_style.weight = b.theme.sans_weight_bold;
        close_icon_style.pad_x = 0.0;
        close_icon_style.pad_y = 0.0;
        close_icon_style.content_placement = framewise::TextContentPlacement::INK_CENTER;

        let styles: &[(&str, ButtonStyle, Option<f32>)] = &[
            (
                "Apply changes",
                ButtonStyle::primary_from_theme(&b.theme),
                None,
            ),
            ("Cancel", ButtonStyle::secondary_from_theme(&b.theme), None),
            ("Reset", ButtonStyle::ghost_from_theme(&b.theme), None),
            (
                "Publish v0.2",
                ButtonStyle::accent_from_theme(&b.theme),
                None,
            ),
            ("◎", circle_icon_style, Some(b.theme.h_md)),
            ("×", close_icon_style, Some(b.theme.h_md)),
        ];
        let mut bx = 0.0;
        for (i, (label, style, width)) in styles.iter().enumerate() {
            let w = width.unwrap_or_else(|| button_preferred_width(label, *style, b.text_backend));
            let btn = {
                let state = &mut state.btn_variants[i];
                let layout_params = Rect::new(bx, 0.0, w, b.theme.h_md);
                let text: &str = label;
                let style = *style;
                let spec = ButtonSpec::new(text).style(style);
                button(spec, layout_params, state, &mut b)
            };
            if btn.input.clicked && i == 2 {
                *should_reset = true;
            }
            bx += w + 16.0;
        }

        b.finish();
    }

    // state matrix
    group_y(b, "states · default button");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), ManualLayout {});
        let mut y = 0.0;

        let col_labels = ["DEFAULT", "HOVER", "PRESSED", "FOCUSED", "DISABLED"];
        let row_labels = ["secondary", "primary", "accent", "ghost"];
        let row_styles: &[ButtonStyle] = &[
            spec_button_text_left(ButtonStyle::secondary_from_theme(&b.theme)),
            spec_button_text_left(ButtonStyle::primary_from_theme(&b.theme)),
            spec_button_text_left(ButtonStyle::accent_from_theme(&b.theme)),
            spec_button_text_left(ButtonStyle::ghost_from_theme(&b.theme)),
        ];
        let label_w = 110.0_f32;
        let col_gap = 18.0_f32;
        let row_gap = 14.0_f32;
        let cell_w = ((content_w - label_w - col_gap * 5.0) / 5.0).max(0.0);

        // column headers
        for (ci, col) in col_labels.iter().enumerate() {
            let col_x = label_w + col_gap + ci as f32 * (cell_w + col_gap);
            // Add STATIC badge for fake state columns
            if (1..=3).contains(&ci) {
                let r = b.layout(Rect::new(col_x, y - 14.0, 44.0, 12.0), SizeRequest::UNKNOWN);
                static_badge(&mut b, r);
            }
            {
                let layout_params = Rect::new(col_x, y, cell_w, 16.0);
                let color = b.theme.muted;
                let spec = LabelSpec::new(col).style(LabelStyle {
                    text_style: b.theme.overline_text_style(10.0),
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
        }
        y += 16.0 + row_gap;

        for (ri, row_label) in row_labels.iter().enumerate() {
            {
                let layout_params = Rect::new(0.0, y, label_w, b.theme.h_md);
                let spec = LabelSpec::new(row_label).style(LabelStyle {
                    text_style: b.theme.overline_text_style(12.0).with_letter_spacing(0.04),
                    text_color: b.theme.ink,
                    content_placement: framewise::TextContentPlacement::logical(
                        framewise::ContentPlacement::Align(Align::Start),
                        framewise::ContentPlacement::Align(Align::Center),
                    ),
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
            for ci in 0..5 {
                let col_x = label_w + col_gap + ci as f32 * (cell_w + col_gap);
                let rect = Rect::new(col_x, y, cell_w, b.theme.h_md);
                match ci {
                    1 => draw_button_fake_state(
                        &mut b,
                        rect,
                        "Action",
                        row_styles[ri],
                        true,
                        false,
                        false,
                    ),
                    2 => draw_button_fake_state(
                        &mut b,
                        rect,
                        "Action",
                        row_styles[ri],
                        false,
                        true,
                        false,
                    ),
                    3 => draw_button_fake_state(
                        &mut b,
                        rect,
                        "Action",
                        row_styles[ri],
                        false,
                        false,
                        true,
                    ),
                    _ => {
                        let disabled = ci == 4;
                        let idx = ri * 2 + ci / 4; // ci=0 → idx 0 (default), ci=4 → idx 1 (disabled)
                        let _btn = {
                            let state = &mut state.btn_matrix[idx];
                            let style = row_styles[ri];
                            let spec = ButtonSpec::new("Action").style(style).disabled(disabled);
                            button(spec, rect, state, &mut b)
                        };
                    }
                }
            }
            y += b.theme.h_md + row_gap;
        }
        b.finish();
    }

    // sizes & groups
    group_y(b, "sizes  ·  groups");
    {
        let mut b = b.child_with_layout(
            ColumnLayoutParams::fixed(content_w, b.theme.h_lg),
            RowLayout,
        );

        let mut compact_height_style = ButtonStyle::secondary_from_theme(&b.theme);
        compact_height_style.pad_y = 2.0;
        let size_defs: &[(&str, f32, ButtonStyle)] = &[
            ("22 px", b.theme.h_sm, compact_height_style),
            (
                "28 px",
                b.theme.h_md,
                ButtonStyle::secondary_from_theme(&b.theme),
            ),
            (
                "36 px",
                b.theme.h_lg,
                ButtonStyle::secondary_from_theme(&b.theme),
            ),
        ];
        for (i, (label, h, style)) in size_defs.iter().enumerate() {
            let w = button_preferred_width(label, *style, b.text_backend);
            let _btn = {
                let state = &mut state.btn_sizes[i];
                let layout_params = RowLayoutParams::fixed(w, *h).align_y(Align::Center);
                let text: &str = label;
                let style = *style;
                let spec = ButtonSpec::new(text).style(style);
                button(spec, layout_params, state, &mut b)
            };
            if i + 1 < size_defs.len() {
                b.spacer(16.0);
            }
        }
        b.spacer(24.0);

        // NumberEdit frame stepper: ← | Frame N | →
        #[cfg(feature = "number_edit")]
        {
            let style = ButtonStyle::secondary_from_theme(&b.theme);
            let frame_stepper_w = button_preferred_width("\u{2190}", style, b.text_backend)
                + button_preferred_width("Frame 248", style, b.text_backend)
                + button_preferred_width("\u{2192}", style, b.text_backend);
            let spec = NumberEditSpec::new_from_theme(&b.theme)
                .min(0.0)
                .no_max()
                .step(1.0)
                .page_step(10.0)
                .drag_enabled(false)
                .value_fill_enabled(false)
                .value_formatter(|v: f32| format!("Frame {v:.0}"))
                .style(NumberEditStyle::button_stepper_from_theme(&b.theme));
            number_edit(
                spec,
                RowLayoutParams::fixed(frame_stepper_w, b.theme.h_md).align_y(Align::Center),
                &mut state.btn_frame_stepper,
                &mut b,
            );
        }
        #[cfg(not(feature = "number_edit"))]
        {
            let frame_label = format!("Frame {}", state.btn_frame);
            let grp1: &[(&str, ButtonStyle)] = &[
                ("\u{2190}", ButtonStyle::secondary_from_theme(&b.theme)),
                (&frame_label, ButtonStyle::secondary_from_theme(&b.theme)),
                ("\u{2192}", ButtonStyle::secondary_from_theme(&b.theme)),
            ];
            for (i, (label, style)) in grp1.iter().enumerate() {
                let w = button_preferred_width(label, *style, b.text_backend);
                let btn = {
                    let state = &mut state.btn_grp1[i];
                    let layout_params =
                        RowLayoutParams::fixed(w, b.theme.h_md).align_y(Align::Center);
                    let text: &str = label;
                    let style = *style;
                    let spec = ButtonSpec::new(text).style(style);
                    button(spec, layout_params, state, &mut b)
                };
                if btn.input.clicked {
                    match i {
                        0 => state.btn_frame -= 1,
                        2 => state.btn_frame += 1,
                        _ => {}
                    }
                }
            }
        }
        b.spacer(16.0);

        // button group 2: Build | Run | Ship
        let grp2: &[(&str, ButtonStyle)] = &[
            ("Build", ButtonStyle::secondary_from_theme(&b.theme)),
            ("Run", ButtonStyle::secondary_from_theme(&b.theme)),
            ("Ship", ButtonStyle::primary_from_theme(&b.theme)),
        ];
        for (i, (label, style)) in grp2.iter().enumerate() {
            let w = button_preferred_width(label, *style, b.text_backend);
            let _btn = {
                let state = &mut state.btn_grp2[i];
                let layout_params = RowLayoutParams::fixed(w, b.theme.h_md).align_y(Align::Center);
                let text: &str = label;
                let style = *style;
                let spec = ButtonSpec::new(text).style(style);
                button(spec, layout_params, state, &mut b)
            };
        }

        b.finish();
    }
}

#[cfg(feature = "text_edit")]
fn section_02_text_inputs<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    sec_y(b, content_w, "02", "Text inputs", "mono caret in rust signals the live insertion point. focus ring sits inside the border so widgets don't shift.");

    group_y(b, "states · single-line");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 110.0), ManualLayout);
        let mut y = 0.0;
        let lx = 0.0;
        let col_labels = ["DEFAULT", "HOVER", "FOCUSED", "ERROR", "DISABLED"];
        let row_labels = ["empty", "filled"];
        let cell_w = 220.0_f32;
        let label_w = 110.0_f32;
        let col_gap = 18.0_f32;
        let placeholder = "frame_buffer";

        for (ci, col) in col_labels.iter().enumerate() {
            if (1..=2).contains(&ci) {
                let r = b.layout(
                    Rect::new(
                        label_w + ci as f32 * (cell_w + col_gap),
                        y - 14.0,
                        44.0,
                        12.0,
                    ),
                    SizeRequest::UNKNOWN,
                );
                static_badge(&mut b, r);
            }
            {
                let layout_params =
                    Rect::new(label_w + ci as f32 * (cell_w + col_gap), y, cell_w, 16.0);
                let color = b.theme.muted;
                let spec = LabelSpec::new(col).style(LabelStyle {
                    text_style: b.theme.overline_text_style(10.0),
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
        }
        y += 20.0;

        for (ri, row_label) in row_labels.iter().enumerate() {
            {
                let layout_params = Rect::new(lx, y, label_w - 4.0, b.theme.h_md);
                let spec = LabelSpec::new(row_label).style(LabelStyle {
                    text_style: b.theme.overline_text_style(12.0).with_letter_spacing(0.04),
                    text_color: b.theme.ink,
                    content_placement: framewise::TextContentPlacement::logical(
                        framewise::ContentPlacement::Align(Align::Start),
                        framewise::ContentPlacement::Align(Align::Center),
                    ),
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
            for ci in 0..5 {
                let idx = ri * 5 + ci;
                let error = ci == 3;
                let disabled = ci == 4;
                let layout_params = Rect::new(
                    label_w + ci as f32 * (cell_w + col_gap),
                    y,
                    cell_w,
                    b.theme.h_md,
                );
                match ci {
                    1 => {
                        let value = state.te_matrix[idx].value.clone();
                        draw_text_edit_fake_state(
                            &mut b,
                            layout_params,
                            &value,
                            Some(placeholder),
                            true,
                            false,
                        );
                    }
                    2 => {
                        let value = state.te_matrix[idx].value.clone();
                        draw_text_edit_fake_state(
                            &mut b,
                            layout_params,
                            &value,
                            Some(placeholder),
                            false,
                            true,
                        );
                    }
                    _ => {
                        let state = &mut state.te_matrix[idx];
                        let style = {
                            let mut s = TextEditStyle::from_theme(&b.theme);
                            s.size = b.theme.text_md;
                            s
                        };
                        let spec = TextEditSpec::default()
                            .style(style)
                            .placeholder(placeholder)
                            .error(error)
                            .disabled(disabled);
                        let _info = text_edit(spec, layout_params, state, &mut b);
                    }
                }
            }
            y += b.theme.h_md + 8.0;
        }
        b.finish();
    }

    group_y(b, "labelled  ·  prefixed  ·  multiline");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 130.0), ManualLayout);
        let y = 0.0;
        let lx = 0.0;
        let field_x = lx;
        {
            let layout_params = Rect::new(field_x, y, 220.0, 20.0);
            let color = b.theme.muted;
            let spec = LabelSpec::new("CRATE NAME").style(LabelStyle {
                text_style: b
                    .theme
                    .overline_text_style(b.theme.text_sm)
                    .with_letter_spacing(0.10),
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        let _info = {
            let state = &mut state.te_labelled;
            let layout_params = Rect::new(field_x, y + 18.0, 220.0, b.theme.h_md);
            let style = {
                let mut s = TextEditStyle::from_theme(&b.theme);
                s.size = b.theme.text_md;
                s
            };
            let spec = TextEditSpec::default().style(style);
            text_edit(spec, layout_params, state, &mut b)
        };
        {
            let layout_params = Rect::new(field_x, y + 18.0 + b.theme.h_md + 4.0, 220.0, 20.0);
            let color = b.theme.muted;
            let spec = LabelSpec::new("a–z, 0–9, hyphen; max 64").style(LabelStyle {
                text_style: framewise::TextStyle::new(
                    b.theme.mono_font,
                    b.theme.text_sm,
                    b.theme.sans_weight_regular,
                    framewise::TextFlow::single_line(),
                )
                .with_letter_spacing(0.04),
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // Prefixed field
        let pf_x = 248.0;
        {
            let layout_params = Rect::new(pf_x, y, 240.0, 20.0);
            let color = b.theme.muted;
            let spec = LabelSpec::new("VERSION").style(LabelStyle {
                text_style: b
                    .theme
                    .overline_text_style(b.theme.text_sm)
                    .with_letter_spacing(0.10),
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        let _info = {
            let state = &mut state.te_prefixed;
            let layout_params = Rect::new(pf_x, y + 18.0, 240.0, b.theme.h_md);
            let style = {
                let mut s = TextEditStyle::from_theme(&b.theme);
                s.size = b.theme.text_md;
                s
            };
            let spec = TextEditSpec::default().style(style);
            prefixed_text_edit("v", spec, layout_params, state, &mut b)
        };
        let err_y = y + 18.0 + b.theme.h_md + 4.0;
        {
            let badge_rect = Rect::new(pf_x, err_y + 1.0, 12.0, 12.0);
            let rect = b.layout(badge_rect, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![DrawCmd::FillRect {
                    rect,
                    color: b.theme.rust,
                    z: 0,
                }],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);

            let spec = LabelSpec::new("!").style(LabelStyle {
                text_style: framewise::TextStyle::new(
                    b.theme.mono_font,
                    9.0,
                    b.theme.sans_weight_bold,
                    framewise::TextFlow::single_line(),
                )
                .with_line_height(framewise::LineHeight::Relative(1.0)),
                text_color: b.theme.paper,
                content_placement: framewise::TextContentPlacement::INK_CENTER,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, badge_rect, &mut b);
        }
        {
            let layout_params = Rect::new(pf_x + 18.0, err_y, 222.0, 20.0);
            let color = b.theme.rust;
            let spec = LabelSpec::new("semver mismatch — bump minor").style(LabelStyle {
                text_style: framewise::TextStyle::new(
                    b.theme.mono_font,
                    b.theme.text_sm,
                    b.theme.sans_weight_regular,
                    framewise::TextFlow::single_line(),
                ),
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // Multiline field
        let ml_x = 516.0;
        {
            let layout_params = Rect::new(ml_x, y, 280.0, 20.0);
            let color = b.theme.muted;
            let spec = LabelSpec::new("DESCRIPTION").style(LabelStyle {
                text_style: b
                    .theme
                    .overline_text_style(b.theme.text_sm)
                    .with_letter_spacing(0.10),
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        let _info = {
            let state = &mut state.te_multiline;
            let layout_params = Rect::new(ml_x, y + 18.0, 280.0, 88.0);
            let style = {
                let mut s = TextEditStyle::from_theme(&b.theme);
                s.size = b.theme.text_md;
                s.padding_y = 8.0;
                s.line_height = b.theme.body_line_height;
                s
            };
            let spec = TextEditSpec::default().style(style).multiline_wrapped();
            text_edit(spec, layout_params, state, &mut b)
        };
        b.finish();
    }
}

#[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
fn section_03_toggles<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    // ── 03 · CHECK · RADIO · SWITCH ──────────────────────────────────────────
    sec_y(
        b,
        content_w,
        "03",
        "Checkboxes, radios & switches",
        "on-state is always ink. rust appears only when keyboard focus is on the control.",
    );

    group_y(b, "checkbox");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), ManualLayout {});
        let mut y = 0.0_f32;
        let label_w = 96.0_f32;
        let cell_w = 100.0_f32;

        let label_text_style = b.theme.overline_text_style(12.0).with_letter_spacing(0.04);
        let row_height = 20.0_f32;
        let checkbox_offset_y = 3.0_f32;

        let col_labels = ["OFF", "ON", "MIXED", "FOCUSED", "DISABLED"];
        for (ci, col) in col_labels.iter().enumerate() {
            // Add STATIC badge for fake state columns
            if ci == 3 {
                let r = b.layout(
                    Rect::new(label_w + ci as f32 * cell_w, y - 14.0, 44.0, 12.0),
                    SizeRequest::UNKNOWN,
                );
                static_badge(&mut b, r);
            }
            {
                let layout_params = Rect::new(label_w + ci as f32 * cell_w, y, cell_w - 4.0, 14.0);
                let color = b.theme.muted;
                let spec = LabelSpec::new(col).style(LabelStyle {
                    text_style: b.theme.overline_text_style(10.0),
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
        }
        y += 18.0;

        // Row 1: box only
        {
            let layout_params = Rect::new(0.0, y, label_w - 4.0, row_height);
            let spec = LabelSpec::new("box").style(LabelStyle {
                text_style: label_text_style,
                text_color: b.theme.ink,
                content_placement: framewise::TextContentPlacement::logical(
                    framewise::ContentPlacement::Align(Align::Start),
                    framewise::ContentPlacement::Align(Align::Center),
                ),
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        let box_specs: &[(CheckedState, bool, bool)] = &[
            (CheckedState::Unchecked, false, false),
            (CheckedState::Checked, false, false),
            (CheckedState::Indeterminate, false, false),
            (CheckedState::Checked, true, false),
            (CheckedState::Checked, false, true),
        ];
        for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
            let rect = Rect::new(
                label_w + ci as f32 * cell_w,
                y + checkbox_offset_y,
                14.0,
                14.0,
            );
            if ci < 3 {
                let _info = {
                    let state = &mut state.cb_matrix[ci];
                    let spec = CheckboxSpec::default_from_theme(&b.theme).allowed_checked_states(
                        if ci < 2 {
                            vec![CheckedState::Unchecked, CheckedState::Checked]
                        } else {
                            vec![
                                CheckedState::Unchecked,
                                CheckedState::Checked,
                                CheckedState::Indeterminate,
                            ]
                        },
                    );
                    checkbox(spec, rect, state, &mut b)
                };
            } else if ci == 3 {
                draw_checkbox_fake_state(&mut b, rect, *cs, *focused, *disabled);
            } else {
                let _info = {
                    let state = &mut state.cb_matrix[3];
                    let spec = CheckboxSpec::default_from_theme(&b.theme).disabled(true);
                    checkbox(spec, rect, state, &mut b)
                };
            }
        }
        y += row_height + 12.0;

        // Row 2: with label
        {
            let layout_params = Rect::new(0.0, y, label_w - 4.0, row_height);
            let spec = LabelSpec::new("with label").style(LabelStyle {
                text_style: label_text_style,
                text_color: b.theme.ink,
                content_placement: framewise::TextContentPlacement::logical(
                    framewise::ContentPlacement::Align(Align::Start),
                    framewise::ContentPlacement::Align(Align::Center),
                ),
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
            let cx = label_w + ci as f32 * cell_w;
            if ci < 3 {
                let state = &mut state.cb_matrix[4 + ci];
                let layout_params = Rect::new(cx, y, cell_w, row_height);
                let spec =
                    CheckboxSpec::default_from_theme(&b.theme).allowed_checked_states(if ci < 2 {
                        vec![CheckedState::Unchecked, CheckedState::Checked]
                    } else {
                        vec![
                            CheckedState::Unchecked,
                            CheckedState::Checked,
                            CheckedState::Indeterminate,
                        ]
                    });

                labelled_checkbox(spec, "vsync", layout_params, state, &mut b);
            } else if ci == 3 {
                draw_checkbox_fake_state(
                    &mut b,
                    Rect::new(cx, y + checkbox_offset_y, 14.0, 14.0),
                    *cs,
                    *focused,
                    *disabled,
                );
                let label_alpha = if *disabled {
                    b.theme.muted
                } else {
                    b.theme.ink
                };
                {
                    let layout_params = Rect::new(cx + 21.0, y + checkbox_offset_y, 60.0, 14.0);
                    let size = b.theme.text_sm;
                    let spec = LabelSpec::new("vsync").style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&b.theme)).text_style
                        },
                        text_color: label_alpha,
                        ..LabelStyle::from_theme(&b.theme)
                    });
                    label(spec, layout_params, &mut b)
                };
            } else {
                let state = &mut state.cb_matrix[7];
                let layout_params = Rect::new(cx, y, cell_w, row_height);
                let spec = CheckboxSpec::default_from_theme(&b.theme).disabled(true);
                labelled_checkbox(spec, "vsync", layout_params, state, &mut b);
            }
        }
        b.finish();
    }

    group_y(b, "radio  .  switch");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout {});
        {
            let mut b =
                b.child_with_layout(RowLayoutParams::auto().fixed_x(200.0), ColumnLayout {});
            let radio_labels = ["immediate-mode", "retained-mode", "hybrid", "deferred"];
            for (i, radio_label) in radio_labels.iter().enumerate() {
                b.spacer(LinearSpacer::always(16.0));
                match i {
                    0 | 1 => {
                        let info = {
                            let state = &mut state.radio_states[i];
                            labelled_radio(
                                RadioSpec::default_from_theme(&b.theme),
                                radio_label,
                                ColumnLayoutParams::auto(),
                                state,
                                &mut b,
                            )
                        };
                        if info.input.clicked {
                            state.radio_states[0].checked = i == 0;
                            state.radio_states[1].checked = i == 1;
                            state.radio_states[2].checked = false;
                        }
                    }
                    2 => {
                        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout {});
                        let r = draw_radio_fake_state(
                            &mut b,
                            Vec2::new(14.0, 14.0),
                            false,
                            true,
                            false,
                        )
                        .content_bounds;
                        static_badge(&mut b, Rect::new(r.x - 50.0, r.y, 144.0, 14.0));
                        b.spacer(8.0);
                        {
                            let size = b.theme.text_md;
                            let color = b.theme.ink;
                            let spec = LabelSpec::new(radio_label).style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    size,
                                    ..(LabelStyle::from_theme(&b.theme)).text_style
                                },
                                text_color: color,
                                ..LabelStyle::from_theme(&b.theme)
                            });
                            label(spec, RowLayoutParams::auto(), &mut b)
                        };
                        b.finish();
                    }
                    3 => {
                        let state = &mut state.radio_states[2];
                        labelled_radio(
                            RadioSpec::default_from_theme(&b.theme).disabled(true),
                            radio_label,
                            ColumnLayoutParams::auto(),
                            state,
                            &mut b,
                        );
                    }
                    _ => unreachable!(),
                }
            }
            b.finish();
        }
        {
            let mut b = b.child_with_layout(RowLayoutParams::auto(), ColumnLayout {});
            let switch_labels = [
                "debug overlay",
                "show layout grid",
                "vsync",
                "multisampling",
            ];
            for (i, switch_label) in switch_labels.iter().enumerate() {
                b.spacer(LinearSpacer::always(16.0));
                let label_color = if i == 3 { b.theme.muted } else { b.theme.ink };
                match i {
                    2 => {
                        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout {});
                        let r = draw_switch_fake_state(
                            &mut b,
                            RowLayoutParams::fixed(30.0, 16.0),
                            true,
                            true,
                            false,
                        )
                        .content_bounds;
                        static_badge(&mut b, Rect::new(r.x - 50.0, r.y, 144.0, 14.0));
                        b.spacer(8.0);
                        {
                            let size = b.theme.text_md;
                            let spec = LabelSpec::new(switch_label).style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    size,
                                    ..(LabelStyle::from_theme(&b.theme)).text_style
                                },
                                text_color: label_color,
                                ..LabelStyle::from_theme(&b.theme)
                            });
                            label(spec, RowLayoutParams::fixed(140.0, 16.0), &mut b)
                        };
                        b.finish();
                    }
                    3 => {
                        let state = &mut state.switch_states[2];
                        labelled_switch(
                            SwitchSpec::default_from_theme(&b.theme).disabled(true),
                            switch_label,
                            ColumnLayoutParams::auto(),
                            state,
                            &mut b,
                        );
                    }
                    _ => {
                        let state = &mut state.switch_states[i];
                        labelled_switch(
                            SwitchSpec::default_from_theme(&b.theme),
                            switch_label,
                            ColumnLayoutParams::auto(),
                            state,
                            &mut b,
                        );
                    }
                }
            }
            b.finish();
        }
        b.finish();
    }
}

#[cfg(all(feature = "slider", feature = "number_edit", feature = "color_swatch"))]
fn section_04_sliders<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    // ── 04 · SLIDERS · DRAGS ─────────────────────────────────────────────────
    sec_y(
        b,
        content_w,
        "04",
        "Sliders & numeric drags",
        "drag-number reads like a function parameter — label + value, scrubbable in either direction. fill bar shows magnitude.",
    );

    group_y(b, "slider · single value");
    {
        let mut b =
            b.child_with_layout(ColumnLayoutParams::auto().fixed_x(content_w), ColumnLayout);
        let slider_w = 260.0_f32;
        let values = [
            (0.1, &mut state.slider1_state, false, false, false),
            (0.1, &mut state.slider2_state, false, false, true),
            (0.1, &mut state.slider3_state, false, true, false),
            (1.0, &mut state.slider4_state, true, false, false),
        ];

        for (step, slider_state, show_ticks, is_static, is_disabled) in values {
            b.spacer(16.0);
            let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
            if is_static {
                draw_slider_fake_state(
                    &mut b,
                    RowLayoutParams::auto().fixed_x(slider_w),
                    0.88,
                    true,
                    true,
                );
            } else {
                let mut spec = if show_ticks {
                    let mut style = SliderStyle::from_theme(&b.theme);
                    style.track_marks = Some(TrackMarksStyle::from_theme(&b.theme, 1.0));
                    SliderSpec::default()
                        .style(style)
                        .max(9.0)
                        .page_step(1.0)
                        .step(1.0)
                        .value_snap(Some(1.0))
                } else {
                    SliderSpec::default_from_theme(&b.theme)
                        .max(1.0)
                        .page_step(step)
                        .step(step)
                };
                if is_disabled {
                    spec = spec.disabled(true);
                }
                slider(
                    spec,
                    RowLayoutParams::auto().fixed_x(slider_w),
                    slider_state,
                    &mut b,
                );
            }

            b.spacer(8.0);

            let text = if is_static {
                format!("{:.2}", 0.88)
            } else if show_ticks {
                format!("{:.0} / 9", slider_state.value.lower())
            } else {
                format!("{:.2}", slider_state.value.lower())
            };
            let spec = LabelSpec::new(&text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    font: b.theme.mono_font,
                    size: b.theme.text_mono,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: b.theme.ink,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, RowLayoutParams::auto(), &mut b);

            if is_static {
                let badge_rect = b.layout(RowLayoutParams::fixed(70.0, 12.0), SizeRequest::UNKNOWN);
                static_badge(&mut b, badge_rect);
            }

            b.finish();
        }
        b.finish();
    }

    group_y(b, "range slider");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
        let track_w = 260.0_f32;
        let spec = SliderSpec::default()
            .style(SliderStyle::range_from_theme(&b.theme))
            .max(1.0)
            .page_step(0.1)
            .step(0.01);

        slider(
            spec,
            RowLayoutParams::auto().fixed_x(track_w),
            &mut state.slider_range_state,
            &mut b,
        );

        b.spacer(8.0);

        let text = if let SliderValue::Range { lower, upper } = state.slider_range_state.value {
            format!("{:.2}–{:.2}", lower, upper)
        } else {
            String::new()
        };

        let spec = LabelSpec::new(&text).style(LabelStyle {
            text_style: framewise::TextStyle {
                font: b.theme.mono_font,
                size: b.theme.text_mono,
                ..(LabelStyle::from_theme(&b.theme)).text_style
            },
            text_color: b.theme.ink,
            ..LabelStyle::from_theme(&b.theme)
        });
        label(spec, RowLayoutParams::auto(), &mut b);
        b.finish();
    }

    group_y(b, "drag-number");
    {
        const DRAG_W: f32 = 168.0;
        const GAP: f32 = 12.0;
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 42.0), ManualLayout);
        let mut x = 0.0;
        let rect = Rect::new(x, 14.0, DRAG_W, b.theme.h_md);
        prefixed_number_edit(
            "X",
            NumberEditSpec::new_from_theme(&b.theme).max(800.0),
            rect,
            &mut state.number_edit_state[0],
            &mut b,
        );
        x += DRAG_W + GAP;
        let rect = Rect::new(x, 14.0, DRAG_W, b.theme.h_md);
        prefixed_number_edit(
            "Y",
            NumberEditSpec::new_from_theme(&b.theme).max(600.0),
            rect,
            &mut state.number_edit_state[1],
            &mut b,
        );
        x += DRAG_W + GAP;
        let badge_rect = b.layout(Rect::new(x, 0.0, 72.0, 12.0), SizeRequest::UNKNOWN);
        static_badge(&mut b, badge_rect);
        let rect = Rect::new(x, 14.0, DRAG_W, b.theme.h_md);
        draw_prefixed_number_edit_fake_state(
            &mut b, rect, "W", 576.0, 0.0, 800.0, true, true, false,
        );
        x += DRAG_W + GAP;
        let rect = Rect::new(x, 14.0, DRAG_W, b.theme.h_md);
        prefixed_number_edit(
            "H",
            NumberEditSpec::new_from_theme(&b.theme)
                .max(600.0)
                .disabled(true),
            rect,
            &mut state.number_edit_state[2],
            &mut b,
        );
        b.finish();
    }

    group_y(b, "numeric stepper  ·  colour swatch");
    {
        let mut b = b.child_with_layout(
            ColumnLayoutParams::fixed(content_w, b.theme.h_md),
            ManualLayout,
        );
        let stepper_x = 0.0;
        let origin = b.layout(Rect::new(0.0, 0.0, 0.0, 0.0), SizeRequest::UNKNOWN);
        let abs_rect = |x: f32, y: f32, w: f32, h: f32| Rect::new(origin.x + x, origin.y + y, w, h);
        let local_rect = |x: f32, y: f32, w: f32, h: f32| Rect::new(x, y, w, h);
        b.append_cmds(DrawCommands::from_vec(
            vec![
                DrawCmd::FillRect {
                    rect: abs_rect(stepper_x, 0.0, 64.0, b.theme.h_md),
                    color: b.theme.paper_hover,
                    z: 0,
                },
                DrawCmd::BorderRect {
                    rect: abs_rect(stepper_x, 0.0, 64.0, b.theme.h_md),
                    color: b.theme.line_on_paper,
                    width: 1.0,
                    placement: framewise::BorderPlacement::Inside,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: abs_rect(stepper_x + 64.0, 0.0, 40.0, b.theme.h_md),
                    color: b.theme.paper_elev,
                    z: 0,
                },
                DrawCmd::BorderRect {
                    rect: abs_rect(stepper_x + 64.0, 0.0, 40.0, b.theme.h_md),
                    color: b.theme.line_on_paper,
                    width: 1.0,
                    placement: framewise::BorderPlacement::Inside,
                    z: 0,
                },
            ],
            b.cmds.physical_pixels_per_logical_pixel(),
        ));
        for (text, rect, color) in [
            ("padding", local_rect(6.0, 7.0, 56.0, 14.0), b.theme.muted),
            ("12", local_rect(72.0, 7.0, 24.0, 14.0), b.theme.ink),
        ] {
            let spec = LabelSpec::new(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: b.theme.text_sm,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, rect, &mut b);
        }

        number_edit(
            NumberEditSpec::new_from_theme(&b.theme)
                .range(0.0, 100.0)
                .drag_enabled(false)
                .value_fill_enabled(false)
                .style(NumberEditStyle::compact_stepper_from_theme(&b.theme)),
            local_rect(120.0, 0.0, 96.0, b.theme.h_sm),
            &mut state.number_edit_state[3],
            &mut b,
        );

        let swatches: &[(Color, &str)] = &[(b.theme.ink, "#15130f"), (b.theme.rust, "#c25a2c")];
        let mut x = 220.0;
        for (color, hex) in swatches {
            let spec = ColorSwatchSpec::new(*color).border(Some(framewise::types::Stroke::new(
                b.theme.line_on_paper,
                1.0,
            )));
            let rect = local_rect(x, 0.0, 18.0, b.theme.h_md);
            color_swatch(spec, rect, &mut b);
            let spec = LabelSpec::new(hex).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: b.theme.text_sm,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: b.theme.ink,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, local_rect(x + 22.0, 7.0, 60.0, 14.0), &mut b);
            x += 86.0;
        }
        b.finish();
    }
}

#[cfg(all(
    feature = "select",
    feature = "segmented",
    feature = "chip",
    feature = "menu"
))]
fn section_05_selection<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    sec_y(b, content_w, "05", "Selection", "selects, segmented controls, and menus share one rule: the chosen thing is filled ink, paper text. no surprises.");

    group_y(b, "select  ·  segmented  ·  chips");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 120.0), ManualLayout);
        const LAYOUT_OPTS: &[&str] = &["Layout: row", "Layout: column", "Layout: grid"];
        let value = if state.sel_state.selected_index < LAYOUT_OPTS.len() {
            LAYOUT_OPTS[state.sel_state.selected_index]
        } else {
            ""
        };
        let sel_state = &mut state.sel_state;
        let rect = Rect::new(0.0, 0.0, 180.0, b.theme.h_md);
        select(
            SelectSpec::new_from_theme(value, LAYOUT_OPTS, &b.theme),
            rect,
            sel_state,
            &mut b,
        );

        let badge_rect = b.layout(
            Rect::new(0.0, b.theme.h_md + 12.0, 70.0, 12.0),
            SizeRequest::UNKNOWN,
        );
        static_badge(&mut b, badge_rect);
        let rect = Rect::new(0.0, b.theme.h_md + 28.0, 180.0, b.theme.h_md);
        draw_select_fake_state(
            &mut b,
            rect,
            "Layout row",
            LAYOUT_OPTS,
            true,
            true,
            Some(0),
            false,
        );

        const SEGS1: &[&str] = &["row", "column", "grid", "flex"];
        {
            let state = &mut state.seg1_state;
            let layout_params = Rect::new(220.0, 0.0, 0.0, b.theme.h_md);
            let spec = SegmentedSpec::new_from_theme(SEGS1, &b.theme);
            segmented(spec, layout_params, state, &mut b);
        };
        const SEGS2: &[&str] = &["start", "center", "end"];
        {
            let state = &mut state.seg2_state;
            let layout_params = Rect::new(220.0, b.theme.h_md + 8.0, 0.0, b.theme.h_md);
            let spec = SegmentedSpec::new_from_theme(SEGS2, &b.theme);
            segmented(spec, layout_params, state, &mut b);
        };

        let chip_labels = ["opengl", "vulkan", "metal", "wgpu"];
        let mut chip_x = 560.0;
        for (i, label) in chip_labels.iter().enumerate() {
            let chip_style = ChipStyle {
                text_style: TextStyle {
                    font: b.theme.sans_font,
                    size: b.theme.text_sm,
                    ..ChipStyle::from_theme(&b.theme).text_style
                },
                ..ChipStyle::from_theme(&b.theme)
            };
            let layout = layout_text(
                b.text_backend,
                label,
                chip_style.text_style,
                framewise::text::TextBounds::UNBOUNDED,
            );
            let metrics = layout.metrics();
            let chip_w = (metrics.logical_size.x + 16.0).max(32.0);
            {
                let state = &mut state.chip_states[i];
                let layout_params = Rect::new(chip_x, 0.0, chip_w, 22.0);
                let spec = ChipSpec::new(label).style(chip_style);
                chip(spec, layout_params, state, &mut b);
            };
            chip_x += chip_w + 6.0;
        }
        let chip_style = ChipStyle {
            text_style: TextStyle {
                font: b.theme.sans_font,
                size: b.theme.text_sm,
                ..ChipStyle::from_theme(&b.theme).text_style
            },
            ..ChipStyle::from_theme(&b.theme)
        };
        let add_layout = layout_text(
            b.text_backend,
            "+ add backend",
            chip_style.text_style,
            framewise::text::TextBounds::UNBOUNDED,
        );
        let add_metrics = add_layout.metrics();
        let add_w = (add_metrics.logical_size.x + 16.0).max(32.0);
        {
            let state = &mut state.chip_states[4];
            let layout_params = Rect::new(560.0, 30.0, add_w, 22.0);
            let spec = ChipSpec::new("+ add backend").style(chip_style);
            chip(spec, layout_params, state, &mut b);
        };
        b.finish();
    }

    group_y(b, "dropdown menu (open)");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 300.0), ManualLayout);
        static ITEMS1: &[MenuItem<'static>] = &[
            MenuItem::Group("FRAME"),
            MenuItem::Item {
                label: "New panel",
                shortcut: Some("⌘ N"),
                selected: false,
                disabled: false,
            },
            MenuItem::Item {
                label: "Duplicate",
                shortcut: Some("⌘ D"),
                selected: true,
                disabled: false,
            },
            MenuItem::Item {
                label: "Detach",
                shortcut: Some("⌘ ⇧ D"),
                selected: false,
                disabled: false,
            },
            MenuItem::Separator,
            MenuItem::Group("INSPECT"),
            MenuItem::Item {
                label: "Show layout grid",
                shortcut: Some("G"),
                selected: false,
                disabled: false,
            },
            MenuItem::Item {
                label: "Show id tree",
                shortcut: Some("⌘ ⇧ I"),
                selected: false,
                disabled: false,
            },
            MenuItem::Separator,
            MenuItem::Item {
                label: "Replay last frame",
                shortcut: Some("F2"),
                selected: false,
                disabled: true,
            },
        ];
        menu(
            MenuSpec::new_from_theme(ITEMS1, &b.theme),
            Rect::new(0.0, 0.0, 240.0, 0.0),
            &mut b,
        );

        static ITEMS2: &[MenuItem<'static>] = &[
            MenuItem::Group("THEME"),
            MenuItem::Item {
                label: "framewise · default",
                shortcut: None,
                selected: false,
                disabled: false,
            },
            MenuItem::Item {
                label: "framewise · ink",
                shortcut: None,
                selected: true,
                disabled: false,
            },
            MenuItem::Item {
                label: "framewise · paper",
                shortcut: None,
                selected: false,
                disabled: false,
            },
            MenuItem::Item {
                label: "custom…",
                shortcut: None,
                selected: false,
                disabled: false,
            },
        ];
        menu(
            MenuSpec::new_from_theme(ITEMS2, &b.theme),
            Rect::new(264.0, 0.0, 200.0, 0.0),
            &mut b,
        );
        b.finish();
    }
}

#[cfg(feature = "scroll_area")]
fn section_06_scrollbars<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    // ── 06 · SCROLLBARS ──────────────────────────────────────────────────────
    sec_y(b, content_w, "06", "Scrollbars",
        "always visible. thumb length encodes how much of the content fits in view; thumb position encodes scroll offset. dragging shifts the thumb to rust.");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), ManualLayout {});

        let box_gap = 24.0_f32;
        let cap_h = 20.0_f32;

        // Box 1: vertical, idle
        let b1 = Rect::new(0.0, 40.0, 180.0, 130.0);
        let b1_content = Vec2::new(180.0, 320.0);
        {
            let rect = b.layout(b1, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![DrawCmd::BorderRect {
                    rect,
                    color: b.theme.line_on_paper,
                    width: 1.0,
                    placement: framewise::BorderPlacement::Inside,
                    z: 0,
                }],
                b.cmds.physical_pixels_per_logical_pixel(),
            );

            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    ScrollAreaSpec::default_from_theme(&b.theme).vertical(ScrollAxis {
                        extent: ScrollExtent::fixed(b1_content.y),
                        vis: ScrollbarVisibility::Always,
                    }),
                    b1,
                    &mut state.scroll_vert,
                    ManualLayout,
                    &mut b,
                )
                .ctx
            };
            let code_lines = [
                "fn frame(ctx: &mut Ctx) {",
                "  ctx.window(\"Inspector\", |w| {",
                "    w.text(\"position\");",
                "    w.drag(\"x\", &mut pos.x);",
                "    w.drag(\"y\", &mut pos.y);",
                "    w.separator();",
                "    w.text(\"size\");",
                "    w.drag(\"w\", &mut size.w);",
                "    w.drag(\"h\", &mut size.h);",
                "    w.slider(\"alpha\", &mut a, 0..1);",
                "  });",
                "}",
            ];
            for (i, line) in code_lines.iter().enumerate() {
                {
                    let layout_params = Rect::new(6.0, i as f32 * 18.0 + 6.0, 160.0, 14.0);
                    let size = sa.theme.text_sm;
                    let color = sa.theme.muted;
                    let spec = LabelSpec::new(line).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&sa.theme)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&sa.theme)
                    });
                    label(spec, layout_params, &mut sa)
                };
            }
            sa.finish();
        }
        {
            let layout_params = Rect::new(b1.x, 40.0 + b1.h + 4.0, b1.w, cap_h);
            let size = b.theme.text_sm;
            let color = b.theme.muted;
            let spec = LabelSpec::new("vertical · idle").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // Box 2: vertical, dragging (same implementation, user can drag)
        let b2_x = b1.x + b1.w + box_gap;
        let b2 = Rect::new(b2_x, 40.0, 180.0, 130.0);
        let b2_content = Vec2::new(180.0, 300.0);
        {
            let rect = b.layout(b2, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![DrawCmd::BorderRect {
                    rect,
                    color: b.theme.line_on_paper,
                    width: 1.0,
                    placement: framewise::BorderPlacement::Inside,
                    z: 0,
                }],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    ScrollAreaSpec::default_from_theme(&b.theme).vertical(ScrollAxis {
                        extent: ScrollExtent::fixed(b2_content.y),
                        vis: ScrollbarVisibility::Always,
                    }),
                    b2,
                    &mut state.scroll_horiz,
                    ManualLayout,
                    &mut b,
                )
                .ctx
            };
            for i in 0..15 {
                {
                    let layout_params = Rect::new(6.0, i as f32 * 18.0 + 6.0, 160.0, 14.0);
                    let text: &str = &format!("// entry {:02}/24 — frame state", i + 1);
                    let size = sa.theme.text_sm;
                    let color = sa.theme.muted;
                    let spec = LabelSpec::new(text).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&sa.theme)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&sa.theme)
                    });
                    label(spec, layout_params, &mut sa)
                };
            }
            sa.finish();
        }
        {
            let layout_params = Rect::new(b2.x, 40.0 + b2.h + 4.0, b2.w, cap_h);
            let size = b.theme.text_sm;
            let color = b.theme.muted;
            let spec = LabelSpec::new("vertical · dragging (rust)").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // Box 3: horizontal
        let b3_x = b2_x + b2.w + box_gap;
        let b3 = Rect::new(b3_x, 40.0 + 15.0, 300.0, 100.0);
        let b3_content = Vec2::new(700.0, 100.0);
        {
            let rect = b.layout(b3, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![DrawCmd::BorderRect {
                    rect,
                    color: b.theme.line_on_paper,
                    width: 1.0,
                    placement: framewise::BorderPlacement::Inside,
                    z: 0,
                }],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    ScrollAreaSpec::default_from_theme(&b.theme)
                        .horizontal(ScrollAxis {
                            extent: ScrollExtent::fixed(b3_content.x),
                            vis: ScrollbarVisibility::Always,
                        })
                        .vertical(ScrollAxis {
                            extent: ScrollExtent::FIT,
                            vis: ScrollbarVisibility::Auto,
                        }),
                    b3,
                    &mut state.scroll_both,
                    ManualLayout,
                    &mut b,
                )
                .ctx
            };
            {
                let layout_params = Rect::new(6.0, 6.0, 680.0, 14.0);
                let size = sa.theme.text_sm;
                let color = sa.theme.muted;
                let spec = LabelSpec::new("frame.draw_rect( … )  frame.draw_text( \"hello, framewise\" )  frame.draw_image( logo )  frame.layout.push( Row )").style(LabelStyle { text_style: framewise::TextStyle { size, ..(LabelStyle::from_theme(&sa.theme)).text_style }, text_color: color, ..LabelStyle::from_theme(&sa.theme) });
                label(spec, layout_params, &mut sa)
            };
            sa.finish();
        }
        {
            let layout_params = Rect::new(b3.x, 40.0 + b3.h + 15.0 + 4.0, b3.w, cap_h);
            let size = b.theme.text_sm;
            let color = b.theme.muted;
            let spec = LabelSpec::new("horizontal").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // Box 4: both axes
        let b4_x = b3_x + b3.w + box_gap;
        let b4 = Rect::new(b4_x, 40.0, 220.0, 130.0);
        let b4_content = Vec2::new(320.0, 240.0);
        {
            let rect = b.layout(b4, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![DrawCmd::BorderRect {
                    rect,
                    color: b.theme.line_on_paper,
                    width: 1.0,
                    placement: framewise::BorderPlacement::Inside,
                    z: 0,
                }],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    ScrollAreaSpec::default_from_theme(&b.theme)
                        .horizontal(ScrollAxis {
                            extent: ScrollExtent::fixed(b4_content.x),
                            vis: ScrollbarVisibility::Always,
                        })
                        .vertical(ScrollAxis {
                            extent: ScrollExtent::fixed(b4_content.y),
                            vis: ScrollbarVisibility::Always,
                        }),
                    b4,
                    &mut state.scroll_both_axes,
                    ManualLayout,
                    &mut b,
                )
                .ctx
            };
            {
                let layout_params = Rect::new(12.0, 10.0, 160.0, 32.0);
                let size = sa.theme.text_sm;
                let color = sa.theme.muted;
                let spec =
                    LabelSpec::new("scroll surface with both bars + corner").style(LabelStyle {
                        text_style: framewise::TextStyle {
                            flow: framewise::text::TextFlow::wrapped(),
                            size,
                            ..(LabelStyle::from_theme(&sa.theme)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&sa.theme)
                    });
                label(spec, layout_params, &mut sa)
            };
            sa.finish();
        }
        {
            let layout_params = Rect::new(b4.x, 40.0 + b4.h + 4.0, b4.w, cap_h);
            let size = b.theme.text_sm;
            let color = b.theme.muted;
            let spec = LabelSpec::new("both axes").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        b.finish();
    }
}

#[cfg(feature = "tabs")]
fn section_07_tabs<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    // ── 07 · TABS ────────────────────────────────────────────────────────────
    sec_y(b, content_w, "07", "Tabs", "underline tabs for plain navigation. the rust underbar is the only chrome — no rounded pills, no shadow.");
    {
        const TABS1: &[&str] = &["Inspector", "Layout", "Timing", "Logs", "Replay"];
        tabs(
            TabsSpec::new_from_theme(TABS1, &b.theme),
            ColumnLayoutParams::fixed(content_w.min(640.0), 36.0),
            &mut state.tabs1_state,
            b,
        );
        b.spacer(20.0);

        const TABS2: &[&str] = &["frame.rs", "layout.rs", "theme.rs", "state.rs"];
        tabs(
            TabsSpec::new_from_theme(TABS2, &b.theme),
            ColumnLayoutParams::fixed(content_w.min(480.0), 36.0),
            &mut state.tabs2_state,
            b,
        );
    }
}

#[cfg(all(
    feature = "progress_bar",
    feature = "meter",
    feature = "spinner",
    feature = "status"
))]
fn section_08_progress<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    _time: f64,
) {
    // ── 08 · PROGRESS · METERS · STATUS ──────────────────────────────────────
    sec_y(b, content_w, "08", "Progress, meters & status",
        "indeterminate progress uses rust; determinate stays ink. status pills carry the only dot of color on the bar.");

    group_y(b, "progress");
    {
        let bar_items: &[(f32, bool, &str)] = &[
            (0.12, false, "12% · compiling"),
            (0.68, false, "68% · linking"),
            (0.94, true, "94% · uploading textures"),
            (f32::NAN, true, "indeterminate"),
        ];
        let bar_w = 240.0_f32;
        for (val, active, bar_label) in bar_items {
            let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
            progress_bar(
                ProgressBarSpec::new_from_theme(*val, &b.theme).active(*active),
                RowLayoutParams::fixed(bar_w, 3.0),
                &mut b,
            );
            {
                let size = b.theme.text_sm;
                let color = b.theme.muted;
                let spec = LabelSpec::new(bar_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, RowLayoutParams::auto(), &mut b)
            };
            b.finish();
        }
    }

    group_y(b, "meters");
    {
        let meters: &[(&str, f32, Option<f32>)] = &[
            ("CPU", 0.6, None),
            ("GPU", 0.8, Some(0.9)),
            ("FRAME", 1.0, None),
        ];
        for (meter_label, val, peak) in meters {
            let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
            {
                let size = b.theme.text_sm;
                let color = b.theme.muted;
                let spec = LabelSpec::new(meter_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, RowLayoutParams::fixed(36.0, 14.0), &mut b)
            };
            if *meter_label == "FRAME" {
                {
                    let size = b.theme.text_sm;
                    let color = b.theme.ink;
                    let spec = LabelSpec::new("2.4 ms").style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&b.theme)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&b.theme)
                    });
                    label(spec, RowLayoutParams::fixed(60.0, 16.0), &mut b);
                }
            } else {
                meter(
                    MeterSpec::new_from_theme(*val, &b.theme)
                        .peak(*peak)
                        .bars(10),
                    RowLayoutParams::fixed(100.0, 12.0),
                    &mut b,
                );
            }
            b.finish();
        }
    }

    group_y(b, "spinners  ·  status");
    {
        {
            let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
            spinner(
                SpinnerSpec::default_from_theme(&b.theme),
                RowLayoutParams::fixed(16.0, 16.0),
                &mut b,
            );
            {
                let size = b.theme.text_sm;
                let color = b.theme.muted;
                let spec = LabelSpec::new("loading").style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, RowLayoutParams::fixed(60.0, 14.0), &mut b)
            };

            spinner(
                SpinnerSpec::default_from_theme(&b.theme).large(true),
                RowLayoutParams::fixed(24.0, 24.0),
                &mut b,
            );
            {
                let layout_params = RowLayoutParams::fixed(50.0, 14.0);
                let size = b.theme.text_sm;
                let color = b.theme.muted;
                let spec = LabelSpec::new("large").style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
            b.finish();
        }

        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
        let status_items: &[(&str, StatusVariant)] = &[
            ("IDLE", StatusVariant::Neutral),
            ("READY", StatusVariant::Ok),
            ("FRAME DROP", StatusVariant::Warn),
            ("PANIC", StatusVariant::Err),
            ("RENDERING", StatusVariant::Live),
        ];
        for (label, variant) in status_items {
            status(
                StatusSpec::new_from_theme(label, *variant, &b.theme),
                RowLayoutParams::fixed(120.0, 12.0),
                &mut b,
            );
        }
        b.finish();
    }
}

#[cfg(feature = "tree")]
fn section_09_tree<CF>(b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>, content_w: f32) {
    // ── 09 · TREE / LIST ─────────────────────────────────────────────────────
    sec_y(b, content_w, "09", "Tree & list",
        "monospaced rows, ascii carets, ids on the right. the selected row is filled ink — it is unambiguously the focus.");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::auto(), ManualLayout {});

        static WIDGET_TREE: &[TreeRow<'static>] = &[
            TreeRow {
                indent: 0,
                caret: Some(true),
                label: "App",
                meta: Some("#0001"),
                selected: false,
            },
            TreeRow {
                indent: 1,
                caret: Some(true),
                label: "MenuBar",
                meta: Some("#0002"),
                selected: false,
            },
            TreeRow {
                indent: 2,
                caret: None,
                label: "File",
                meta: Some("#0003"),
                selected: false,
            },
            TreeRow {
                indent: 2,
                caret: None,
                label: "Edit",
                meta: Some("#0004"),
                selected: false,
            },
            TreeRow {
                indent: 1,
                caret: Some(true),
                label: "Workspace",
                meta: Some("#0010"),
                selected: false,
            },
            TreeRow {
                indent: 2,
                caret: Some(true),
                label: "Canvas",
                meta: Some("#0011"),
                selected: false,
            },
            TreeRow {
                indent: 3,
                caret: None,
                label: "Layer \"frame\"",
                meta: Some("#0014"),
                selected: true,
            },
            TreeRow {
                indent: 3,
                caret: None,
                label: "Layer \"ui\"",
                meta: Some("#0015"),
                selected: false,
            },
            TreeRow {
                indent: 2,
                caret: Some(false),
                label: "Inspector",
                meta: Some("#0020"),
                selected: false,
            },
            TreeRow {
                indent: 1,
                caret: Some(false),
                label: "StatusBar",
                meta: Some("#0030"),
                selected: false,
            },
        ];
        let tree_spec = TreeSpec::new_from_theme(WIDGET_TREE, &b.theme);
        tree(tree_spec, Rect::new(0.0, 0.0, 320.0, 0.0), &mut b);

        static FILE_LIST: &[TreeRow<'static>] = &[
            TreeRow {
                indent: 0,
                caret: None,
                label: "frame_buffer.rs",
                meta: Some("2.1 kb"),
                selected: false,
            },
            TreeRow {
                indent: 0,
                caret: None,
                label: "layout.rs",
                meta: Some("5.4 kb"),
                selected: false,
            },
            TreeRow {
                indent: 0,
                caret: None,
                label: "renderer.rs",
                meta: Some("12.0 kb"),
                selected: true,
            },
            TreeRow {
                indent: 0,
                caret: None,
                label: "state.rs",
                meta: Some("3.8 kb"),
                selected: false,
            },
            TreeRow {
                indent: 0,
                caret: None,
                label: "theme.rs",
                meta: Some("1.6 kb"),
                selected: false,
            },
            TreeRow {
                indent: 0,
                caret: None,
                label: "widget/",
                meta: Some("11 files"),
                selected: false,
            },
            TreeRow {
                indent: 0,
                caret: None,
                label: "main.rs",
                meta: Some("0.4 kb"),
                selected: false,
            },
        ];
        let file_tree_spec = TreeSpec::new_from_theme(FILE_LIST, &b.theme);
        tree(
            file_tree_spec,
            Rect::new(320.0 + 20.0, 0.0, 240.0, 0.0),
            &mut b,
        );

        b.finish();
    }
}

#[cfg(all(feature = "tooltip", feature = "keycap"))]
fn section_10_tooltips<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
) {
    // ── 10 · TOOLTIPS · KEYCAPS ──────────────────────────────────────────────
    sec_y(
        b,
        content_w,
        "10",
        "Tooltips & keycaps",
        "tooltips invert the palette — ink on paper becomes paper on ink. keycaps borrow the input border.",
    );

    group_y(b, "tooltips");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 112.0), ManualLayout);
        let mut y = 0.0;
        tooltip(
            TooltipSpec::new_from_theme(
                "Drag to scrub — hold ⌥ for fine.",
                TooltipVariant::Dark,
                &b.theme,
            ),
            Rect::new(0.0, y, 0.0, 0.0),
            &mut b,
        );
        y += 28.0 + 8.0;

        tooltip(
            TooltipSpec::new_from_theme(
                "Re-described every frame from current application state. No retained nodes.",
                TooltipVariant::Dark,
                &b.theme,
            ),
            Rect::new(0.0, y, 0.0, 0.0),
            &mut b,
        );
        y += 28.0 + 8.0;

        tooltip(
            TooltipSpec::new_from_theme(
                "⚠ shader recompiled b frame (12 ms)",
                TooltipVariant::Rust,
                &b.theme,
            ),
            Rect::new(0.0, y, 0.0, 0.0),
            &mut b,
        );
        b.finish();
    }

    group_y(b, "keycaps");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 112.0), ManualLayout);
        let mut y = 0.0;
        let key_rows: &[(&[&str], &str)] = &[
            (&["⌘", "⇧", "P"], "command palette"),
            (&["G"], "toggle layout grid"),
            (&["F2"], "replay last frame"),
            (&["⌥", "drag"], "fine scrub"),
        ];
        for (keys, desc) in key_rows {
            let mut kx = 0.0;
            for key in *keys {
                let kw = (key.len() as f32 * 7.0 + 12.0).max(24.0);
                keycap(
                    KeycapSpec::new_from_theme(key, &b.theme),
                    Rect::new(kx, y, kw, 22.0),
                    &mut b,
                );
                kx += kw + 4.0;
            }
            {
                let layout_params = Rect::new(kx + 4.0, y + 3.0, 200.0, 14.0);
                let size = b.theme.text_sm;
                let color = b.theme.muted;
                let spec = LabelSpec::new(desc).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
            y += 28.0;
        }
        b.finish();
    }
}

#[cfg(all(feature = "window", feature = "number_edit", feature = "checkbox"))]
fn section_11_window<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    sec_y(b, content_w, "11", "Window & panel chrome",
        "title bar inverts to ink. window controls are typographic — no traffic-light cosplay. status strip carries live state.");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 300.0), ManualLayout);
        let win_buttons = [
            WindowButton { symbol: "−" },
            WindowButton { symbol: "▢" },
            WindowButton { symbol: "×" },
        ];
        let win_rect = Rect::new(0.0, 0.0, 360.0, 280.0);
        let mut win = {
            let spec = WindowSpec::new_from_theme("Inspector", &b.theme)
                .buttons(&win_buttons)
                .status_bar(true)
                .status_text("RENDERING  frame #00248  2.4 ms");
            begin_window(spec, win_rect, ManualLayout, &mut b).ctx
        };

        let mut iy = 0.0;
        let mut drx = 0.0;
        let cr_w = win_rect.w - 32.0;
        for (i, (label, min, max)) in [("X", 0.0_f32, 800.0_f32), ("Y", 0.0, 600.0)]
            .iter()
            .enumerate()
        {
            let _info = {
                let state = &mut state.win11_number_edits[i];
                let min = *min;
                let max = *max;
                let layout_params = Rect::new(drx, iy, (cr_w / 2.0) - 4.0, win.theme.h_md);
                let spec = NumberEditSpec::new_from_theme(&win.theme).min(min).max(max);
                prefixed_number_edit(label, spec, layout_params, state, &mut win)
            };
            drx += (cr_w / 2.0) + 4.0;
        }
        iy += win.theme.h_md + 6.0;
        drx = 0.0;
        for (i, (label, min, max)) in [("W", 0.0_f32, 800.0_f32), ("H", 0.0, 600.0)]
            .iter()
            .enumerate()
        {
            let _info = {
                let state = &mut state.win11_number_edits[2 + i];
                let min = *min;
                let max = *max;
                let layout_params = Rect::new(drx, iy, (cr_w / 2.0) - 4.0, win.theme.h_md);
                let spec = NumberEditSpec::new_from_theme(&win.theme).min(min).max(max);
                prefixed_number_edit(label, spec, layout_params, state, &mut win)
            };
            drx += (cr_w / 2.0) + 4.0;
        }
        iy += win.theme.h_md + 10.0;
        {
            let layout_params = Rect::new(0.0, iy, cr_w, 1.0);
            let spec = DividerSpec::default_from_theme(&win.theme);
            divider(spec, layout_params, &mut win)
        };
        iy += 10.0;
        let check_labels = ["clip to parent", "debug overlay"];
        for (i, check_label) in check_labels.iter().enumerate() {
            let _cb_info = {
                let state = &mut state.win11_cbs[i];
                let layout_params = Rect::new(0.0, iy, 14.0, 14.0);
                let spec = CheckboxSpec::default_from_theme(&win.theme);
                checkbox(spec, layout_params, state, &mut win)
            };
            {
                let layout_params = Rect::new(18.0, iy, cr_w - 18.0, 14.0);
                let size = win.theme.text_md;
                let color = win.theme.ink;
                let spec = LabelSpec::new(check_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&win.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&win.theme)
                });
                label(spec, layout_params, &mut win)
            };
            iy += 22.0;
        }
        win.finish();

        let dw = Rect::new(388.0, 0.0, 300.0, 240.0);
        let dark_bg = Color::from_srgb_u8(26, 24, 20, 255);
        let darker = Color::from_srgb_u8(12, 11, 9, 255);
        let dark_bdr = Color::from_srgb_u8(58, 53, 45, 255);
        let light = b.theme.paper;
        let muted_l = b.theme.muted;

        {
            let rect = b.layout(dw, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![
                    DrawCmd::FillRect {
                        rect,
                        color: dark_bg,
                        z: 0,
                    },
                    DrawCmd::BorderRect {
                        rect,
                        color: dark_bdr,
                        width: 1.0,
                        placement: framewise::BorderPlacement::Inside,
                        z: 0,
                    },
                    DrawCmd::FillRect {
                        rect: Rect::new(rect.x, rect.y, rect.w, 26.0),
                        color: darker,
                        z: 0,
                    },
                ],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(dw.x + 10.0, 6.0, 180.0, 14.0);
            let size = b.theme.text_sm;
            let spec = LabelSpec::new("FRAMEWISE · DARK").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        {
            let layout_params = Rect::new(dw.x + dw.w - 28.0, 6.0, 20.0, 14.0);
            let size = b.theme.text_sm;
            let spec = LabelSpec::new("✕").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        let cx = dw.x + 16.0;
        let cyw = 26.0 + 16.0;
        {
            let layout_params = Rect::new(cx, cyw, 50.0, 22.0);
            let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![
                    DrawCmd::FillRect {
                        rect: Rect::new(rect.x, rect.y, 24.0, 22.0),
                        color: Color::from_srgb_u8(42, 37, 32, 255),
                        z: 0,
                    },
                    DrawCmd::BorderRect {
                        rect: Rect::new(rect.x, rect.y, 24.0, 22.0),
                        color: dark_bdr,
                        width: 1.0,
                        placement: framewise::BorderPlacement::Inside,
                        z: 0,
                    },
                    DrawCmd::FillRect {
                        rect: Rect::new(rect.x + 28.0, rect.y, 22.0, 22.0),
                        color: Color::from_srgb_u8(42, 37, 32, 255),
                        z: 0,
                    },
                    DrawCmd::BorderRect {
                        rect: Rect::new(rect.x + 28.0, rect.y, 22.0, 22.0),
                        color: dark_bdr,
                        width: 1.0,
                        placement: framewise::BorderPlacement::Inside,
                        z: 0,
                    },
                ],
                b.cmds.physical_pixels_per_logical_pixel(),
            );

            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(cx + 7.0, cyw + 5.0, 12.0, 12.0);
            let size = b.theme.text_sm;
            let spec = LabelSpec::new("⌘").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        {
            let layout_params = Rect::new(cx + 35.0, cyw + 5.0, 12.0, 12.0);
            let size = b.theme.text_sm;
            let spec = LabelSpec::new("K").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };
        {
            let layout_params = Rect::new(cx + 56.0, cyw + 5.0, 140.0, 12.0);
            let size = b.theme.text_sm;
            let spec = LabelSpec::new("search everything").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: muted_l,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // fake dark input
        let inp_y = cyw + 28.0;
        {
            let layout_params = Rect::new(cx, inp_y, dw.w - 32.0, 26.0);
            let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![
                    DrawCmd::FillRect {
                        rect,
                        color: darker,
                        z: 0,
                    },
                    DrawCmd::BorderRect {
                        rect,
                        color: dark_bdr,
                        width: 1.0,
                        placement: framewise::BorderPlacement::Inside,
                        z: 0,
                    },
                ],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(cx + 8.0, inp_y + 7.0, dw.w - 48.0, 12.0);
            let size = b.theme.text_sm;
            let spec = LabelSpec::new("type a command…").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&b.theme)).text_style
                },
                text_color: muted_l,
                ..LabelStyle::from_theme(&b.theme)
            });
            label(spec, layout_params, &mut b)
        };

        // fake dark tabs
        let tab_y = inp_y + 30.0;
        {
            let layout_params = Rect::new(cx, tab_y + 26.0, dw.w - 16.0, 1.0);
            let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
            let cmds = DrawCommands::from_vec(
                vec![DrawCmd::FillRect {
                    rect: Rect::new(rect.x, rect.y - 0.5, rect.w, 1.0),
                    color: dark_bdr,
                    z: 0,
                }],
                b.cmds.physical_pixels_per_logical_pixel(),
            );
            b.append_cmds(cmds);
        };
        let tab_items = ["Files", "Symbols", "Frames"];
        let mut tab_x = cx;
        for (i, item) in tab_items.iter().enumerate() {
            {
                let layout_params = Rect::new(tab_x, tab_y + 5.0, 60.0, 14.0);
                let size = b.theme.text_sm;
                let color = if i == 0 { light } else { muted_l };
                let spec = LabelSpec::new(item).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
            if i == 0 {
                {
                    let layout_params = Rect::new(tab_x, tab_y + 24.0, 40.0, 2.0);
                    let rect = b.layout(layout_params, SizeRequest::UNKNOWN);
                    let cmds = DrawCommands::from_vec(
                        vec![DrawCmd::FillRect {
                            rect,
                            color: b.theme.rust,
                            z: 0,
                        }],
                        b.cmds.physical_pixels_per_logical_pixel(),
                    );

                    b.append_cmds(cmds);
                };
            }
            tab_x += 60.0;
        }
        let file_y = tab_y + 32.0;
        for (i, file) in ["▸ renderer.rs", "▸ layout.rs", "▸ widget/button.rs"]
            .iter()
            .enumerate()
        {
            {
                let layout_params = Rect::new(cx, file_y + i as f32 * 18.0, 200.0, 14.0);
                let size = b.theme.text_sm;
                let spec = LabelSpec::new(file).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&b.theme)).text_style
                    },
                    text_color: muted_l,
                    ..LabelStyle::from_theme(&b.theme)
                });
                label(spec, layout_params, &mut b)
            };
        }

        b.finish();
    }
}

#[cfg(all(
    feature = "window",
    feature = "tabs",
    feature = "segmented",
    feature = "slider",
    feature = "switch",
    feature = "number_edit",
    feature = "color_swatch",
    feature = "checkbox",
    feature = "button",
    feature = "menu"
))]
fn section_12_in_use<CF>(
    b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    content_w: f32,
    state: &mut SpecWidgetsState,
) {
    sec_y(b, content_w, "12", "In use",
        "the widgets composed into the kind of panel they were designed for — a settings sheet inside an inspector window.");
    {
        let mut b = b.child_with_layout(ColumnLayoutParams::fixed(content_w, 500.0), ManualLayout);
        let y = 0.0;
        // Left: Renderer Settings window
        let win_w_left = 440.0_f32;
        let win_h_full = 480.0_f32;
        let win_buttons = [
            WindowButton { symbol: "−" },
            WindowButton { symbol: "▢" },
            WindowButton { symbol: "×" },
        ];
        let wr = Rect::new(0.0, y, win_w_left, win_h_full);
        let mut win = {
            let spec = WindowSpec::new_from_theme("Renderer Settings", &b.theme)
                .buttons(&win_buttons)
                .status_bar(true)
                .status_text("RENDERING  frame #00248  2.4 ms  Vulkan 1.3 · 4× msaa");
            begin_window(spec, wr, ManualLayout, &mut b).ctx
        };
        let cr_w = win_w_left - 32.0;

        // Tabs inside window
        let tabs_items = ["General", "Frame", "Output", "Debug"];
        let _tabs_info = {
            let state = &mut state.iu_tabs;
            let items: &[&str] = &tabs_items;
            let layout_params = Rect::new(0.0, 0.0, cr_w, 28.0);
            let spec = TabsSpec::new_from_theme(items, &win.theme);
            tabs(spec, layout_params, state, &mut win)
        };

        // Form rows
        let form_y_start = 38.0;
        let label_w = 84.0_f32;
        let widget_x = label_w + 8.0;
        let widget_w = cr_w - label_w - 8.0;
        let row_h = 28.0_f32;
        let row_gap = 8.0_f32;
        let mut fy = form_y_start;

        // backend (segmented)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("BACKEND").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        let backends = ["OpenGL", "Vulkan", "Metal", "wgpu"];
        let _backend_info = {
            let state = &mut state.iu_backend;
            let items: &[&str] = &backends;
            let layout_params = Rect::new(widget_x, fy, 0.0, row_h);
            let spec = SegmentedSpec::new_from_theme(items, &win.theme);
            segmented(spec, layout_params, state, &mut win)
        };
        fy += row_h + row_gap;

        // target fps (slider)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("TARGET FPS").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        {
            let step = 10.0;
            let layout_params = Rect::new(widget_x, fy, widget_w - 40.0, row_h);
            let spec = SliderSpec::default_from_theme(&win.theme)
                .min(24.0)
                .max(240.0)
                .page_step(step)
                .step(step);
            slider(spec, layout_params, &mut state.iu_fps_slider, &mut win);
        };
        {
            let layout_params = Rect::new(widget_x + widget_w - 34.0, fy + 7.0, 34.0, 14.0);
            let text: &str = &format!("{:.0}", state.iu_fps_slider.value.lower());
            let size = win.theme.text_sm;
            let color = win.theme.ink;
            let spec = LabelSpec::new(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        fy += row_h + row_gap;

        // vsync (switch)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("VSYNC").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        let _switch_res = {
            let state = &mut state.iu_vsync;
            let layout_params = Rect::new(widget_x, fy + 6.0, widget_w, 16.0);
            labelled_switch(
                SwitchSpec::default_from_theme(&win.theme),
                "match display",
                layout_params,
                state,
                &mut win,
            )
        };
        fy += row_h + row_gap;

        // msaa (segmented)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("MSAA").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        let msaa_opts = ["off", "2×", "4×", "8×"];
        let _seg_res = {
            let state = &mut state.iu_msaa;
            let items: &[&str] = &msaa_opts;
            let layout_params = Rect::new(widget_x, fy, 0.0, row_h);
            let spec = SegmentedSpec::new_from_theme(items, &win.theme);
            segmented(spec, layout_params, state, &mut win)
        };
        fy += row_h + row_gap;

        // viewport (number edits)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("VIEWPORT").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        let _w_res = {
            let state = &mut state.iu_vp_w;
            let layout_params = Rect::new(widget_x, fy, (widget_w / 2.0) - 4.0, row_h);
            let spec = NumberEditSpec::new_from_theme(&win.theme)
                .max(7680.0)
                .value_formatter(|v: f32| format!("{v:.0}"));
            prefixed_number_edit("W", spec, layout_params, state, &mut win)
        };

        let _h_res = {
            let state = &mut state.iu_vp_h;
            let layout_params = Rect::new(
                widget_x + (widget_w / 2.0) + 4.0,
                fy,
                (widget_w / 2.0) - 4.0,
                row_h,
            );
            let spec = NumberEditSpec::new_from_theme(&win.theme)
                .max(7680.0)
                .value_formatter(|v: f32| format!("{v:.0}"));
            prefixed_number_edit("H", spec, layout_params, state, &mut win)
        };
        fy += row_h + row_gap;

        // accent (color swatch + button)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("ACCENT").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        let spec = ColorSwatchSpec::new(win.theme.rust).border(Some(
            framewise::types::Stroke::new(win.theme.line_on_paper, 1.0),
        ));
        color_swatch(spec, Rect::new(widget_x, fy + 4.0, 18.0, 20.0), &mut win);
        {
            let layout_params = Rect::new(widget_x + 22.0, fy + 7.0, 60.0, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.ink;
            let spec = LabelSpec::new("#c25a2c").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        fy += row_h + row_gap;

        // options (checkboxes)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = win.theme.text_sm;
            let color = win.theme.muted;
            let spec = LabelSpec::new("OPTIONS").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&win.theme)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&win.theme)
            });
            label(spec, layout_params, &mut win)
        };
        let opt_labels = [
            "show layout grid",
            "log every frame",
            "tessellate (per-mesh)",
        ];
        for (i, opt_label) in opt_labels.iter().enumerate() {
            let opt_y = fy + i as f32 * 22.0;
            let _cb_res = {
                let state = &mut state.iu_options[i];
                let layout_params = Rect::new(widget_x, opt_y + 4.0, 14.0, 14.0);
                let spec = CheckboxSpec::default_from_theme(&win.theme);
                checkbox(spec, layout_params, state, &mut win)
            };

            {
                let layout_params = Rect::new(widget_x + 18.0, opt_y + 4.0, widget_w - 18.0, 14.0);
                let size = win.theme.text_md;
                let color = win.theme.ink;
                let spec = LabelSpec::new(opt_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&win.theme)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&win.theme)
                });
                label(spec, layout_params, &mut win)
            };
        }
        fy += 3.0 * 22.0 + 4.0;

        {
            let layout_params = Rect::new(0.0, fy, cr_w, 1.0);
            let spec = DividerSpec::default_from_theme(&win.theme);
            divider(spec, layout_params, &mut win)
        };
        fy += 10.0;

        // button row
        let mut btn_x = cr_w;
        let btns: &[(&str, ButtonStyle)] = &[
            ("Apply", ButtonStyle::primary_from_theme(&win.theme)),
            ("Cancel", ButtonStyle::primary_from_theme(&win.theme)),
            ("Reset", ButtonStyle::ghost_from_theme(&win.theme)),
        ];
        for (i, (label, style)) in btns.iter().enumerate() {
            let bw = label.len() as f32 * 7.0 + 20.0;
            btn_x -= bw;
            let _btn = {
                let state = &mut state.iu_btns[i];
                let layout_params = Rect::new(btn_x, fy, bw, win.theme.h_md);
                let text: &str = label;
                let style = *style;
                let spec = ButtonSpec::new(text).style(style);
                button(spec, layout_params, state, &mut win)
            };
            btn_x -= 8.0;
        }
        win.finish();

        // Right column
        let rcol_x = win_w_left + 24.0;
        let rcol_w = (content_w - win_w_left - 24.0).max(0.0);

        // Frame Log window
        let fl_h = 310.0_f32;
        let fl_buttons = [
            WindowButton { symbol: "⌕" },
            WindowButton { symbol: "⏸" },
            WindowButton { symbol: "×" },
        ];
        let fl_rect = Rect::new(rcol_x, y, rcol_w, fl_h);
        let mut fl_win = {
            let spec = WindowSpec::new_from_theme("Frame Log", &b.theme)
                .buttons(&fl_buttons)
                .status_bar(true)
                .status_text("RECORDING  248 frames  2.6 ms avg");
            begin_window(spec, fl_rect, ManualLayout, &mut b).ctx
        };
        let fl_cr_w = rcol_w - 32.0;
        let fl_cr_h = fl_h - 80.0; // 26 title + 22 status + 32 padding

        // Scroll area for log content
        let fl_scroll_rect = Rect::new(0.0, 0.0, fl_cr_w, fl_cr_h);
        let log_lines: &[(&str, &str, bool)] = &[
            ("00248 · 2.40ms", "frame begin", false),
            ("00248 · 2.41ms", "layout(row) · 14 nodes", false),
            ("00248 · 2.45ms", "draw_rect( inspector )", false),
            ("00248 · 2.48ms", "draw_text( \"Inspector\", 14px )", false),
            ("00248 · 2.61ms", "drag_started( \"X\", 320.00 )", true),
            ("00248 · 2.74ms", "state.x ← 322.00", false),
            ("00248 · 2.89ms", "invalidate( panel#0011 )", false),
            ("00248 · 3.10ms", "frame end · uploaded 14 commands", false),
            ("00249 · 2.36ms", "frame begin", false),
            ("00249 · 2.40ms", "layout(row) · 14 nodes", false),
            ("00249 · 2.50ms", "draw_rect( inspector )", false),
            ("00249 · 2.52ms", "state.x ← 324.00", false),
        ];
        let log_content_h = log_lines.len() as f32 * 18.0 + 8.0;
        {
            let mut log_page = {
                let content_size = Vec2::new(fl_scroll_rect.w, log_content_h);
                let inner_layout = ManualLayout;
                begin_scroll_area(
                    ScrollAreaSpec::default_from_theme(&fl_win.theme).vertical(ScrollAxis {
                        extent: ScrollExtent::fixed(content_size.y),
                        vis: ScrollbarVisibility::Auto,
                    }),
                    fl_scroll_rect,
                    &mut state.iu_log_scroll,
                    inner_layout,
                    &mut fl_win,
                )
                .ctx
            };
            let loy = 4.0;
            for (i, (ts_str, msg, highlight)) in log_lines.iter().enumerate() {
                let row_y = loy + i as f32 * 18.0;
                let ts_w = 100.0_f32;
                {
                    let layout_params = Rect::new(6.0, row_y, ts_w, 14.0);
                    let size = log_page.theme.text_sm;
                    let color = log_page.theme.muted;
                    let spec = LabelSpec::new(ts_str).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&log_page.theme)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&log_page.theme)
                    });
                    label(spec, layout_params, &mut log_page)
                };
                let msg_color = if *highlight {
                    log_page.theme.rust
                } else {
                    log_page.theme.ink
                };
                {
                    let layout_params = Rect::new(
                        6.0 + ts_w + 8.0,
                        row_y,
                        fl_scroll_rect.w - ts_w - 14.0,
                        14.0,
                    );
                    let size = log_page.theme.text_sm;
                    let spec = LabelSpec::new(msg).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&log_page.theme)).text_style
                        },
                        text_color: msg_color,
                        ..LabelStyle::from_theme(&log_page.theme)
                    });
                    label(spec, layout_params, &mut log_page)
                };
            }
            log_page.finish();
        }
        fl_win.finish();

        // Quick Actions window
        let qa_y = y + fl_h + 16.0;
        let qa_buttons = [WindowButton { symbol: "×" }];
        let qa_rect = Rect::new(rcol_x, qa_y, rcol_w, 174.0);
        let mut qa_win = {
            let spec = WindowSpec::new_from_theme("Quick actions", &b.theme)
                .buttons(&qa_buttons)
                .status_bar(false)
                .status_text("");
            begin_window(spec, qa_rect, ManualLayout, &mut b).ctx
        };
        let qa_cr_w = rcol_w - 32.0;

        let qa_items = [
            MenuItem::Item {
                label: "Render frame",
                shortcut: Some("F1"),
                selected: false,
                disabled: false,
            },
            MenuItem::Item {
                label: "Replay last frame",
                shortcut: Some("F2"),
                selected: true,
                disabled: false,
            },
            MenuItem::Item {
                label: "Show id tree",
                shortcut: Some("⌘ ⇧ I"),
                selected: false,
                disabled: false,
            },
            MenuItem::Separator,
            MenuItem::Item {
                label: "Dump state to clipboard",
                shortcut: Some("⌘ ⇧ C"),
                selected: false,
                disabled: false,
            },
        ];
        menu(
            MenuSpec::new_from_theme(&qa_items, &qa_win.theme),
            Rect::new(0.0, -8.0, qa_cr_w, 0.0),
            &mut qa_win,
        );
        qa_win.finish();
        let _ = win_h_full;
        b.finish();
    }
}

fn footer_section<CF>(b: &mut WidgetContext<SampleTextBackend, ColumnState, CF>, content_w: f32) {
    const FOOTER_MARGIN_TOP: f32 = 40.0;
    const FOOTER_TOP_PAD: f32 = 28.0;
    const FOOTER_ITEM_GAP: f32 = 32.0;
    const FOOTER_PAIR_GAP: f32 = 8.0;
    const FOOTER_ROW_GAP: f32 = 32.0;
    const FOOTER_MEASURE_PAD: f32 = 4.0;

    let footer_text = b
        .theme
        .overline_text_style(b.theme.text_sm)
        .with_letter_spacing(0.10);
    let key_style = LabelStyle {
        text_style: footer_text,
        text_color: b.theme.ink,
        ..LabelStyle::from_theme(&b.theme)
    };
    let value_style = LabelStyle {
        text_style: footer_text,
        text_color: b.theme.muted,
        ..LabelStyle::from_theme(&b.theme)
    };

    let title_key = "FRAMEWISE";
    let title_value = "· WIDGET SPECIFICATION";
    let title_key_layout = layout_text(
        b.text_backend,
        title_key,
        footer_text,
        framewise::text::TextBounds::UNBOUNDED,
    );
    let title_key_metrics = title_key_layout.metrics();
    let title_value_layout = layout_text(
        b.text_backend,
        title_value,
        footer_text,
        framewise::text::TextBounds::UNBOUNDED,
    );
    let title_value_metrics = title_value_layout.metrics();
    let title_w = title_key_metrics.logical_size.x
        + FOOTER_PAIR_GAP
        + title_value_metrics.logical_size.x
        + FOOTER_MEASURE_PAD;
    let title_h = title_key_metrics
        .logical_size
        .y
        .max(title_value_metrics.logical_size.y);

    let mut footer =
        b.child_with_layout(ColumnLayoutParams::auto().fixed_x(content_w), ColumnLayout);

    footer.spacer(LinearSpacer::always(FOOTER_MARGIN_TOP));
    divider(
        DividerSpec::default_from_theme(&footer.theme),
        ColumnLayoutParams::fixed(content_w, 1.0),
        &mut footer,
    );
    footer.spacer(FOOTER_TOP_PAD);

    let foot_items: &[(&str, &str)] = &[
        ("SPEC", "V0.1 · 12 SECTIONS"),
        ("RADIUS", "0 PX"),
        ("BORDERS", "1 PX INK"),
        ("FOCUS", "2 PX RUST OUTSET"),
        ("DENSITY", "28 PX ROW · 14 PX LABEL · 12 PX MONO"),
    ];
    {
        let mut meta_row = footer.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
        for (key, val) in foot_items {
            let mut pair = meta_row.child_with_layout(RowLayoutParams::auto(), RowLayout);
            label(
                LabelSpec::new(key).style(key_style),
                RowLayoutParams::auto(),
                &mut pair,
            );
            pair.spacer(FOOTER_PAIR_GAP);
            label(
                LabelSpec::new(val).style(value_style),
                RowLayoutParams::auto(),
                &mut pair,
            );
            pair.finish();
            meta_row.spacer(FOOTER_ITEM_GAP);
        }
        meta_row.finish();
    }

    footer.spacer(FOOTER_ROW_GAP);

    {
        let mut title_row = footer.child_with_layout(
            ColumnLayoutParams::fixed(title_w, title_h).align_x(Align::End),
            RowLayout,
        );
        label(
            LabelSpec::new(title_key).style(key_style),
            RowLayoutParams::auto(),
            &mut title_row,
        );
        title_row.spacer(FOOTER_PAIR_GAP);
        label(
            LabelSpec::new(title_value).style(value_style),
            RowLayoutParams::auto(),
            &mut title_row,
        );
        title_row.finish();
    }

    footer.spacer(LinearSpacer::always(128.0));

    footer.finish();
}
