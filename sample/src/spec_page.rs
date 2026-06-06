#![allow(clippy::too_many_arguments)]
//! Interactive widget specification page — mirrors mockups/Framewise Widgets.html.
//!
//! The page is feature-gated section by section: a small core (label, divider,
//! scroll_area) renders the scaffolding, and each `section_NN_*` fn below is
//! compiled in only when the widgets it demonstrates are enabled.

use crate::text::SampleTextSystem;
#[allow(unused_imports)]
use framewise::text::TextSystem;
use framewise::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    layout::{IntrinsicSize, LayoutState, Placement},
    layouts::ManualLayout,
    text::{TextFlow, TextStyle},
    theme::Theme,
    types::{Rect, Vec2},
    widget::WidgetContext,
    Align, ColumnLayout, ColumnState, HorizontalAlign, LayoutViolationPolicy, ManualState,
    Placement2D, RowLayout, Size,
};

// Core widgets — required by the page scaffolding (section headers, captions,
// hero, footer, and the page-level scroll wrapper).
use framewise::widgets::divider::divider;
use framewise::widgets::label::label;
use framewise::widgets::scroll_area::{
    begin_scroll_area, ScrollAreaSpecBuilder, ScrollAxis, ScrollExtent, ScrollState,
    ScrollbarVisibility,
};
use framewise::widgets::{DividerSpecBuilder, LabelSpecBuilder, LabelStyle};

#[allow(unused_imports)]
use framewise::types::Color;

// Per-widget imports — present only when the owning feature is enabled. Marked
// `unused_imports`-allowed because a widget feature can be on while the (possibly
// multi-widget) section that consumes it is off.
#[cfg(feature = "checkbox")]
#[allow(unused_imports)]
use framewise::widgets::checkbox::{
    checkbox, raw::checkbox as raw_checkbox, raw::CheckboxSpec, CheckboxSpecBuilder, CheckboxState,
    CheckboxStyle, CheckedState,
};
#[cfg(feature = "chip")]
#[allow(unused_imports)]
use framewise::widgets::chip::{chip, ChipSpecBuilder, ChipState, ChipStyle};
#[cfg(feature = "color_swatch")]
#[allow(unused_imports)]
use framewise::widgets::color_swatch::{color_swatch, ColorSwatchSpecBuilder};
#[cfg(feature = "drag_number")]
#[allow(unused_imports)]
use framewise::widgets::drag_number::{
    drag_number, raw::drag_number as raw_drag_number, raw::DragNumberSpec, DragNumberSpecBuilder,
    DragNumberState, DragNumberStyle,
};
#[cfg(feature = "keycap")]
#[allow(unused_imports)]
use framewise::widgets::keycap::{keycap, KeycapSpecBuilder};
#[cfg(feature = "menu")]
#[allow(unused_imports)]
use framewise::widgets::menu::{menu, MenuItem, MenuSpecBuilder};
#[cfg(feature = "meter")]
#[allow(unused_imports)]
use framewise::widgets::meter::{meter, MeterSpecBuilder};
#[cfg(feature = "progress_bar")]
#[allow(unused_imports)]
use framewise::widgets::progress_bar::{progress_bar, ProgressBarSpecBuilder};
#[cfg(feature = "radio")]
#[allow(unused_imports)]
use framewise::widgets::radio::{
    radio, raw::radio as raw_radio, raw::RadioSpec, RadioSpecBuilder, RadioState, RadioStyle,
};
#[cfg(feature = "segmented")]
#[allow(unused_imports)]
use framewise::widgets::segmented::{segmented, SegmentedSpecBuilder, SegmentedState};
#[cfg(feature = "select")]
#[allow(unused_imports)]
use framewise::widgets::select::{
    raw::select as raw_select, raw::SelectSpec, select, SelectSpecBuilder, SelectState, SelectStyle,
};
#[cfg(feature = "slider")]
#[allow(unused_imports)]
use framewise::widgets::slider::{slider, SliderSpecBuilder, SliderState};
#[cfg(feature = "spinner")]
#[allow(unused_imports)]
use framewise::widgets::spinner::{spinner, SpinnerSpecBuilder};
#[cfg(feature = "status")]
#[allow(unused_imports)]
use framewise::widgets::status::{status, StatusSpecBuilder, StatusVariant};
#[cfg(feature = "switch")]
#[allow(unused_imports)]
use framewise::widgets::switch::{
    raw::switch as raw_switch, raw::SwitchSpec, switch, SwitchSpecBuilder, SwitchState, SwitchStyle,
};
#[cfg(feature = "tabs")]
#[allow(unused_imports)]
use framewise::widgets::tabs::{tabs, TabsSpecBuilder, TabsState};
#[cfg(feature = "text_edit")]
#[allow(unused_imports)]
use framewise::widgets::text_edit::{text_edit, TextEditSpecBuilder, TextEditState};
#[cfg(feature = "tooltip")]
#[allow(unused_imports)]
use framewise::widgets::tooltip::{tooltip, TooltipSpecBuilder, TooltipVariant};
#[cfg(feature = "tree")]
#[allow(unused_imports)]
use framewise::widgets::tree::{tree, TreeRow, TreeSpecBuilder};
#[cfg(feature = "window")]
#[allow(unused_imports)]
use framewise::widgets::window::{begin_window, WindowButton, WindowSpecBuilder};
#[cfg(feature = "button")]
#[allow(unused_imports)]
use framewise::widgets::{
    button::{button, raw::button as raw_button, ButtonState, ButtonStyle},
    ButtonSpecBuilder,
};

// ── Fake State Helpers ────────────────────────────────────────────────────────

#[cfg(feature = "checkbox")]
fn draw_checkbox_fake_state<T: TextSystem, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, ColumnState, CF>,
    layout_params: LS::Params,
    state_val: CheckedState,
    is_focused: bool,
    is_disabled: bool,
) {
    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
    let mut state = CheckboxState {
        checked: state_val,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = CheckboxSpec {
        rect,
        disabled: is_disabled,
        style: CheckboxStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
    };

    let result = raw_checkbox(spec, &mut state, &dummy_input, &mut dummy_focus_sys);
    {
        let cmds = result.draw;
        b.append_cmds(cmds);
    };
}

#[cfg(feature = "radio")]
fn draw_radio_fake_state<T: TextSystem, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, ColumnState, CF>,
    layout_params: LS::Params,
    checked: bool,
    is_focused: bool,
    is_disabled: bool,
) {
    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
    let mut state = RadioState {
        checked,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = RadioSpec {
        rect,
        disabled: is_disabled,
        style: RadioStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
    };

    let result = raw_radio(spec, &mut state, &dummy_input, &mut dummy_focus_sys);
    {
        let cmds = result.draw;
        b.append_cmds(cmds);
    };
}

#[cfg(feature = "switch")]
fn draw_switch_fake_state<T: TextSystem, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, ColumnState, CF>,
    layout_params: LS::Params,
    checked: bool,
    is_focused: bool,
    is_disabled: bool,
) {
    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
    let mut state = SwitchState {
        checked,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = SwitchSpec {
        rect,
        disabled: is_disabled,
        style: SwitchStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
    };

    let result = raw_switch(spec, &mut state, &dummy_input, &mut dummy_focus_sys);
    {
        let cmds = result.draw;
        b.append_cmds(cmds);
    };
}

#[cfg(feature = "select")]
fn draw_select_fake_state<'s, T: TextSystem, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, ColumnState, CF>,
    layout_params: LS::Params,
    value: &'s str,
    options: &'s [&'s str],
    is_open: bool,
    is_focused: bool,
    hovered_row: Option<usize>,
    is_disabled: bool,
) {
    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
    let mut state = SelectState {
        open: is_open,
        hovered: hovered_row,
        ..Default::default()
    };

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = SelectSpec {
        rect,
        value,
        items: options,
        disabled: is_disabled,
        style: SelectStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
    };

    let result = raw_select(
        spec,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.text_system,
    );
    {
        let cmds = result.draw;
        b.append_cmds(cmds);
    };
}

#[cfg(feature = "drag_number")]
fn draw_drag_number_fake_state<T: TextSystem, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, ColumnState, CF>,
    layout_params: LS::Params,
    label: &str,
    val: f32,
    min: f32,
    max: f32,
    is_active: bool,
) {
    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
    let mut state = DragNumberState {
        value: val,
        is_dragging: is_active,
        ..Default::default()
    };

    let dummy_input = Input::default();
    let spec = DragNumberSpec {
        rect,
        text: label,
        min,
        max,
        disabled: false,
        style: DragNumberStyle::from_theme(&b.theme),
        clip_rect: b.clip_rect,
    };

    let mut dummy_focus_sys = FocusSystem::new();
    let result = raw_drag_number(
        spec,
        &mut state,
        &dummy_input,
        &mut dummy_focus_sys,
        b.text_system,
    );
    {
        let cmds = result.draw;
        b.append_cmds(cmds);
    };
}

#[cfg(feature = "button")]
fn draw_button_fake_state<T: TextSystem, LS: LayoutState, CF>(
    b: &mut WidgetContext<T, LS, CF>,
    layout_params: LS::Params,
    text: &str,
    style: ButtonStyle,
    hover: bool,
    pressed: bool,
    focused: bool,
) {
    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
    let mut state = ButtonState::default();
    let mut dummy_focus_sys = FocusSystem::new();

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
        if focused {
            dummy_focus_sys.take_focus(state.focus_id);
        }
        Input::default()
    };

    let spec = ButtonSpecBuilder::new()
        .text(text)
        .style(style)
        .rect(rect)
        .clip_rect(None)
        .build();

    raw_button(
        spec,
        &mut state,
        &fake_input,
        &mut dummy_focus_sys,
        b.text_system,
        b.cmds,
    );
}

// ── Page state ────────────────────────────────────────────────────────────────

/// Top-level state for the spec page.
///
/// `page_scroll` drives the page-level scroll wrapper and is borrowed for the
/// whole frame; the per-section widget state lives in `w` so sections can take a
/// `&mut SpecWidgets` that is disjoint from that borrow.
#[derive(Default)]
pub struct SpecPageState {
    pub page_scroll: ScrollState,
    pub w: SpecWidgets,
}

/// Per-section widget state. Each field is gated by the feature(s) of the
/// section that owns it, mirroring the `section_NN_*` dispatch.
pub struct SpecWidgets {
    // 01 Buttons
    #[cfg(feature = "button")]
    pub btn_variants: Vec<ButtonState>, // [secondary, primary, accent, ghost]
    #[cfg(feature = "button")]
    pub btn_matrix: Vec<ButtonState>, // 4 variants × 2 real states (default + disabled) = 8
    #[cfg(feature = "button")]
    pub btn_sizes: Vec<ButtonState>, // [sm, md, lg]
    #[cfg(feature = "button")]
    pub btn_grp1: Vec<ButtonState>, // [←, Frame 248, →]
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
    pub cb_matrix: Vec<CheckboxState>, // 2 rows × 3 interactive cols (off, on, mixed) = 6
    #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
    pub radio_states: Vec<RadioState>, // items 0,1,2 — item 3 (focused) stays fake
    #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
    pub switch_states: Vec<SwitchState>, // items 0,1,3 — item 2 (focused) stays fake

