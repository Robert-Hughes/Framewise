use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layouts::linear::{ColumnLayout, ColumnLayoutParams, RowLayout, RowLayoutParams},
    theme::Theme,
    types::Rect,
    widget::WidgetContext,
    widgets::label::{label, LabelSpecBuilder, LabelStyle},
    widgets::text_edit::{text_edit, NewlinePolicy, TextEditSpecBuilder, TextEditState},
    Align, TextLineAlign,
};

pub struct TextEditDemoState {
    pub page: crate::demo_page::DemoPageState,
    pub te_allow: TextEditState,
    pub te_replace: TextEditState,
    pub te_reject: TextEditState,
    pub te_wrap: TextEditState,
    pub te_aligns: [[TextEditState; 3]; 3],
}

impl Default for TextEditDemoState {
    fn default() -> Self {
        Self {
            page: crate::demo_page::DemoPageState::default(),
            te_allow: TextEditState::new("one one one\ntwotwotwo\nthreeeeeeeee"),
            te_replace: TextEditState::new("one one one\ntwotwotwo\nthreeeeeeeee"),
            te_reject: TextEditState::new("one one one\ntwotwotwo\nthreeeeeeeee"),
            te_wrap: TextEditState::new("This is a wrapping text edit widget. Try typing a very long sentence here to see how word wrapping wraps the characters/words to the next line dynamically inside the widget's box! You can also resize the window or click anywhere in the text edit to place the cursor and test vertical navigation across wrapped lines."),
            te_aligns: [
                [
                    TextEditState::new("Top\nLeft"),
                    TextEditState::new("Top\nCenter"),
                    TextEditState::new("Top\nRight"),
                ],
                [
                    TextEditState::new("Middle\nLeft"),
                    TextEditState::new("Middle\nCenter"),
                    TextEditState::new("Middle\nRight"),
                ],
                [
                    TextEditState::new("Bottom\nLeft"),
                    TextEditState::new("Bottom\nCenter"),
                    TextEditState::new("Bottom\nRight"),
                ],
            ],
        }
    }
}

pub fn draw_text_edit_demo(
    state: &mut TextEditDemoState,
    focus_system: &mut FocusSystem,
    input: &Input,
    time: f64,
    win_size: (f32, f32),
    text_system: &mut SampleTextSystem,
    debug_layout: bool,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let is_unbounded = win_h.is_infinite();

    let mut cmds = framewise::DrawCommands::new();
    let space = if is_unbounded {
        framewise::LayoutSpace::unbounded_height(0.0, 0.0, win_w)
    } else {
        Rect::new(0.0, 0.0, win_w, win_h).into()
    };

    let mut root_ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        ColumnLayout,
        space,
        &mut cmds,
    );
    root_ctx.time = time;

    if is_unbounded {
        let mut outer = crate::demo_page::begin_demo_page_no_scroll(
            &mut root_ctx,
            "TextEdit Demo",
            debug_layout,
            true,
            ColumnLayout,
        );
        draw_text_edit_demo_content(&mut outer.ctx, state);
        outer.ctx.finish();
    } else {
        let mut page_state = std::mem::take(&mut state.page);
        {
            let mut outer = crate::demo_page::begin_demo_page(
                &mut root_ctx,
                "TextEdit Demo",
                &mut page_state,
                debug_layout,
                ColumnLayout,
            );
            draw_text_edit_demo_content(&mut outer.ctx, state);
            outer.ctx.finish();
        }
        state.page = page_state;
    }

    root_ctx.finish();

    cmds
}

