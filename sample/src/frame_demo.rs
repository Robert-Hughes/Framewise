use crate::text::SampleTextBackend;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::Align,
    layouts::linear::{ColumnLayout, ColumnLayoutParams, RowLayout, RowLayoutParams},
    theme::Theme,
    types::Rect,
    widget::WidgetContext,
    widgets::button::{button, ButtonSpecBuilder, ButtonState, ButtonStyle},
    widgets::frame::{begin_frame, FrameResult, FrameSpecBuilder, FrameStyle},
    widgets::label::{label, LabelSpecBuilder},
};

// ── State ──────────────────────────────────────────────────────────────────────

pub struct FrameDemoState {
    pub page: crate::demo_page::DemoPageState,
    // Dynamic List Controls & State
    pub add_btn: ButtonState,
    pub remove_btn: ButtonState,
    pub dynamic_clicks: [u32; 5],
    pub dynamic_btns: [ButtonState; 5],
    pub item_count: usize,

    // Sizing Showcase State
    pub fixed_btn: ButtonState,
    pub width_auto_btn: ButtonState,
    pub height_auto_btn: ButtonState,
    pub height_auto_btn2: ButtonState,
    pub fully_auto_btn: ButtonState,
    pub fixed_clicks: u32,
    pub width_auto_clicks: u32,
    pub height_auto_clicks: u32,
    pub fully_auto_clicks: u32,

    // Nesting & Alignment State
    pub outer_btn: ButtonState,
    pub inner_center_btn: ButtonState,
    pub inner_end_btn: ButtonState,
    pub nested_clicks: [u32; 3],

    // Cross-axis alignment showcase buttons
    pub align_small_btn: ButtonState,
    pub align_med_btn: ButtonState,
    pub align_large_btn: ButtonState,
}

impl Default for FrameDemoState {
    fn default() -> Self {
        Self {
            page: Default::default(),
            add_btn: ButtonState::default(),
            remove_btn: ButtonState::default(),
            dynamic_clicks: [0; 5],
            dynamic_btns: [
                ButtonState::default(),
                ButtonState::default(),
                ButtonState::default(),
                ButtonState::default(),
                ButtonState::default(),
            ],
            item_count: 2, // start with 2 items

            fixed_btn: ButtonState::default(),
            width_auto_btn: ButtonState::default(),
            height_auto_btn: ButtonState::default(),
            height_auto_btn2: ButtonState::default(),
            fully_auto_btn: ButtonState::default(),
            fixed_clicks: 0,
            width_auto_clicks: 0,
            height_auto_clicks: 0,
            fully_auto_clicks: 0,

            outer_btn: ButtonState::default(),
            inner_center_btn: ButtonState::default(),
            inner_end_btn: ButtonState::default(),
            nested_clicks: [0; 3],

            align_small_btn: ButtonState::default(),
            align_med_btn: ButtonState::default(),
            align_large_btn: ButtonState::default(),
        }
    }
}

// ── Draw ──────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_frame_page(
    state: &mut FrameDemoState,
    focus_system: &mut FocusSystem,
    input: &Input,
    output: &mut framewise::Output,
    _time: f64,
    win_size: (f32, f32),
    physical_pixels_per_logical_pixel: f32,
    text_backend: &mut SampleTextBackend,
    debug_layout: bool,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let is_unbounded = win_h.is_infinite();

    let mut cmds = framewise::DrawCommands::new(physical_pixels_per_logical_pixel);
    let space = if is_unbounded {
        framewise::LayoutSpace::unbounded_height(0.0, 0.0, win_w)
    } else {
        Rect::new(0.0, 0.0, win_w, win_h).into()
    };

    let mut ctx = WidgetContext::root(
        Theme::default(),
        text_backend,
        focus_system,
        input,
        output,
        ColumnLayout,
        space,
        &mut cmds,
    );

    if is_unbounded {
        let mut outer = crate::demo_page::begin_demo_page_no_scroll(
            &mut ctx,
            "Frame Demo",
            debug_layout,
            true,
            ColumnLayout,
        );
        draw_frame_page_content(&mut outer.ctx, state, win_w);
        outer.ctx.finish();
    } else {
        let mut page_state = std::mem::take(&mut state.page);
        {
            let mut outer = crate::demo_page::begin_demo_page(
                &mut ctx,
                "Frame Demo",
                &mut page_state,
                debug_layout,
                ColumnLayout,
            );
            draw_frame_page_content(&mut outer.ctx, state, win_w);
            outer.ctx.finish();
        }
        state.page = page_state;
    }

    ctx.finish();

    cmds
}