    // 04 Sliders & numeric drags
    #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
    pub slider1_state: SliderState,
    #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
    pub slider2_state: SliderState,
    #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
    pub slider3_state: SliderState,
    #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
    pub slider4_state: SliderState, // stepped 0–9
    #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
    pub dn_showcase: Vec<DragNumberState>, // X(320), Y(144), H(400) — W stays fake

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
    #[cfg(all(feature = "window", feature = "drag_number", feature = "checkbox"))]
    pub win11_drags: Vec<DragNumberState>, // X(320), Y(144), W(576), H(400)
    #[cfg(all(feature = "window", feature = "drag_number", feature = "checkbox"))]
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
        feature = "drag_number",
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
        feature = "drag_number",
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
        feature = "drag_number",
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
        feature = "drag_number",
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
        feature = "drag_number",
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
        feature = "drag_number",
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
        feature = "drag_number",
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
        feature = "drag_number",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_vp_w: DragNumberState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "drag_number",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_vp_h: DragNumberState,
    #[cfg(all(
        feature = "window",
        feature = "tabs",
        feature = "segmented",
        feature = "slider",
        feature = "switch",
        feature = "drag_number",
        feature = "color_swatch",
        feature = "checkbox",
        feature = "button",
        feature = "menu"
    ))]
    pub iu_options: Vec<CheckboxState>,
}

impl Default for SpecWidgets {
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
            btn_variants: (0..4).map(|_| ButtonState::default()).collect(),
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
            ],
            #[cfg(feature = "button")]
            btn_sizes: (0..3).map(|_| ButtonState::default()).collect(),
            #[cfg(feature = "button")]
            btn_grp1: (0..3).map(|_| ButtonState::default()).collect(),
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
                "A small, procedural Rust library for describing GUI elements per frame.",
            ),
            #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
            slider1_state: SliderState {
                value: 0.14,
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
            slider2_state: SliderState {
                value: 0.62,
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
            slider3_state: SliderState {
                value: 0.88,
                ..Default::default()
            },
            #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
            slider4_state: SliderState {
                value: 3.0,
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
            #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
            dn_showcase: vec![
                DragNumberState {
                    value: 320.0,
                    ..Default::default()
                },
                DragNumberState {
                    value: 144.0,
                    ..Default::default()
                },
                DragNumberState {
                    value: 400.0,
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
            #[cfg(all(feature = "window", feature = "drag_number", feature = "checkbox"))]
            win11_drags: vec![
                DragNumberState {
                    value: 320.0,
                    ..Default::default()
                },
                DragNumberState {
                    value: 144.0,
                    ..Default::default()
                },
                DragNumberState {
                    value: 576.0,
                    ..Default::default()
                },
                DragNumberState {
                    value: 400.0,
                    ..Default::default()
                },
            ],
            #[cfg(all(feature = "window", feature = "drag_number", feature = "checkbox"))]
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
                feature = "drag_number",
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
                feature = "drag_number",
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
                feature = "drag_number",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_fps_slider: SliderState {
                value: 60.0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "drag_number",
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
                feature = "drag_number",
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
                feature = "drag_number",
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
                feature = "drag_number",
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
                feature = "drag_number",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_vp_w: DragNumberState {
                value: 1920.0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "drag_number",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            iu_vp_h: DragNumberState {
                value: 1080.0,
                ..Default::default()
            },
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "drag_number",
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

// ── Layout constants ──────────────────────────────────────────────────────────

const MARGIN: f32 = 64.0;
// GROUP_GAP/COL_GAP are only referenced by some sections; unused when those are
// compiled out.
#[allow(dead_code)]
const GROUP_GAP: f32 = 28.0;
#[allow(dead_code)]
const COL_GAP: f32 = 16.0;

// ── Draw helpers ──────────────────────────────────────────────────────────────

// Used only by sections that show fake/static states; may be unused in minimal builds.
#[allow(dead_code)]
fn static_badge<CF>(
    b: &mut WidgetContext<SampleTextSystem, ManualState, CF>,
    t: &Theme,
    x: f32,
    y: f32,
) {
    {
        let layout_params = Rect::new(x, y, 44.0, 12.0);
        let size = 9.0;
        let color = t.muted;
        let spec_builder = LabelSpecBuilder::new().text("(STATIC)").style(LabelStyle {
            text_style: framewise::TextStyle {
                size,
                ..(LabelStyle::from_theme(t)).text_style
            },
            text_color: color,
            ..LabelStyle::from_theme(t)
        });
        label(b, spec_builder, layout_params)
    };
}

fn sec_y<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    w: f32,
    num: &str,
    title: &str,
    detail_text: &str,
) {
    b.layout(Placement2D::fixed(0.0, 80.0), IntrinsicSize::UNKNOWN); // Spacer
    {
        let mut b = b.child_with_layout(
            Placement2D {
                width: Placement::Fill,
                height: Placement::Sized {
                    size: Size::Auto,
                    align: Align::Start,
                },
            },
            RowLayout { spacing: 16.0 },
        );
        {
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text(num).style(LabelStyle {
                text_style: framewise::TextStyle {
                    font: t.mono_font,
                    size: t.text_sm,
                    letter_spacing: 0.16,
                    ..(LabelStyle::from_theme(t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(t)
            });
            label(&mut b, spec_builder, Placement2D::auto())
        };
        {
            let color = t.ink;
            let font = t.sans_font;
            let spec_builder = LabelSpecBuilder::new().text(title).style(LabelStyle {
                text_style: framewise::TextStyle {
                    font,
                    size: 22.0,
                    weight: 500,
                    ..(LabelStyle::from_theme(t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(t)
            });
            label(&mut b, spec_builder, Placement2D::auto())
        };
        {
            let mut b = b.child_with_layout(
                Placement2D {
                    width: Placement::Fill,
                    height: Placement::Sized {
                        size: Size::Auto,
                        align: Align::Start,
                    },
                },
                ColumnLayout { spacing: 0.0 },
            );
            let size = t.text_mono;
            let color = t.muted;
            let font = t.mono_font;
            let spec_builder = LabelSpecBuilder::new().text(detail_text).style(LabelStyle {
                text_style: TextStyle {
                    font,
                    size,
                    flow: {
                        let mut tf = TextFlow::wrapped();
                        tf.horizontal_align = HorizontalAlign::End;
                        tf
                    },
                    ..(LabelStyle::from_theme(t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(t)
            });
            label(
                &mut b,
                spec_builder,
                Placement2D {
                    width: Placement::Sized {
                        size: Size::Fixed(330.0),
                        align: Align::End,
                    },
                    height: Placement::Sized {
                        size: Size::Fixed(48.0),
                        align: Align::Start,
                    },
                },
            );
            b.finish();
        };
        b.finish();
    }
    {
        let spec_builder = DividerSpecBuilder::new();
        divider(b, spec_builder, Placement2D::fixed(w, 36.0))
    };
}

#[allow(dead_code)]
fn group_y<CF>(b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>, t: &Theme, text: &str) {
    {
        let text: &str = &text.to_uppercase();
        let size = t.text_sm;
        let color = t.muted;
        let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
            text_style: framewise::TextStyle {
                size,
                ..(LabelStyle::from_theme(t)).text_style
            },
            text_color: color,
            ..LabelStyle::from_theme(t)
        });
        label(b, spec_builder, Placement2D::fixed(400.0, 16.0))
    };
}

// ── Main function ─────────────────────────────────────────────────────────────

pub fn draw_spec_page(
    ts: &mut SampleTextSystem,
    focus_system: &mut FocusSystem,
    state: &mut SpecPageState,
    input: &Input,
    time: f64,
    win_w: f32,
    win_h: f32,
    debug_layout: bool,
) -> DrawCommands {
    let t = Theme::framewise();

    let content_w = (win_w - MARGIN * 2.0).min(1100.0);

    let win_rect = Rect::new(0.0, 0.0, win_w, win_h);
    let mut cmds = DrawCommands::new();
    let mut b = {
        let mut w_ctx = WidgetContext::root(
            t,
            ts,
            focus_system,
            input,
            ManualLayout,
            win_rect,
            &mut cmds,
        );
        w_ctx.theme.sans_font = t.mono_font;
        w_ctx.debug_layout = debug_layout;
        // Highlight unsatisfiable layout requests in red rather than panicking (Panic is
        // the default, kept for tests).
        w_ctx.layout_policy = LayoutViolationPolicy::Highlight;
        w_ctx
    };

    // Background fill (outside clip so it covers the whole viewport).
    b.cmds.push(DrawCmd::FillRect {
        rect: win_rect,
        color: t.paper,
    });

    // Scroll area provides clip + scroll offset for all page content.
    #[cfg(feature = "button")]
    let mut should_reset = false;
    {
        let mut page = begin_scroll_area(
            &mut b,
            ScrollAreaSpecBuilder::new().vertical(ScrollAxis {
                extent: ScrollExtent::Unbounded,
                vis: ScrollbarVisibility::Auto,
            }),
            win_rect,
            &mut state.page_scroll,
            RowLayout { spacing: 0.0 },
        )
        .ctx;
        page.layout(
            Placement2D::fixed(content_w / 4.0, 0.0),
            IntrinsicSize::UNKNOWN,
        ); // Spacer
        let mut content_column = page.child_with_layout(
            Placement2D {
                width: Placement::Sized {
                    size: Size::Fixed(content_w),
                    align: Align::Center,
                },
                height: Placement::Sized {
                    size: Size::Auto,
                    align: Align::Start,
                },
            },
            ColumnLayout { spacing: 16.0 },
        );
        {
            let b = &mut content_column;

            // ── HERO ─────────────────────────────────────────────────────────────────
            header_section(b, t, content_w);

            // Sections are feature-gated: each draws its block, advances `y`, and is
            // skipped entirely when its widgets aren't in the build.
            #[cfg(feature = "button")]
            {
                section_01_buttons(b, &t, content_w, &mut state.w, &mut should_reset);
            }
            #[cfg(feature = "text_edit")]
            {
                section_02_text_inputs(b, &t, content_w, &mut state.w);
            }
            #[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
            {
                section_03_toggles(b, &t, content_w, &mut state.w);
            }
            #[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
            {
                section_04_sliders(b, &t, content_w, &mut state.w);
            }
            #[cfg(all(
                feature = "select",
                feature = "segmented",
                feature = "chip",
                feature = "menu"
            ))]
            {
                section_05_selection(b, &t, content_w, &mut state.w);
            }
            #[cfg(feature = "scroll_area")]
            {
                section_06_scrollbars(b, &t, content_w, &mut state.w);
            }
            #[cfg(feature = "tabs")]
            {
                section_07_tabs(b, &t, content_w, &mut state.w);
            }
            #[cfg(all(
                feature = "progress_bar",
                feature = "meter",
                feature = "spinner",
                feature = "status"
            ))]
            {
                section_08_progress(b, &t, content_w, time);
            }
            #[cfg(feature = "tree")]
            {
                section_09_tree(b, &t, content_w, y);
            }
            #[cfg(all(feature = "tooltip", feature = "keycap"))]
            {
                section_10_tooltips(b, &t, content_w, y);
            }
            #[cfg(all(feature = "window", feature = "drag_number", feature = "checkbox"))]
            {
                section_11_window(b, &t, content_w, &mut state.w);
            }
            #[cfg(all(
                feature = "window",
                feature = "tabs",
                feature = "segmented",
                feature = "slider",
                feature = "switch",
                feature = "drag_number",
                feature = "color_swatch",
                feature = "checkbox",
                feature = "button",
                feature = "menu"
            ))]
            {
                section_12_in_use(b, &t, content_w, &mut state.w);
            }

            // ── FOOTER ───────────────────────────────────────────────────────────────
            footer_section(b, t, content_w);
        } // end content block (drops `b` alias, releases borrow on `page`)
        content_column.finish();
        page.finish()
    }; // end page_cmds block
       // `state`/`time` are only consumed by feature-gated sections; silence unused
       // warnings in builds where those sections are compiled out.
    let _ = (&state, time);
    #[cfg(feature = "button")]
    if should_reset {
        *state = SpecPageState::default();
    }
    b.finish();
    cmds
}

fn header_section<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: Theme,
    content_w: f32,
) {
    let mut b = b.child_with_layout(Placement2D::fixed(content_w, MARGIN + 320.0), ManualLayout);
    let logo_rect = b.layout(Rect::new(0.0, MARGIN, 96.0, 96.0), IntrinsicSize::UNKNOWN);
    b.append_cmds(hero_logo(&t, logo_rect.x, logo_rect.y));
    let tx = 124.0;
    // 28px gap + 96px logo = 124px
    let hero_w = content_w - 124.0;
    // Overline
    {
        let layout_params = Rect::new(tx, MARGIN, hero_w, 16.0);
        let size = t.text_sm;
        let color = t.muted;
        let spec_builder = LabelSpecBuilder::new()
            .text("FRAMEWISE · WIDGET SPECIFICATION · V0.1")
            .style(LabelStyle {
                text_style: t.overline_text_style(size),
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
        label(&mut b, spec_builder, layout_params)
    };
    // Two-line Title (56px size, Bold, line-height 0.95)
    {
        let layout_params = Rect::new(tx, MARGIN + 22.0, hero_w.min(540.0), 140.0);
        let color = t.ink;
        let spec_builder = LabelSpecBuilder::new()
            .text("A widget set that explains itself.")
            .style(LabelStyle {
                text_style: t.heading_text_style(56.0),
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
        label(&mut b, spec_builder, layout_params)
    };
    // Description (15px size, regular, line-height 1.55)
    {
        let layout_params = Rect::new(tx, MARGIN + 168.0, hero_w.min(520.0), 80.0);
        let color = Color::from_srgb_u8(58, 53, 45, 255);
        let spec_builder = LabelSpecBuilder::new()
            .text("Sharp corners, hairline borders, monospaced numerics. One accent — rust — reserved for focus, drag, and primary action. Every widget describes its state explicitly; nothing is hidden behind animation or chrome.")
            .style(LabelStyle {
                text_style: { let mut ts = t.body_text_style(15.0); ts.font = t.heading_font; ts },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
        label(&mut b, spec_builder, layout_params)
    };
    // Color Meta Row
    {
        let mut b = b.child_with_layout(
            Rect::new(tx, MARGIN + 258.0, content_w, 100.0),
            RowLayout { spacing: 16.0 },
        );
        let meta_items: &[(&str, &str)] = &[
            ("INK", "#15130F"), //TODO: actually show these as colour swatches!
            ("PAPER", "#F4F1EA"),
            ("RUST", "#C25A2C"),
            ("TYPE", "INTER TIGHT · JETBRAINS MONO"),
        ];
        for (key, val) in meta_items {
            // key in ink, bold / medium
            {
                let size = t.text_sm;
                let color = t.ink;
                let spec_builder = LabelSpecBuilder::new().text(key).style(LabelStyle {
                    text_style: t.overline_text_style(size).with_letter_spacing(0.12),
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut b, spec_builder, Placement2D::auto())
            };
            {
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(val).style(LabelStyle {
                    text_style: t.overline_text_style(size).with_letter_spacing(0.12),
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut b, spec_builder, Placement2D::auto())
            };
            b.layout(Placement2D::fixed(8.0, 0.0), IntrinsicSize::UNKNOWN); // Spacer
        }
    }
    {
        let spec_builder = DividerSpecBuilder::new();
        divider(
            &mut b,
            spec_builder,
            Rect::new(0.0, MARGIN + 320.0, content_w, 1.0),
        )
    };

    b.finish();
}
fn hero_logo(t: &Theme, x0: f32, y0: f32) -> DrawCommands {
    let mut cmds = DrawCommands::new();

    // Logo (Framewise mark), scaled from 200×200 viewBox → 96×96 px
    let ls = 0.48_f32;
    let lx0 = x0;
    let lw = 4.8_f32;

    // Since lines are drawn using "butt end caps" (which terminate flat at endpoints),
    // we manually extend/overlap connected segment coordinates by half the stroke width
    // (5.0 viewBox units / 2.4 screen pixels) to form perfect miter-like joins and
    // simulate square cap endings.
    let ext = 5.0_f32;

    cmds.extend(vec![
        // left bracket
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + (56. + ext) * ls, y0 + 40. * ls),
            p1: Vec2::new(lx0 + (40. - ext) * ls, y0 + 40. * ls),
            color: t.ink,
            width: lw,
        },
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 40. * ls, y0 + (40. - ext) * ls),
            p1: Vec2::new(lx0 + 40. * ls, y0 + (160. + ext) * ls),
            color: t.ink,
            width: lw,
        },
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + (40. - ext) * ls, y0 + 160. * ls),
            p1: Vec2::new(lx0 + (56. + ext) * ls, y0 + 160. * ls),
            color: t.ink,
            width: lw,
        },
        // top horizontal
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + (78. - ext) * ls, y0 + 40. * ls),
            p1: Vec2::new(lx0 + (140. + ext) * ls, y0 + 40. * ls),
            color: t.ink,
            width: lw,
        },
        // middle horizontal (rust)
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + (78. - ext) * ls, y0 + 96. * ls),
            p1: Vec2::new(lx0 + (120. + ext) * ls, y0 + 96. * ls),
            color: t.rust,
            width: lw,
        },
        // vertical
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 78. * ls, y0 + (40. - ext) * ls),
            p1: Vec2::new(lx0 + 78. * ls, y0 + (160. + ext) * ls),
            color: t.ink,
            width: lw,
        },
    ]);

    cmds
}

