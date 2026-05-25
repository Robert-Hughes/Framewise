use crate::text::SampleTextSystem;
use framewise::text::TextSystem;
use framewise::widgets::{ButtonSpecBuilder, DividerSpecBuilder, LabelSpecBuilder};
use framewise::widgets::slider::SliderSpecBuilder;
use framewise::widgets::text_edit::TextEditSpecBuilder;
/// Interactive widget specification page — mirrors mockups/Framewise Widgets.html.
use framewise::{
    draw::DrawCmd,
    focus::FocusSystem,
    input::Input,
    layout::{Layout, LayoutState, ManualLayout, OffsetLayout, OffsetState},
    theme::Theme,
    types::{Color, Rect, Vec2},
    widget::{WidgetContext, LayoutInfo},
    widgets::{
        button::{button, button_raw, ButtonSpec, ButtonState, ButtonStyle, ButtonInfo},
        checkbox::{checkbox, checkbox_raw, CheckboxState, CheckState, CheckboxSpec, CheckboxInfo, CheckboxSpecBuilder},
        chip::{chip, chip_raw, ChipState, ChipSpec, ChipStyle, ChipInfo, ChipResult, ChipSpecBuilder},
        color_swatch::{color_swatch, color_swatch_raw, ColorSwatchSpec, ColorSwatchInfo, ColorSwatchSpecBuilder},
        drag_number::{drag_number, drag_number_raw, DragNumberState, DragNumberSpec, DragNumberInfo, DragNumberSpecBuilder},
        divider::{divider, DividerSpec, DividerInfo, DividerResult},
        frame::{frame, frame_raw, FrameSpec, FrameStyle, FrameInfo, FrameResult},
        keycap::{keycap, keycap_raw, KeycapSpec, KeycapInfo, KeycapSpecBuilder},
        label::{label, label_raw, LabelSpec, LabelInfo},
        menu::{menu, menu_raw, MenuItem, MenuSpec, MenuSpecBuilder},
        meter::{meter, meter_raw, MeterSpec, MeterInfo, MeterSpecBuilder},
        progress_bar::{progress_bar, progress_bar_raw, ProgressBarSpec, ProgressBarStyle, ProgressBarSpecBuilder},
        radio::{radio, radio_raw, RadioState, RadioSpec, RadioInfo, RadioSpecBuilder},
        scroll_area::{begin_scroll_area, begin_scroll_area_raw, end_scroll_area, end_scroll_area_raw, ScrollState, ScrollbarVisibility, ScrollAreaScope},
        segmented::{segmented, segmented_raw, SegmentedSpec, SegmentedStyle, SegmentedState, SegmentedInfo, SegmentedSpecBuilder},
        select::{select, select_raw, SelectSpec, SelectState, SelectInfo, SelectSpecBuilder},
        slider::{slider, slider_raw, SliderStyle, SliderState, SliderSpec, Orientation as SliderOrientation},
        spinner::{spinner, spinner_raw, SpinnerSpec, SpinnerStyle, SpinnerSpecBuilder},
        status::{status, status_raw, StatusVariant, StatusSpec, StatusSpecBuilder},
        switch::{switch, switch_raw, SwitchState, SwitchSpec, SwitchInfo, SwitchSpecBuilder},
        tabs::{tabs, tabs_raw, TabsSpec, TabsStyle, TabsState, TabsInfo, TabsSpecBuilder},
        text_edit::{text_edit, text_edit_raw, TextEditState, TextEditSpec, TextEditInfo},
        tooltip::{tooltip, tooltip_raw, TooltipVariant, TooltipSpec, TooltipSpecBuilder},
        tree::{tree, tree_raw, TreeRow, TreeSpec, TreeSpecBuilder},
        window::{begin_window, begin_window_raw, end_window, end_window_raw, WindowButton, WindowScope, WindowSpec, WindowSpecBuilder},
    },
};

use std::ops::{Deref, DerefMut};

// ── BuilderCtx Compatibility ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BuilderCtx {
    pub theme: Theme,
    pub bg_color: Color,
    pub accent_color: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub button_style: ButtonStyle,
    pub frame_style: FrameStyle,
    pub text_size: f32,
    pub text_font: framewise::text::FontId,
    pub time: f64,
    pub clip_rect: Option<Rect>,
}

impl Default for BuilderCtx {
    fn default() -> Self {
        let theme = Theme::default();
        let frame_style = theme.frame_style();
        Self {
            theme,
            bg_color: Color::WHITE,
            accent_color: Color::BLACK,
            text_color: Color::BLACK,
            border_color: Color::BLACK,
            button_style: ButtonStyle::default(),
            frame_style,
            text_size: 14.0,
            text_font: framewise::text::FontId::default(),
            time: 0.0,
            clip_rect: None,
        }
    }
}

// ── WidgetRenderCompat Compatibility Trait ──────────────────────────────────────

pub trait WidgetRenderCompat<'a, T: TextSystem> {
    type Info;
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info);
}

// 1. ColorSwatch
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::ColorSwatchSpecBuilder {
    type Info = framewise::widgets::ColorSwatchInfo;
    fn render(self, rect: Rect, _theme: &Theme, _ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).build();
        let result = color_swatch_raw(spec);
        (result.draw.0, ColorSwatchInfo { layout: result.layout })
    }
}

// 2. Menu
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::MenuSpecBuilder<'a, T> {
    type Info = ();
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).with_text_system(ts).build();
        let result = menu_raw(spec);
        (result.draw.0, ())
    }
}

// 3. ProgressBar
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::ProgressBarSpecBuilder {
    type Info = ();
    fn render(self, rect: Rect, theme: &Theme, _ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).build();
        let result = progress_bar_raw(spec);
        (result.draw.0, ())
    }
}

// 4. Meter
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::MeterSpecBuilder {
    type Info = framewise::widgets::MeterInfo;
    fn render(self, rect: Rect, _theme: &Theme, _ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).build();
        let result = meter_raw(spec);
        (result.draw.0, MeterInfo { layout: result.layout })
    }
}

// 5. Spinner
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::SpinnerSpecBuilder {
    type Info = ();
    fn render(self, rect: Rect, theme: &Theme, _ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).build();
        let result = spinner_raw(spec);
        (result.draw.0, ())
    }
}

// 6. Status
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::StatusSpecBuilder<'a, T> {
    type Info = ();
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).with_text_system(ts).build();
        let result = status_raw(spec);
        (result.draw.0, ())
    }
}

// 7. Tree
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::TreeSpecBuilder<'a, T> {
    type Info = ();
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).with_text_system(ts).build();
        let result = tree_raw(spec);
        (result.draw.0, ())
    }
}

// 8. Tooltip
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::TooltipSpecBuilder<'a, T> {
    type Info = ();
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).with_text_system(ts).build();
        let result = tooltip_raw(spec);
        (result.draw.0, ())
    }
}

// 9. Keycap
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::KeycapSpecBuilder<'a, T> {
    type Info = framewise::widgets::KeycapInfo;
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).with_text_system(ts).build();
        let result = keycap_raw(spec);
        (result.draw.0, KeycapInfo { layout: result.layout })
    }
}

// 10. Select
impl<'a, T: TextSystem> WidgetRenderCompat<'a, T> for framewise::widgets::SelectSpecBuilder<'a> {
    type Info = framewise::widgets::SelectInfo;
    fn render(self, rect: Rect, theme: &Theme, ts: &'a mut T) -> (Vec<DrawCmd>, Self::Info) {
        let spec = self.with_rect(rect).with_theme(theme).build();
        let state = SelectState::default();
        let input = Input::default();
        let mut dummy_focus_sys = FocusSystem::new();
        let result = select_raw(state, spec, &input, &mut dummy_focus_sys, ts);
        (result.draw.0, SelectInfo {
            layout: result.layout,
            input: result.input,
            state: result.state,
            focused: result.focused,
        })
    }
}

// ── Builder Compatibility ─────────────────────────────────────────────────────

pub struct Builder<'a, T: TextSystem, S: LayoutState> {
    pub ctx: WidgetContext<'a, T, S>,
    pub scroll_scope: Option<ScrollAreaScope>,
    pub window_scope: Option<WindowScope>,
}

impl<'a, T: TextSystem, S: LayoutState> Deref for Builder<'a, T, S> {
    type Target = WidgetContext<'a, T, S>;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<'a, T: TextSystem, S: LayoutState> DerefMut for Builder<'a, T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl<'a, T: TextSystem, S: LayoutState> Builder<'a, T, S> {
    pub fn new(
        ctx: BuilderCtx,
        text_system: &'a mut T,
        focus_sys: &'a mut FocusSystem,
        layout_state: S,
    ) -> Self {
        let mut w_ctx = WidgetContext::new(ctx.theme, text_system, focus_sys, layout_state);
        w_ctx.bg_color = ctx.bg_color;
        w_ctx.accent_color = ctx.accent_color;
        w_ctx.text_color = ctx.text_color;
        w_ctx.border_color = ctx.border_color;
        w_ctx.button_style = ctx.button_style;
        w_ctx.frame_style = ctx.frame_style;
        w_ctx.text_size = ctx.text_size;
        w_ctx.text_font = ctx.text_font;
        w_ctx.time = ctx.time;
        w_ctx.clip_rect = ctx.clip_rect;
        Self { ctx: w_ctx, scroll_scope: None, window_scope: None }
    }

    pub fn append_cmds(&mut self, cmds: Vec<DrawCmd>) {
        self.ctx.append_cmds(cmds);
    }

    pub fn finish(mut self) -> Vec<DrawCmd> {
        if let Some(scope) = self.scroll_scope.take() {
            let post_cmds = end_scroll_area_raw(scope, self.ctx.focus_sys);
            self.ctx.append_cmds(post_cmds);
        }
        if let Some(scope) = self.window_scope.take() {
            let post_cmds = end_window_raw(scope);
            self.ctx.append_cmds(post_cmds);
        }
        self.ctx.finish()
    }

