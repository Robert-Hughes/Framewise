use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Extent, SizeReq},
    layouts::{ColumnLayout, CrossAlign, RowLayout, WrapLayout},
    theme::Theme,
    types::{Rect, Vec2},
    widget::WidgetContext,
    widgets::button::{button, ButtonSpecBuilder, ButtonState, ButtonStyle},
};

// ── State ──────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ButtonPageState {
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
    // Section 5: intrinsic sizing — auto-width row + fill-width column
    pub intrinsic_btns: [ButtonState; 5],
    // Section 6: wrapping flow of auto-width buttons
    pub wrap_btns: [ButtonState; 8],
    // Section 7: alignment showcase
    pub align_btns: [ButtonState; 12],
}

// ── Draw ──────────────────────────────────────────────────────────────────────

pub fn draw_button_page(
    state: &mut ButtonPageState,
    focus_system: &mut FocusSystem,
    input: &Input,
    _time: f64,
    win_size: (f32, f32),
    text_system: &mut SampleTextSystem,
    debug_layout: bool,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let pad = 20.0;

    let mut cmds = framewise::DrawCommands::new();
    let mut ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        framewise::layouts::ManualLayout,
        Rect::new(0.0, 0.0, win_w, win_h),
        &mut cmds,
    );
    ctx.debug_layout = debug_layout;

    // Root column — all sections stack vertically with 24px gaps
    let mut outer = ctx.child_with_layout(
        Rect::new(pad, pad, win_w - 2.0 * pad, win_h - 2.0 * pad),
        ColumnLayout {
            spacing: 24.0,
            align: CrossAlign::Start,
        },
    );

    let theme = outer.theme;
    let primary = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent = ButtonStyle::accent_from_theme(&theme);
    let ghost = ButtonStyle::ghost_from_theme(&theme);

    // ── Section 1: Toolbar row — one button per style ─────────────────────────
    // Nesting: root col > row
    {
        let mut row = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 40.0).into(),
            RowLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            },
        );

        let styles = [primary, secondary, accent, ghost];
        let labels = ["Primary", "Secondary", "Accent", "Ghost"];
        for i in 0..4 {
            let r = button(
                &mut row,
                ButtonSpecBuilder::new().text(labels[i]).style(styles[i]),
                Vec2::new(160.0, 40.0).into(),
                &mut state.toolbar_btns[i],
            );
            if r.input.clicked {
                state.toolbar_clicks[i] += 1;
            }
        }
        row.finish();
    }

    // ── Section 2: Style showcase — two side-by-side columns ──────────────────
    // Nesting: root col > row > [col, col]
    // Left col: primary + secondary; right col: accent + ghost; each with disabled variant
    {
        let mut row = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 200.0).into(),
            RowLayout {
                spacing: 30.0,
                align: CrossAlign::Start,
            },
        );

        {
            let mut col = row.child_with_layout(
                Vec2::new(260.0, 200.0).into(),
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );

            let entries = [
                ("Primary", primary, false),
                ("Primary Disabled", primary, true),
                ("Secondary", secondary, false),
                ("Secondary Disabled", secondary, true),
            ];
            for (i, (text, style, disabled)) in entries.iter().enumerate() {
                button(
                    &mut col,
                    ButtonSpecBuilder::new()
                        .text(text)
                        .style(*style)
                        .disabled(*disabled),
                    Vec2::new(240.0, 44.0).into(),
                    &mut state.style_btns[i],
                );
            }
            col.finish();
        }

        {
            let mut col = row.child_with_layout(
                Vec2::new(260.0, 200.0).into(),
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );

            let entries = [
                ("Accent", accent, false),
                ("Accent Disabled", accent, true),
                ("Ghost", ghost, false),
                ("Ghost Disabled", ghost, true),
            ];
            for (i, (text, style, disabled)) in entries.iter().enumerate() {
                button(
                    &mut col,
                    ButtonSpecBuilder::new()
                        .text(text)
                        .style(*style)
                        .disabled(*disabled),
                    Vec2::new(240.0, 44.0).into(),
                    &mut state.style_btns[4 + i],
                );
            }
            col.finish();
        }

        row.finish();
    }

    // ── Section 3: Counter — 4-level nesting ──────────────────────────────────
    // root col > outer_row > col > inner_row
    // Outer row: [label btn | control column]
    // Control column: [dec/display/inc row | reset btn]
    {
        let mut outer_row = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 96.0).into(),
            RowLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            },
        );

        button(
            &mut outer_row,
            ButtonSpecBuilder::new().text("Counter").style(ghost),
            Vec2::new(100.0, 96.0).into(),
            &mut state.counter_btns[0],
        );

        {
            let mut col = outer_row.child_with_layout(
                Vec2::new(420.0, 96.0).into(),
                ColumnLayout {
                    spacing: 12.0,
                    align: CrossAlign::Start,
                },
            );

            {
                let mut inner_row = col.child_with_layout(
                    Vec2::new(420.0, 48.0).into(),
                    RowLayout {
                        spacing: 12.0,
                        align: CrossAlign::Start,
                    },
                );

                let r = button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("−").style(secondary),
                    Vec2::new(120.0, 48.0).into(),
                    &mut state.counter_btns[1],
                );
                if r.input.clicked {
                    state.counter -= 1;
                }

                let count_text = state.counter.to_string();
                button(
                    &mut inner_row,
                    ButtonSpecBuilder::new()
                        .text(&count_text)
                        .style(ghost)
                        .disabled(true),
                    Vec2::new(120.0, 48.0).into(),
                    &mut state.counter_btns[2],
                );

                let r = button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("+").style(accent),
                    Vec2::new(120.0, 48.0).into(),
                    &mut state.counter_btns[3],
                );
                if r.input.clicked {
                    state.counter += 1;
                }

                inner_row.finish();
            }

            let r = button(
                &mut col,
                ButtonSpecBuilder::new().text("Reset").style(ghost),
                Vec2::new(420.0, 36.0).into(),
                &mut state.counter_btns[4],
            );
            if r.input.clicked {
                state.counter = 0;
            }

            col.finish();
        }

        outer_row.finish();
    }

    // ── Section 4: Action grid — row containing three styled columns ──────────
    // Nesting: root col > row > [col, col, col]
    {
        let mut row = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 124.0).into(),
            RowLayout {
                spacing: 20.0,
                align: CrossAlign::Start,
            },
        );

        let group_labels = [
            ["Save", "Save As", "Export"],
            ["Cut", "Copy", "Paste"],
            ["Undo", "Redo", "Clear"],
        ];
        let group_styles = [primary, secondary, accent];

        #[allow(clippy::needless_range_loop)]
        for g in 0..3 {
            let mut col = row.child_with_layout(
                Vec2::new(180.0, 124.0).into(),
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );

            for j in 0..3 {
                let idx = g * 3 + j;
                let r = button(
                    &mut col,
                    ButtonSpecBuilder::new()
                        .text(group_labels[g][j])
                        .style(group_styles[g]),
                    Vec2::new(160.0, 36.0).into(),
                    &mut state.grid_btns[idx],
                );
                if r.input.clicked {
                    state.grid_clicks[idx] += 1;
                }
            }
            col.finish();
        }

        row.finish();
    }

    // ── Section 5: Intrinsic sizing ───────────────────────────────────────────
    // Auto-width buttons (each hugs its own label) in a row, then a fill-width
    // button in a column — sized by the layout from the button's intrinsic size,
    // no explicit widths.
    {
        let mut col = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 96.0).into(),
            ColumnLayout {
                spacing: 12.0,
                align: CrossAlign::Start,
            },
        );

        {
            let mut row = col.child_with_layout(
                Vec2::new(win_w - 2.0 * pad, 40.0).into(),
                RowLayout {
                    spacing: 10.0,
                    align: CrossAlign::Start,
                },
            );

            // Each button's width comes from its label; height is fixed.
            let auto = SizeReq {
                width: Extent::Auto,
                height: Extent::Fixed(40.0),
            };
            let labels = ["OK", "Cancel", "Apply to All", "Don't Save"];
            for (i, label) in labels.iter().enumerate() {
                button(
                    &mut row,
                    ButtonSpecBuilder::new().text(label).style(secondary),
                    auto,
                    &mut state.intrinsic_btns[i],
                );
            }
            row.finish();
        }

        // A button that fills the column's full width, intrinsic height.
        let fill = SizeReq {
            width: Extent::Fill,
            height: Extent::Auto,
        };
        button(
            &mut col,
            ButtonSpecBuilder::new()
                .text("Fills the row width")
                .style(primary),
            fill,
            &mut state.intrinsic_btns[4],
        );

        col.finish();
    }

    // ── Section 6: Wrapping flow ──────────────────────────────────────────────
    // Auto-width buttons flowed left-to-right, wrapping onto the next line when
    // the row fills.
    {
        let mut wrap = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 96.0).into(),
            WrapLayout {
                spacing: 8.0,
                line_spacing: 8.0,
            },
        );

        let auto = SizeReq {
            width: Extent::Auto,
            height: Extent::Fixed(32.0),
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
                &mut wrap,
                ButtonSpecBuilder::new().text(label).style(ghost),
                auto,
                &mut state.wrap_btns[i],
            );
        }
        wrap.finish();
    }

    // ── Section 7: Alignment Showcase ──────────────────────────────────────────
    // Stacks centered and end-aligned columns and rows side-by-side to verify
    // cross-axis alignment math under Exact bounds.
    {
        let mut row = outer.child_with_layout(
            Vec2::new(win_w - 2.0 * pad, 180.0).into(),
            RowLayout {
                spacing: 20.0,
                align: CrossAlign::Start,
            },
        );

        // Sub-column 1: Centered column (Exact width 200, contains 3 different width buttons)
        {
            let mut col = row.child_with_layout(
                Vec2::new(200.0, 180.0).into(),
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::Center,
                },
            );

            button(
                &mut col,
                ButtonSpecBuilder::new().text("Center S").style(accent),
                Vec2::new(80.0, 36.0).into(),
                &mut state.align_btns[0],
            );
            button(
                &mut col,
                ButtonSpecBuilder::new().text("Center Med").style(secondary),
                Vec2::new(140.0, 36.0).into(),
                &mut state.align_btns[1],
            );
            button(
                &mut col,
                ButtonSpecBuilder::new().text("Center Lrg").style(primary),
                Vec2::new(180.0, 36.0).into(),
                &mut state.align_btns[2],
            );

            col.finish();
        }

        // Sub-column 2: End-aligned column (Exact width 200, contains 3 different width buttons)
        {
            let mut col = row.child_with_layout(
                Vec2::new(200.0, 180.0).into(),
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::End,
                },
            );

            button(
                &mut col,
                ButtonSpecBuilder::new().text("End S").style(accent),
                Vec2::new(80.0, 36.0).into(),
                &mut state.align_btns[3],
            );
            button(
                &mut col,
                ButtonSpecBuilder::new().text("End Med").style(secondary),
                Vec2::new(140.0, 36.0).into(),
                &mut state.align_btns[4],
            );
            button(
                &mut col,
                ButtonSpecBuilder::new().text("End Lrg").style(primary),
                Vec2::new(180.0, 36.0).into(),
                &mut state.align_btns[5],
            );

            col.finish();
        }

        // Sub-column 3: Row alignment demonstration (Vertical alignment)
        // Stacks centered and end-aligned rows nested in a Start-aligned column.
        {
            let mut col = row.child_with_layout(
                Vec2::new(300.0, 180.0).into(),
                ColumnLayout {
                    spacing: 12.0,
                    align: CrossAlign::Start,
                },
            );

            // Centered Row (Exact height 60)
            {
                let mut inner_row = col.child_with_layout(
                    Vec2::new(300.0, 60.0).into(),
                    RowLayout {
                        spacing: 8.0,
                        align: CrossAlign::Center,
                    },
                );
                button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("Row C1").style(primary),
                    Vec2::new(80.0, 30.0).into(),
                    &mut state.align_btns[6],
                );
                button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("Row C2").style(secondary),
                    Vec2::new(100.0, 48.0).into(),
                    &mut state.align_btns[7],
                );
                inner_row.finish();
            }

            // End-Aligned Row (Exact height 60)
            {
                let mut inner_row = col.child_with_layout(
                    Vec2::new(300.0, 60.0).into(),
                    RowLayout {
                        spacing: 8.0,
                        align: CrossAlign::End,
                    },
                );
                button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("Row E1").style(primary),
                    Vec2::new(80.0, 30.0).into(),
                    &mut state.align_btns[8],
                );
                button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("Row E2").style(secondary),
                    Vec2::new(100.0, 48.0).into(),
                    &mut state.align_btns[9],
                );
                inner_row.finish();
            }

            col.finish();
        }

        row.finish();
    }

    outer.finish();

    cmds
}