#[cfg(feature = "button")]
fn section_01_buttons<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    state: &mut SpecWidgets,
    should_reset: &mut bool,
) {
    let t = *t;
    // ── 01 · BUTTONS ─────────────────────────────────────────────────────────
    sec_y(b, &t, content_w, "01", "Buttons", "primary fills with ink, accent with rust, ghost stays transparent until hovered. focus = 2px rust ring, outset.");

    // variants row
    group_y(b, &t, "variants");
    {
        let mut b = b.child_with_layout(Placement2D::auto(), ManualLayout {});

        let styles: &[(&str, ButtonStyle, bool)] = &[
            ("Apply changes", ButtonStyle::primary_from_theme(&t), false),
            ("Cancel", ButtonStyle::primary_from_theme(&t), false),
            ("Reset", ButtonStyle::ghost_from_theme(&t), false),
            ("Publish v0.2", ButtonStyle::accent_from_theme(&t), false),
        ];
        let mut bx = 0.0;
        for (i, (label, style, _)) in styles.iter().enumerate() {
            let w = label.len() as f32 * 7.0 + 24.0;
            let btn = {
                let state = &mut state.btn_variants[i];
                let layout_params = Rect::new(bx, 0.0, w, t.h_md);
                let text: &str = label;
                let style = *style;
                let spec_builder = ButtonSpecBuilder::new().text(text).style(style);
                button(&mut b, spec_builder, layout_params, state)
            };
            if btn.input.clicked && i == 2 {
                *should_reset = true;
            }
            bx += w + COL_GAP;
        }

        b.finish();
    }

    // state matrix
    group_y(b, &t, "states · default button");
    {
        let mut b = b.child_with_layout(Placement2D::auto(), ManualLayout {});
        let mut y = 20.0;

        let col_labels = ["DEFAULT", "HOVER", "PRESSED", "FOCUSED", "DISABLED"];
        let row_labels = ["secondary", "primary", "accent", "ghost"];
        let row_styles: &[ButtonStyle] = &[
            ButtonStyle::primary_from_theme(&t),
            ButtonStyle::primary_from_theme(&t),
            ButtonStyle::accent_from_theme(&t),
            ButtonStyle::ghost_from_theme(&t),
        ];
        let label_w = 80.0_f32;
        let cell_w = 88.0_f32;

        // column headers
        for (ci, col) in col_labels.iter().enumerate() {
            // Add STATIC badge for fake state columns
            if (1..=3).contains(&ci) {
                static_badge(&mut b, &t, label_w + ci as f32 * cell_w, y - 14.0);
            }
            {
                let layout_params = Rect::new(label_w + ci as f32 * cell_w, y, cell_w - 8.0, 16.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(col).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut b, spec_builder, layout_params)
            };
        }
        y += 20.0;

        for (ri, row_label) in row_labels.iter().enumerate() {
            {
                let layout_params = Rect::new(0.0, y, label_w - 8.0, t.h_md);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(row_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut b, spec_builder, layout_params)
            };
            for ci in 0..5 {
                let rect = Rect::new(label_w + ci as f32 * cell_w, y, cell_w - 8.0, t.h_md);
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
                            let spec_builder = ButtonSpecBuilder::new()
                                .text("Action")
                                .style(style)
                                .disabled(disabled);
                            button(&mut b, spec_builder, rect, state)
                        };
                    }
                }
            }
            y += t.h_md + 4.0;
        }
        b.finish();
    }

    // sizes & groups
    group_y(b, &t, "sizes  ·  groups");
    {
        let mut b = b.child_with_layout(Placement2D::auto(), ManualLayout {});

        let size_defs: &[(&str, f32, ButtonStyle)] = &[
            ("22 px", t.h_sm, ButtonStyle::primary_from_theme(&t)),
            ("28 px", t.h_md, ButtonStyle::primary_from_theme(&t)),
            ("36 px", t.h_lg, ButtonStyle::primary_from_theme(&t)),
        ];
        let mut bx = 0.0;
        for (i, (label, h, style)) in size_defs.iter().enumerate() {
            let w = label.len() as f32 * 7.0 + 20.0;
            let _btn = {
                let state = &mut state.btn_sizes[i];
                let layout_params = Rect::new(bx, 40.0, w, *h);
                let text: &str = label;
                let style = *style;
                let spec_builder = ButtonSpecBuilder::new().text(text).style(style);
                button(&mut b, spec_builder, layout_params, state)
            };
            bx += w + COL_GAP;
        }
        bx += 24.0;

        // button group 1: ← | Frame 248 | →
        let grp1: &[(&str, ButtonStyle)] = &[
            ("←", ButtonStyle::primary_from_theme(&t)),
            ("Frame 248", ButtonStyle::primary_from_theme(&t)),
            ("→", ButtonStyle::primary_from_theme(&t)),
        ];
        // draw group border
        for (i, (label, style)) in grp1.iter().enumerate() {
            let w = label.len() as f32 * 7.0 + 20.0;
            let _btn = {
                let state = &mut state.btn_grp1[i];
                let layout_params = Rect::new(bx, 40.0, w, t.h_md);
                let text: &str = label;
                let style = *style;
                let spec_builder = ButtonSpecBuilder::new().text(text).style(style);
                button(&mut b, spec_builder, layout_params, state)
            };
            bx += w;
        }
        bx += COL_GAP;

        // button group 2: Build | Run | Ship
        let grp2: &[(&str, ButtonStyle)] = &[
            ("Build", ButtonStyle::primary_from_theme(&t)),
            ("Run", ButtonStyle::primary_from_theme(&t)),
            ("Ship", ButtonStyle::primary_from_theme(&t)),
        ];
        for (i, (label, style)) in grp2.iter().enumerate() {
            let w = label.len() as f32 * 7.0 + 20.0;
            let _btn = {
                let state = &mut state.btn_grp2[i];
                let layout_params = Rect::new(bx, 40.0, w, t.h_md);
                let text: &str = label;
                let style = *style;
                let spec_builder = ButtonSpecBuilder::new().text(text).style(style);
                button(&mut b, spec_builder, layout_params, state)
            };
            bx += w;
        }
        let _ = bx;

        b.finish();
    }
}