    pub fn child_with_layout<L: Layout>(
        &mut self,
        parent_layout_params: S::Params,
        layout_config: L,
    ) -> Builder<'_, T, L::State> {
        let bounds = self.ctx.layout(parent_layout_params);
        let mut w_ctx = WidgetContext::new(
            self.ctx.theme.clone(),
            self.ctx.text_system,
            self.ctx.focus_sys,
            layout_config.begin(bounds),
        );
        w_ctx.bg_color = self.ctx.bg_color;
        w_ctx.accent_color = self.ctx.accent_color;
        w_ctx.text_color = self.ctx.text_color;
        w_ctx.border_color = self.ctx.border_color;
        w_ctx.button_style = self.ctx.button_style;
        w_ctx.frame_style = self.ctx.frame_style;
        w_ctx.text_size = self.ctx.text_size;
        w_ctx.text_font = self.ctx.text_font;
        w_ctx.time = self.ctx.time;
        w_ctx.clip_rect = self.ctx.clip_rect;
        Builder { ctx: w_ctx, scroll_scope: None, window_scope: None }
    }

    pub fn scroll_area<L: Layout>(
        &mut self,
        layout_params: S::Params,
        content_size: Vec2,
        h_vis: ScrollbarVisibility,
        v_vis: ScrollbarVisibility,
        state: &'a mut ScrollState,
        inner_layout: L,
        input: &Input,
    ) -> Builder<'_, T, OffsetState<L::State>> {
        let (widget_context, scope) = begin_scroll_area(
            &mut self.ctx,
            layout_params,
            content_size,
            h_vis,
            v_vis,
            state,
            inner_layout,
            input,
        );
        Builder { ctx: widget_context, scroll_scope: Some(scope), window_scope: None }
    }

    pub fn window<L: Layout>(
        &mut self,
        layout_params: S::Params,
        widget_spec_builder: WindowSpecBuilder<'_, T>,
        inner_layout: L,
    ) -> Builder<'_, T, L::State> {
        let rect = self.ctx.layout(layout_params);
        let widget_spec = widget_spec_builder
            .with_rect(rect)
            .with_theme(&self.ctx.theme)
            .with_text_system(self.ctx.text_system)
            .build();

        let (pre_cmds, scope, content_bounds) = begin_window_raw(widget_spec);
        self.ctx.append_cmds(pre_cmds);

        let mut w_ctx = WidgetContext::new(
            self.ctx.theme.clone(),
            self.ctx.text_system,
            self.ctx.focus_sys,
            inner_layout.begin(content_bounds),
        );
        w_ctx.bg_color = self.ctx.bg_color;
        w_ctx.accent_color = self.ctx.accent_color;
        w_ctx.text_color = self.ctx.text_color;
        w_ctx.border_color = self.ctx.border_color;
        w_ctx.button_style = self.ctx.button_style;
        w_ctx.frame_style = self.ctx.frame_style;
        w_ctx.text_size = self.ctx.text_size;
        w_ctx.text_font = self.ctx.text_font;
        w_ctx.time = self.ctx.time;
        w_ctx.clip_rect = Some(self.ctx.clip_rect.map_or(content_bounds, |pc| pc.intersect(&content_bounds)));

        Builder { ctx: w_ctx, scroll_scope: None, window_scope: Some(scope) }
    }

    pub fn label(&mut self, layout_params: S::Params, text: &str) -> LabelInfo {
        let spec_builder = LabelSpecBuilder::new(text.to_string())
            .size(self.ctx.text_size)
            .font(self.ctx.text_font)
            .text_color(self.ctx.text_color)
            .rule(false);
        label(&mut self.ctx, layout_params, spec_builder)
    }

    pub fn label_styled(&mut self, layout_params: S::Params, text: &str, size: f32, color: Color, rule: bool) -> LabelInfo {
        let spec_builder = LabelSpecBuilder::new(text.to_string())
            .size(size)
            .font(self.ctx.text_font)
            .text_color(color)
            .rule(rule);
        label(&mut self.ctx, layout_params, spec_builder)
    }

    pub fn label_styled_font(&mut self, layout_params: S::Params, text: &str, size: f32, color: Color, rule: bool, font: framewise::text::FontId) -> LabelInfo {
        let spec_builder = LabelSpecBuilder::new(text.to_string())
            .size(size)
            .font(font)
            .text_color(color)
            .rule(rule);
        label(&mut self.ctx, layout_params, spec_builder)
    }

    pub fn divider(&mut self, layout_params: S::Params) -> DividerInfo {
        let spec_builder = DividerSpecBuilder::new()
            .color(self.ctx.theme.line)
            .width(1.0);
        divider(&mut self.ctx, layout_params, spec_builder)
    }

    pub fn button(&mut self, state: ButtonState, layout_params: S::Params, text: String, input: &Input) -> ButtonInfo {
        let spec_builder = ButtonSpecBuilder::new(text)
            .style(self.ctx.button_style)
            .disabled(false);
        button(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn button_styled(&mut self, state: ButtonState, layout_params: S::Params, text: &str, style: ButtonStyle, disabled: bool, input: &Input) -> ButtonInfo {
        let spec_builder = ButtonSpecBuilder::new(text.to_string())
            .style(style)
            .disabled(disabled);
        button(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn checkbox(&mut self, state: CheckboxState, layout_params: S::Params, input: &Input) -> CheckboxInfo {
        let spec_builder = CheckboxSpecBuilder::new(state.check)
            .disabled(false)
            .style(self.ctx.theme.checkbox_style())
            .clip_rect(self.ctx.clip_rect);
        checkbox(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn radio(&mut self, state: RadioState, layout_params: S::Params, input: &Input) -> RadioInfo {
        let spec_builder = RadioSpecBuilder::new()
            .selected(state.selected)
            .disabled(false)
            .style(self.ctx.theme.radio_style())
            .clip_rect(self.ctx.clip_rect);
        radio(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn switch(&mut self, state: SwitchState, layout_params: S::Params, input: &Input) -> SwitchInfo {
        self.switch_styled(state, layout_params, false, input)
    }

    pub fn switch_styled(&mut self, state: SwitchState, layout_params: S::Params, disabled: bool, input: &Input) -> SwitchInfo {
        let spec_builder = SwitchSpecBuilder::new()
            .on(state.on)
            .disabled(disabled)
            .style(self.ctx.theme.switch_style())
            .clip_rect(self.ctx.clip_rect);
        switch(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn select(&mut self, state: SelectState, options: &[&str], layout_params: S::Params, input: &Input) -> SelectInfo {
        self.select_styled(state, options, layout_params, false, input)
    }

    pub fn select_styled(&mut self, state: SelectState, options: &[&str], layout_params: S::Params, disabled: bool, input: &Input) -> SelectInfo {
        let value = if state.selected_index < options.len() {
            options[state.selected_index]
        } else {
            ""
        };
        let spec_builder = SelectSpecBuilder::new()
            .value(value)
            .font(self.ctx.text_font)
            .options(options)
            .disabled(disabled)
            .style(self.ctx.theme.select_style())
            .clip_rect(self.ctx.clip_rect);
        let result = select_raw(state, spec_builder.build(), input, self.ctx.focus_sys, self.ctx.text_system);
        self.ctx.append_cmds(result.draw.0);
        SelectInfo {
            layout: result.layout,
            input: result.input,
            state: result.state,
            focused: result.focused,
        }
    }

    pub fn segmented(&mut self, state: SegmentedState, items: &[&str], layout_params: S::Params, input: &Input) -> SegmentedInfo {
        let spec_builder = SegmentedSpecBuilder::new()
            .items(items)
            .font(self.ctx.text_font)
            .active_index(state.active_index)
            .disabled(false)
            .style(self.ctx.theme.segmented_style())
            .clip_rect(self.ctx.clip_rect);
        segmented(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn text_edit(&mut self, state: TextEditState, layout_params: S::Params, input: &Input) -> TextEditInfo {
        self.text_edit_ext(state, layout_params, false, false, input)
    }

    pub fn text_edit_ext(&mut self, state: TextEditState, layout_params: S::Params, error: bool, disabled: bool, input: &Input) -> TextEditInfo {
        let spec_builder = TextEditSpecBuilder::new()
            .style(self.ctx.theme.text_edit_style())
            .clip_rect(self.ctx.clip_rect)
            .error(error)
            .disabled(disabled);
        text_edit(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn drag_number(&mut self, state: DragNumberState, label: &str, min: f32, max: f32, layout_params: S::Params, input: &Input) -> DragNumberInfo {
        let spec_builder = DragNumberSpecBuilder::new()
            .label(label)
            .font(self.ctx.text_font)
            .value(state.value)
            .min(min)
            .max(max)
            .disabled(false)
            .style(self.ctx.theme.drag_number_style())
            .clip_rect(self.ctx.clip_rect);
        drag_number(&mut self.ctx, state,layout_params, spec_builder, input)
    }

    pub fn chip(&mut self, state: ChipState, label: &str, layout_params: S::Params, input: &Input) -> ChipInfo {
        let spec_builder = ChipSpecBuilder::new()
            .label(label)
            .font(self.ctx.text_font)
            .disabled(false)
            .style(self.ctx.theme.chip_style())
            .clip_rect(self.ctx.clip_rect);
        chip(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn tabs(&mut self, state: TabsState, items: &[&str], layout_params: S::Params, input: &Input) -> TabsInfo {
        let spec_builder = TabsSpecBuilder::new()
            .items(items)
            .font(self.ctx.text_font)
            .active_index(state.active_index)
            .disabled(false)
            .style(self.ctx.theme.tabs_style())
            .clip_rect(self.ctx.clip_rect);
        tabs(&mut self.ctx, state, layout_params, spec_builder, input)
    }

    pub fn slider(
        &mut self,
        state: &mut SliderState,
        value: &mut f32,
        min: f32,
        max: f32,
        step: f32,
        orientation: SliderOrientation,
        layout_params: S::Params,
        input: &Input,
    ) {
        let spec_builder = SliderSpecBuilder::new()
            .min(min)
            .max(max)
            .page_step(step)
            .step(step)
            .orientation(orientation)
            .thumb_size_ratio(None)
            .style(self.ctx.theme.slider_style())
            .clip_rect(self.ctx.clip_rect)
            .claim_scroll_at_ends(true);
        slider(&mut self.ctx, state, value, layout_params, spec_builder, input);
    }

    pub fn custom(&mut self, layout_params: S::Params, draw_fn: impl FnOnce(Rect) -> Vec<DrawCmd>) {
        let rect = self.ctx.layout(layout_params);
        let cmds = draw_fn(rect);
        self.ctx.append_cmds(cmds);
    }

    pub fn add<'b, WR, WSB>(&'b mut self, layout_params: S::Params, _widget_func: WR, widget_spec_builder: WSB) -> WSB::Info
    where
        WSB: WidgetRenderCompat<'a, T>,
    {
        let rect = self.ctx.layout(layout_params);
        let (cmds, info) = unsafe {
            let ts_ptr = self.ctx.text_system as *mut T;
            widget_spec_builder.render(rect, &self.ctx.theme, &mut *ts_ptr)
        };
        self.ctx.append_cmds(cmds);
        info
    }
}

// ── Fake State Helpers ────────────────────────────────────────────────────────

fn draw_checkbox_fake_state<T: TextSystem, S: LayoutState>(
    b: &mut Builder<'_, T, S>,
    layout_params: S::Params,
    state_val: CheckState,
    is_focused: bool,
    is_disabled: bool,
) {
    let rect = b.ctx.layout(layout_params);
    let mut state = CheckboxState::default();
    state.check = state_val;

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = CheckboxSpec {
        rect,
        state: state_val,
        disabled: is_disabled,
        style: b.ctx.theme.checkbox_style(),
        clip_rect: b.ctx.clip_rect,
    };

    let result = checkbox_raw(
        state,
        spec,
        &dummy_input,
        &mut dummy_focus_sys,
    );
    b.append_cmds(result.draw.0);
}

fn draw_radio_fake_state<T: TextSystem, S: LayoutState>(
    b: &mut Builder<'_, T, S>,
    layout_params: S::Params,
    selected: bool,
    is_focused: bool,
    is_disabled: bool,
) {
    let rect = b.ctx.layout(layout_params);
    let mut state = RadioState::default();
    state.selected = selected;

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = RadioSpec {
        rect,
        selected,
        disabled: is_disabled,
        style: b.ctx.theme.radio_style(),
        clip_rect: b.ctx.clip_rect,
    };

    let result = radio_raw(
        state,
        spec,
        &dummy_input,
        &mut dummy_focus_sys,
    );
    b.append_cmds(result.draw.0);
}

fn draw_switch_fake_state<T: TextSystem, S: LayoutState>(
    b: &mut Builder<'_, T, S>,
    layout_params: S::Params,
    on: bool,
    is_focused: bool,
    is_disabled: bool,
) {
    let rect = b.ctx.layout(layout_params);
    let mut state = SwitchState::default();
    state.on = on;

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = SwitchSpec {
        rect,
        on,
        disabled: is_disabled,
        style: b.ctx.theme.switch_style(),
        clip_rect: b.ctx.clip_rect,
    };

    let result = switch_raw(
        state,
        spec,
        &dummy_input,
        &mut dummy_focus_sys,
    );
    b.append_cmds(result.draw.0);
}

fn draw_select_fake_state<'a, 's, T: TextSystem, S: LayoutState>(
    b: &mut Builder<'a, T, S>,
    layout_params: S::Params,
    value: &'s str,
    options: &'s [&'s str],
    is_open: bool,
    is_focused: bool,
    hovered_row: Option<usize>,
    is_disabled: bool,
) {
    let rect = b.ctx.layout(layout_params);
    let mut state = SelectState::default();
    state.open = is_open;
    state.hovered = hovered_row;

    let mut dummy_focus_sys = FocusSystem::new();
    if is_focused {
        dummy_focus_sys.take_focus(state.focus_id);
    }

    let dummy_input = Input::default();
    let spec = SelectSpec {
        rect,
        value,
        font: b.ctx.theme.sans_font,
        options,
        disabled: is_disabled,
        style: b.ctx.theme.select_style(),
        clip_rect: b.ctx.clip_rect,
    };

    let result = select_raw(
        state,
        spec,
        &dummy_input,
        &mut dummy_focus_sys,
        b.ctx.text_system,
    );
    b.append_cmds(result.draw.0);
}

fn draw_drag_number_fake_state<'a, T: TextSystem, S: LayoutState>(
    b: &mut Builder<'a, T, S>,
    layout_params: S::Params,
    label: &'a str,
    val: f32,
    min: f32,
    max: f32,
    is_active: bool,
) {
    let rect = b.ctx.layout(layout_params);
    let mut state = DragNumberState::default();
    state.value = val;
    state.is_dragging = is_active;

    let dummy_input = Input::default();
    let spec = DragNumberSpec {
        ts: b.ctx.text_system,
        rect,
        label,
        font: b.ctx.theme.sans_font,
        value: val,
        min,
        max,
        disabled: false,
        style: b.ctx.theme.drag_number_style(),
        clip_rect: b.ctx.clip_rect,
    };

    let mut dummy_focus_sys = FocusSystem::new();
    let result = drag_number_raw(
        state,
        spec,
        &dummy_input,
        &mut dummy_focus_sys,
    );
    b.append_cmds(result.draw.0);
}

fn draw_button_fake_state<T: TextSystem, S: LayoutState>(
    b: &mut Builder<'_, T, S>,
    layout_params: S::Params,
    text: &str,
    style: ButtonStyle,
    hover: bool,
    pressed: bool,
    focused: bool,
) {
    let rect = b.ctx.layout(layout_params);
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

    let spec = ButtonSpec {
        rect,
        text: text.to_string(),
        style,
        clip_rect: None,
        disabled: false,
    };

    let result = button_raw(state, spec, &fake_input, b.ctx.text_system, &mut dummy_focus_sys);
    b.append_cmds(result.draw.0);
}

// ── Page state ────────────────────────────────────────────────────────────────

pub struct SpecPageState {
    pub page_scroll: ScrollState,

    // 01 Buttons
    pub btn_variants: Vec<ButtonState>, // [secondary, primary, accent, ghost]
    pub btn_matrix: Vec<ButtonState>,   // 4 variants × 2 real states (default + disabled) = 8
    pub cb_matrix: Vec<CheckboxState>,  // 2 rows × 3 interactive cols (off, on, mixed) = 6
    pub btn_sizes: Vec<ButtonState>,    // [sm, md, lg]
    pub btn_grp1: Vec<ButtonState>,     // [←, Frame 248, →]
    pub btn_grp2: Vec<ButtonState>,     // [Build, Run, Ship]

    // 02 Text Inputs
    pub te_matrix: Vec<TextEditState>, // 2 rows × 5 cols = 10
    pub te_labelled: TextEditState,
    pub te_prefixed: TextEditState,
    pub te_multiline: TextEditState,

    // 03 Radios & switches
    pub radio_states: Vec<RadioState>,   // items 0,1,2 — item 3 (focused) stays fake
    pub switch_states: Vec<SwitchState>, // items 0,1,3 — item 2 (focused) stays fake

    // 04 Sliders
    pub slider1_state: SliderState,
    pub slider1_val: f32,
    pub slider2_state: SliderState,
    pub slider2_val: f32,
    pub slider3_state: SliderState,
    pub slider3_val: f32,
    pub slider4_state: SliderState,
    pub slider4_val: f32, // stepped 0–9

    // 04 Drag-number showcase
    pub dn_showcase: Vec<DragNumberState>, // X(320), Y(144), H(400) — W stays fake

    // 05 Selection
    pub sel_state: SelectState,
    pub seg1_state: SegmentedState,
    pub seg2_state: SegmentedState,
    pub chip_states: Vec<ChipState>, // opengl, vulkan, metal, wgpu, + add backend

    // 07 Tabs
    pub tabs1_state: TabsState,
    pub tabs2_state: TabsState,

    // 11 Window chrome (Inspector inner content)
    pub win11_drags: Vec<DragNumberState>, // X(320), Y(144), W(576), H(400)
    pub win11_cbs: Vec<CheckboxState>,     // clip to parent (On), debug overlay (Off)

    // 06 Scroll areas
    pub scroll_vert: ScrollState,
    pub scroll_horiz: ScrollState,
    pub scroll_both: ScrollState,
    pub scroll_both_axes: ScrollState,

    // 12 In Use
    pub iu_backend: SegmentedState,
    pub iu_tabs: TabsState,
    pub iu_fps_slider: SliderState,
    pub iu_fps_val: f32,
    pub iu_btns: Vec<ButtonState>, // [Reset, Cancel, Apply]
    pub iu_log_scroll: ScrollState,
    pub iu_vsync: SwitchState,
    pub iu_msaa: SegmentedState,
    pub iu_vp_w: DragNumberState,
    pub iu_vp_h: DragNumberState,
    pub iu_options: Vec<CheckboxState>,
}

impl Default for SpecPageState {
    fn default() -> Self {
        let mut te_matrix: Vec<TextEditState> = Vec::with_capacity(10);
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
            page_scroll: ScrollState::default(),
            btn_variants: (0..4).map(|_| ButtonState::default()).collect(),
            btn_matrix: (0..8).map(|_| ButtonState::default()).collect(),
            cb_matrix: vec![
                CheckboxState { check: CheckState::Off, ..Default::default() },
                CheckboxState { check: CheckState::On, ..Default::default() },
                CheckboxState { check: CheckState::Indeterminate, ..Default::default() },
                CheckboxState { check: CheckState::Off, ..Default::default() },
                CheckboxState { check: CheckState::On, ..Default::default() },
                CheckboxState { check: CheckState::Indeterminate, ..Default::default() },
            ],
            btn_sizes: (0..3).map(|_| ButtonState::default()).collect(),
            btn_grp1: (0..3).map(|_| ButtonState::default()).collect(),
            btn_grp2: (0..3).map(|_| ButtonState::default()).collect(),
            te_matrix,
            te_labelled: TextEditState::new("framewise"),
            te_prefixed: TextEditState::new("0.1.0"),
            te_multiline: TextEditState::new(
                "A small, procedural Rust library for describing GUI elements per frame.",
            ),
            slider1_state: SliderState::default(),
            slider1_val: 0.14,
            slider2_state: SliderState::default(),
            slider2_val: 0.62,
            slider3_state: SliderState::default(),
            slider3_val: 0.88,
            slider4_state: SliderState::default(),
            slider4_val: 3.0,
            radio_states: vec![
                RadioState { selected: true, ..Default::default() },
                RadioState { selected: false, ..Default::default() },
                RadioState { selected: false, ..Default::default() },
            ],
            switch_states: vec![
                SwitchState { on: false, ..Default::default() },
                SwitchState { on: true, ..Default::default() },
                SwitchState { on: false, ..Default::default() }, // multisampling disabled
            ],
            dn_showcase: vec![
                DragNumberState { value: 320.0, ..Default::default() },
                DragNumberState { value: 144.0, ..Default::default() },
                DragNumberState { value: 400.0, ..Default::default() },
            ],
            sel_state: SelectState::default(),
            seg1_state: SegmentedState { active_index: 0, ..Default::default() },
            seg2_state: SegmentedState { active_index: 1, ..Default::default() },
            chip_states: vec![
                ChipState { active: true, ..Default::default() },
                ChipState { active: false, ..Default::default() },
                ChipState { active: false, ..Default::default() },
                ChipState { active: false, ..Default::default() },
                ChipState { active: false, ..Default::default() },
            ],
            tabs1_state: TabsState { active_index: 0, ..Default::default() },
            tabs2_state: TabsState { active_index: 1, ..Default::default() },
            win11_drags: vec![
                DragNumberState { value: 320.0, ..Default::default() },
                DragNumberState { value: 144.0, ..Default::default() },
                DragNumberState { value: 576.0, ..Default::default() },
                DragNumberState { value: 400.0, ..Default::default() },
            ],
            win11_cbs: vec![
                CheckboxState { check: CheckState::On, ..Default::default() },
                CheckboxState { check: CheckState::Off, ..Default::default() },
            ],
            scroll_vert: ScrollState::default(),
            scroll_horiz: ScrollState::default(),
            scroll_both: ScrollState::default(),
            scroll_both_axes: ScrollState::default(),
            iu_backend: SegmentedState { active_index: 1, ..Default::default() },
            iu_tabs: TabsState { active_index: 0, ..Default::default() },
            iu_fps_slider: SliderState::default(),
            iu_fps_val: 60.0,
            iu_btns: (0..3).map(|_| ButtonState::default()).collect(),
            iu_log_scroll: ScrollState::default(),
            iu_vsync: SwitchState { on: true, ..Default::default() },
            iu_msaa: SegmentedState { active_index: 2, ..Default::default() },
            iu_vp_w: DragNumberState { value: 1920.0, ..Default::default() },
            iu_vp_h: DragNumberState { value: 1080.0, ..Default::default() },
            iu_options: vec![
                CheckboxState { check: CheckState::On, ..Default::default() },
                CheckboxState { check: CheckState::Off, ..Default::default() },
                CheckboxState { check: CheckState::Indeterminate, ..Default::default() },
            ],
        }
    }
}

// ── Layout constants ──────────────────────────────────────────────────────────

const MARGIN: f32 = 64.0;
const SEC_GAP: f32 = 64.0;
const GROUP_GAP: f32 = 28.0;
const COL_GAP: f32 = 16.0;

pub const CONTENT_HEIGHT: f32 = 5800.0;

// ── Draw helpers ──────────────────────────────────────────────────────────────

fn sec_y<S: LayoutState<Params = Rect>>(
    b: &mut Builder<SampleTextSystem, S>,
    t: &Theme,
    lx: f32,
    y: f32,
    w: f32,
    num: &str,
    title: &str,
) {
    b.divider(Rect::new(lx, y, w, 36.0));
    b.label_styled(Rect::new(lx, y, 40.0, 20.0), num, t.text_sm, t.muted, false);
    b.label_styled_font(
        Rect::new(lx + 44.0, y, w - 44.0, 22.0),
        title,
        18.0,
        t.ink,
        false,
        t.sans_font,
    );
}

fn label_styled_sans<S: LayoutState<Params = Rect>>(
    b: &mut Builder<SampleTextSystem, S>,
    t: &Theme,
    rect: Rect,
    text: &str,
    size: f32,
    color: Color,
) {
    b.label_styled_font(rect, text, size, color, false, t.sans_font);
}

fn group_y<S: LayoutState<Params = Rect>>(
    b: &mut Builder<SampleTextSystem, S>,
    t: &Theme,
    lx: f32,
    y: f32,
    text: &str,
) {
    b.label_styled(
        Rect::new(lx, y, 400.0, 16.0),
        &text.to_uppercase(),
        t.text_sm,
        t.muted,
        false,
    );
}

// ── Main function ─────────────────────────────────────────────────────────────

pub fn draw_spec_page(
    ts: &mut SampleTextSystem,
    focus_sys: &mut FocusSystem,
    state: &mut SpecPageState,
    input: &Input,
    time: f64,
    win_w: f32,
    win_h: f32,
) -> Vec<DrawCmd> {
    let t = Theme::framewise();

    let content_w = (win_w - MARGIN * 2.0).min(1100.0);
    let lx = (win_w - content_w) * 0.5;

    let mut ctx = BuilderCtx::default();
    ctx.text_color = t.ink;
    ctx.bg_color = t.paper;
    ctx.text_size = t.text_md;
    ctx.text_font = t.mono_font;
    ctx.time = time;
    ctx.button_style = ButtonStyle::default();

    let win_rect = Rect::new(0.0, 0.0, win_w, win_h);
    let mut b = Builder::new(ctx, ts, focus_sys, ManualLayout.begin(win_rect));

    // Background fill (outside clip so it covers the whole viewport).
    let bg = frame_raw(FrameSpec {
        rect: win_rect,
        style: FrameStyle {
            background: t.paper,
            border: t.paper,
            border_width: 0.0,
            padding: 0.0,
        },
    });
    b.append_cmds(bg.draw.0);

    // Scroll area provides clip + scroll offset for all page content.
    let page_cmds = {
        let mut page = b.scroll_area(
            win_rect,
            Vec2::new(content_w, CONTENT_HEIGHT),
            ScrollbarVisibility::None,
            ScrollbarVisibility::Auto,
            &mut state.page_scroll,
            ManualLayout,
            input,
        );
        {
            let mut b = &mut page;

            // ── HERO ─────────────────────────────────────────────────────────────────
            {
                b.custom(Rect::new(lx, MARGIN, 96.0, 96.0), |rect| {
                    hero_logo(&t, rect.x, rect.y)
                });

                let tx = lx + 124.0; // 28px gap + 96px logo = 124px
                let hero_w = content_w - 124.0;

                // Overline
                b.label_styled(
                    Rect::new(tx, MARGIN, hero_w, 16.0),
                    "FRAMEWISE · WIDGET SPECIFICATION · V0.1",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Two-line Title (56px size, Bold, line-height 0.95)
                b.label_styled_font(
                    Rect::new(tx, MARGIN + 22.0, hero_w, 53.0),
                    "A widget set that",
                    56.0,
                    t.ink,
                    false,
                    t.sans_bold_font,
                );
                b.label_styled_font(
                    Rect::new(tx, MARGIN + 22.0 + 53.0, hero_w, 53.0),
                    "explains itself.",
                    56.0,
                    t.ink,
                    false,
                    t.sans_bold_font,
                );

                // Description (15px size, regular, line-height 1.55)
                b.label_styled_font(
                    Rect::new(tx, MARGIN + 144.0, hero_w.min(600.0), 23.0),
                    "Sharp corners, hairline borders, monospaced numerics. One accent — rust —",
                    15.0,
                    Color::from_srgb_u8(58, 53, 45, 255), // #3a352d
                    false,
                    t.sans_font,
                );
                b.label_styled_font(
                    Rect::new(tx, MARGIN + 144.0 + 23.0, hero_w.min(600.0), 23.0),
                    "reserved for focus, drag, and primary action. Every widget describes its state",
                    15.0,
                    Color::from_srgb_u8(58, 53, 45, 255), // #3a352d
                    false,
                    t.sans_font,
                );
                b.label_styled_font(
                    Rect::new(tx, MARGIN + 144.0 + 46.0, hero_w.min(600.0), 23.0),
                    "explicitly; nothing is hidden behind animation or chrome.",
                    15.0,
                    Color::from_srgb_u8(58, 53, 45, 255), // #3a352d
                    false,
                    t.sans_font,
                );

                // Color Meta Row
                let meta_items: &[(&str, &str)] = &[
                    ("INK", "#15130F"),
                    ("PAPER", "#F4F1EA"),
                    ("RUST", "#C25A2C"),
                    ("TYPE", "INTER TIGHT · JETBRAINS MONO"),
                ];
                let mut mx = tx;
                let my = MARGIN + 234.0;
                for (key, val) in meta_items {
                    // key in ink, bold / medium
                    b.label_styled_font(
                        Rect::new(mx, my, 60.0, 14.0),
                        key,
                        t.text_sm,
                        t.ink,
                        false,
                        t.sans_bold_font,
                    );
                    let key_w = key.len() as f32 * 7.5 + 4.0;
                    b.label_styled_font(
                        Rect::new(mx + key_w, my, 200.0, 14.0),
                        val,
                        t.text_sm,
                        t.muted,
                        false,
                        t.sans_font,
                    );
                    mx += key_w + val.len() as f32 * 6.5 + 24.0;
                }
            }

            let mut y = MARGIN + 310.0;

            // ── 01 · BUTTONS ─────────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "01", "Buttons");
            y += 46.0;

            // variants row
            group_y(&mut b, &t, lx, y, "variants");
            y += 20.0;
            {
                let styles: &[(&str, ButtonStyle, bool)] = &[
                    ("Apply changes", ButtonStyle::primary(), false),
                    ("Cancel", ButtonStyle::default(), false),
                    ("Reset", ButtonStyle::ghost(), false),
                    ("Publish v0.2", ButtonStyle::accent(), false),
                ];
                let mut bx = lx;
                for (i, (label, style, _)) in styles.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 24.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_variants[i]),
                        Rect::new(bx, y, w, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_variants[i] = btn.state;
                    bx += w + COL_GAP;
                }
            }
            y += t.h_md + GROUP_GAP;

            // state matrix
            group_y(&mut b, &t, lx, y, "states · default button");
            y += 20.0;
            {
                let col_labels = ["DEFAULT", "HOVER", "PRESSED", "FOCUSED", "DISABLED"];
                let row_labels = ["secondary", "primary", "accent", "ghost"];
                let row_styles: &[ButtonStyle] = &[
                    ButtonStyle::default(),
                    ButtonStyle::primary(),
                    ButtonStyle::accent(),
                    ButtonStyle::ghost(),
                ];
                let label_w = 80.0_f32;
                let cell_w = 88.0_f32;

                // column headers
                for (ci, col) in col_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx + label_w + ci as f32 * cell_w, y, cell_w - 8.0, 16.0),
                        col,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                }
                y += 20.0;

                for (ri, row_label) in row_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx, y, label_w - 8.0, t.h_md),
                        row_label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    for ci in 0..5 {
                        let rect = Rect::new(lx + label_w + ci as f32 * cell_w, y, cell_w - 8.0, t.h_md);
                        match ci {
                            1 => draw_button_fake_state(&mut b, rect, "Action", row_styles[ri].clone(), true, false, false),
                            2 => draw_button_fake_state(&mut b, rect, "Action", row_styles[ri].clone(), false, true, false),
                            3 => draw_button_fake_state(&mut b, rect, "Action", row_styles[ri].clone(), false, false, true),
                            _ => {
                                let disabled = ci == 4;
                                let idx = ri * 2 + ci / 4; // ci=0 → idx 0 (default), ci=4 → idx 1 (disabled)
                                let btn = b.button_styled(
                                    std::mem::take(&mut state.btn_matrix[idx]),
                                    rect,
                                    "Action",
                                    row_styles[ri].clone(),
                                    disabled,
                                    input,
                                );
                                state.btn_matrix[idx] = btn.state;
                            }
                        }
                    }
                    y += t.h_md + 4.0;
                }
            }
            y += GROUP_GAP;

            // sizes & groups
            group_y(&mut b, &t, lx, y, "sizes  ·  groups");
            y += 20.0;
            {
                let size_defs: &[(&str, f32, ButtonStyle)] = &[
                    ("22 px", t.h_sm, ButtonStyle::default()),
                    ("28 px", t.h_md, ButtonStyle::default()),
                    ("36 px", t.h_lg, ButtonStyle::default()),
                ];
                let mut bx = lx;
                for (i, (label, h, style)) in size_defs.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 20.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_sizes[i]),
                        Rect::new(bx, y, w, *h),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_sizes[i] = btn.state;
                    bx += w + COL_GAP;
                }
                bx += 24.0;

                // button group 1: ← | Frame 248 | →
                let grp1: &[(&str, ButtonStyle)] = &[
                    ("←", ButtonStyle::default()),
                    ("Frame 248", ButtonStyle::default()),
                    ("→", ButtonStyle::default()),
                ];
                // draw group border
                for (i, (label, style)) in grp1.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 20.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_grp1[i]),
                        Rect::new(bx, y, w, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_grp1[i] = btn.state;
                    bx += w;
                }
                bx += COL_GAP;

                // button group 2: Build | Run | Ship
                let grp2: &[(&str, ButtonStyle)] = &[
                    ("Build", ButtonStyle::default()),
                    ("Run", ButtonStyle::default()),
                    ("Ship", ButtonStyle::primary()),
                ];
                for (i, (label, style)) in grp2.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 20.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_grp2[i]),
                        Rect::new(bx, y, w, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_grp2[i] = btn.state;
                    bx += w;
                }
                let _ = bx;
            }
            y += t.h_md + SEC_GAP;

            // ── 02 · TEXT INPUTS ─────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "02", "Text inputs");
            y += 46.0;

            group_y(&mut b, &t, lx, y, "states · single-line");
            y += 20.0;
            {
                let col_labels = ["DEFAULT", "HOVER", "FOCUSED", "ERROR", "DISABLED"];
                let row_labels = ["empty", "filled"];
                let cell_w = 160.0_f32;
                let label_w = 60.0_f32;

                for (ci, col) in col_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx + label_w + ci as f32 * (cell_w + 8.0), y, cell_w, 16.0),
                        col,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                }
                y += 20.0;

                for (ri, row_label) in row_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx, y, label_w - 4.0, t.h_md),
                        row_label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    for ci in 0..5 {
                        let idx = ri * 5 + ci;
                        let error = ci == 3;
                        let disabled = ci == 4;
                        let info = b.text_edit_ext(
                            std::mem::take(&mut state.te_matrix[idx]),
                            Rect::new(lx + label_w + ci as f32 * (cell_w + 8.0), y, cell_w, t.h_md),
                            error,
                            disabled,
                            input,
                        );
                        state.te_matrix[idx] = info.state;
                    }
                    y += t.h_md + 8.0;
                }
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "labelled  ·  prefixed  ·  multiline");
            y += 20.0;
            {
                // Labelled field
                let field_x = lx;
                b.label_styled(
                    Rect::new(field_x, y, 120.0, 14.0),
                    "CRATE NAME",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let info = b.text_edit_ext(
                    std::mem::take(&mut state.te_labelled),
                    Rect::new(field_x, y + 18.0, 160.0, t.h_md),
                    false,
                    false,
                    input,
                );
                state.te_labelled = info.state;
                b.label_styled(
                    Rect::new(field_x, y + 18.0 + t.h_md + 4.0, 200.0, 14.0),
                    "a–z, 0–9, hyphen; max 64",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Prefixed field (draw prefix addon manually)
                let pf_x = lx + 200.0;
                b.label_styled(
                    Rect::new(pf_x, y, 120.0, 14.0),
                    "VERSION",
                    t.text_sm,
                    t.muted,
                    false,
                );
                b.custom(Rect::new(pf_x, y + 18.0, 24.0, t.h_md), |rect| {
                    vec![
                        DrawCmd::FillRect { rect, color: t.ink },
                        DrawCmd::StrokeRect {
                            rect,
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(pf_x + 6.0, y + 18.0 + 7.0, 16.0, 14.0),
                    "v",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let info = b.text_edit_ext(
                    std::mem::take(&mut state.te_prefixed),
                    Rect::new(pf_x + 24.0, y + 18.0, 120.0, t.h_md),
                    false,
                    false,
                    input,
                );
                state.te_prefixed = info.state;
                b.label_styled(
                    Rect::new(pf_x, y + 18.0 + t.h_md + 4.0, 200.0, 14.0),
                    "semver mismatch — bump minor",
                    t.text_sm,
                    t.rust,
                    false,
                );

                // Multiline field
                let ml_x = lx + 420.0;
                b.label_styled(
                    Rect::new(ml_x, y, 120.0, 14.0),
                    "DESCRIPTION",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let info = b.text_edit_ext(
                    std::mem::take(&mut state.te_multiline),
                    Rect::new(ml_x, y + 18.0, 280.0, 68.0),
                    false,
                    false,
                    input,
                );
                state.te_multiline = info.state;
            }
            y += 18.0 + 68.0 + 4.0 + 14.0 + SEC_GAP;

            // ── 03 · CHECK · RADIO · SWITCH ──────────────────────────────────────────
            sec_y(
                &mut b,
                &t,
                lx,
                y,
                content_w,
                "03",
                "Checkboxes, radios & switches",
            );
            y += 46.0;

            group_y(&mut b, &t, lx, y, "checkbox");
            y += 20.0;
            {
                let col_labels = ["OFF", "ON", "MIXED", "FOCUSED", "DISABLED"];
                let label_w = 80.0_f32;
                let cell_w = 100.0_f32;
                for (ci, col) in col_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx + label_w + ci as f32 * cell_w, y, cell_w - 4.0, 14.0),
                        col,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                }
                y += 18.0;

                // Row 1: box only
                b.label_styled(
                    Rect::new(lx, y, label_w - 4.0, 14.0),
                    "box",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let box_specs: &[(CheckState, bool, bool)] = &[
                    (CheckState::Off, false, false),
                    (CheckState::On, false, false),
                    (CheckState::Indeterminate, false, false),
                    (CheckState::On, true, false),
                    (CheckState::On, false, true),
                ];
                for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
                    let rect = Rect::new(lx + label_w + ci as f32 * cell_w, y, 14.0, 14.0);
                    if ci < 3 {
                        let info = b.checkbox(
                            std::mem::take(&mut state.cb_matrix[ci]),
                            rect,
                            input,
                        );
                        state.cb_matrix[ci] = info.state;
                    } else {
                        draw_checkbox_fake_state(&mut b, rect, *cs, *focused, *disabled);
                    }
                }
                y += 14.0 + 12.0;

                // Row 2: with label
                b.label_styled(
                    Rect::new(lx, y, label_w - 4.0, 14.0),
                    "with label",
                    t.text_sm,
                    t.muted,
                    false,
                );
                for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
                    let cx = lx + label_w + ci as f32 * cell_w;
                    if ci < 3 {
                        let info = b.checkbox(
                            std::mem::take(&mut state.cb_matrix[3 + ci]),
                            Rect::new(cx, y, 14.0, 14.0),
                            input,
                        );
                        state.cb_matrix[3 + ci] = info.state;
                    } else {
                        draw_checkbox_fake_state(
                            &mut b,
                            Rect::new(cx, y, 14.0, 14.0),
                            *cs,
                            *focused,
                            *disabled,
                        );
                    }

                    let label_alpha = if *disabled { t.muted } else { t.ink };
                    b.label_styled(
                        Rect::new(cx + 18.0, y, 60.0, 14.0),
                        "vsync",
                        t.text_sm,
                        label_alpha,
                        false,
                    );
                }
                y += 14.0;
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "radio  ·  switch");
            y += 20.0;
            {
                let radio_labels = ["immediate-mode", "retained-mode", "hybrid", "deferred"];
                for (i, label) in radio_labels.iter().enumerate() {
                    let ry = y + i as f32 * 22.0;
                    if i < 3 {
                        let info = b.radio(
                            std::mem::take(&mut state.radio_states[i]),
                            Rect::new(lx, ry, 14.0, 14.0),
                            input,
                        );
                        state.radio_states[i] = info.state;
                        if info.input.clicked {
                            for j in 0..3 {
                                state.radio_states[j].selected = j == i;
                            }
                        }
                    } else {
                        draw_radio_fake_state(&mut b, Rect::new(lx, ry, 14.0, 14.0), false, true, false);
                    }
                    b.label_styled(
                        Rect::new(lx + 18.0, ry, 140.0, 14.0),
                        label,
                        t.text_md,
                        t.ink,
                        false,
                    );
                }
                let sw_x = lx + 220.0;
                let switch_labels = ["debug overlay", "show layout grid", "vsync", "multisampling"];
                for (i, label) in switch_labels.iter().enumerate() {
                    let ry = y + i as f32 * 22.0;
                    let label_color = if i == 3 { t.muted } else { t.ink };
                    match i {
                        2 => draw_switch_fake_state(&mut b, Rect::new(sw_x, ry, 30.0, 16.0), true, true, false),
                        3 => {
                            let info = b.switch_styled(
                                std::mem::take(&mut state.switch_states[2]),
                                Rect::new(sw_x, ry, 30.0, 16.0),
                                true,
                                input,
                            );
                            state.switch_states[2] = info.state;
                        }
                        _ => {
                            let info = b.switch(
                                std::mem::take(&mut state.switch_states[i]),
                                Rect::new(sw_x, ry, 30.0, 16.0),
                                input,
                            );
                            state.switch_states[i] = info.state;
                        }
                    }
                    b.label_styled(
                        Rect::new(sw_x + 36.0, ry, 140.0, 16.0),
                        label,
                        t.text_md,
                        label_color,
                        false,
                    );
                }
            }
            y += 4.0 * 22.0 + SEC_GAP;

            // ── 04 · SLIDERS · DRAGS ─────────────────────────────────────────────────
            sec_y(
                &mut b,
                &t,
                lx,
                y,
                content_w,
                "04",
                "Sliders & numeric drags",
            );
            y += 46.0;

            group_y(&mut b, &t, lx, y, "slider · single value");
            y += 20.0;
            {
                let slider_w = 360.0_f32;
                let row_gap = 14.0_f32;

                b.slider(
                    &mut state.slider1_state,
                    &mut state.slider1_val,
                    0.0,
                    1.0,
                    0.1,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.2}", state.slider1_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                y += t.h_md + row_gap;

                b.slider(
                    &mut state.slider2_state,
                    &mut state.slider2_val,
                    0.0,
                    1.0,
                    0.1,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.2}", state.slider2_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                y += t.h_md + row_gap;

                b.slider(
                    &mut state.slider3_state,
                    &mut state.slider3_val,
                    0.0,
                    1.0,
                    0.1,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.2}", state.slider3_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                y += t.h_md + row_gap;

                // Stepped slider (0–9) with tick marks
                b.slider(
                    &mut state.slider4_state,
                    &mut state.slider4_val,
                    0.0,
                    9.0,
                    1.0,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.0} / 9", state.slider4_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                // tick marks below track
                let tick_y = y + t.h_md + 2.0;
                let tick_h = 4.0;
                let usable = slider_w - 12.0;
                for i in 0..=9usize {
                    let tx = lx + 6.0 + (i as f32 / 9.0) * usable;
                    b.custom(Rect::new(tx - 0.5, tick_y, 1.0, tick_h), |rect| {
                        vec![DrawCmd::FillRect {
                            rect,
                            color: t.line,
                        }]
                    });
                }
                y += t.h_md + 8.0;
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "range slider");
            y += 20.0;
            {
                let track_w = 360.0_f32;
                let mid_y = y + t.h_md * 0.5;
                b.custom(Rect::new(lx, mid_y - 0.75, track_w, 12.0), |rect| {
                    let lx = rect.x;
                    let track_w = rect.w;
                    let mid_y = rect.y + 0.75;
                    let t1 = 0.24_f32;
                    let t2 = 0.76_f32;
                    let fill_x1 = lx + track_w * t1;
                    let fill_x2 = lx + track_w * t2;
                    let ts = 12.0_f32; // thumb size
                    let half_ts = ts * 0.5;

                    vec![
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
                    ]
                });
                b.label_styled(
                    Rect::new(lx + track_w + 12.0, y + 6.0, 80.0, 14.0),
                    ".24–.76",
                    t.text_sm,
                    t.ink,
                    false,
                );
            }
            y += t.h_md + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "drag-number (imgui-style)");
            y += 20.0;
            {
                let mut bx = lx;
                // X — real
                let info = b.drag_number(std::mem::take(&mut state.dn_showcase[0]), "X", 0.0, 800.0, Rect::new(bx, y, 100.0, t.h_md), input);
                state.dn_showcase[0] = info.state;
                bx += 100.0 + 8.0;
                // Y — real
                let info = b.drag_number(std::mem::take(&mut state.dn_showcase[1]), "Y", 0.0, 600.0, Rect::new(bx, y, 100.0, t.h_md), input);
                state.dn_showcase[1] = info.state;
                bx += 100.0 + 8.0;
                // W — fake (forced active/dragging)
                draw_drag_number_fake_state(&mut b, Rect::new(bx, y, 100.0, t.h_md), "W", 576.0, 0.0, 800.0, true);
                bx += 100.0 + 8.0;
                // H — real
                let info = b.drag_number(std::mem::take(&mut state.dn_showcase[2]), "H", 0.0, 600.0, Rect::new(bx, y, 100.0, t.h_md), input);
                state.dn_showcase[2] = info.state;
            }
            y += t.h_md + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "numeric stepper  ·  colour swatch");
            y += 20.0;
            {
                // prefix + value display
                let stepper_x = lx;
                b.custom(Rect::new(stepper_x, y, 64.0, t.h_md), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: t.hover,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(stepper_x + 6.0, y + 7.0, 56.0, 14.0),
                    "padding",
                    t.text_sm,
                    t.muted,
                    false,
                );
                b.custom(Rect::new(stepper_x + 64.0, y, 40.0, t.h_md), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(stepper_x + 72.0, y + 7.0, 24.0, 14.0),
                    "12",
                    t.text_sm,
                    t.ink,
                    false,
                );

                // +/- buttons as text
                let sx = stepper_x + 120.0;
                b.custom(Rect::new(sx, y, 84.0, t.h_sm), |rect| {
                    vec![
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
                    ]
                });
                b.label_styled(
                    Rect::new(sx + 6.0, y + 4.0, 10.0, 14.0),
                    "−",
                    t.text_sm,
                    t.ink,
                    false,
                );
                b.label_styled(
                    Rect::new(sx + 28.0, y + 4.0, 28.0, 14.0),
                    "12",
                    t.text_sm,
                    t.ink,
                    false,
                );
                b.label_styled(
                    Rect::new(sx + 68.0, y + 4.0, 10.0, 14.0),
                    "+",
                    t.text_sm,
                    t.ink,
                    false,
                );

                // color swatches
                let sw_x = sx + 100.0;
                let swatches: &[(Color, &str)] = &[(t.ink, "#15130f"), (t.rust, "#c25a2c")];
                let mut bx = sw_x;
                for (color, hex) in swatches {
                    b.add(
                        Rect::new(bx, y, 18.0, t.h_md),
                        color_swatch::<SampleTextSystem, framewise::layout::ManualState>,
                        framewise::widgets::ColorSwatchSpecBuilder::new()
                            .color(*color)
                            .border(t.line),
                    );
                    b.label_styled(
                        Rect::new(bx + 22.0, y + 7.0, 60.0, 14.0),
                        hex,
                        t.text_sm,
                        t.ink,
                        false,
                    );
                    bx += 86.0;
                }
            }
            y += t.h_md + SEC_GAP;

            // ── 05 · SELECTION ───────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "05", "Selection");
            y += 46.0;

            group_y(&mut b, &t, lx, y, "select  ·  segmented  ·  chips");
            y += 20.0;
            {
                // Select widgets
                const LAYOUT_OPTS: &[&str] = &["Layout: row", "Layout: column", "Layout: grid"];
                let sel_info = b.select(
                    std::mem::take(&mut state.sel_state),
                    LAYOUT_OPTS,
                    Rect::new(lx, y, 160.0, t.h_md),
                    input,
                );
                state.sel_state = sel_info.state;
                draw_select_fake_state(
                    &mut b,
                    Rect::new(lx, y + t.h_md + 4.0, 160.0, t.h_md),
                    "Layout row",
                    LAYOUT_OPTS,
                    true,
                    true,
                    Some(0),
                    false,
                );

                // Segmented controls
                let seg_x = lx + 200.0;
                const SEGS1: &[&str] = &["row", "column", "grid", "flex"];
                let seg1_info = b.segmented(
                    std::mem::take(&mut state.seg1_state),
                    SEGS1,
                    Rect::new(seg_x, y, 0.0, t.h_md),
                    input,
                );
                state.seg1_state = seg1_info.state;
                const SEGS2: &[&str] = &["start", "center", "end"];
                let seg2_info = b.segmented(
                    std::mem::take(&mut state.seg2_state),
                    SEGS2,
                    Rect::new(seg_x, y + t.h_md + 4.0, 0.0, t.h_md),
                    input,
                );
                state.seg2_state = seg2_info.state;

                // Chips
                let chip_labels = ["opengl", "vulkan", "metal", "wgpu"];
                let chip_y = y;
                let mut chip_x = lx + 560.0;
                for (i, label) in chip_labels.iter().enumerate() {
                    let layout = b.text_system.prepare(label, t.text_sm, t.mono_font);
                    let chip_w = (layout.size.x + 16.0).max(32.0);
                    let chip_info = b.chip(
                        std::mem::take(&mut state.chip_states[i]),
                        label,
                        Rect::new(chip_x, chip_y, chip_w, 22.0),
                        input,
                    );
                    state.chip_states[i] = chip_info.state;
                    chip_x += chip_w + 6.0;
                }
                let add_layout = b
                    .text_system
                    .prepare("+ add backend", t.text_sm, t.mono_font);
                let add_w = (add_layout.size.x + 16.0).max(32.0);
                let add_info = b.chip(
                    std::mem::take(&mut state.chip_states[4]),
                    "+ add backend",
                    Rect::new(lx + 560.0, y + 28.0, add_w, 22.0),
                    input,
                );
                state.chip_states[4] = add_info.state;
            }
            let select_open_h = 3.0 * 26.0 + 8.0;
            y += t.h_md + 4.0 + t.h_md + select_open_h + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "dropdown menu (open)");
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
                b.add(
                    Rect::new(lx, y, 240.0, 0.0),
                    menu::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::MenuSpecBuilder::new().items(ITEMS1),
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
                b.add(
                    Rect::new(lx + 264.0, y, 200.0, 0.0),
                    menu::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::MenuSpecBuilder::new().items(ITEMS2),
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

            // ── 06 · SCROLLBARS ──────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "06", "Scrollbars");
            y += 46.0;
            {
                let box_gap = 24.0_f32;
                let cap_h = 20.0_f32;

                // Box 1: vertical, idle
                let b1 = Rect::new(lx, y, 180.0, 130.0);
                let b1_content = Vec2::new(180.0, 320.0);
                b.custom(b1, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let mut sa = b.scroll_area(
                        b1,
                        b1_content,
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut state.scroll_vert,
                        ManualLayout,
                        input,
                    );
                    let code_lines = [
                        "fn frame(ctx: &mut Ctx) {",
                        "  ctx.window(\"Inspector\", |w| {",
                        "    w.label(\"position\");",
                        "    w.drag(\"x\", &mut pos.x);",
                        "    w.drag(\"y\", &mut pos.y);",
                        "    w.separator();",
                        "    w.label(\"size\");",
                        "    w.drag(\"w\", &mut size.w);",
                        "    w.drag(\"h\", &mut size.h);",
                        "    w.slider(\"alpha\", &mut a, 0..1);",
                        "  });",
                        "}",
                    ];
                    for (i, line) in code_lines.iter().enumerate() {
                        sa.label_styled(
                            Rect::new(6.0, i as f32 * 18.0 + 6.0, 160.0, 14.0),
                            line,
                            t.text_sm,
                            t.muted,
                            false,
                        );
                    }
                    let sa_cmds = sa.finish();
                    b.append_cmds(sa_cmds);
                }
                b.label_styled(
                    Rect::new(b1.x, y + b1.h + 4.0, b1.w, cap_h),
                    "vertical · idle",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Box 2: vertical, dragging (same implementation, user can drag)
                let b2_x = b1.x + b1.w + box_gap;
                let b2 = Rect::new(b2_x, y, 180.0, 130.0);
                let b2_content = Vec2::new(180.0, 300.0);
                b.custom(b2, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let mut sa = b.scroll_area(
                        b2,
                        b2_content,
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut state.scroll_horiz,
                        ManualLayout,
                        input,
                    );
                    for i in 0..15 {
                        sa.label_styled(
                            Rect::new(6.0, i as f32 * 18.0 + 6.0, 160.0, 14.0),
                            &format!("// entry {:02}/24 — frame state", i + 1),
                            t.text_sm,
                            t.muted,
                            false,
                        );
                    }
                    let sa_cmds = sa.finish();
                    b.append_cmds(sa_cmds);
                }
                b.label_styled(
                    Rect::new(b2.x, y + b2.h + 4.0, b2.w, cap_h),
                    "vertical · dragging (rust)",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Box 3: horizontal
                let b3_x = b2_x + b2.w + box_gap;
                let b3 = Rect::new(b3_x, y + 15.0, 300.0, 100.0);
                let b3_content = Vec2::new(700.0, 100.0);
                b.custom(b3, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let mut sa = b.scroll_area(
                        b3,
                        b3_content,
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut state.scroll_both,
                        ManualLayout,
                        input,
                    );
                    sa.label_styled(
                        Rect::new(6.0, 6.0, 680.0, 14.0),
                        "frame.draw_rect( … )  frame.draw_text( \"hello, framewise\" )  frame.draw_image( logo )  frame.layout.push( Row )",
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    let sa_cmds = sa.finish();
                    b.append_cmds(sa_cmds);
                }
                b.label_styled(
                    Rect::new(b3.x, y + b3.h + 15.0 + 4.0, b3.w, cap_h),
                    "horizontal",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Box 4: both axes
                let b4_x = b3_x + b3.w + box_gap;
                let b4 = Rect::new(b4_x, y, 220.0, 130.0);
                let b4_content = Vec2::new(320.0, 240.0);
                b.custom(b4, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let mut sa = b.scroll_area(
                        b4,
                        b4_content,
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::Always,
                        &mut state.scroll_both_axes,
                        ManualLayout,
                        input,
                    );
                    sa.label_styled(
                        Rect::new(12.0, 10.0, 280.0, 14.0),
                        "scroll surface with",
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    sa.label_styled(
                        Rect::new(12.0, 28.0, 280.0, 14.0),
                        "both bars + corner",
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    let sa_cmds = sa.finish();
                    b.append_cmds(sa_cmds);
                }
                b.label_styled(
                    Rect::new(b4.x, y + b4.h + 4.0, b4.w, cap_h),
                    "both axes",
                    t.text_sm,
                    t.muted,
                    false,
                );

                y += 140.0 + cap_h + 8.0;
            }
            y += SEC_GAP;

            // ── 07 · TABS ────────────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "07", "Tabs");
            y += 46.0;
            {
                const TABS1: &[&str] = &["Inspector", "Layout", "Timing", "Logs", "Replay"];
                let t1_info = b.tabs(
                    std::mem::take(&mut state.tabs1_state),
                    TABS1,
                    Rect::new(lx, y, content_w.min(640.0), 36.0),
                    input,
                );
                state.tabs1_state = t1_info.state;
                y += 36.0 + 20.0;

                const TABS2: &[&str] = &["frame.rs", "layout.rs", "theme.rs", "state.rs"];
                let t2_info = b.tabs(
                    std::mem::take(&mut state.tabs2_state),
                    TABS2,
                    Rect::new(lx, y, content_w.min(480.0), 36.0),
                    input,
                );
                state.tabs2_state = t2_info.state;
                y += 36.0;
            }
            y += SEC_GAP;

            // ── 08 · PROGRESS · METERS · STATUS ──────────────────────────────────────
            sec_y(
                &mut b,
                &t,
                lx,
                y,
                content_w,
                "08",
                "Progress, meters & status",
            );
            y += 46.0;

            group_y(&mut b, &t, lx, y, "progress");
            y += 20.0;
            {
                let bar_items: &[(f32, bool, &str)] = &[
                    (0.12, false, "12% · compiling"),
                    (0.68, false, "68% · linking"),
                    (0.94, true, "94% · uploading textures"),
                    (f32::NAN, true, "indeterminate"),
                ];
                let bar_w = 240.0_f32;
                for (val, active, label) in bar_items {
                    b.add(
                        Rect::new(lx, y + 8.0, bar_w, 3.0),
                        progress_bar::<SampleTextSystem, framewise::layout::ManualState>,
                        ProgressBarSpecBuilder::new(*val)
                            .phase((time as f32) * 0.5)
                            .active(*active),
                    );
                    b.label_styled(
                        Rect::new(lx + bar_w + 12.0, y + 2.0, 180.0, 14.0),
                        label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    y += 22.0;
                }
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "meters");
            y += 20.0;
            {
                let meters: &[(&str, f32, Option<f32>)] = &[
                    ("CPU", 0.6, None),
                    ("GPU", 0.8, Some(0.9)),
                    ("FRAME", 1.0, None),
                ];
                let mut bx = lx;
                for (label, val, peak) in meters {
                    b.label_styled(
                        Rect::new(bx, y, 36.0, 14.0),
                        label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    bx += 40.0;
                    if *label == "FRAME" {
                        b.label_styled(
                            Rect::new(bx, y - 1.0, 60.0, 16.0),
                            "2.4 ms",
                            t.text_sm,
                            t.ink,
                            false,
                        );
                        bx += 70.0;
                    } else {
                        b.add(
                            Rect::new(bx, y, 100.0, 12.0),
                            meter::<SampleTextSystem, framewise::layout::ManualState>,
                            framewise::widgets::MeterSpecBuilder::new()
                                .value(*val)
                                .peak(*peak)
                                .bars(10),
                        );
                        bx += 108.0;
                    }
                }
            }
            y += 14.0 + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "spinners  ·  status");
            y += 20.0;
            {
                b.add(
                    Rect::new(lx, y, 16.0, 16.0),
                    spinner::<SampleTextSystem, framewise::layout::ManualState>,
                    SpinnerSpecBuilder::new(),
                );
                b.label_styled(
                    Rect::new(lx + 20.0, y + 1.0, 60.0, 14.0),
                    "loading",
                    t.text_sm,
                    t.muted,
                    false,
                );

                b.add(
                    Rect::new(lx + 90.0, y - 4.0, 24.0, 24.0),
                    spinner::<SampleTextSystem, framewise::layout::ManualState>,
                    SpinnerSpecBuilder::new().large(true),
                );
                b.label_styled(
                    Rect::new(lx + 118.0, y + 1.0, 50.0, 14.0),
                    "large",
                    t.text_sm,
                    t.muted,
                    false,
                );

                let status_items: &[(&str, StatusVariant)] = &[
                    ("IDLE", StatusVariant::Neutral),
                    ("READY", StatusVariant::Ok),
                    ("FRAME DROP", StatusVariant::Warn),
                    ("PANIC", StatusVariant::Err),
                    ("RENDERING", StatusVariant::Live),
                ];
                let mut sx = lx + 180.0;
                for (label, variant) in status_items {
                    b.add(
                        Rect::new(sx, y + 1.0, 120.0, 12.0),
                        status::<SampleTextSystem, framewise::layout::ManualState>,
                        framewise::widgets::StatusSpecBuilder::new()
                            .label(label)
                            .variant(*variant),
                    );
                    sx += 110.0;
                }
            }
            y += 16.0 + SEC_GAP;

            // ── 09 · TREE / LIST ─────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "09", "Tree & list");
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
                b.add(
                    Rect::new(lx, y, 320.0, 0.0),
                    tree::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::TreeSpecBuilder::new().rows(WIDGET_TREE),
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
                b.add(
                    Rect::new(lx + 360.0, y, 240.0, 0.0),
                    tree::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::TreeSpecBuilder::new().rows(FILE_LIST),
                );

                y += WIDGET_TREE.len().max(FILE_LIST.len()) as f32 * 20.0 + 12.0;
            }
            y += SEC_GAP;

            // ── 10 · TOOLTIPS · KEYCAPS ──────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "10", "Tooltips & keycaps");
            y += 46.0;

            group_y(&mut b, &t, lx, y, "tooltips");
            y += 20.0;
            {
                b.add(
                    Rect::new(lx, y, 0.0, 0.0),
                    tooltip::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::TooltipSpecBuilder::new()
                        .text("Drag to scrub — hold ⌥ for fine.")
                        .variant(TooltipVariant::Dark),
                );
                y += 28.0 + 8.0;

                b.add(Rect::new(lx, y, 0.0, 0.0), tooltip::<SampleTextSystem, framewise::layout::ManualState>, framewise::widgets::TooltipSpecBuilder::new().text("Re-described every frame from current application state. No retained nodes.").variant(TooltipVariant::Dark));
                y += 28.0 + 8.0;

                b.add(
                    Rect::new(lx, y, 0.0, 0.0),
                    tooltip::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::TooltipSpecBuilder::new()
                        .text("⚠ shader recompiled this frame (12 ms)")
                        .variant(TooltipVariant::Rust),
                );
                y += 28.0;
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "keycaps");
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
                        b.add(
                            Rect::new(kx, y, kw, 22.0),
                            keycap::<SampleTextSystem, framewise::layout::ManualState>,
                            framewise::widgets::KeycapSpecBuilder::new()
                                .label(key)
                                .bg(t.paper_elev)
                                .border(t.line)
                                .text_color(t.ink)
                                .text_size(t.text_sm),
                        );
                        kx += kw + 4.0;
                    }
                    b.label_styled(
                        Rect::new(kx + 4.0, y + 3.0, 200.0, 14.0),
                        desc,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    y += 28.0;
                }
            }
            y += SEC_GAP;

            // ── 11 · WINDOW CHROME ───────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "11", "Window & panel chrome");
            y += 46.0;
            {
                // Light window: Inspector with content
                let win_buttons = [
                    WindowButton { symbol: "−" },
                    WindowButton { symbol: "▢" },
                    WindowButton { symbol: "×" },
                ];
                let win_rect = Rect::new(lx, y, 360.0, 280.0);
                let mut win = b.window(
                    win_rect,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Inspector")
                        .buttons(&win_buttons)
                        .status_bar(true)
                        .status_text("RENDERING  frame #00248  2.4 ms"),
                    ManualLayout,
                );

                // Inner content: drag numbers + checkboxes
                let mut iy = 0.0;
                let mut drx = 0.0;
                let cr_w = win_rect.w - 32.0;
                for (i, (label, min, max)) in [("X", 0.0_f32, 800.0_f32), ("Y", 0.0, 600.0)].iter().enumerate() {
                    let info = win.drag_number(std::mem::take(&mut state.win11_drags[i]), label, *min, *max, Rect::new(drx, iy, (cr_w / 2.0) - 4.0, t.h_md), input);
                    state.win11_drags[i] = info.state;
                    drx += (cr_w / 2.0) + 4.0;
                }
                iy += t.h_md + 6.0;
                drx = 0.0;
                for (i, (label, min, max)) in [("W", 0.0_f32, 800.0_f32), ("H", 0.0, 600.0)].iter().enumerate() {
                    let info = win.drag_number(std::mem::take(&mut state.win11_drags[2 + i]), label, *min, *max, Rect::new(drx, iy, (cr_w / 2.0) - 4.0, t.h_md), input);
                    state.win11_drags[2 + i] = info.state;
                    drx += (cr_w / 2.0) + 4.0;
                }
                iy += t.h_md + 10.0;
                win.divider(Rect::new(0.0, iy, cr_w, 1.0));
                iy += 10.0;
                let check_labels = ["clip to parent", "debug overlay"];
                for (i, label) in check_labels.iter().enumerate() {
                    let cb_info = win.checkbox(
                        std::mem::take(&mut state.win11_cbs[i]),
                        Rect::new(0.0, iy, 14.0, 14.0),
                        input,
                    );
                    state.win11_cbs[i] = cb_info.state;
                    win.label_styled(
                        Rect::new(18.0, iy, cr_w - 18.0, 14.0),
                        label,
                        t.text_md,
                        t.ink,
                        false,
                    );
                    iy += 22.0;
                }
                let cmds = win.finish();
                b.append_cmds(cmds);

                // Dark variant window (drawn with DrawCmds)
                let dw = Rect::new(lx + 388.0, y, 300.0, 240.0);
                let dark_bg = Color::from_srgb_u8(26, 24, 20, 255);
                let darker = Color::from_srgb_u8(12, 11, 9, 255);
                let dark_bdr = Color::from_srgb_u8(58, 53, 45, 255);
                let light = t.paper;
                let muted_l = t.muted;

                b.custom(dw, |rect| {
                    vec![
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
                    ]
                });
                b.label_styled(
                    Rect::new(dw.x + 10.0, y + 6.0, 180.0, 14.0),
                    "FRAMEWISE · DARK",
                    t.text_sm,
                    light,
                    false,
                );
                b.label_styled(
                    Rect::new(dw.x + dw.w - 28.0, y + 6.0, 20.0, 14.0),
                    "✕",
                    t.text_sm,
                    light,
                    false,
                );

                let cx = dw.x + 16.0;
                let cyw = y + 26.0 + 16.0;
                // keycap row
                b.custom(Rect::new(cx, cyw, 50.0, 22.0), |rect| {
                    vec![
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
                    ]
                });
                b.label_styled(
                    Rect::new(cx + 7.0, cyw + 5.0, 12.0, 12.0),
                    "⌘",
                    t.text_sm,
                    light,
                    false,
                );
                b.label_styled(
                    Rect::new(cx + 35.0, cyw + 5.0, 12.0, 12.0),
                    "K",
                    t.text_sm,
                    light,
                    false,
                );
                b.label_styled(
                    Rect::new(cx + 56.0, cyw + 5.0, 140.0, 12.0),
                    "search everything",
                    t.text_sm,
                    muted_l,
                    false,
                );

                // fake dark input
                let inp_y = cyw + 28.0;
                b.custom(Rect::new(cx, inp_y, dw.w - 32.0, 26.0), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: darker,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: dark_bdr,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(cx + 8.0, inp_y + 7.0, dw.w - 48.0, 12.0),
                    "type a command…",
                    t.text_sm,
                    muted_l,
                    false,
                );

                // fake dark tabs
                let tab_y = inp_y + 30.0;
                b.custom(Rect::new(cx, tab_y + 26.0, dw.w - 16.0, 1.0), |rect| {
                    vec![DrawCmd::StrokeLine {
                        p0: Vec2::new(rect.x, rect.y),
                        p1: Vec2::new(rect.x + rect.w, rect.y),
                        color: dark_bdr,
                        width: 1.0,
                    }]
                });
                let tab_items = ["Files", "Symbols", "Frames"];
                let mut tab_x = cx;
                for (i, item) in tab_items.iter().enumerate() {
                    b.label_styled(
                        Rect::new(tab_x, tab_y + 5.0, 60.0, 14.0),
                        item,
                        t.text_sm,
                        if i == 0 { light } else { muted_l },
                        false,
                    );
                    if i == 0 {
                        b.custom(Rect::new(tab_x, tab_y + 24.0, 40.0, 2.0), |rect| {
                            vec![DrawCmd::FillRect {
                                rect,
                                color: t.rust,
                            }]
                        });
                    }
                    tab_x += 60.0;
                }
                let file_y = tab_y + 32.0;
                for (i, file) in ["▸ renderer.rs", "▸ layout.rs", "▸ widget/button.rs"]
                    .iter()
                    .enumerate()
                {
                    b.label_styled(
                        Rect::new(cx, file_y + i as f32 * 18.0, 200.0, 14.0),
                        file,
                        t.text_sm,
                        muted_l,
                        false,
                    );
                }

                y += 280.0 + SEC_GAP;
            }

            // ── 12 · IN USE ──────────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "12", "In use");
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
                let mut win = b.window(
                    wr,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Renderer Settings")
                        .buttons(&win_buttons)
                        .status_bar(true)
                        .status_text("RENDERING  frame #00248  2.4 ms  Vulkan 1.3 · 4× msaa"),
                    ManualLayout,
                );
                let cr_w = win_w_left - 32.0;

                // Tabs inside window
                let tabs_items = ["General", "Frame", "Output", "Debug"];
                let tabs_info = win.tabs(
                    std::mem::take(&mut state.iu_tabs),
                    &tabs_items,
                    Rect::new(0.0, 0.0, cr_w, 28.0),
                    input,
                );
                state.iu_tabs = tabs_info.state;

                // Form rows
                let form_y_start = 38.0;
                let label_w = 84.0_f32;
                let widget_x = label_w + 8.0;
                let widget_w = cr_w - label_w - 8.0;
                let row_h = 28.0_f32;
                let row_gap = 8.0_f32;
                let mut fy = form_y_start;

                // backend (segmented)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "BACKEND",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let backends = ["OpenGL", "Vulkan", "Metal", "wgpu"];
                let backend_info = win.segmented(
                    std::mem::take(&mut state.iu_backend),
                    &backends,
                    Rect::new(widget_x, fy, 0.0, row_h),
                    input,
                );
                state.iu_backend = backend_info.state;
                fy += row_h + row_gap;

                // target fps (slider)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "TARGET FPS",
                    t.text_sm,
                    t.muted,
                    false,
                );
                win.slider(
                    &mut state.iu_fps_slider,
                    &mut state.iu_fps_val,
                    24.0,
                    240.0,
                    10.0,
                    SliderOrientation::Horizontal,
                    Rect::new(widget_x, fy, widget_w - 40.0, row_h),
                    input,
                );
                win.label_styled(
                    Rect::new(widget_x + widget_w - 34.0, fy + 7.0, 34.0, 14.0),
                    &format!("{:.0}", state.iu_fps_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                fy += row_h + row_gap;

                // vsync (switch)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "VSYNC",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let switch_res = win.switch(
                    std::mem::take(&mut state.iu_vsync),
                    Rect::new(widget_x, fy + 6.0, 30.0, 16.0),
                    input,
                );
                state.iu_vsync = switch_res.state;
                win.label_styled(
                    Rect::new(widget_x + 36.0, fy + 7.0, 120.0, 14.0),
                    "match display",
                    t.text_sm,
                    t.ink,
                    false,
                );
                fy += row_h + row_gap;

                // msaa (segmented)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "MSAA",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let msaa_opts = ["off", "2×", "4×", "8×"];
                let seg_res = win.segmented(
                    std::mem::take(&mut state.iu_msaa),
                    &msaa_opts,
                    Rect::new(widget_x, fy, 0.0, row_h),
                    input,
                );
                state.iu_msaa = seg_res.state;
                fy += row_h + row_gap;

                // viewport (drag numbers)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "VIEWPORT",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let w_res = win.drag_number(
                    std::mem::take(&mut state.iu_vp_w),
                    "W",
                    0.0,
                    7680.0,
                    Rect::new(widget_x, fy, (widget_w / 2.0) - 4.0, row_h),
                    input,
                );
                state.iu_vp_w = w_res.state;

                let h_res = win.drag_number(
                    std::mem::take(&mut state.iu_vp_h),
                    "H",
                    0.0,
                    7680.0,
                    Rect::new(widget_x + (widget_w / 2.0) + 4.0, fy, (widget_w / 2.0) - 4.0, row_h),
                    input,
                );
                state.iu_vp_h = h_res.state;
                fy += row_h + row_gap;

                // accent (color swatch + button)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "ACCENT",
                    t.text_sm,
                    t.muted,
                    false,
                );
                win.add(
                    Rect::new(widget_x, fy + 4.0, 18.0, 20.0),
                    color_swatch::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::ColorSwatchSpecBuilder::new()
                        .color(t.rust)
                        .border(t.line),
                );
                win.label_styled(
                    Rect::new(widget_x + 22.0, fy + 7.0, 60.0, 14.0),
                    "#c25a2c",
                    t.text_sm,
                    t.ink,
                    false,
                );
                fy += row_h + row_gap;

                // options (checkboxes)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "OPTIONS",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let opt_labels = ["show layout grid", "log every frame", "tessellate (per-mesh)"];
                for (i, label) in opt_labels.iter().enumerate() {
                    let opt_y = fy + i as f32 * 22.0;
                    let cb_res = win.checkbox(
                        std::mem::take(&mut state.iu_options[i]),
                        Rect::new(widget_x, opt_y + 4.0, 14.0, 14.0),
                        input,
                    );
                    state.iu_options[i] = cb_res.state;

                    win.label_styled(
                        Rect::new(widget_x + 18.0, opt_y + 4.0, widget_w - 18.0, 14.0),
                        label,
                        t.text_md,
                        t.ink,
                        false,
                    );
                }
                fy += 3.0 * 22.0 + 4.0;

                win.divider(Rect::new(0.0, fy, cr_w, 1.0));
                fy += 10.0;

                // button row
                let mut btn_x = cr_w;
                let btns: &[(&str, ButtonStyle)] = &[
                    ("Apply", ButtonStyle::primary()),
                    ("Cancel", ButtonStyle::default()),
                    ("Reset", ButtonStyle::ghost()),
                ];
                for (i, (label, style)) in btns.iter().enumerate() {
                    let bw = label.len() as f32 * 7.0 + 20.0;
                    btn_x -= bw;
                    let btn = win.button_styled(
                        std::mem::take(&mut state.iu_btns[i]),
                        Rect::new(btn_x, fy, bw, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.iu_btns[i] = btn.state;
                    btn_x -= 8.0;
                }
                let cmds = win.finish();
                b.append_cmds(cmds);

                // Right column
                let rcol_x = lx + win_w_left + 24.0;
                let rcol_w = (content_w - win_w_left - 24.0).max(0.0);

                // Frame Log window
                let fl_h = 310.0_f32;
                let fl_buttons = [
                    WindowButton { symbol: "⌕" },
                    WindowButton { symbol: "⏸" },
                    WindowButton { symbol: "×" },
                ];
                let fl_rect = Rect::new(rcol_x, y, rcol_w, fl_h);
                let mut fl_win = b.window(
                    fl_rect,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Frame Log")
                        .buttons(&fl_buttons)
                        .status_bar(true)
                        .status_text("RECORDING  248 frames  2.6 ms avg"),
                    ManualLayout,
                );
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
                    let mut log_page = fl_win.scroll_area(
                        fl_scroll_rect,
                        Vec2::new(fl_scroll_rect.w, log_content_h),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Auto,
                        &mut state.iu_log_scroll,
                        framewise::layout::ManualLayout,
                        input,
                    );
                    let loy = 4.0;
                    for (i, (ts_str, msg, highlight)) in log_lines.iter().enumerate() {
                        let row_y = loy + i as f32 * 18.0;
                        let ts_w = 100.0_f32;
                        log_page.label_styled(
                            Rect::new(6.0, row_y, ts_w, 14.0),
                            ts_str,
                            t.text_sm,
                            t.muted,
                            false,
                        );
                        let msg_color = if *highlight { t.rust } else { t.ink };
                        log_page.label_styled(
                            Rect::new(
                                6.0 + ts_w + 8.0,
                                row_y,
                                fl_scroll_rect.w - ts_w - 14.0,
                                14.0,
                            ),
                            msg,
                            t.text_sm,
                            msg_color,
                            false,
                        );
                    }
                    let log_cmds = log_page.finish();
                    fl_win.append_cmds(log_cmds);
                }
                let cmds = fl_win.finish();
                b.append_cmds(cmds);

                // Quick Actions window
                let qa_y = y + fl_h + 16.0;
                let qa_buttons = [WindowButton { symbol: "×" }];
                let qa_rect = Rect::new(rcol_x, qa_y, rcol_w, 174.0);
                let mut qa_win = b.window(
                    qa_rect,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Quick actions")
                        .buttons(&qa_buttons)
                        .status_bar(false)
                        .status_text(""),
                    ManualLayout,
                );
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
                qa_win.add(
                    Rect::new(0.0, -8.0, qa_cr_w, 0.0),
                    menu::<SampleTextSystem, framewise::layout::ManualState>,
                    framewise::widgets::MenuSpecBuilder::new().items(&qa_items),
                );
                let cmds = qa_win.finish();
                b.append_cmds(cmds);

                y += win_h_full;
            }
            y += SEC_GAP;

            // ── FOOTER ───────────────────────────────────────────────────────────────
            {
                b.divider(Rect::new(lx, y, content_w, 1.0));
                y += 10.0;
                let foot_items: &[(&str, &str)] = &[
                    ("SPEC", "V0.1 · 12 SECTIONS"),
                    ("RADIUS", "0 PX"),
                    ("BORDERS", "1 PX INK"),
                    ("FOCUS", "2 PX RUST OUTSET"),
                    ("DENSITY", "28 PX ROW · 14 PX LABEL · 12 PX MONO"),
                ];
                let mut fx = lx;
                for (key, val) in foot_items {
                    b.label_styled(Rect::new(fx, y, 32.0, 14.0), key, t.text_sm, t.ink, false);
                    let kw = key.len() as f32 * 7.0 + 8.0;
                    b.label_styled(
                        Rect::new(fx + kw, y, 220.0, 14.0),
                        val,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    fx += kw + val.len() as f32 * 6.5 + 24.0;
                }
                b.label_styled(
                    Rect::new(lx + content_w - 200.0, y, 200.0, 14.0),
                    "FRAMEWISE · WIDGET SPECIFICATION",
                    t.text_sm,
                    t.ink,
                    false,
                );
            }
            let _ = (y, b);
        } // end content block (drops `b` alias, releases borrow on `page`)
        page.finish()
    }; // end page_cmds block
    b.append_cmds(page_cmds);
    b.finish()
}

fn hero_logo(t: &Theme, lx: f32, y0: f32) -> Vec<DrawCmd> {
    let mut cmds = vec![];

    // Logo (Framewise mark), scaled from 200×200 viewBox → 96×96 px
    let ls = 0.48_f32;
    let lx0 = lx;
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

// ── Helpers ───────────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn opts_dropdown_h(n: usize) -> f32 {
    n as f32 * 26.0 + 8.0
}
