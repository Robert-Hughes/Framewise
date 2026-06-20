use framewise::{
    draw::DrawCommands,
    focus::FocusSystem,
    layout::{Layout, LayoutState},
    layouts::linear::ColumnState,
    widgets::label::{
        raw::{LabelCalcSizeRequestSpec, LabelSpec as RawLabelSpec},
        LabelSpecBuilder, LabelStyle,
    },
    ColumnLayoutParams, LayoutViolationPolicy, Rect, TextBackend, WidgetContext,
};

#[derive(Default)]
pub struct DemoPageState {
    #[cfg(feature = "scroll_area")]
    pub scroll: framewise::widgets::scroll_area::ScrollState,
}

pub struct DemoPageResult<'b, T: TextBackend, LS: LayoutState, CF> {
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

pub struct DemoPageNoScrollResult<'b, T: TextBackend, LS: LayoutState, CF> {
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

#[cfg(feature = "scroll_area")]
#[allow(clippy::type_complexity)]
pub fn begin_demo_page<'a, 'b, T: TextBackend, L: Layout, CF>(
    parent_ctx: &'b mut WidgetContext<'a, T, ColumnState, CF>,
    title: &str,
    state: &'b mut DemoPageState,
    debug_layout: bool,
    inner_layout: L,
) -> DemoPageResult<
    'b,
    T,
    framewise::layouts::OffsetState<L::State>,
    impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect) + 'b,
> {
    use framewise::types::Vec2;

    let pad = 20.0;
    let clip = parent_ctx.clip_rect;
    let spec = framewise::widgets::scroll_area::ScrollAreaSpecBuilder::new()
        .vertical(framewise::widgets::scroll_area::ScrollAxis {
            extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
            vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
        })
        .defaults_from_theme(&parent_ctx.theme)
        .build();

    let calc_spec = framewise::widgets::scroll_area::raw::ScrollAreaCalcSizeRequestSpec {};
    let size_request =
        framewise::widgets::scroll_area::raw::calc_scroll_area_intrinsic_size(&calc_spec);
    let bounds = parent_ctx.layout(ColumnLayoutParams::auto().fill_x().fill_y(), size_request);
    let input = parent_ctx.input;
    let raw_spec = framewise::widgets::scroll_area::raw::ScrollAreaSpec {
        rect: bounds,
        horizontal: spec.horizontal,
        vertical: spec.vertical,
        clip_rect: clip,
        time: parent_ctx.time,
        scrollbar_width: spec.scrollbar_width,
        scrollbar_style: spec.scrollbar_style,
        layer: parent_ctx.layer,
        keyboard_focusable: true,
    };

    let framewise::widgets::scroll_area::raw::ScrollAreaResult {
        token,
        content_bounds,
        offset,
        inner_space,
    } = framewise::widgets::scroll_area::raw::begin_scroll_area(
        raw_spec,
        &mut state.scroll,
        parent_ctx.input,
        parent_ctx.focus_system,
        parent_ctx.cmds,
    );

    let theme = parent_ctx.theme;
    let title_style = LabelStyle {
        text_style: theme.heading_text_style(24.0),
        text_color: theme.ink,
        rule: true,
        rule_color: theme.line,
        content_placement: framewise::TextContentPlacement::TOP_LEFT,
    };

    let label_spec = LabelSpecBuilder::new().text(title).style(title_style);
    let label_spec = label_spec.build();
    let label_spec = LabelCalcSizeRequestSpec {
        text: label_spec.text,
        style: label_spec.style,
    };
    let label_request = framewise::widgets::label::raw::calc_label_request_size(
        &label_spec,
        parent_ctx.text_backend,
    );
    let title_h = label_request.preferred.map_or(24.0, |p| p.y);

    // Draw the title using the offset coordinates of the scroll area, inset by pad
    let title_rect = Rect::new(
        content_bounds.x + pad - offset.x,
        content_bounds.y + pad - offset.y,
        (content_bounds.w - 2.0 * pad).max(0.0),
        title_h,
    );
    let spec = RawLabelSpec {
        rect: title_rect,
        text: title,
        style: title_style,
        layer: parent_ctx.layer,
    };
    framewise::widgets::label::raw::label(spec, parent_ctx.text_backend, parent_ctx.cmds);

    let offset_y = pad + title_h + 24.0;
    let mut adjusted_space = inner_space;
    adjusted_space.x += pad;
    adjusted_space.width = match adjusted_space.width {
        framewise::layout::AxisBound::Exact(w) => {
            framewise::layout::AxisBound::Exact((w - 2.0 * pad).max(0.0))
        }
        framewise::layout::AxisBound::AtMost(w) => {
            framewise::layout::AxisBound::AtMost((w - 2.0 * pad).max(0.0))
        }
        framewise::layout::AxisBound::Unbounded => framewise::layout::AxisBound::Unbounded,
    };
    adjusted_space.y += offset_y;
    adjusted_space.height = match adjusted_space.height {
        framewise::layout::AxisBound::Exact(h) => {
            framewise::layout::AxisBound::Exact((h - offset_y).max(0.0))
        }
        framewise::layout::AxisBound::AtMost(h) => {
            framewise::layout::AxisBound::AtMost((h - offset_y).max(0.0))
        }
        framewise::layout::AxisBound::Unbounded => framewise::layout::AxisBound::Unbounded,
    };

    let offset_layout = framewise::layouts::OffsetLayout {
        offset,
        inner: inner_layout,
    };

    let new_clip = Some(clip.map_or(content_bounds, |pc| pc.intersect(&content_bounds)));

    let on_finish = move |focus_system: &mut FocusSystem,
                          _text_backend: &mut T,
                          cmds: &mut DrawCommands,
                          resolved_space: Rect| {
        let full_resolved_space = Rect::new(
            resolved_space.x - pad,
            resolved_space.y - offset_y,
            resolved_space.w + 2.0 * pad,
            resolved_space.h + offset_y + pad, // bottom padding of `pad` (20.0px)
        );
        let content_extent = Vec2::new(full_resolved_space.w, full_resolved_space.h);
        framewise::widgets::scroll_area::raw::end_scroll_area(
            token,
            content_extent,
            &mut state.scroll,
            input,
            focus_system,
            cmds,
        );
    };

    let mut child_ctx = parent_ctx.child_with_layout_and_on_finish_and_clip_rect(
        offset_layout.begin(adjusted_space),
        on_finish,
        new_clip,
    );
    child_ctx.debug_layout = debug_layout;
    child_ctx.layout_policy = LayoutViolationPolicy::Highlight;

    DemoPageResult { ctx: child_ctx }
}