#[cfg(feature = "text_edit")]
fn section_02_text_inputs<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 02 · TEXT INPUTS ─────────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "02", "Text inputs", "mono caret in rust signals the live insertion point. focus ring sits inside the border so widgets don't shift.");
    y += 46.0;

    group_y(b, &t, y, "states · single-line");
    y += 20.0;
    {
        let col_labels = ["DEFAULT", "HOVER", "FOCUSED", "ERROR", "DISABLED"];
        let row_labels = ["empty", "filled"];
        let cell_w = 160.0_f32;
        let label_w = 60.0_f32;

        for (ci, col) in col_labels.iter().enumerate() {
            {
                let layout_params =
                    Rect::new(label_w + ci as f32 * (cell_w + 8.0), y, cell_w, 16.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(col).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
        }
        y += 20.0;

        for (ri, row_label) in row_labels.iter().enumerate() {
            {
                let layout_params = Rect::new(lx, y, label_w - 4.0, t.h_md);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(row_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
            for ci in 0..5 {
                let idx = ri * 5 + ci;
                let error = ci == 3;
                let disabled = ci == 4;
                let _info = {
                    let state = &mut state.te_matrix[idx];
                    let layout_params =
                        Rect::new(label_w + ci as f32 * (cell_w + 8.0), y, cell_w, t.h_md);
                    let spec_builder = TextEditSpecBuilder::new().error(error).disabled(disabled);
                    text_edit(b, spec_builder, layout_params, state)
                };
            }
            y += t.h_md + 8.0;
        }
    }
    y += GROUP_GAP;

    group_y(b, &t, y, "labelled  ·  prefixed  ·  multiline");
    y += 20.0;
    {
        // Labelled field
        let field_x = lx;
        {
            let layout_params = Rect::new(field_x, y, 120.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("CRATE NAME")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };
        let _info = {
            let state = &mut state.te_labelled;
            let layout_params = Rect::new(field_x, y + 18.0, 160.0, t.h_md);
            let spec_builder = TextEditSpecBuilder::new();
            text_edit(b, spec_builder, layout_params, state)
        };
        {
            let layout_params = Rect::new(field_x, y + 18.0 + t.h_md + 4.0, 200.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("a–z, 0–9, hyphen; max 64")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };

        // Prefixed field (draw prefix addon manually)
        let pf_x = 200.0;
        {
            let layout_params = Rect::new(pf_x, y, 120.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("VERSION").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(pf_x, y + 18.0, 24.0, t.h_md);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect { rect, color: t.ink },
                DrawCmd::StrokeRect {
                    rect,
                    color: t.line,
                    width: 1.0,
                },
            ]);
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(pf_x + 6.0, y + 18.0 + 7.0, 16.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("v").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        let _info = {
            let state = &mut state.te_prefixed;
            let layout_params = Rect::new(pf_x + 24.0, y + 18.0, 120.0, t.h_md);
            let spec_builder = TextEditSpecBuilder::new();
            text_edit(b, spec_builder, layout_params, state)
        };
        {
            let layout_params = Rect::new(pf_x, y + 18.0 + t.h_md + 4.0, 200.0, 14.0);
            let size = t.text_sm;
            let color = t.rust;
            let spec_builder = LabelSpecBuilder::new()
                .text("semver mismatch — bump minor")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };

        // Multiline field
        let ml_x = 420.0;
        {
            let layout_params = Rect::new(ml_x, y, 120.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("DESCRIPTION")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };
        let _info = {
            let state = &mut state.te_multiline;
            let layout_params = Rect::new(ml_x, y + 18.0, 280.0, 68.0);
            let spec_builder = TextEditSpecBuilder::new();
            text_edit(b, spec_builder, layout_params, state)
        };
    }
    y += 18.0 + 68.0 + 4.0 + 14.0 + SEC_GAP;
    y
}

#[cfg(all(feature = "checkbox", feature = "radio", feature = "switch"))]
fn section_03_toggles<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 03 · CHECK · RADIO · SWITCH ──────────────────────────────────────────
    sec_y(
        b,
        &t,
        lx,
        y,
        content_w,
        "03",
        "Checkboxes, radios & switches",
        "on-state is always ink. rust appears only when keyboard focus is on the control.",
    );
    y += 46.0;

    group_y(b, &t, y, "checkbox");
    y += 20.0;
    {
        let col_labels = ["OFF", "ON", "MIXED", "FOCUSED", "DISABLED"];
        let label_w = 80.0_f32;
        let cell_w = 100.0_f32;
        for (ci, col) in col_labels.iter().enumerate() {
            // Add STATIC badge for fake state columns
            if (3..=4).contains(&ci) {
                static_badge(b, &t, label_w + ci as f32 * cell_w, y - 14.0);
            }
            {
                let layout_params = Rect::new(label_w + ci as f32 * cell_w, y, cell_w - 4.0, 14.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(col).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
        }
        y += 18.0;

        // Row 1: box only
        {
            let layout_params = Rect::new(lx, y, label_w - 4.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("box").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        let box_specs: &[(CheckedState, bool, bool)] = &[
            (CheckedState::Unchecked, false, false),
            (CheckedState::Checked, false, false),
            (CheckedState::Indeterminate, false, false),
            (CheckedState::Checked, true, false),
            (CheckedState::Checked, false, true),
        ];
        for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
            let rect = Rect::new(label_w + ci as f32 * cell_w, y, 14.0, 14.0);
            if ci < 3 {
                let _info = {
                    let state = &mut state.cb_matrix[ci];
                    let spec_builder = CheckboxSpecBuilder::new();
                    checkbox(b, spec_builder, rect, state)
                };
            } else {
                draw_checkbox_fake_state(b, rect, *cs, *focused, *disabled);
            }
        }
        y += 14.0 + 12.0;

        // Row 2: with label
        {
            let layout_params = Rect::new(lx, y, label_w - 4.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("with label")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };
        for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
            let cx = label_w + ci as f32 * cell_w;
            if ci < 3 {
                let _info = {
                    let state = &mut state.cb_matrix[3 + ci];
                    let layout_params = Rect::new(cx, y, 14.0, 14.0);
                    let spec_builder = CheckboxSpecBuilder::new();
                    checkbox(b, spec_builder, layout_params, state)
                };
            } else {
                draw_checkbox_fake_state(b, Rect::new(cx, y, 14.0, 14.0), *cs, *focused, *disabled);
            }

            let label_alpha = if *disabled { t.muted } else { t.ink };
            {
                let layout_params = Rect::new(cx + 18.0, y, 60.0, 14.0);
                let size = t.text_sm;
                let spec_builder = LabelSpecBuilder::new().text("vsync").style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: label_alpha,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
        }
        y += 14.0;
    }
    y += GROUP_GAP;

    group_y(b, &t, y, "radio  ·  switch");
    y += 20.0;
    {
        let radio_labels = ["immediate-mode", "retained-mode", "hybrid", "deferred"];
        for (i, radio_label) in radio_labels.iter().enumerate() {
            let ry = y + i as f32 * 22.0;
            if i < 3 {
                let info = {
                    let state = &mut state.radio_states[i];
                    let layout_params = Rect::new(lx, ry, 14.0, 14.0);
                    let spec_builder = RadioSpecBuilder::new();
                    radio(b, spec_builder, layout_params, state)
                };
                if info.input.clicked {
                    for j in 0..3 {
                        state.radio_states[j].checked = j == i;
                    }
                }
            } else {
                static_badge(b, &t, lx - 48.0, ry);
                draw_radio_fake_state(b, Rect::new(lx, ry, 14.0, 14.0), false, true, false);
            }
            {
                let layout_params = Rect::new(18.0, ry, 140.0, 14.0);
                let size = t.text_md;
                let color = t.ink;
                let spec_builder = LabelSpecBuilder::new().text(radio_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
        }
        let sw_x = 220.0;
        let switch_labels = [
            "debug overlay",
            "show layout grid",
            "vsync",
            "multisampling",
        ];
        for (i, switch_label) in switch_labels.iter().enumerate() {
            let ry = y + i as f32 * 22.0;
            let label_color = if i == 3 { t.muted } else { t.ink };
            match i {
                2 => {
                    static_badge(b, &t, sw_x - 48.0, ry);
                    draw_switch_fake_state(b, Rect::new(sw_x, ry, 30.0, 16.0), true, true, false);
                }
                3 => {
                    let _info = {
                        let state = &mut state.switch_states[2];
                        let layout_params = Rect::new(sw_x, ry, 30.0, 16.0);
                        let spec_builder = SwitchSpecBuilder::new().disabled(true);
                        switch(b, spec_builder, layout_params, state)
                    };
                }
                _ => {
                    let _info = {
                        let state = &mut state.switch_states[i];
                        let layout_params = Rect::new(sw_x, ry, 30.0, 16.0);
                        {
                            let spec_builder = SwitchSpecBuilder::new();
                            switch(b, spec_builder, layout_params, state)
                        }
                    };
                }
            }
            {
                let layout_params = Rect::new(sw_x + 36.0, ry, 140.0, 16.0);
                let size = t.text_md;
                let spec_builder = LabelSpecBuilder::new()
                    .text(switch_label)
                    .style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size: size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: label_color,
                        ..LabelStyle::from_theme(&t)
                    });
                label(b, spec_builder, layout_params)
            };
        }
    }
    y += 4.0 * 22.0 + SEC_GAP;
    y
}

#[cfg(all(feature = "slider", feature = "drag_number", feature = "color_swatch"))]
fn section_04_sliders<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 04 · SLIDERS · DRAGS ─────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "04", "Sliders & numeric drags",
        "drag-number reads like a function parameter — label + value, scrubbable in either direction. fill bar shows magnitude.");
    y += 46.0;

    group_y(b, &t, y, "slider · single value");
    y += 20.0;
    {
        let slider_w = 360.0_f32;
        let row_gap = 14.0_f32;

        {
            let step = 0.1;
            let layout_params = Rect::new(lx, y, slider_w, t.h_md);
            let spec_builder = SliderSpecBuilder::new().max(1.0).page_step(step).step(step);
            slider(b, spec_builder, layout_params, &mut state.slider1_state);
        };
        {
            let layout_params = Rect::new(slider_w + 12.0, y + 6.0, 80.0, 14.0);
            let text: &str = &format!("{:.2}", state.slider1_state.value);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        y += t.h_md + row_gap;

        {
            let step = 0.1;
            let layout_params = Rect::new(lx, y, slider_w, t.h_md);
            let spec_builder = SliderSpecBuilder::new().max(1.0).page_step(step).step(step);
            slider(b, spec_builder, layout_params, &mut state.slider2_state);
        };
        {
            let layout_params = Rect::new(slider_w + 12.0, y + 6.0, 80.0, 14.0);
            let text: &str = &format!("{:.2}", state.slider2_state.value);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        y += t.h_md + row_gap;

        {
            let step = 0.1;
            let layout_params = Rect::new(lx, y, slider_w, t.h_md);
            let spec_builder = SliderSpecBuilder::new().max(1.0).page_step(step).step(step);
            slider(b, spec_builder, layout_params, &mut state.slider3_state);
        };
        {
            let layout_params = Rect::new(slider_w + 12.0, y + 6.0, 80.0, 14.0);
            let text: &str = &format!("{:.2}", state.slider3_state.value);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        y += t.h_md + row_gap;

        // Stepped slider (0–9) with tick marks
        {
            let step = 1.0;
            let layout_params = Rect::new(lx, y, slider_w, t.h_md);
            let spec_builder = SliderSpecBuilder::new().max(9.0).page_step(step);
            slider(b, spec_builder, layout_params, &mut state.slider4_state);
        };
        {
            let layout_params = Rect::new(slider_w + 12.0, y + 6.0, 80.0, 14.0);
            let text: &str = &format!("{:.0} / 9", state.slider4_state.value);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        // tick marks below track
        let tick_y = y + t.h_md + 2.0;
        let tick_h = 4.0;
        let usable = slider_w - 12.0;
        for i in 0..=9usize {
            let tx = 6.0 + (i as f32 / 9.0) * usable;
            {
                let layout_params = Rect::new(tx - 0.5, tick_y, 1.0, tick_h);
                let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
                let cmds = DrawCommands::from_vec(vec![DrawCmd::FillRect {
                    rect,
                    color: t.line,
                }]);
                b.append_cmds(cmds);
            };
        }
        y += t.h_md + 8.0;
    }
    y += GROUP_GAP;

    group_y(b, &t, y, "range slider");
    y += 20.0;
    {
        let track_w = 360.0_f32;
        let mid_y = y + t.h_md * 0.5;
        {
            let layout_params = Rect::new(lx, mid_y - 0.75, track_w, 12.0);
            let r = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = {
                let lx = r.x;
                let track_w = r.w;
                let mid_y = r.y + 0.75;
                let t1 = 0.24_f32;
                let t2 = 0.76_f32;
                let fill_x1 = track_w * t1;
                let fill_x2 = track_w * t2;
                let ts = 12.0_f32; // thumb size
                let half_ts = ts * 0.5;

                DrawCommands::from_vec(vec![
                    // full track
                    DrawCmd::FillRect {
                        rect: Rect::new(lx, mid_y - 0.75, track_w, 1.5),
                        color: t.line,
                    },
                    // fill bar
                    DrawCmd::FillRect {
                        rect: Rect::new(fill_x1, mid_y - 0.75, fill_x2 - fill_x1, 1.5),
                        color: t.ink,
                    },
                    // thumb 1
                    DrawCmd::FillRect {
                        rect: Rect::new(fill_x1 - half_ts, mid_y - half_ts, ts, ts),
                        color: t.paper_elev,
                    },
                    DrawCmd::StrokeRect {
                        rect: Rect::new(fill_x1 - half_ts, mid_y - half_ts, ts, ts),
                        color: t.ink,
                        width: 1.5,
                    },
                    // thumb 2
                    DrawCmd::FillRect {
                        rect: Rect::new(fill_x2 - half_ts, mid_y - half_ts, ts, ts),
                        color: t.paper_elev,
                    },
                    DrawCmd::StrokeRect {
                        rect: Rect::new(fill_x2 - half_ts, mid_y - half_ts, ts, ts),
                        color: t.ink,
                        width: 1.5,
                    },
                ])
            };
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(track_w + 12.0, y + 6.0, 80.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text(".24–.76").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
    }
    y += t.h_md + GROUP_GAP;

    group_y(b, &t, y, "drag-number (imgui-style)");
    y += 20.0;
    {
        let mut bx = lx;
        // X — real
        let _info = {
            let state = &mut state.dn_showcase[0];
            let layout_params = Rect::new(bx, y, 100.0, t.h_md);
            let spec_builder = DragNumberSpecBuilder::new().text("X").max(800.0);
            drag_number(b, spec_builder, layout_params, state)
        };
        bx += 100.0 + 8.0;
        // Y — real
        let _info = {
            let state = &mut state.dn_showcase[1];
            let layout_params = Rect::new(bx, y, 100.0, t.h_md);
            let spec_builder = DragNumberSpecBuilder::new().text("Y").max(600.0);
            drag_number(b, spec_builder, layout_params, state)
        };
        bx += 100.0 + 8.0;
        // W — fake (forced active/dragging)
        static_badge(b, &t, bx, y - 14.0);
        draw_drag_number_fake_state(
            b,
            Rect::new(bx, y, 100.0, t.h_md),
            "W",
            576.0,
            0.0,
            800.0,
            true,
        );
        bx += 100.0 + 8.0;
        // H — real
        let _info = {
            let state = &mut state.dn_showcase[2];
            let layout_params = Rect::new(bx, y, 100.0, t.h_md);
            let spec_builder = DragNumberSpecBuilder::new().text("H").max(600.0);
            drag_number(b, spec_builder, layout_params, state)
        };
    }
    y += t.h_md + GROUP_GAP;

    group_y(b, &t, y, "numeric stepper  ·  colour swatch");
    y += 20.0;
    {
        // prefix + value display
        let stepper_x = lx;
        {
            let layout_params = Rect::new(stepper_x, y, 64.0, t.h_md);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect,
                    color: t.hover,
                },
                DrawCmd::StrokeRect {
                    rect,
                    color: t.line,
                    width: 1.0,
                },
            ]);
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(stepper_x + 6.0, y + 7.0, 56.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("padding").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(stepper_x + 64.0, y, 40.0, t.h_md);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect,
                    color: t.paper_elev,
                },
                DrawCmd::StrokeRect {
                    rect,
                    color: t.line,
                    width: 1.0,
                },
            ]);
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(stepper_x + 72.0, y + 7.0, 24.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text("12").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };

        // +/- buttons as text
        let sx = stepper_x + 120.0;
        {
            let layout_params = Rect::new(sx, y, 84.0, t.h_sm);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(rect.x, rect.y, 22.0, t.h_sm),
                    color: t.paper_elev,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(rect.x, rect.y, 22.0, t.h_sm),
                    color: t.line,
                    width: 1.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(rect.x + 22., rect.y, 40.0, t.h_sm),
                    color: t.paper_elev,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(rect.x + 22., rect.y, 40.0, t.h_sm),
                    color: t.line,
                    width: 1.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(rect.x + 62., rect.y, 22.0, t.h_sm),
                    color: t.paper_elev,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(rect.x + 62., rect.y, 22.0, t.h_sm),
                    color: t.line,
                    width: 1.0,
                },
            ]);
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(sx + 6.0, y + 4.0, 10.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text("−").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(sx + 28.0, y + 4.0, 28.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text("12").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(sx + 68.0, y + 4.0, 10.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text("+").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };

        // color swatches
        let sw_x = sx + 100.0;
        let swatches: &[(Color, &str)] = &[(t.ink, "#15130f"), (t.rust, "#c25a2c")];
        let mut bx = sw_x;
        for (color, hex) in swatches {
            color_swatch(
                b,
                ColorSwatchSpecBuilder::new().color(*color).border(t.line),
                Rect::new(bx, y, 18.0, t.h_md),
            );
            {
                let layout_params = Rect::new(bx + 22.0, y + 7.0, 60.0, 14.0);
                let size = t.text_sm;
                let color = t.ink;
                let spec_builder = LabelSpecBuilder::new().text(hex).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
            bx += 86.0;
        }
    }
    y += t.h_md + SEC_GAP;
    y
}

#[cfg(all(
    feature = "select",
    feature = "segmented",
    feature = "chip",
    feature = "menu"
))]
fn section_05_selection<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 05 · SELECTION ───────────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "05", "Selection", "selects, segmented controls, and menus share one rule: the chosen thing is filled ink, paper text. no surprises.");
    y += 46.0;

    group_y(b, &t, y, "select  ·  segmented  ·  chips");
    y += 20.0;
    {
        // Select widgets
        const LAYOUT_OPTS: &[&str] = &["Layout: row", "Layout: column", "Layout: grid"];
        let value = if state.sel_state.selected_index < LAYOUT_OPTS.len() {
            LAYOUT_OPTS[state.sel_state.selected_index]
        } else {
            ""
        };
        let sel_state = &mut state.sel_state;
        let _sel_info = select(
            b,
            SelectSpecBuilder::new().value(value).items(LAYOUT_OPTS),
            Rect::new(lx, y, 160.0, t.h_md),
            sel_state,
        );

        static_badge(b, &t, lx - 48.0, y + t.h_md + 4.0);
        draw_select_fake_state(
            b,
            Rect::new(lx, y + t.h_md + 4.0, 160.0, t.h_md),
            "Layout row",
            LAYOUT_OPTS,
            true,
            true,
            Some(0),
            false,
        );

        // Segmented controls
        let seg_x = 200.0;
        const SEGS1: &[&str] = &["row", "column", "grid", "flex"];
        let _seg1_info = {
            let state = &mut state.seg1_state;
            let layout_params = Rect::new(seg_x, y, 0.0, t.h_md);
            let spec_builder = SegmentedSpecBuilder::new().items(SEGS1);
            segmented(b, spec_builder, layout_params, state)
        };
        const SEGS2: &[&str] = &["start", "center", "end"];
        let _seg2_info = {
            let state = &mut state.seg2_state;
            let layout_params = Rect::new(seg_x, y + t.h_md + 4.0, 0.0, t.h_md);
            let spec_builder = SegmentedSpecBuilder::new().items(SEGS2);
            segmented(b, spec_builder, layout_params, state)
        };

        // Chips
        let chip_labels = ["opengl", "vulkan", "metal", "wgpu"];
        let chip_y = y;
        let mut chip_x = 560.0;
        for (i, label) in chip_labels.iter().enumerate() {
            let layout = b.text_system.prepare(label, t.text_sm, t.mono_font);
            let chip_w = (layout.size.x + 16.0).max(32.0);
            let _chip_info = {
                let state = &mut state.chip_states[i];
                let layout_params = Rect::new(chip_x, chip_y, chip_w, 22.0);
                let spec_builder = ChipSpecBuilder::new().text(label).style(ChipStyle {
                    font: b.theme.sans_font,
                    ..ChipStyle::from_theme(&b.theme)
                });
                chip(b, spec_builder, layout_params, state)
            };
            chip_x += chip_w + 6.0;
        }
        let add_layout = b
            .text_system
            .prepare("+ add backend", t.text_sm, t.mono_font);
        let add_w = (add_layout.size.x + 16.0).max(32.0);
        let _add_info = {
            let state = &mut state.chip_states[4];
            let layout_params = Rect::new(560.0, y + 28.0, add_w, 22.0);
            let spec_builder = ChipSpecBuilder::new()
                .text("+ add backend")
                .style(ChipStyle {
                    font: b.theme.sans_font,
                    ..ChipStyle::from_theme(&b.theme)
                });
            chip(b, spec_builder, layout_params, state)
        };
    }
    let select_open_h = 3.0 * 26.0 + 8.0;
    y += t.h_md + 4.0 + t.h_md + select_open_h + GROUP_GAP;

    group_y(b, &t, y, "dropdown menu (open)");
    y += 20.0;
    {
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
            b,
            MenuSpecBuilder::new().items(ITEMS1),
            Rect::new(lx, y, 240.0, 0.0),
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
            b,
            MenuSpecBuilder::new().items(ITEMS2),
            Rect::new(264.0, y, 200.0, 0.0),
        );

        let menu1_h: f32 = ITEMS1
            .iter()
            .map(|i| match i {
                MenuItem::Item { .. } => 26.0,
                MenuItem::Separator => 9.0,
                MenuItem::Group(_) => 22.0,
            })
            .sum::<f32>()
            + 8.0;
        y += menu1_h;
    }
    y += SEC_GAP;
    y
}

#[cfg(feature = "scroll_area")]
fn section_06_scrollbars<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 06 · SCROLLBARS ──────────────────────────────────────────────────────
    sec_y(b, &t, content_w, "06", "Scrollbars",
        "always visible. thumb length encodes how much of the content fits in view; thumb position encodes scroll offset. dragging shifts the thumb to rust.");
    {
        let mut b = b.child_with_layout(Placement2D::auto(), ManualLayout {});

        let box_gap = 24.0_f32;
        let cap_h = 20.0_f32;

        // Box 1: vertical, idle
        let b1 = Rect::new(0.0, 40.0, 180.0, 130.0);
        let b1_content = Vec2::new(180.0, 320.0);
        {
            let rect = b.layout(b1, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![DrawCmd::StrokeRect {
                rect,
                color: t.line,
                width: 1.0,
            }]);

            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    &mut b,
                    ScrollAreaSpecBuilder::new().vertical(ScrollAxis {
                        extent: ScrollExtent::fixed(b1_content.y),
                        vis: ScrollbarVisibility::Always,
                    }),
                    b1,
                    &mut state.scroll_vert,
                    ManualLayout,
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
                    let size = t.text_sm;
                    let color = t.muted;
                    let spec_builder = LabelSpecBuilder::new().text(line).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&t)
                    });
                    label(&mut sa, spec_builder, layout_params)
                };
            }
            sa.finish();
        }
        {
            let layout_params = Rect::new(b1.x, 40.0 + b1.h + 4.0, b1.w, cap_h);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("vertical · idle")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(&mut b, spec_builder, layout_params)
        };

        // Box 2: vertical, dragging (same implementation, user can drag)
        let b2_x = b1.x + b1.w + box_gap;
        let b2 = Rect::new(b2_x, 40.0, 180.0, 130.0);
        let b2_content = Vec2::new(180.0, 300.0);
        {
            let rect = b.layout(b2, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![DrawCmd::StrokeRect {
                rect,
                color: t.line,
                width: 1.0,
            }]);
            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    &mut b,
                    ScrollAreaSpecBuilder::new().vertical(ScrollAxis {
                        extent: ScrollExtent::fixed(b2_content.y),
                        vis: ScrollbarVisibility::Always,
                    }),
                    b2,
                    &mut state.scroll_horiz,
                    ManualLayout,
                )
                .ctx
            };
            for i in 0..15 {
                {
                    let layout_params = Rect::new(6.0, i as f32 * 18.0 + 6.0, 160.0, 14.0);
                    let text: &str = &format!("// entry {:02}/24 — frame state", i + 1);
                    let size = t.text_sm;
                    let color = t.muted;
                    let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&t)
                    });
                    label(&mut sa, spec_builder, layout_params)
                };
            }
            sa.finish();
        }
        {
            let layout_params = Rect::new(b2.x, 40.0 + b2.h + 4.0, b2.w, cap_h);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("vertical · dragging (rust)")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(&mut b, spec_builder, layout_params)
        };

        // Box 3: horizontal
        let b3_x = b2_x + b2.w + box_gap;
        let b3 = Rect::new(b3_x, 40.0 + 15.0, 300.0, 100.0);
        let b3_content = Vec2::new(700.0, 100.0);
        {
            let rect = b.layout(b3, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![DrawCmd::StrokeRect {
                rect,
                color: t.line,
                width: 1.0,
            }]);
            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    &mut b,
                    ScrollAreaSpecBuilder::new()
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
                )
                .ctx
            };
            {
                let layout_params = Rect::new(6.0, 6.0, 680.0, 14.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new()
                    .text("frame.draw_rect( … )  frame.draw_text( \"hello, framewise\" )  frame.draw_image( logo )  frame.layout.push( Row )")
                    .style(LabelStyle { text_style: framewise::TextStyle { size, ..(LabelStyle::from_theme(&t)).text_style }, text_color: color, ..LabelStyle::from_theme(&t) });
                label(&mut sa, spec_builder, layout_params)
            };
            sa.finish();
        }
        {
            let layout_params = Rect::new(b3.x, 40.0 + b3.h + 15.0 + 4.0, b3.w, cap_h);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("horizontal")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(&mut b, spec_builder, layout_params)
        };

        // Box 4: both axes
        let b4_x = b3_x + b3.w + box_gap;
        let b4 = Rect::new(b4_x, 40.0, 220.0, 130.0);
        let b4_content = Vec2::new(320.0, 240.0);
        {
            let rect = b.layout(b4, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![DrawCmd::StrokeRect {
                rect,
                color: t.line,
                width: 1.0,
            }]);
            b.append_cmds(cmds);
        };
        {
            let mut sa = {
                begin_scroll_area(
                    &mut b,
                    ScrollAreaSpecBuilder::new()
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
                )
                .ctx
            };
            {
                let layout_params = Rect::new(12.0, 10.0, 160.0, 32.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new()
                    .text("scroll surface with both bars + corner")
                    .style(LabelStyle {
                        text_style: framewise::TextStyle {
                            flow: framewise::text::TextFlow::wrapped(),
                            size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&t)
                    });
                label(&mut sa, spec_builder, layout_params)
            };
            sa.finish();
        }
        {
            let layout_params = Rect::new(b4.x, 40.0 + b4.h + 4.0, b4.w, cap_h);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("both axes").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut b, spec_builder, layout_params)
        };

        b.finish();
    }
}

