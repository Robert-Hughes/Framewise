use crate::text::SampleTextBackend;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Align, Placement, Placement2D},
    layouts::{
        linear::{ColumnLayout, ColumnLayoutParams, RowLayout, RowLayoutParams},
        WrapLayout,
    },
    theme::Theme,
    types::Rect,
    widget::WidgetContext,
    widgets::button::{button, ButtonSpec, ButtonState, ButtonStyle},
    widgets::label::{label, LabelSpecBuilder, LabelStyle},
};

// ── State ──────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ButtonPageState {
    pub page: crate::demo_page::DemoPageState,
    // Section 1: one button per style
    pub toolbar_btns: [ButtonState; 4],
    pub toolbar_clicks: [u32; 4],
    // Section 2: enabled + disabled variant for each style (8 total)
    pub style_btns: [ButtonState; 8],
    // Section 3: counter label, decrement, display, increment, reset
    pub counter_btns: [ButtonState; 5],
    pub counter: i32,
    // Section 4: 3×3 action grid
    pub grid_btns: [ButtonState; 9],
    pub grid_clicks: [u32; 9],
    // Section 5: size-request sizing — auto-width row + fill-width column
    pub auto_btns: [ButtonState; 5],
    // Section 6: wrapping flow of auto-width buttons
    pub wrap_btns: [ButtonState; 8],
    // Section 7: alignment showcase
    pub align_btns: [ButtonState; 12],
    // Section 8: content placement showcase
    pub content_btns: [ButtonState; 11],
}

// ── Draw ──────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_button_page(
    state: &mut ButtonPageState,
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
            "Button Demo",
            debug_layout,
            true,
            ColumnLayout,
        );
        draw_button_page_content(&mut outer.ctx, state);
        outer.ctx.finish();
    } else {
        let mut page_state = std::mem::take(&mut state.page);
        {
            let mut outer = crate::demo_page::begin_demo_page(
                &mut ctx,
                "Button Demo",
                &mut page_state,
                debug_layout,
                ColumnLayout,
            );
            draw_button_page_content(&mut outer.ctx, state);
            outer.ctx.finish();
        }
        state.page = page_state;
    }

    ctx.finish();

    cmds
}

