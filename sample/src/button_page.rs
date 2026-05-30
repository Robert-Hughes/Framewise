use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{ColumnLayout, Layout, RowLayout},
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
}

// ── Draw ──────────────────────────────────────────────────────────────────────

pub fn draw_button_page(
    state: &mut ButtonPageState,
    focus_system: &mut FocusSystem,
    input: &Input,
    _time: f64,
    win_size: (f32, f32),
    text_system: &mut SampleTextSystem,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let pad = 20.0;

    let mut cmds = framewise::DrawCommands::new();
    let mut ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        framewise::layout::ManualLayout.begin(Rect::new(0.0, 0.0, win_w, win_h)),
        &mut cmds,
    );

    // Root column — all sections stack vertically with 24px gaps
    let mut outer = ctx.child_with_layout(
        Rect::new(pad, pad, win_w - 2.0 * pad, win_h - 2.0 * pad),
        ColumnLayout { spacing: 24.0 },
    );

    let theme = outer.theme.clone();
    let primary   = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent    = ButtonStyle::accent_from_theme(&theme);
    let ghost     = ButtonStyle::ghost_from_theme(&theme);

    // ── Section 1: Toolbar row — one button per style ─────────────────────────
    // Nesting: root col > row
    {
        let mut row =
            outer.child_with_layout(Vec2::new(win_w - 2.0 * pad, 40.0), RowLayout { spacing: 10.0 });

        let styles = [primary, secondary, accent, ghost];
        let labels = ["Primary", "Secondary", "Accent", "Ghost"];
        for i in 0..4 {
            let r = button(
                &mut row,
                ButtonSpecBuilder::new().text(labels[i]).style(styles[i]),
                Vec2::new(160.0, 40.0),
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
        let mut row = outer
            .child_with_layout(Vec2::new(win_w - 2.0 * pad, 200.0), RowLayout { spacing: 30.0 });

        {
            let mut col =
                row.child_with_layout(Vec2::new(260.0, 200.0), ColumnLayout { spacing: 8.0 });

            let entries = [
                ("Primary",            primary,   false),
                ("Primary Disabled",   primary,   true),
                ("Secondary",          secondary, false),
                ("Secondary Disabled", secondary, true),
            ];
            for (i, (text, style, disabled)) in entries.iter().enumerate() {
                button(
                    &mut col,
                    ButtonSpecBuilder::new().text(text).style(*style).disabled(*disabled),
                    Vec2::new(240.0, 44.0),
                    &mut state.style_btns[i],
                );
            }
            col.finish();
        }

        {
            let mut col =
                row.child_with_layout(Vec2::new(260.0, 200.0), ColumnLayout { spacing: 8.0 });

            let entries = [
                ("Accent",          accent, false),
                ("Accent Disabled", accent, true),
                ("Ghost",           ghost,  false),
                ("Ghost Disabled",  ghost,  true),
            ];
            for (i, (text, style, disabled)) in entries.iter().enumerate() {
                button(
                    &mut col,
                    ButtonSpecBuilder::new().text(text).style(*style).disabled(*disabled),
                    Vec2::new(240.0, 44.0),
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
        let mut outer_row =
            outer.child_with_layout(Vec2::new(win_w - 2.0 * pad, 96.0), RowLayout { spacing: 10.0 });

        button(
            &mut outer_row,
            ButtonSpecBuilder::new().text("Counter").style(ghost),
            Vec2::new(100.0, 96.0),
            &mut state.counter_btns[0],
        );

        {
            let mut col =
                outer_row.child_with_layout(Vec2::new(420.0, 96.0), ColumnLayout { spacing: 12.0 });

            {
                let mut inner_row =
                    col.child_with_layout(Vec2::new(420.0, 48.0), RowLayout { spacing: 12.0 });

                let r = button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("−").style(secondary),
                    Vec2::new(120.0, 48.0),
                    &mut state.counter_btns[1],
                );
                if r.input.clicked {
                    state.counter -= 1;
                }

                let count_text = state.counter.to_string();
                button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text(&count_text).style(ghost).disabled(true),
                    Vec2::new(120.0, 48.0),
                    &mut state.counter_btns[2],
                );

                let r = button(
                    &mut inner_row,
                    ButtonSpecBuilder::new().text("+").style(accent),
                    Vec2::new(120.0, 48.0),
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
                Vec2::new(420.0, 36.0),
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
        let mut row = outer
            .child_with_layout(Vec2::new(win_w - 2.0 * pad, 124.0), RowLayout { spacing: 20.0 });

        let group_labels = [
            ["Save",  "Save As", "Export"],
            ["Cut",   "Copy",    "Paste"],
            ["Undo",  "Redo",    "Clear"],
        ];
        let group_styles = [primary, secondary, accent];

        for g in 0..3 {
            let mut col =
                row.child_with_layout(Vec2::new(180.0, 124.0), ColumnLayout { spacing: 8.0 });

            for j in 0..3 {
                let idx = g * 3 + j;
                let r = button(
                    &mut col,
                    ButtonSpecBuilder::new().text(group_labels[g][j]).style(group_styles[g]),
                    Vec2::new(160.0, 36.0),
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

    outer.finish();

    cmds
}