#[cfg(feature = "tabs")]
fn section_07_tabs<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 07 · TABS ────────────────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "07", "Tabs", "underline tabs for plain navigation. the rust underbar is the only chrome — no rounded pills, no shadow.");
    y += 46.0;
    {
        const TABS1: &[&str] = &["Inspector", "Layout", "Timing", "Logs", "Replay"];
        let _t1_info = {
            let state = &mut state.tabs1_state;
            let layout_params = Rect::new(lx, y, content_w.min(640.0), 36.0);
            let spec_builder = TabsSpecBuilder::new().items(TABS1);
            tabs(b, spec_builder, layout_params, state)
        };
        y += 36.0 + 20.0;

        const TABS2: &[&str] = &["frame.rs", "layout.rs", "theme.rs", "state.rs"];
        let _t2_info = {
            let state = &mut state.tabs2_state;
            let layout_params = Rect::new(lx, y, content_w.min(480.0), 36.0);
            let spec_builder = TabsSpecBuilder::new().items(TABS2);
            tabs(b, spec_builder, layout_params, state)
        };
        y += 36.0;
    }
    y += SEC_GAP;
    y
}

#[cfg(all(
    feature = "progress_bar",
    feature = "meter",
    feature = "spinner",
    feature = "status"
))]
fn section_08_progress<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    time: f64,
) {
    let t = *t;
    // ── 08 · PROGRESS · METERS · STATUS ──────────────────────────────────────
    sec_y(b, &t, y, content_w, "08", "Progress, meters & status",
        "indeterminate progress uses rust; determinate stays ink. status pills carry the only dot of color on the bar.");
    y += 46.0;

    group_y(b, &t, y, "progress");
    y += 20.0;
    {
        let bar_items: &[(f32, bool, &str)] = &[
            (0.12, false, "12% · compiling"),
            (0.68, false, "68% · linking"),
            (0.94, true, "94% · uploading textures"),
            (f32::NAN, true, "indeterminate"),
        ];
        let bar_w = 240.0_f32;
        for (val, active, bar_label) in bar_items {
            progress_bar(
                b,
                ProgressBarSpecBuilder::new()
                    .value(*val)
                    .phase((time as f32) * 0.5)
                    .active(*active),
                Rect::new(lx, y + 8.0, bar_w, 3.0),
            );
            {
                let layout_params = Rect::new(bar_w + 12.0, y + 2.0, 180.0, 14.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(bar_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
            y += 22.0;
        }
    }
    y += GROUP_GAP;

    group_y(b, &t, y, "meters");
    y += 20.0;
    {
        let meters: &[(&str, f32, Option<f32>)] = &[
            ("CPU", 0.6, None),
            ("GPU", 0.8, Some(0.9)),
            ("FRAME", 1.0, None),
        ];
        let mut bx = lx;
        for (meter_label, val, peak) in meters {
            {
                let layout_params = Rect::new(bx, y, 36.0, 14.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(meter_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
            bx += 40.0;
            if *meter_label == "FRAME" {
                {
                    let layout_params = Rect::new(bx, y - 1.0, 60.0, 16.0);
                    let size = t.text_sm;
                    let color = t.ink;
                    let spec_builder = LabelSpecBuilder::new().text("2.4 ms").style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size: size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&t)
                    });
                    label(b, spec_builder, layout_params)
                };
                bx += 70.0;
            } else {
                meter(
                    b,
                    MeterSpecBuilder::new().value(*val).peak(*peak).bars(10),
                    Rect::new(bx, y, 100.0, 12.0),
                );
                bx += 108.0;
            }
        }
    }
    y += 14.0 + GROUP_GAP;

    group_y(b, &t, y, "spinners  ·  status");
    y += 20.0;
    {
        spinner(b, SpinnerSpecBuilder::new(), Rect::new(lx, y, 16.0, 16.0));
        {
            let layout_params = Rect::new(20.0, y + 1.0, 60.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("loading").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };

        spinner(
            b,
            SpinnerSpecBuilder::new().large(true),
            Rect::new(90.0, y - 4.0, 24.0, 24.0),
        );
        {
            let layout_params = Rect::new(118.0, y + 1.0, 50.0, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("large").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };

        let status_items: &[(&str, StatusVariant)] = &[
            ("IDLE", StatusVariant::Neutral),
            ("READY", StatusVariant::Ok),
            ("FRAME DROP", StatusVariant::Warn),
            ("PANIC", StatusVariant::Err),
            ("RENDERING", StatusVariant::Live),
        ];
        let mut sx = 180.0;
        for (label, variant) in status_items {
            status(
                b,
                StatusSpecBuilder::new().text(label).variant(*variant),
                Rect::new(sx, y + 1.0, 120.0, 12.0),
            );
            sx += 110.0;
        }
    }
    y += 16.0 + SEC_GAP;
    y
}

#[cfg(feature = "tree")]
fn section_09_tree<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
) {
    let t = *t;
    // ── 09 · TREE / LIST ─────────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "09", "Tree & list",
        "monospaced rows, ascii carets, ids on the right. the selected row is filled ink — it is unambiguously the focus.");
    y += 46.0;
    {
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
        tree(
            b,
            TreeSpecBuilder::new().items(WIDGET_TREE),
            Rect::new(lx, y, 320.0, 0.0),
        );

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
        tree(
            b,
            TreeSpecBuilder::new().items(FILE_LIST),
            Rect::new(360.0, y, 240.0, 0.0),
        );

        y += WIDGET_TREE.len().max(FILE_LIST.len()) as f32 * 20.0 + 12.0;
    }
    y += SEC_GAP;
    y
}

#[cfg(all(feature = "tooltip", feature = "keycap"))]
fn section_10_tooltips<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
) {
    let t = *t;
    // ── 10 · TOOLTIPS · KEYCAPS ──────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "10", "Tooltips & keycaps",
        "tooltips invert the palette — ink on paper becomes paper on ink. keycaps borrow the input border.");
    y += 46.0;

    group_y(b, &t, y, "tooltips");
    y += 20.0;
    {
        tooltip(
            b,
            TooltipSpecBuilder::new()
                .text("Drag to scrub — hold ⌥ for fine.")
                .variant(TooltipVariant::Dark),
            Rect::new(lx, y, 0.0, 0.0),
        );
        y += 28.0 + 8.0;

        tooltip(
            b,
            TooltipSpecBuilder::new()
                .text("Re-described every frame from current application state. No retained nodes.")
                .variant(TooltipVariant::Dark),
            Rect::new(lx, y, 0.0, 0.0),
        );
        y += 28.0 + 8.0;

        tooltip(
            b,
            TooltipSpecBuilder::new()
                .text("⚠ shader recompiled b frame (12 ms)")
                .variant(TooltipVariant::Rust),
            Rect::new(lx, y, 0.0, 0.0),
        );
        y += 28.0;
    }
    y += GROUP_GAP;

    group_y(b, &t, y, "keycaps");
    y += 20.0;
    {
        let key_rows: &[(&[&str], &str)] = &[
            (&["⌘", "⇧", "P"], "command palette"),
            (&["G"], "toggle layout grid"),
            (&["F2"], "replay last frame"),
            (&["⌥", "drag"], "fine scrub"),
        ];
        for (keys, desc) in key_rows {
            let mut kx = lx;
            for key in *keys {
                let kw = (key.len() as f32 * 7.0 + 12.0).max(24.0);
                keycap(
                    b,
                    KeycapSpecBuilder::new().text(key),
                    Rect::new(kx, y, kw, 22.0),
                );
                kx += kw + 4.0;
            }
            {
                let layout_params = Rect::new(kx + 4.0, y + 3.0, 200.0, 14.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(desc).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
            y += 28.0;
        }
    }
    y += SEC_GAP;
    y
}

#[cfg(all(feature = "window", feature = "drag_number", feature = "checkbox"))]
fn section_11_window<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 11 · WINDOW CHROME ───────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "11", "Window & panel chrome",
        "title bar inverts to ink. window controls are typographic — no traffic-light cosplay. status strip carries live state."                );
    y += 46.0;
    {
        // Light window: Inspector with content
        let win_buttons = [
            WindowButton { symbol: "−" },
            WindowButton { symbol: "▢" },
            WindowButton { symbol: "×" },
        ];
        let win_rect = Rect::new(lx, y, 360.0, 280.0);
        let mut win = {
            let widget_spec_builder = WindowSpecBuilder::new()
                .title("Inspector")
                .buttons(&win_buttons)
                .status_bar(true)
                .status_text("RENDERING  frame #00248  2.4 ms");
            begin_window(b, widget_spec_builder, win_rect, ManualLayout).ctx
        };

        // Inner content: drag numbers + checkboxes
        let mut iy = 0.0;
        let mut drx = 0.0;
        let cr_w = win_rect.w - 32.0;
        for (i, (label, min, max)) in [("X", 0.0_f32, 800.0_f32), ("Y", 0.0, 600.0)]
            .iter()
            .enumerate()
        {
            let _info = {
                let state = &mut state.win11_drags[i];
                let min = *min;
                let max = *max;
                let layout_params = Rect::new(drx, iy, (cr_w / 2.0) - 4.0, t.h_md);
                let spec_builder = DragNumberSpecBuilder::new().text(label).min(min).max(max);
                drag_number(&mut win, spec_builder, layout_params, state)
            };
            drx += (cr_w / 2.0) + 4.0;
        }
        iy += t.h_md + 6.0;
        drx = 0.0;
        for (i, (label, min, max)) in [("W", 0.0_f32, 800.0_f32), ("H", 0.0, 600.0)]
            .iter()
            .enumerate()
        {
            let _info = {
                let state = &mut state.win11_drags[2 + i];
                let min = *min;
                let max = *max;
                let layout_params = Rect::new(drx, iy, (cr_w / 2.0) - 4.0, t.h_md);
                let spec_builder = DragNumberSpecBuilder::new().text(label).min(min).max(max);
                drag_number(&mut win, spec_builder, layout_params, state)
            };
            drx += (cr_w / 2.0) + 4.0;
        }
        iy += t.h_md + 10.0;
        {
            let layout_params = Rect::new(0.0, iy, cr_w, 1.0);
            let spec_builder = DividerSpecBuilder::new();
            divider(&mut win, spec_builder, layout_params)
        };
        iy += 10.0;
        let check_labels = ["clip to parent", "debug overlay"];
        for (i, check_label) in check_labels.iter().enumerate() {
            let _cb_info = {
                let state = &mut state.win11_cbs[i];
                let layout_params = Rect::new(0.0, iy, 14.0, 14.0);
                let spec_builder = CheckboxSpecBuilder::new();
                checkbox(&mut win, spec_builder, layout_params, state)
            };
            {
                let layout_params = Rect::new(18.0, iy, cr_w - 18.0, 14.0);
                let size = t.text_md;
                let color = t.ink;
                let spec_builder = LabelSpecBuilder::new().text(check_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut win, spec_builder, layout_params)
            };
            iy += 22.0;
        }
        win.finish();

        // Dark variant window (drawn with DrawCmds)
        let dw = Rect::new(388.0, y, 300.0, 240.0);
        let dark_bg = Color::from_srgb_u8(26, 24, 20, 255);
        let darker = Color::from_srgb_u8(12, 11, 9, 255);
        let dark_bdr = Color::from_srgb_u8(58, 53, 45, 255);
        let light = t.paper;
        let muted_l = t.muted;

        {
            let rect = b.layout(dw, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect,
                    color: dark_bg,
                },
                DrawCmd::StrokeRect {
                    rect,
                    color: dark_bdr,
                    width: 1.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(rect.x, rect.y, rect.w, 26.0),
                    color: darker,
                },
            ]);
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(dw.x + 10.0, y + 6.0, 180.0, 14.0);
            let size = t.text_sm;
            let spec_builder = LabelSpecBuilder::new()
                .text("FRAMEWISE · DARK")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: light,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(dw.x + dw.w - 28.0, y + 6.0, 20.0, 14.0);
            let size = t.text_sm;
            let spec_builder = LabelSpecBuilder::new().text("✕").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };

        let cx = dw.x + 16.0;
        let cyw = y + 26.0 + 16.0;
        // keycap row
        {
            let layout_params = Rect::new(cx, cyw, 50.0, 22.0);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(rect.x, rect.y, 24.0, 22.0),
                    color: Color::from_srgb_u8(42, 37, 32, 255),
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(rect.x, rect.y, 24.0, 22.0),
                    color: dark_bdr,
                    width: 1.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(rect.x + 28.0, rect.y, 22.0, 22.0),
                    color: Color::from_srgb_u8(42, 37, 32, 255),
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(rect.x + 28.0, rect.y, 22.0, 22.0),
                    color: dark_bdr,
                    width: 1.0,
                },
            ]);

            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(cx + 7.0, cyw + 5.0, 12.0, 12.0);
            let size = t.text_sm;
            let spec_builder = LabelSpecBuilder::new().text("⌘").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(cx + 35.0, cyw + 5.0, 12.0, 12.0);
            let size = t.text_sm;
            let spec_builder = LabelSpecBuilder::new().text("K").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: light,
                ..LabelStyle::from_theme(&t)
            });
            label(b, spec_builder, layout_params)
        };
        {
            let layout_params = Rect::new(cx + 56.0, cyw + 5.0, 140.0, 12.0);
            let size = t.text_sm;
            let spec_builder =
                LabelSpecBuilder::new()
                    .text("search everything")
                    .style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size: size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: muted_l,
                        ..LabelStyle::from_theme(&t)
                    });
            label(b, spec_builder, layout_params)
        };

        // fake dark input
        let inp_y = cyw + 28.0;
        {
            let layout_params = Rect::new(cx, inp_y, dw.w - 32.0, 26.0);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect,
                    color: darker,
                },
                DrawCmd::StrokeRect {
                    rect,
                    color: dark_bdr,
                    width: 1.0,
                },
            ]);
            b.append_cmds(cmds);
        };
        {
            let layout_params = Rect::new(cx + 8.0, inp_y + 7.0, dw.w - 48.0, 12.0);
            let size = t.text_sm;
            let spec_builder = LabelSpecBuilder::new()
                .text("type a command…")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: muted_l,
                    ..LabelStyle::from_theme(&t)
                });
            label(b, spec_builder, layout_params)
        };

        // fake dark tabs
        let tab_y = inp_y + 30.0;
        {
            let layout_params = Rect::new(cx, tab_y + 26.0, dw.w - 16.0, 1.0);
            let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
            let cmds = DrawCommands::from_vec(vec![DrawCmd::StrokeLine {
                p0: Vec2::new(rect.x, rect.y),
                p1: Vec2::new(rect.x + rect.w, rect.y),
                color: dark_bdr,
                width: 1.0,
            }]);
            b.append_cmds(cmds);
        };
        let tab_items = ["Files", "Symbols", "Frames"];
        let mut tab_x = cx;
        for (i, item) in tab_items.iter().enumerate() {
            {
                let layout_params = Rect::new(tab_x, tab_y + 5.0, 60.0, 14.0);
                let size = t.text_sm;
                let color = if i == 0 { light } else { muted_l };
                let spec_builder = LabelSpecBuilder::new().text(item).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
            if i == 0 {
                {
                    let layout_params = Rect::new(tab_x, tab_y + 24.0, 40.0, 2.0);
                    let rect = b.layout(layout_params, IntrinsicSize::UNKNOWN);
                    let cmds = DrawCommands::from_vec(vec![DrawCmd::FillRect {
                        rect,
                        color: t.rust,
                    }]);

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
                let size = t.text_sm;
                let spec_builder = LabelSpecBuilder::new().text(file).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: muted_l,
                    ..LabelStyle::from_theme(&t)
                });
                label(b, spec_builder, layout_params)
            };
        }

        y += 280.0 + SEC_GAP;
    }
    y
}