pub(crate) fn draw_button_page_content<'a, 'b, CF>(
    outer: &'b mut WidgetContext<
        'a,
        SampleTextBackend,
        framewise::layouts::OffsetState<framewise::ColumnState>,
        CF,
    >,
    state: &mut ButtonPageState,
) {
    let theme = outer.theme;

    let primary = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent = ButtonStyle::accent_from_theme(&theme);
    let ghost = ButtonStyle::ghost_from_theme(&theme);

    // ── Section 1: Toolbar row — one button per style ─────────────────────────
    // Nesting: root col > row
    {
        let mut row =
            outer.child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(40.0), RowLayout);

        let styles = [primary, secondary, accent, ghost];
        let labels = ["Primary", "Secondary", "Accent", "Ghost"];
        for i in 0..4 {
            let r = button(
                ButtonSpec::new(labels[i]).style(styles[i]),
                RowLayoutParams::fixed(160.0, 40.0),
                &mut state.toolbar_btns[i],
                &mut row,
            );
            if r.input.clicked {
                state.toolbar_clicks[i] += 1;
            }
            row.spacer(10.0);
        }
        row.finish();
    }
    outer.spacer(24.0);

    // ── Section 2: Style showcase — two side-by-side columns ──────────────────
    // Nesting: root col > row > [col, col]
    // Left col: primary + secondary; right col: accent + ghost; each with disabled variant
    {
        let mut row = outer.child_with_layout(
            ColumnLayoutParams::auto().fill_x().fixed_y(200.0),
            RowLayout,
        );

        {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(260.0, 200.0), ColumnLayout);

            let entries = [
                ("Primary", primary, false),
                ("Primary Disabled", primary, true),
                ("Secondary", secondary, false),
                ("Secondary Disabled", secondary, true),
            ];
            for (i, (text, style, disabled)) in entries.iter().enumerate() {
                button(
                    ButtonSpec::new(text).style(*style).disabled(*disabled),
                    ColumnLayoutParams::fixed(240.0, 44.0),
                    &mut state.style_btns[i],
                    &mut col,
                );
                col.spacer(8.0);
            }
            col.finish();
        }
        row.spacer(30.0);

        {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(260.0, 200.0), ColumnLayout);

            let entries = [
                ("Accent", accent, false),
                ("Accent Disabled", accent, true),
                ("Ghost", ghost, false),
                ("Ghost Disabled", ghost, true),
            ];
            for (i, (text, style, disabled)) in entries.iter().enumerate() {
                button(
                    ButtonSpec::new(text).style(*style).disabled(*disabled),
                    ColumnLayoutParams::fixed(240.0, 44.0),
                    &mut state.style_btns[4 + i],
                    &mut col,
                );
                col.spacer(8.0);
            }
            col.finish();
        }

        row.finish();
    }
    outer.spacer(24.0);

    // ── Section 3: Counter — 4-level nesting ──────────────────────────────────
    // root col > outer_row > col > inner_row
    // Outer row: [label btn | control column]
    // Control column: [dec/display/inc row | reset btn]
    {
        let mut outer_row =
            outer.child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(96.0), RowLayout);

        label(
            &mut outer_row,
            LabelSpecBuilder::new().text("Counter"),
            RowLayoutParams::fixed(100.0, 96.0),
        );
        outer_row.spacer(10.0);

        {
            let mut col =
                outer_row.child_with_layout(RowLayoutParams::fixed(420.0, 96.0), ColumnLayout);

            {
                let mut inner_row =
                    col.child_with_layout(ColumnLayoutParams::fixed(420.0, 48.0), RowLayout);

                let r = button(
                    ButtonSpec::new("−").style(secondary),
                    RowLayoutParams::fixed(120.0, 48.0),
                    &mut state.counter_btns[1],
                    &mut inner_row,
                );
                if r.input.clicked {
                    state.counter -= 1;
                }
                inner_row.spacer(12.0);

                let count_text = state.counter.to_string();
                label(
                    &mut inner_row,
                    LabelSpecBuilder::new().text(&count_text),
                    RowLayoutParams::fixed(120.0, 48.0),
                );
                inner_row.spacer(12.0);

                let r = button(
                    ButtonSpec::new("+").style(accent),
                    RowLayoutParams::fixed(120.0, 48.0),
                    &mut state.counter_btns[3],
                    &mut inner_row,
                );
                if r.input.clicked {
                    state.counter += 1;
                }

                inner_row.finish();
            }
            col.spacer(12.0);

            let r = button(
                ButtonSpec::new("Reset").style(ghost),
                ColumnLayoutParams::fixed(420.0, 36.0),
                &mut state.counter_btns[4],
                &mut col,
            );
            if r.input.clicked {
                state.counter = 0;
            }

            col.finish();
        }

        outer_row.finish();
    }
    outer.spacer(24.0);

    // ── Section 4: Action grid — row containing three styled columns ──────────
    // Nesting: root col > row > [col, col, col]
    {
        let mut row = outer.child_with_layout(
            ColumnLayoutParams::auto().fill_x().fixed_y(124.0),
            RowLayout,
        );

        let group_labels = [
            ["Save", "Save As", "Export"],
            ["Cut", "Copy", "Paste"],
            ["Undo", "Redo", "Clear"],
        ];
        let group_styles = [primary, secondary, accent];

        #[allow(clippy::needless_range_loop)]
        for g in 0..3 {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(180.0, 124.0), ColumnLayout);

            for j in 0..3 {
                let idx = g * 3 + j;
                let r = button(
                    ButtonSpec::new(group_labels[g][j]).style(group_styles[g]),
                    ColumnLayoutParams::fixed(160.0, 36.0),
                    &mut state.grid_btns[idx],
                    &mut col,
                );
                if r.input.clicked {
                    state.grid_clicks[idx] += 1;
                }
                col.spacer(8.0);
            }
            col.finish();
            row.spacer(20.0);
        }

        row.finish();
    }
    outer.spacer(24.0);

    // ── Section 5: Size-request sizing ────────────────────────────────────────
    // Auto-width buttons (each hugs its own label) in a row, then a fill-width
    // button in a column — sized by the layout from the button's size request,
    // no explicit widths.
    {
        let mut col = outer.child_with_layout(
            ColumnLayoutParams::auto().fill_x().fixed_y(96.0),
            ColumnLayout,
        );

        {
            let mut row =
                col.child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(40.0), RowLayout);

            // Each button's width comes from its label; height is fixed.
            let auto = RowLayoutParams::auto().fixed_y(40.0);
            let labels = ["OK", "Cancel", "Apply to All", "Don't Save"];
            for (i, label) in labels.iter().enumerate() {
                button(
                    ButtonSpec::new(label).style(secondary),
                    auto,
                    &mut state.auto_btns[i],
                    &mut row,
                );
                row.spacer(10.0);
            }
            row.finish();
        }
        col.spacer(12.0);

        // A button that fills the column's full width and requests its height.
        let fill = ColumnLayoutParams::auto().fill_x();
        button(
            ButtonSpec::new("Fills the row width").style(primary),
            fill,
            &mut state.auto_btns[4],
            &mut col,
        );

        col.finish();
    }
    outer.spacer(24.0);

    // ── Section 6: Wrapping flow ──────────────────────────────────────────────
    // Auto-width buttons flowed left-to-right, wrapping onto the next line when
    // the row fills.
    {
        let mut wrap = outer.child_with_layout(
            ColumnLayoutParams::auto().fill_x().fixed_y(96.0),
            WrapLayout {
                spacing: 8.0,
                line_spacing: 8.0,
            },
        );

        let auto = Placement2D {
            width: Placement::auto(),
            height: Placement::fixed(32.0),
        };
        let labels = [
            "New",
            "Open",
            "Save",
            "Save As…",
            "Close",
            "Print Preview",
            "Export",
            "Quit",
        ];
        for (i, label) in labels.iter().enumerate() {
            button(
                ButtonSpec::new(label).style(ghost),
                auto,
                &mut state.wrap_btns[i],
                &mut wrap,
            );
        }
        wrap.finish();
    }
    outer.spacer(24.0);

    // ── Section 7: Alignment Showcase ──────────────────────────────────────────
    // Stacks centered and end-aligned columns and rows side-by-side to verify
    // cross-axis alignment math under Exact bounds.
    {
        let mut row = outer.child_with_layout(
            ColumnLayoutParams::auto().fill_x().fixed_y(180.0),
            RowLayout,
        );

        // Sub-column 1: Centered column (Exact width 200, contains 3 different width buttons)
        {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(200.0, 180.0), ColumnLayout);

            button(
                ButtonSpec::new("Center S").style(accent),
                ColumnLayoutParams::fixed(80.0, 36.0).align_x(Align::Center),
                &mut state.align_btns[0],
                &mut col,
            );
            col.spacer(8.0);
            button(
                ButtonSpec::new("Center Med").style(secondary),
                ColumnLayoutParams::fixed(140.0, 36.0).align_x(Align::Center),
                &mut state.align_btns[1],
                &mut col,
            );
            col.spacer(8.0);
            button(
                ButtonSpec::new("Center Lrg").style(primary),
                ColumnLayoutParams::fixed(180.0, 36.0).align_x(Align::Center),
                &mut state.align_btns[2],
                &mut col,
            );

            col.finish();
        }
        row.spacer(20.0);

        // Sub-column 2: End-aligned column (Exact width 200, contains 3 different width buttons)
        {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(200.0, 180.0), ColumnLayout);

            button(
                ButtonSpec::new("End S").style(accent),
                ColumnLayoutParams::fixed(80.0, 36.0).align_x(Align::End),
                &mut state.align_btns[3],
                &mut col,
            );
            col.spacer(8.0);
            button(
                ButtonSpec::new("End Med").style(secondary),
                ColumnLayoutParams::fixed(140.0, 36.0).align_x(Align::End),
                &mut state.align_btns[4],
                &mut col,
            );
            col.spacer(8.0);
            button(
                ButtonSpec::new("End Lrg").style(primary),
                ColumnLayoutParams::fixed(180.0, 36.0).align_x(Align::End),
                &mut state.align_btns[5],
                &mut col,
            );

            col.finish();
        }
        row.spacer(20.0);

        // Sub-column 3: Row alignment demonstration (Vertical alignment)
        // Stacks centered and end-aligned rows nested in a Start-aligned column.
        {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(300.0, 180.0), ColumnLayout);

            // Centered Row (Exact height 60)
            {
                let mut inner_row =
                    col.child_with_layout(ColumnLayoutParams::fixed(300.0, 60.0), RowLayout);
                button(
                    ButtonSpec::new("Row C1").style(primary),
                    RowLayoutParams::fixed(80.0, 30.0).align_y(Align::Center),
                    &mut state.align_btns[6],
                    &mut inner_row,
                );
                inner_row.spacer(8.0);
                button(
                    ButtonSpec::new("Row C2").style(secondary),
                    RowLayoutParams::fixed(100.0, 48.0).align_y(Align::Center),
                    &mut state.align_btns[7],
                    &mut inner_row,
                );
                inner_row.finish();
            }
            col.spacer(12.0);

            // End-Aligned Row (Exact height 60)
            {
                let mut inner_row =
                    col.child_with_layout(ColumnLayoutParams::fixed(300.0, 60.0), RowLayout);
                button(
                    ButtonSpec::new("Row E1").style(primary),
                    RowLayoutParams::fixed(80.0, 30.0).align_y(Align::End),
                    &mut state.align_btns[8],
                    &mut inner_row,
                );
                inner_row.spacer(8.0);
                button(
                    ButtonSpec::new("Row E2").style(secondary),
                    RowLayoutParams::fixed(100.0, 48.0).align_y(Align::End),
                    &mut state.align_btns[9],
                    &mut inner_row,
                );
                inner_row.finish();
            }

            col.finish();
        }

        row.finish();
    }

    // ── Section 8: Content Placement Showcase ────────────────────────────────
    // Fixed-size buttons keep the same widget bounds while moving the prepared
    // text block inside the padded content rect.
    {
        let positions = [
            ("top left", Align::Start, Align::Start),
            ("top center", Align::Center, Align::Start),
            ("top right", Align::End, Align::Start),
            ("middle left", Align::Start, Align::Center),
            ("center", Align::Center, Align::Center),
            ("middle right", Align::End, Align::Center),
            ("bottom left", Align::Start, Align::End),
            ("bottom center", Align::Center, Align::End),
            ("bottom right", Align::End, Align::End),
        ];

        for (row_index, row_positions) in positions.chunks(3).enumerate() {
            let mut row = outer
                .child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(54.0), RowLayout);

            for (col_index, (text, x, y)) in row_positions.iter().enumerate() {
                let index = row_index * 3 + col_index;
                let style = ButtonStyle {
                    content_placement: framewise::TextContentPlacement::logical(
                        framewise::ContentPlacement::Align(*x),
                        framewise::ContentPlacement::Align(*y),
                    ),
                    ..secondary
                };
                button(
                    ButtonSpec::new(text).style(style),
                    RowLayoutParams::fixed(150.0, 48.0),
                    &mut state.content_btns[index],
                    &mut row,
                );
                row.spacer(8.0);
            }

            row.finish();
        }

        let mut row =
            outer.child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(29.0), RowLayout);
        let icon_flow = framewise::text::TextFlow {
            overflow_x: framewise::text::OverflowX::Keep,
            overflow_y: framewise::text::OverflowY::Keep,
            line_align: framewise::text::TextLineAlign::Start,
        };
        let logical_icon = ButtonStyle {
            content_placement: framewise::TextContentPlacement::CENTER,
            text_style: framewise::TextStyle {
                size: 30.0,
                font: theme.sans_font,
                flow: icon_flow,
                ..secondary.text_style
            },
            pad_x: 0.0,
            pad_y: 0.0,
            ..secondary
        };
        let ink_icon = ButtonStyle {
            content_placement: framewise::TextContentPlacement::INK_CENTER,
            ..logical_icon
        };
        let comparison_label = LabelStyle {
            content_placement: framewise::TextContentPlacement::CENTER,
            ..LabelStyle::from_theme(&theme)
        };
        label(
            &mut row,
            LabelSpecBuilder::new()
                .text("logical center:")
                .style(comparison_label),
            RowLayoutParams::fixed(92.0, 29.0),
        );
        row.spacer(6.0);
        button(
            ButtonSpec::new("×").style(logical_icon),
            RowLayoutParams::fixed(29.0, 29.0),
            &mut state.content_btns[9],
            &mut row,
        );
        row.spacer(18.0);
        label(
            &mut row,
            LabelSpecBuilder::new()
                .text("approx ink center:")
                .style(comparison_label),
            RowLayoutParams::fixed(72.0, 29.0),
        );
        row.spacer(6.0);
        button(
            ButtonSpec::new("×").style(ink_icon),
            RowLayoutParams::fixed(29.0, 29.0),
            &mut state.content_btns[10],
            &mut row,
        );
        row.finish();
    }
}