pub(crate) fn draw_frame_page_content<'a, 'b, CF>(
    outer: &'b mut WidgetContext<
        'a,
        SampleTextBackend,
        framewise::layouts::OffsetState<framewise::ColumnState>,
        CF,
    >,
    state: &mut FrameDemoState,
    win_w: f32,
) {
    let pad = 20.0;
    let theme = outer.theme;

    // Root Row — two columns side-by-side (Left column: Dynamic list & sizes; Right column: Nesting & alignments)
    let mut root_row = outer.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

    let primary = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent = ButtonStyle::accent_from_theme(&theme);

    let frame_style = FrameStyle::from_theme(&theme);

    // ── Left Column: Dynamic List & Sizing Dimensions ─────────────────────────
    {
        let mut left_col = root_row.child_with_layout(
            RowLayoutParams::auto().fixed_x((win_w - 2.0 * pad - 30.0) * 0.5),
            ColumnLayout,
        );

        // Heading: Left Column Title
        label(
            &mut left_col,
            LabelSpecBuilder::new().text("1. Dynamic & Axis Sizing Showcase"),
            ColumnLayoutParams::auto().fill_x().fixed_y(30.0),
        );
        left_col.spacer(20.0);

        // Sub-row: Add / Remove Controls
        {
            let mut control_row = left_col
                .child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(40.0), RowLayout);

            let add_r = button(
                &mut control_row,
                ButtonSpecBuilder::new()
                    .text("Add Dynamic Button")
                    .style(primary),
                RowLayoutParams::fixed(200.0, 40.0),
                &mut state.add_btn,
            );
            if add_r.input.clicked && state.item_count < 5 {
                state.item_count += 1;
            }
            control_row.spacer(12.0);

            let rem_r = button(
                &mut control_row,
                ButtonSpecBuilder::new()
                    .text("Remove Dynamic Button")
                    .style(accent),
                RowLayoutParams::fixed(200.0, 40.0),
                &mut state.remove_btn,
            );
            if rem_r.input.clicked && state.item_count > 0 {
                state.item_count -= 1;
            }

            control_row.finish();
        }
        left_col.spacer(20.0);

        // Subheading: Dynamic Frame
        let label_style = format!("Auto-Sizing Frame (Current Items: {})", state.item_count);
        label(
            &mut left_col,
            LabelSpecBuilder::new().text(&label_style),
            ColumnLayoutParams::auto().fill_x().fixed_y(24.0),
        );
        left_col.spacer(20.0);

        // ── DYNAMIC FRAME ──
        // Sizes height to children, fills width
        {
            let FrameResult {
                ctx: mut dynamic_frame,
            } = begin_frame(
                &mut left_col,
                FrameSpecBuilder::new().style(frame_style),
                ColumnLayoutParams::auto().fill_x(),
                ColumnLayout,
            );

            if state.item_count == 0 {
                label(
                    &mut dynamic_frame,
                    LabelSpecBuilder::new().text("Frame is empty! Use buttons above to add items."),
                    ColumnLayoutParams::auto().fill_x().fixed_y(32.0),
                );
            } else {
                for i in 0..state.item_count {
                    let text = format!(
                        "Dynamic Button #{} (Clicks: {})",
                        i + 1,
                        state.dynamic_clicks[i]
                    );
                    let r = button(
                        &mut dynamic_frame,
                        ButtonSpecBuilder::new().text(&text).style(secondary),
                        ColumnLayoutParams::auto().fill_x().fixed_y(36.0),
                        &mut state.dynamic_btns[i],
                    );
                    if r.input.clicked {
                        state.dynamic_clicks[i] += 1;
                    }
                    dynamic_frame.spacer(8.0);
                }
            }

            dynamic_frame.finish();
        }
        left_col.spacer(20.0);

        // Subheading: Axis Sizing Dimensions
        label(
            &mut left_col,
            LabelSpecBuilder::new().text("Comparison of Sizing Dimensions"),
            ColumnLayoutParams::auto().fill_x().fixed_y(24.0),
        );
        left_col.spacer(20.0);

        // Row of 4 different Frame Dimension constraints
        {
            let mut dimensions_row = left_col.child_with_layout(
                ColumnLayoutParams::auto().fill_x().fixed_y(180.0),
                RowLayout,
            );

            // 1. Fixed-Size Frame (200x120)
            {
                let FrameResult { ctx: mut sub_frame } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    RowLayoutParams::fixed(120.0, 120.0),
                    ColumnLayout,
                );

                label(
                    &mut sub_frame,
                    LabelSpecBuilder::new().text("Fixed frame"),
                    ColumnLayoutParams::auto().fill_x().fixed_y(20.0),
                );
                sub_frame.spacer(4.0);

                let text = format!("Cl: {}", state.fixed_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(primary),
                    ColumnLayoutParams::auto().fill_x().fixed_y(36.0),
                    &mut state.fixed_btn,
                );
                if r.input.clicked {
                    state.fixed_clicks += 1;
                }

                sub_frame.finish();
            }
            dimensions_row.spacer(16.0);

            // 2. Width Auto, Height Fixed (Auto width wraps to child text width request!)
            {
                let FrameResult { ctx: mut sub_frame } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    RowLayoutParams::auto().fixed_y(120.0),
                    ColumnLayout,
                );

                label(
                    &mut sub_frame,
                    LabelSpecBuilder::new().text("Auto Width"),
                    ColumnLayoutParams::auto().fixed_y(20.0),
                );
                sub_frame.spacer(4.0);

                let text = format!("Auto Width for me! {}", state.width_auto_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(secondary),
                    ColumnLayoutParams::auto().fixed_y(36.0),
                    &mut state.width_auto_btn,
                );
                if r.input.clicked {
                    state.width_auto_clicks += 1;
                }

                sub_frame.finish();
            }
            dimensions_row.spacer(16.0);

            // 3. Width Fixed, Height Auto
            {
                let FrameResult { ctx: mut sub_frame } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    RowLayoutParams::auto().fixed_x(130.0),
                    ColumnLayout,
                );

                label(
                    &mut sub_frame,
                    LabelSpecBuilder::new().text("Auto Height"),
                    ColumnLayoutParams::auto().fill_x().fixed_y(20.0),
                );
                sub_frame.spacer(4.0);

                let text = format!("Auto H: {}", state.height_auto_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(accent),
                    ColumnLayoutParams::auto().fill_x().fixed_y(36.0),
                    &mut state.height_auto_btn,
                );
                if r.input.clicked {
                    state.height_auto_clicks += 1;
                }
                sub_frame.spacer(4.0);

                button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text("Height").style(accent),
                    ColumnLayoutParams::auto().fill_x(),
                    &mut state.height_auto_btn2,
                );

                sub_frame.finish();
            }
            dimensions_row.spacer(16.0);

            // 4. Fully Auto (Both Width & Height Auto)
            {
                let FrameResult { ctx: mut sub_frame } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    RowLayoutParams::auto(),
                    ColumnLayout,
                );

                label(
                    &mut sub_frame,
                    LabelSpecBuilder::new().text("Fully Auto"),
                    ColumnLayoutParams::auto().fixed_y(20.0),
                );
                sub_frame.spacer(4.0);

                let text = format!("Fully Auto: {}", state.fully_auto_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(secondary),
                    ColumnLayoutParams::auto(),
                    &mut state.fully_auto_btn,
                );
                if r.input.clicked {
                    state.fully_auto_clicks += 1;
                }

                sub_frame.finish();
            }

            dimensions_row.finish();
        }

        left_col.finish();
    }

    root_row.spacer(30.0);

    // ── Right Column: Nesting & Alignments ────────────────────────────────────
    {
        let mut right_col = root_row.child_with_layout(
            RowLayoutParams::auto().fixed_x((win_w - 2.0 * pad - 30.0) * 0.5),
            ColumnLayout,
        );

        // Heading: Right Column Title
        label(
            &mut right_col,
            LabelSpecBuilder::new().text("2. Complex Nesting & Cross-Alignments"),
            ColumnLayoutParams::auto().fill_x().fixed_y(30.0),
        );
        right_col.spacer(20.0);

        // Showcase 1: Symmetrical Nesting Cases (Fixed Panel centered in Fixed Outer)
        label(
            &mut right_col,
            LabelSpecBuilder::new().text("Nesting Showcase (Fixed Panel centered in Fixed Outer)"),
            ColumnLayoutParams::auto().fill_x().fixed_y(24.0),
        );
        right_col.spacer(20.0);

        // Outer Fixed frame (450x180) containing centered inner fixed frame
        {
            let FrameResult {
                ctx: mut outer_fixed,
            } = begin_frame(
                &mut right_col,
                FrameSpecBuilder::new().style(frame_style),
                ColumnLayoutParams::fixed(450.0, 180.0),
                ColumnLayout,
            );

            let text = format!("Outer Fixed Frame (Clicks: {})", state.nested_clicks[0]);
            let r = button(
                &mut outer_fixed,
                ButtonSpecBuilder::new().text(&text).style(secondary),
                ColumnLayoutParams::auto().fill_x().fixed_y(36.0),
                &mut state.outer_btn,
            );
            if r.input.clicked {
                state.nested_clicks[0] += 1;
            }
            outer_fixed.spacer(8.0);

            // Inner Fixed-Width Frame nested inside Fixed outer!
            {
                let FrameResult {
                    ctx: mut inner_auto,
                } = begin_frame(
                    &mut outer_fixed,
                    FrameSpecBuilder::new().style(FrameStyle {
                        background: theme.paper, // distinct dark background
                        ..frame_style
                    }),
                    ColumnLayoutParams::auto()
                        .fixed_x(350.0)
                        .align_x(Align::Center),
                    RowLayout,
                );

                let text_c = format!("Inner Center (Clicks: {})", state.nested_clicks[1]);
                let r1 = button(
                    &mut inner_auto,
                    ButtonSpecBuilder::new().text(&text_c).style(accent),
                    RowLayoutParams::auto(),
                    &mut state.inner_center_btn,
                );
                if r1.input.clicked {
                    state.nested_clicks[1] += 1;
                }
                inner_auto.spacer(12.0);

                let text_e = format!("Inner End (Clicks: {})", state.nested_clicks[2]);
                let r2 = button(
                    &mut inner_auto,
                    ButtonSpecBuilder::new().text(&text_e).style(primary),
                    RowLayoutParams::auto(),
                    &mut state.inner_end_btn,
                );
                if r2.input.clicked {
                    state.nested_clicks[2] += 1;
                }

                inner_auto.finish();
            }

            outer_fixed.finish();
        }
        right_col.spacer(20.0);

        // Showcase 2: Cross-Axis Alignment Demonstration inside a Fit Frame
        label(
            &mut right_col,
            LabelSpecBuilder::new()
                .text("Cross-Axis Alignment within Fit Frame (Auto height, Centered)"),
            ColumnLayoutParams::auto().fill_x().fixed_y(24.0),
        );
        right_col.spacer(20.0);

        // Center-aligned column layout inside an auto-height frame
        {
            let FrameResult {
                ctx: mut fit_centered,
            } = begin_frame(
                &mut right_col,
                FrameSpecBuilder::new().style(frame_style),
                ColumnLayoutParams::auto().fill_x(),
                ColumnLayout,
            );

            // Three buttons of varying widths visually demonstrating centered alignment within dynamic frame
            button(
                &mut fit_centered,
                ButtonSpecBuilder::new().text("Small Width").style(primary),
                ColumnLayoutParams::fixed(120.0, 36.0).align_x(Align::Center),
                &mut state.align_small_btn,
            );
            fit_centered.spacer(10.0);

            button(
                &mut fit_centered,
                ButtonSpecBuilder::new()
                    .text("Medium Width Button")
                    .style(secondary),
                ColumnLayoutParams::fixed(240.0, 36.0).align_x(Align::Center),
                &mut state.align_med_btn,
            );
            fit_centered.spacer(10.0);

            button(
                &mut fit_centered,
                ButtonSpecBuilder::new()
                    .text("Large Width Content Button")
                    .style(accent),
                ColumnLayoutParams::fixed(360.0, 36.0).align_x(Align::Center),
                &mut state.align_large_btn,
            );

            fit_centered.finish();
        }

        right_col.finish();
    }

    root_row.finish();
}