#[cfg(all(
    feature = "window",
    feature = "tabs",
    feature = "segmented",
    feature = "slider",
    feature = "switch",
    feature = "drag_number",
    feature = "color_swatch",
    feature = "checkbox",
    feature = "button",
    feature = "menu"
))]
fn section_12_in_use<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: &Theme,
    content_w: f32,
    mut y: f32,
    state: &mut SpecWidgets,
) {
    let t = *t;
    // ── 12 · IN USE ──────────────────────────────────────────────────────────
    sec_y(b, &t, y, content_w, "12", "In use",
        "the widgets composed into the kind of panel they were designed for — a settings sheet inside an inspector window.");
    y += 46.0;
    {
        // Left: Renderer Settings window
        let win_w_left = 440.0_f32;
        let win_h_full = 480.0_f32;
        let win_buttons = [
            WindowButton { symbol: "−" },
            WindowButton { symbol: "▢" },
            WindowButton { symbol: "×" },
        ];
        let wr = Rect::new(lx, y, win_w_left, win_h_full);
        let mut win = {
            let widget_spec_builder = WindowSpecBuilder::new()
                .title("Renderer Settings")
                .buttons(&win_buttons)
                .status_bar(true)
                .status_text("RENDERING  frame #00248  2.4 ms  Vulkan 1.3 · 4× msaa");
            begin_window(b, widget_spec_builder, wr, ManualLayout).ctx
        };
        let cr_w = win_w_left - 32.0;

        // Tabs inside window
        let tabs_items = ["General", "Frame", "Output", "Debug"];
        let _tabs_info = {
            let state = &mut state.iu_tabs;
            let items: &[&str] = &tabs_items;
            let layout_params = Rect::new(0.0, 0.0, cr_w, 28.0);
            let spec_builder = TabsSpecBuilder::new().items(items);
            tabs(&mut win, spec_builder, layout_params, state)
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
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("BACKEND").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        let backends = ["OpenGL", "Vulkan", "Metal", "wgpu"];
        let _backend_info = {
            let state = &mut state.iu_backend;
            let items: &[&str] = &backends;
            let layout_params = Rect::new(widget_x, fy, 0.0, row_h);
            let spec_builder = SegmentedSpecBuilder::new().items(items);
            segmented(&mut win, spec_builder, layout_params, state)
        };
        fy += row_h + row_gap;

        // target fps (slider)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new()
                .text("TARGET FPS")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(&mut win, spec_builder, layout_params)
        };
        {
            let step = 10.0;
            let layout_params = Rect::new(widget_x, fy, widget_w - 40.0, row_h);
            let spec_builder = SliderSpecBuilder::new()
                .min(24.0)
                .max(240.0)
                .page_step(step)
                .step(step);
            slider(
                &mut win,
                spec_builder,
                layout_params,
                &mut state.iu_fps_slider,
            );
        };
        {
            let layout_params = Rect::new(widget_x + widget_w - 34.0, fy + 7.0, 34.0, 14.0);
            let text: &str = &format!("{:.0}", state.iu_fps_slider.value);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text(text).style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        fy += row_h + row_gap;

        // vsync (switch)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("VSYNC").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        let _switch_res = {
            let state = &mut state.iu_vsync;
            let layout_params = Rect::new(widget_x, fy + 6.0, 30.0, 16.0);
            {
                let spec_builder = SwitchSpecBuilder::new();
                switch(&mut win, spec_builder, layout_params, state)
            }
        };
        {
            let layout_params = Rect::new(widget_x + 36.0, fy + 7.0, 120.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new()
                .text("match display")
                .style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
            label(&mut win, spec_builder, layout_params)
        };
        fy += row_h + row_gap;

        // msaa (segmented)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("MSAA").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        let msaa_opts = ["off", "2×", "4×", "8×"];
        let _seg_res = {
            let state = &mut state.iu_msaa;
            let items: &[&str] = &msaa_opts;
            let layout_params = Rect::new(widget_x, fy, 0.0, row_h);
            let spec_builder = SegmentedSpecBuilder::new().items(items);
            segmented(&mut win, spec_builder, layout_params, state)
        };
        fy += row_h + row_gap;

        // viewport (drag numbers)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("VIEWPORT").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        let _w_res = {
            let state = &mut state.iu_vp_w;
            let layout_params = Rect::new(widget_x, fy, (widget_w / 2.0) - 4.0, row_h);
            let spec_builder = DragNumberSpecBuilder::new().text("W").max(7680.0);
            drag_number(&mut win, spec_builder, layout_params, state)
        };

        let _h_res = {
            let state = &mut state.iu_vp_h;
            let layout_params = Rect::new(
                widget_x + (widget_w / 2.0) + 4.0,
                fy,
                (widget_w / 2.0) - 4.0,
                row_h,
            );
            let spec_builder = DragNumberSpecBuilder::new().text("H").max(7680.0);
            drag_number(&mut win, spec_builder, layout_params, state)
        };
        fy += row_h + row_gap;

        // accent (color swatch + button)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("ACCENT").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        color_swatch(
            &mut win,
            ColorSwatchSpecBuilder::new().color(t.rust).border(t.line),
            Rect::new(widget_x, fy + 4.0, 18.0, 20.0),
        );
        {
            let layout_params = Rect::new(widget_x + 22.0, fy + 7.0, 60.0, 14.0);
            let size = t.text_sm;
            let color = t.ink;
            let spec_builder = LabelSpecBuilder::new().text("#c25a2c").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
        };
        fy += row_h + row_gap;

        // options (checkboxes)
        {
            let layout_params = Rect::new(0.0, fy + 7.0, label_w, 14.0);
            let size = t.text_sm;
            let color = t.muted;
            let spec_builder = LabelSpecBuilder::new().text("OPTIONS").style(LabelStyle {
                text_style: framewise::TextStyle {
                    size: size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
            label(&mut win, spec_builder, layout_params)
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
                let spec_builder = CheckboxSpecBuilder::new();
                checkbox(&mut win, spec_builder, layout_params, state)
            };

            {
                let layout_params = Rect::new(widget_x + 18.0, opt_y + 4.0, widget_w - 18.0, 14.0);
                let size = t.text_md;
                let color = t.ink;
                let spec_builder = LabelSpecBuilder::new().text(opt_label).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size: size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut win, spec_builder, layout_params)
            };
        }
        fy += 3.0 * 22.0 + 4.0;

        {
            let layout_params = Rect::new(0.0, fy, cr_w, 1.0);
            let spec_builder = DividerSpecBuilder::new();
            divider(&mut win, spec_builder, layout_params)
        };
        fy += 10.0;

        // button row
        let mut btn_x = cr_w;
        let btns: &[(&str, ButtonStyle)] = &[
            ("Apply", ButtonStyle::primary_from_theme(&t)),
            ("Cancel", ButtonStyle::primary_from_theme(&t)),
            ("Reset", ButtonStyle::ghost_from_theme(&t)),
        ];
        for (i, (label, style)) in btns.iter().enumerate() {
            let bw = label.len() as f32 * 7.0 + 20.0;
            btn_x -= bw;
            let _btn = {
                let state = &mut state.iu_btns[i];
                let layout_params = Rect::new(btn_x, fy, bw, t.h_md);
                let text: &str = label;
                let style = *style;
                let spec_builder = ButtonSpecBuilder::new().text(text).style(style);
                button(&mut win, spec_builder, layout_params, state)
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
            let widget_spec_builder = WindowSpecBuilder::new()
                .title("Frame Log")
                .buttons(&fl_buttons)
                .status_bar(true)
                .status_text("RECORDING  248 frames  2.6 ms avg");
            begin_window(b, widget_spec_builder, fl_rect, ManualLayout).ctx
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
                    &mut fl_win,
                    ScrollAreaSpecBuilder::new().vertical(ScrollAxis {
                        extent: ScrollExtent::fixed(content_size.y),
                        vis: ScrollbarVisibility::Auto,
                    }),
                    fl_scroll_rect,
                    &mut state.iu_log_scroll,
                    inner_layout,
                )
                .ctx
            };
            let loy = 4.0;
            for (i, (ts_str, msg, highlight)) in log_lines.iter().enumerate() {
                let row_y = loy + i as f32 * 18.0;
                let ts_w = 100.0_f32;
                {
                    let layout_params = Rect::new(6.0, row_y, ts_w, 14.0);
                    let size = t.text_sm;
                    let color = t.muted;
                    let spec_builder = LabelSpecBuilder::new().text(ts_str).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size: size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: color,
                        ..LabelStyle::from_theme(&t)
                    });
                    label(&mut log_page, spec_builder, layout_params)
                };
                let msg_color = if *highlight { t.rust } else { t.ink };
                {
                    let layout_params = Rect::new(
                        6.0 + ts_w + 8.0,
                        row_y,
                        fl_scroll_rect.w - ts_w - 14.0,
                        14.0,
                    );
                    let size = t.text_sm;
                    let spec_builder = LabelSpecBuilder::new().text(msg).style(LabelStyle {
                        text_style: framewise::TextStyle {
                            size: size,
                            ..(LabelStyle::from_theme(&t)).text_style
                        },
                        text_color: msg_color,
                        ..LabelStyle::from_theme(&t)
                    });
                    label(&mut log_page, spec_builder, layout_params)
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
            let widget_spec_builder = WindowSpecBuilder::new()
                .title("Quick actions")
                .buttons(&qa_buttons)
                .status_bar(false)
                .status_text("");
            begin_window(b, widget_spec_builder, qa_rect, ManualLayout).ctx
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
            &mut qa_win,
            MenuSpecBuilder::new().items(&qa_items),
            Rect::new(0.0, -8.0, qa_cr_w, 0.0),
        );
        qa_win.finish();
        y += win_h_full;
    }
    y += SEC_GAP;
    y
}