pub(crate) fn draw_text_edit_demo_content<'a, 'b, CF>(
    ctx: &'b mut WidgetContext<
        'a,
        SampleTextSystem,
        framewise::layouts::OffsetState<framewise::ColumnState>,
        CF,
    >,
    state: &mut TextEditDemoState,
) {
    let theme = ctx.theme;

    // Header section style
    let section_header_style = LabelStyle {
        text_style: framewise::TextStyle::new(
            theme.sans_font,
            18.0,
            theme.sans_weight_bold,
            framewise::text::TextFlow::single_line(),
        ),
        content_placement: framewise::TextContentPlacement::TOP_LEFT,
        text_color: theme.ink,
        rule: true,
        rule_color: theme.line,
    };

    // Subtitle / description style
    let desc_style = LabelStyle {
        text_style: framewise::TextStyle::new(
            theme.sans_font,
            13.0,
            theme.sans_weight_regular,
            framewise::text::TextFlow::single_line(),
        ),
        content_placement: framewise::TextContentPlacement::TOP_LEFT,
        text_color: theme.muted,
        rule: false,
        rule_color: theme.line,
    };

    // ── 1. Allow Policy ──────────────────────────────────────────────────────────
    {
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("1. NewlinePolicy::Allow")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("Allows typing and pasting multiline text. Press Enter to insert a newline.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            ctx,
            TextEditSpecBuilder::new().newline_policy(NewlinePolicy::Allow),
            ColumnLayoutParams::fixed(400.0, 80.0),
            &mut state.te_allow,
        );
    }
    ctx.spacer(24.0);

    // ── 2. ReplaceWithSpace Policy ────────────────────────────────────────────────
    {
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("2. NewlinePolicy::ReplaceWithSpace")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("Replaces newline characters with spaces on paste. Hitting Enter is ignored.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            ctx,
            TextEditSpecBuilder::new().newline_policy(NewlinePolicy::ReplaceWithSpace),
            ColumnLayoutParams::fixed(400.0, 80.0),
            &mut state.te_replace,
        );
    }
    ctx.spacer(24.0);

    // ── 3. Reject Policy ──────────────────────────────────────────────────────────
    {
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("3. NewlinePolicy::Reject")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("Strips/rejects newline characters entirely. Hitting Enter is ignored.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            ctx,
            TextEditSpecBuilder::new().newline_policy(NewlinePolicy::Reject),
            ColumnLayoutParams::fixed(400.0, 80.0),
            &mut state.te_reject,
        );
    }
    ctx.spacer(24.0);

    // ── 4. Wrapping ──────────────────────────────────────────────────────────────
    {
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("4. Wrapping (NewlinePolicy::Allow + wrap(true))")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("Enables word wrapping on the text edit. Long lines wrap to the next line automatically.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            ctx,
            TextEditSpecBuilder::new()
                .newline_policy(NewlinePolicy::Allow)
                .wrap(true),
            ColumnLayoutParams::fixed(400.0, 100.0),
            &mut state.te_wrap,
        );
    }
    ctx.spacer(24.0);

    // ── 5. Alignment Combinations ──────────────────────────────────────────────
    {
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("5. Alignment Combinations (3x3 Grid)")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("Demonstrates the 9 combinations of vertical alignment (Top, Middle, Bottom) and horizontal line alignment (Left, Center, Right).")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        let vertical_options = [Align::Start, Align::Center, Align::End];
        let horizontal_options = [
            TextLineAlign::Start,
            TextLineAlign::Center,
            TextLineAlign::End,
        ];

        for (r, &vertical_align) in vertical_options.iter().enumerate() {
            let mut row =
                ctx.child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(80.0), RowLayout);

            for (c, &line_align) in horizontal_options.iter().enumerate() {
                if c > 0 {
                    row.spacer(12.0);
                }

                text_edit(
                    &mut row,
                    TextEditSpecBuilder::new()
                        .vertical_align(vertical_align)
                        .line_align(line_align)
                        .newline_policy(NewlinePolicy::Allow)
                        .wrap(true),
                    RowLayoutParams::fixed(130.0, 80.0),
                    &mut state.te_aligns[r][c],
                );
            }
            row.finish();
            ctx.spacer(12.0);
        }
    }
}
