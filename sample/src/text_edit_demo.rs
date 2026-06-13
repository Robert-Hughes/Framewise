use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layouts::linear::{ColumnLayout, ColumnLayoutParams},
    theme::Theme,
    types::Rect,
    widget::WidgetContext,
    widgets::label::{label, LabelSpecBuilder, LabelStyle},
    widgets::text_edit::{text_edit, NewlinePolicy, TextEditSpecBuilder, TextEditState},
};

pub struct TextEditDemoState {
    pub page: crate::demo_page::DemoPageState,
    pub te_allow: TextEditState,
    pub te_replace: TextEditState,
    pub te_reject: TextEditState,
}

impl Default for TextEditDemoState {
    fn default() -> Self {
        Self {
            page: crate::demo_page::DemoPageState::default(),
            te_allow: TextEditState::new("one one one\ntwotwotwo\nthreeeeeeeee"),
            te_replace: TextEditState::new("one one one\ntwotwotwo\nthreeeeeeeee"),
            te_reject: TextEditState::new("one one one\ntwotwotwo\nthreeeeeeeee"),
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

    let mut cmds = framewise::DrawCommands::new();
    let mut root_ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        ColumnLayout,
        Rect::new(0.0, 0.0, win_w, win_h),
        &mut cmds,
    );
    root_ctx.time = time;

    let crate::demo_page::DemoPageResult { mut ctx } = crate::demo_page::begin_demo_page(
        &mut root_ctx,
        "TextEdit Demo",
        &mut state.page,
        debug_layout,
        ColumnLayout,
    );

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
            &mut ctx,
            LabelSpecBuilder::new()
                .text("1. NewlinePolicy::Allow")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("Allows typing and pasting multiline text. Press Enter to insert a newline.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            &mut ctx,
            TextEditSpecBuilder::new().newline_policy(NewlinePolicy::Allow),
            ColumnLayoutParams::fixed(400.0, 80.0),
            &mut state.te_allow,
        );
    }
    ctx.spacer(24.0);

    // ── 2. ReplaceWithSpace Policy ────────────────────────────────────────────────
    {
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("2. NewlinePolicy::ReplaceWithSpace")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("Replaces newline characters with spaces on paste. Hitting Enter is ignored.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            &mut ctx,
            TextEditSpecBuilder::new().newline_policy(NewlinePolicy::ReplaceWithSpace),
            ColumnLayoutParams::fixed(400.0, 80.0),
            &mut state.te_replace,
        );
    }
    ctx.spacer(24.0);

    // ── 3. Reject Policy ──────────────────────────────────────────────────────────
    {
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("3. NewlinePolicy::Reject")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(4.0);
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("Strips/rejects newline characters entirely. Hitting Enter is ignored.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        ctx.spacer(10.0);

        text_edit(
            &mut ctx,
            TextEditSpecBuilder::new().newline_policy(NewlinePolicy::Reject),
            ColumnLayoutParams::fixed(400.0, 80.0),
            &mut state.te_reject,
        );
    }

    ctx.finish();
    root_ctx.finish();

    cmds
}