fn footer_section<CF>(
    b: &mut WidgetContext<SampleTextSystem, ColumnState, CF>,
    t: Theme,
    content_w: f32,
) {
    {
        let spec_builder = DividerSpecBuilder::new();
        divider(b, spec_builder, Placement2D::fixed(content_w, 1.0))
    };
    {
        let mut b = b.child_with_layout(Placement2D::auto(), ManualLayout {});

        let foot_items: &[(&str, &str)] = &[
            ("SPEC", "V0.1 · 12 SECTIONS"),
            ("RADIUS", "0 PX"),
            ("BORDERS", "1 PX INK"),
            ("FOCUS", "2 PX RUST OUTSET"),
            ("DENSITY", "28 PX ROW · 14 PX LABEL · 12 PX MONO"),
        ];
        let mut fx = 0.0;
        for (key, val) in foot_items {
            {
                let layout_params = Rect::new(fx, 10.0, 32.0, 14.0);
                let size = t.text_sm;
                let color = t.ink;
                let spec_builder = LabelSpecBuilder::new().text(key).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut b, spec_builder, layout_params)
            };
            let kw = key.len() as f32 * 7.0 + 8.0;
            {
                let layout_params = Rect::new(fx + kw, 10.0, 220.0, 14.0);
                let size = t.text_sm;
                let color = t.muted;
                let spec_builder = LabelSpecBuilder::new().text(val).style(LabelStyle {
                    text_style: framewise::TextStyle {
                        size,
                        ..(LabelStyle::from_theme(&t)).text_style
                    },
                    text_color: color,
                    ..LabelStyle::from_theme(&t)
                });
                label(&mut b, spec_builder, layout_params)
            };
            fx += kw + val.len() as f32 * 6.5 + 24.0;
        }
        b.finish();
    }
    {
        let size = t.text_sm;
        let color = t.ink;
        let spec_builder = LabelSpecBuilder::new()
            .text("FRAMEWISE · WIDGET SPECIFICATION")
            .style(LabelStyle {
                text_style: framewise::TextStyle {
                    size,
                    ..(LabelStyle::from_theme(&t)).text_style
                },
                text_color: color,
                ..LabelStyle::from_theme(&t)
            });
        label(b, spec_builder, Placement2D::auto())
    };
}
