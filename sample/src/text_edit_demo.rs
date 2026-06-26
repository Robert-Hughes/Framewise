use crate::text::SampleTextBackend;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{AxisBound, Layout, LayoutSpace, SizeRequest},
    layouts::linear::{ColumnLayout, ColumnLayoutParams, RowLayout, RowLayoutParams},
    theme::Theme,
    types::{Color, Rect, Stroke},
    widget::WidgetContext,
    widgets::checkbox::{labelled_checkbox, CheckboxSpecBuilder, CheckboxState, CheckedState},
    widgets::label::{label, LabelSpecBuilder, LabelStyle},
    widgets::radio::{labelled_radio, RadioSpecBuilder, RadioState},
    widgets::text_edit::{
        text_edit, NewlinePolicy, TextEditSpecBuilder, TextEditState, TextEditStyle,
    },
    Align, LineHeight, TextLineAlign,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaygroundNewlinePolicy {
    Preserve,
    ReplaceWithSpace,
    TrimAfterFirstNewline,
}

impl PlaygroundNewlinePolicy {
    fn to_newline_policy(self) -> NewlinePolicy {
        match self {
            Self::Preserve => NewlinePolicy::Preserve,
            Self::ReplaceWithSpace => NewlinePolicy::ReplaceWithSpace,
            Self::TrimAfterFirstNewline => NewlinePolicy::TrimAfterFirstNewline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaygroundSizeMode {
    Auto,
    Fixed,
}

pub struct TextEditDemoState {
    pub page: crate::demo_page::DemoPageState,
    pub te_preset_single_line: TextEditState,
    pub te_preset_multiline_unwrapped: TextEditState,
    pub te_preset_multiline_wrapped: TextEditState,
    pub te_playground: TextEditState,
    pub te_playground_wrap: CheckboxState,
    playground_newline_policy: PlaygroundNewlinePolicy,
    pub playground_newline_radios: [RadioState; 3],
    pub playground_line_align: TextLineAlign,
    pub playground_line_align_radios: [RadioState; 3],
    pub playground_vertical_align: Align,
    pub playground_vertical_align_radios: [RadioState; 3],
    playground_width_mode: PlaygroundSizeMode,
    pub playground_width_radios: [RadioState; 2],
    playground_height_mode: PlaygroundSizeMode,
    pub playground_height_radios: [RadioState; 2],
    pub te_aligns: [[TextEditState; 3]; 3],
    pub te_styled: TextEditState,
    pub te_large: TextEditState,
    pub te_large_wrap: CheckboxState,
}

impl Default for TextEditDemoState {
    fn default() -> Self {
        Self {
            page: crate::demo_page::DemoPageState::default(),
            te_preset_single_line: TextEditState::new("Single-line value"),
            te_preset_multiline_unwrapped: TextEditState::new(
                "Hard newlines stay here.\nThis deliberately long unwrapped line should run beyond the visible edge so horizontal scrolling is useful.\nAnother short line.",
            ),
            te_preset_multiline_wrapped: TextEditState::new(
                "This wrapped multiline editor preserves hard newlines while letting long paragraphs wrap naturally inside the fixed viewport.",
            ),
            te_playground: TextEditState::new(
                "Playground text starts with a hard newline below.\nThis long line gives the wrap and scrolling controls enough material to show their effect across fixed and auto sizing modes. Add more text, press Enter, or paste content to inspect the raw configuration space.",
            ),
            te_playground_wrap: CheckboxState {
                checked: CheckedState::Checked,
                ..Default::default()
            },
            playground_newline_policy: PlaygroundNewlinePolicy::Preserve,
            playground_newline_radios: Default::default(),
            playground_line_align: TextLineAlign::Start,
            playground_line_align_radios: Default::default(),
            playground_vertical_align: Align::Start,
            playground_vertical_align_radios: Default::default(),
            playground_width_mode: PlaygroundSizeMode::Fixed,
            playground_width_radios: Default::default(),
            playground_height_mode: PlaygroundSizeMode::Fixed,
            playground_height_radios: Default::default(),
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
            te_styled: TextEditState::new(
                "Styled text_edit\nItalic, tracking, line height.\nTheme? No. Deliberate.",
            ),
            te_large: TextEditState::new(include_str!("text_edit_demo.rs")),
            te_large_wrap: CheckboxState {
                checked: CheckedState::Checked,
                ..Default::default()
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_edit_demo(
    state: &mut TextEditDemoState,
    focus_system: &mut FocusSystem,
    input: &Input,
    output: &mut framewise::Output,
    time: f64,
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

    let mut root_ctx = WidgetContext::root(
        Theme::default(),
        text_backend,
        focus_system,
        input,
        output,
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
        SampleTextBackend,
        framewise::layouts::OffsetState<framewise::ColumnState>,
        CF,
    >,
    state: &mut TextEditDemoState,
) {
    let theme = ctx.theme;

    let section_header_style = LabelStyle {
        text_style: framewise::TextStyle::new(
            theme.sans_font,
            18.0,
            theme.sans_weight_bold,
            framewise::text::TextFlow::single_line(),
        ),
        content_placement: framewise::TextContentPlacement::TOP_LEFT,
        text_color: theme.ink,
        rule: Some(Stroke::new(theme.line_on_paper, 1.0)),
    };

    let desc_style = LabelStyle {
        text_style: framewise::TextStyle::new(
            theme.sans_font,
            13.0,
            theme.sans_weight_regular,
            framewise::text::TextFlow::single_line(),
        ),
        content_placement: framewise::TextContentPlacement::TOP_LEFT,
        text_color: theme.muted,
        rule: None,
    };

    let mut page_row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);
    let mut left = page_row.child_with_layout(RowLayoutParams::auto().fixed_x(430.0), ColumnLayout);
    let ctx = &mut left;

    label(
        ctx,
        LabelSpecBuilder::new()
            .text("1. Standard Presets")
            .style(section_header_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(8.0);

    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Single-line field")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(4.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Newlines are replaced with spaces. Pressing Enter inserts a space.")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(6.0);
    text_edit(
        ctx,
        TextEditSpecBuilder::new().single_line(),
        ColumnLayoutParams::fixed(400.0, 36.0),
        &mut state.te_preset_single_line,
    );

    ctx.spacer(18.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Multiline unwrapped")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(4.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Hard newlines are preserved and long lines do not soft-wrap.")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(6.0);
    text_edit(
        ctx,
        TextEditSpecBuilder::new().multiline_unwrapped(),
        ColumnLayoutParams::fixed(400.0, 100.0),
        &mut state.te_preset_multiline_unwrapped,
    );

    ctx.spacer(18.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Multiline wrapped")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(4.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Hard newlines are preserved and long lines soft-wrap.")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(6.0);
    text_edit(
        ctx,
        TextEditSpecBuilder::new().multiline_wrapped(),
        ColumnLayoutParams::fixed(400.0, 100.0),
        &mut state.te_preset_multiline_wrapped,
    );

    ctx.spacer(24.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("2. Configuration Playground")
            .style(section_header_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(4.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Exposes the raw configuration axes, including combinations that are unusual but supported.")
            .style(desc_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(10.0);

    {
        let mut controls = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);
        let mut col_a =
            controls.child_with_layout(RowLayoutParams::auto().fixed_x(205.0), ColumnLayout);
        label(
            &mut col_a,
            LabelSpecBuilder::new()
                .text("Newline policy")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        state.playground_newline_radios[0].checked =
            state.playground_newline_policy == PlaygroundNewlinePolicy::Preserve;
        if labelled_radio(
            &mut col_a,
            RadioSpecBuilder::new(),
            "Preserve",
            ColumnLayoutParams::auto(),
            &mut state.playground_newline_radios[0],
        )
        .input
        .clicked
        {
            state.playground_newline_policy = PlaygroundNewlinePolicy::Preserve;
        }
        state.playground_newline_radios[1].checked =
            state.playground_newline_policy == PlaygroundNewlinePolicy::ReplaceWithSpace;
        if labelled_radio(
            &mut col_a,
            RadioSpecBuilder::new(),
            "Replace with spaces",
            ColumnLayoutParams::auto(),
            &mut state.playground_newline_radios[1],
        )
        .input
        .clicked
        {
            state.playground_newline_policy = PlaygroundNewlinePolicy::ReplaceWithSpace;
        }
        state.playground_newline_radios[2].checked =
            state.playground_newline_policy == PlaygroundNewlinePolicy::TrimAfterFirstNewline;
        if labelled_radio(
            &mut col_a,
            RadioSpecBuilder::new(),
            "Trim after first newline",
            ColumnLayoutParams::auto(),
            &mut state.playground_newline_radios[2],
        )
        .input
        .clicked
        {
            state.playground_newline_policy = PlaygroundNewlinePolicy::TrimAfterFirstNewline;
        }
        col_a.spacer(8.0);
        labelled_checkbox(
            &mut col_a,
            CheckboxSpecBuilder::new(),
            "Word wrap",
            ColumnLayoutParams::auto(),
            &mut state.te_playground_wrap,
        );
        col_a.finish();

        controls.spacer(16.0);
        let mut col_b = controls.child_with_layout(RowLayoutParams::auto(), ColumnLayout);
        label(
            &mut col_b,
            LabelSpecBuilder::new()
                .text("Horizontal alignment")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        state.playground_line_align_radios[0].checked =
            state.playground_line_align == TextLineAlign::Start;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "Start",
            ColumnLayoutParams::auto(),
            &mut state.playground_line_align_radios[0],
        )
        .input
        .clicked
        {
            state.playground_line_align = TextLineAlign::Start;
        }
        state.playground_line_align_radios[1].checked =
            state.playground_line_align == TextLineAlign::Center;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "Center",
            ColumnLayoutParams::auto(),
            &mut state.playground_line_align_radios[1],
        )
        .input
        .clicked
        {
            state.playground_line_align = TextLineAlign::Center;
        }
        state.playground_line_align_radios[2].checked =
            state.playground_line_align == TextLineAlign::End;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "End",
            ColumnLayoutParams::auto(),
            &mut state.playground_line_align_radios[2],
        )
        .input
        .clicked
        {
            state.playground_line_align = TextLineAlign::End;
        }
        col_b.finish();
        controls.finish();
    }

    ctx.spacer(10.0);
    {
        let mut controls = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);
        let mut col_a =
            controls.child_with_layout(RowLayoutParams::auto().fixed_x(205.0), ColumnLayout);
        label(
            &mut col_a,
            LabelSpecBuilder::new()
                .text("Vertical alignment")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        state.playground_vertical_align_radios[0].checked =
            state.playground_vertical_align == Align::Start;
        if labelled_radio(
            &mut col_a,
            RadioSpecBuilder::new(),
            "Start",
            ColumnLayoutParams::auto(),
            &mut state.playground_vertical_align_radios[0],
        )
        .input
        .clicked
        {
            state.playground_vertical_align = Align::Start;
        }
        state.playground_vertical_align_radios[1].checked =
            state.playground_vertical_align == Align::Center;
        if labelled_radio(
            &mut col_a,
            RadioSpecBuilder::new(),
            "Center",
            ColumnLayoutParams::auto(),
            &mut state.playground_vertical_align_radios[1],
        )
        .input
        .clicked
        {
            state.playground_vertical_align = Align::Center;
        }
        state.playground_vertical_align_radios[2].checked =
            state.playground_vertical_align == Align::End;
        if labelled_radio(
            &mut col_a,
            RadioSpecBuilder::new(),
            "End",
            ColumnLayoutParams::auto(),
            &mut state.playground_vertical_align_radios[2],
        )
        .input
        .clicked
        {
            state.playground_vertical_align = Align::End;
        }
        col_a.finish();

        controls.spacer(16.0);
        let mut col_b = controls.child_with_layout(RowLayoutParams::auto(), ColumnLayout);
        label(
            &mut col_b,
            LabelSpecBuilder::new().text("Size mode").style(desc_style),
            ColumnLayoutParams::auto(),
        );
        state.playground_width_radios[0].checked =
            state.playground_width_mode == PlaygroundSizeMode::Auto;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "Auto width",
            ColumnLayoutParams::auto(),
            &mut state.playground_width_radios[0],
        )
        .input
        .clicked
        {
            state.playground_width_mode = PlaygroundSizeMode::Auto;
        }
        state.playground_width_radios[1].checked =
            state.playground_width_mode == PlaygroundSizeMode::Fixed;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "Fixed width",
            ColumnLayoutParams::auto(),
            &mut state.playground_width_radios[1],
        )
        .input
        .clicked
        {
            state.playground_width_mode = PlaygroundSizeMode::Fixed;
        }
        state.playground_height_radios[0].checked =
            state.playground_height_mode == PlaygroundSizeMode::Auto;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "Auto height",
            ColumnLayoutParams::auto(),
            &mut state.playground_height_radios[0],
        )
        .input
        .clicked
        {
            state.playground_height_mode = PlaygroundSizeMode::Auto;
        }
        state.playground_height_radios[1].checked =
            state.playground_height_mode == PlaygroundSizeMode::Fixed;
        if labelled_radio(
            &mut col_b,
            RadioSpecBuilder::new(),
            "Fixed height",
            ColumnLayoutParams::auto(),
            &mut state.playground_height_radios[1],
        )
        .input
        .clicked
        {
            state.playground_height_mode = PlaygroundSizeMode::Fixed;
        }
        col_b.finish();
        controls.finish();
    }

    ctx.spacer(10.0);
    let playground_max_width = 420.0;
    let playground_max_height = 140.0;
    let fixed_width = state.playground_width_mode == PlaygroundSizeMode::Fixed;
    let fixed_height = state.playground_height_mode == PlaygroundSizeMode::Fixed;
    let playground_layout = match (fixed_width, fixed_height) {
        (true, true) => ColumnLayoutParams::fixed(playground_max_width, playground_max_height),
        (true, false) => ColumnLayoutParams::auto().fixed_x(playground_max_width),
        (false, true) => ColumnLayoutParams::auto().fixed_y(playground_max_height),
        (false, false) => ColumnLayoutParams::auto(),
    };
    let playground_builder = TextEditSpecBuilder::new()
        .newline_policy(state.playground_newline_policy.to_newline_policy())
        .wrap(state.te_playground_wrap.checked == CheckedState::Checked)
        .line_align(state.playground_line_align)
        .vertical_align(state.playground_vertical_align);

    if fixed_width && fixed_height {
        text_edit(
            ctx,
            playground_builder,
            playground_layout,
            &mut state.te_playground,
        );
    } else {
        let playground_rect = ctx.layout(
            ColumnLayoutParams::fixed(playground_max_width, playground_max_height),
            SizeRequest::UNKNOWN,
        );
        let playground_space = LayoutSpace::new(
            playground_rect.x,
            playground_rect.y,
            AxisBound::AtMost(playground_max_width),
            AxisBound::AtMost(playground_max_height),
        );
        let mut playground_limit = ctx
            .child_with_layout_and_on_finish(ColumnLayout.begin(playground_space), |_, _, _, _| {});
        text_edit(
            &mut playground_limit,
            playground_builder,
            playground_layout,
            &mut state.te_playground,
        );
        playground_limit.finish();
    }

    ctx.spacer(24.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("3. Alignment Showcase")
            .style(section_header_style),
        ColumnLayoutParams::auto(),
    );
    ctx.spacer(4.0);
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Vertical alignment is visible when the text content fits inside the viewport. Once content overflows, scrolling takes over.")
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
                    .multiline_wrapped()
                    .vertical_align(vertical_align)
                    .line_align(line_align),
                RowLayoutParams::fixed(130.0, 80.0),
                &mut state.te_aligns[r][c],
            );
        }
        row.finish();
        ctx.spacer(12.0);
    }

    left.finish();
    page_row.spacer(28.0);

    {
        let mut right =
            page_row.child_with_layout(RowLayoutParams::auto().fixed_x(560.0), ColumnLayout);

        label(
            &mut right,
            LabelSpecBuilder::new()
                .text("4. Styled Text Edit")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        right.spacer(4.0);
        label(
            &mut right,
            LabelSpecBuilder::new()
                .text("Exercises text edit typography, spacing, colours, padding, caret, selection, and borders.")
                .style(desc_style),
            ColumnLayoutParams::auto(),
        );
        right.spacer(10.0);

        let styled_text_edit_style = TextEditStyle {
            background: Color::from_srgb_u8(36, 30, 45, 255),
            background_hovered: Color::from_srgb_u8(43, 36, 54, 255),
            error_background: Color::from_srgb_u8(58, 30, 34, 255),
            border: Some(Stroke::new(Color::from_srgb_u8(226, 181, 101, 255), 2.0)),
            focus_border: Some(Stroke::new(Color::from_srgb_u8(121, 199, 184, 255), 3.0)),
            error_border: Some(Stroke::new(Color::from_srgb_u8(236, 105, 86, 255), 2.0)),
            error_stripe_width: 6.0,
            min_height: 92.0,
            padding_x: 14.0,
            padding_y: 10.0,
            font: theme.heading_font,
            size: 15.0,
            weight: theme.heading_weight,
            italic: true,
            letter_spacing: 0.055,
            line_height: LineHeight::Relative(1.35),
            text_color: Color::from_srgb_u8(249, 241, 220, 255),
            placeholder_color: Color::from_srgb_u8(174, 153, 189, 255),
            caret_color: Color::from_srgb_u8(121, 199, 184, 255),
            caret_width: 3.0,
            select_color: Color::from_srgb_f32(121.0 / 255.0, 199.0 / 255.0, 184.0 / 255.0, 0.28),
            disabled_alpha: 0.42,
            ..TextEditStyle::from_theme(&theme)
        };

        text_edit(
            &mut right,
            TextEditSpecBuilder::new()
                .multiline_unwrapped()
                .style(styled_text_edit_style)
                .line_align(TextLineAlign::Center)
                .vertical_align(Align::Center),
            ColumnLayoutParams::fixed(360.0, 110.0),
            &mut state.te_styled,
        );

        right.spacer(24.0);
        label(
            &mut right,
            LabelSpecBuilder::new()
                .text("5. Large Editing Area")
                .style(section_header_style),
            ColumnLayoutParams::auto(),
        );
        right.spacer(8.0);
        labelled_checkbox(
            &mut right,
            CheckboxSpecBuilder::new(),
            "Word wrap",
            ColumnLayoutParams::auto(),
            &mut state.te_large_wrap,
        );
        right.spacer(10.0);

        let word_wrap_enabled = state.te_large_wrap.checked == CheckedState::Checked;
        text_edit(
            &mut right,
            TextEditSpecBuilder::new()
                .multiline_unwrapped()
                .wrap(word_wrap_enabled),
            ColumnLayoutParams::fixed(540.0, 520.0),
            &mut state.te_large,
        );

        right.finish();
    }

    page_row.finish();
}