#[cfg(not(feature = "scroll_area"))]
#[allow(clippy::type_complexity)]
pub fn begin_demo_page<'a, 'b, T: TextBackend, L: Layout, CF>(
    parent_ctx: &'b mut WidgetContext<'a, T, ColumnState, CF>,
    title: &str,
    _state: &'b mut DemoPageState,
    debug_layout: bool,
    inner_layout: L,
) -> DemoPageResult<
    'b,
    T,
    framewise::layouts::OffsetState<L::State>,
    impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect) + 'b,
> {
    let DemoPageNoScrollResult { ctx } =
        begin_demo_page_no_scroll(parent_ctx, title, debug_layout, false, inner_layout);
    DemoPageResult { ctx }
}

#[allow(clippy::type_complexity)]
pub fn begin_demo_page_no_scroll<'a, 'b, T: TextBackend, L: Layout, CF>(
    parent_ctx: &'b mut WidgetContext<'a, T, ColumnState, CF>,
    title: &str,
    debug_layout: bool,
    unbounded_height: bool,
    inner_layout: L,
) -> DemoPageNoScrollResult<
    'b,
    T,
    framewise::layouts::OffsetState<L::State>,
    impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect) + 'b,
> {
    use framewise::layouts::OffsetLayout;
    use framewise::types::Vec2;

    let theme = parent_ctx.theme;
    let title_style = LabelStyle {
        text_style: theme.heading_text_style(24.0),
        text_color: theme.ink,
        rule: true,
        rule_color: theme.line,
        content_placement: framewise::TextContentPlacement::TOP_LEFT,
    };

    // 1. Draw the title inside the parent column
    framewise::widgets::label::label(
        parent_ctx,
        LabelSpecBuilder::new().text(title).style(title_style),
        ColumnLayoutParams::auto().fill_x(),
    );

    // 2. Add spacer
    parent_ctx.spacer(24.0);

    // 3. Create the body child context (using OffsetLayout with zero offset)
    let offset_layout = OffsetLayout {
        offset: Vec2::ZERO,
        inner: inner_layout,
    };

    let layout_params = if unbounded_height {
        ColumnLayoutParams::auto().fill_x()
    } else {
        ColumnLayoutParams::auto().fill_x().fill_y()
    };

    let mut body_ctx = parent_ctx.child_with_layout(layout_params, offset_layout);
    body_ctx.debug_layout = debug_layout;
    body_ctx.layout_policy = LayoutViolationPolicy::Highlight;

    DemoPageNoScrollResult { ctx: body_ctx }
}
