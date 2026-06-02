//! Layout demo page.
//!
//! Showcases the *chrome-less nested layout auto-sizing* introduced on the
//! `intrinsic-sizing-nested-layout-auto` branch: a bare `child_with_layout`
//! (Column / Row / Wrap — no `frame` wrapper) placed with `Extent::Auto` /
//! `Extent::Fill` now fits to its children, instead of falling back / panicking.
//! `Extent::Fixed` slots stay identical to the old eager path.
//!
//! Sections:
//!   A. Auto-height column (no frame) — fits its stacked rows.
//!   B. Auto-width row (no frame)     — hugs the total width of its children.
//!   C. Nested auto-in-auto           — fit-to-children chaining through levels.
//!   D. Intrinsic `Auto` sizing       — each button hugs its own label width.
//!   E. Cross-axis alignment          — Start / Center / End inside fit columns.
//!   F. Fixed vs Auto equivalence     — Fixed ignores child extent, Auto fits.
//!   G. WrapLayout flow               — tags wrap onto new lines, auto-sizing.
//!   H. SplitRow                      — width divided into equal declared cells.
//!   I. Mixed per-axis                — fixed icon + Auto-width labels in one row.
//!   J. Toolbar (emit-reorder)        — search fills leftover, buttons stay intrinsic.
//!   K. `AtMost` ceiling              — nested Auto container clamps to the remainder.
//!   L. RowLayout cross-align         — differing heights aligned in a tall row.

use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Extent, SizeReq},
    layouts::{ColumnLayout, CrossAlign, ManualLayout, RowLayout, SplitRow, WrapLayout},
    theme::Theme,
    types::{Rect, Vec2},
    widget::WidgetContext,
    widgets::button::{
        button, raw::calc_button_intrinsic_size, ButtonSpecBuilder, ButtonState, ButtonStyle,
    },
};

// ── State ──────────────────────────────────────────────────────────────────────

pub struct LayoutDemoState {
    pub a_btns: [ButtonState; 3], // A: auto-height column
    pub a_clicks: [u32; 3],
    pub b_btns: [ButtonState; 3],       // B: auto-width row
    pub c_btns: [ButtonState; 4],       // C: nested auto-in-auto (2 rows × 2)
    pub d_btns: [ButtonState; 3],       // D: intrinsic Auto widths
    pub e_btns: [ButtonState; 9],       // E: 3 align columns × 3 buttons
    pub f_fixed_btns: [ButtonState; 3], // F: fixed-height column
    pub f_auto_btns: [ButtonState; 3],  // F: auto-height column
    pub g_btns: [ButtonState; 8],       // G: wrap tags
    pub h_btns: [ButtonState; 3],       // H: SplitRow equal thirds
    pub h_clicks: [u32; 3],
    pub i_btns: [ButtonState; 3], // I: mixed per-axis (fixed icon + auto labels)
    pub j_search: ButtonState,    // J: toolbar emit-reorder (fill field)
    pub j_btns: [ButtonState; 2], // J: intrinsic right-aligned buttons
    pub k_btns: [ButtonState; 3], // K: AtMost shrink-wrap (fixed block + short hugs + long clamps)
    pub l_btns: [ButtonState; 9], // L: RowLayout cross-align (3 rows × 3)
}

impl Default for LayoutDemoState {
    fn default() -> Self {
        Self {
            a_btns: Default::default(),
            a_clicks: [0; 3],
            b_btns: Default::default(),
            c_btns: Default::default(),
            d_btns: Default::default(),
            e_btns: Default::default(),
            f_fixed_btns: Default::default(),
            f_auto_btns: Default::default(),
            g_btns: Default::default(),
            h_btns: Default::default(),
            h_clicks: [0; 3],
            i_btns: Default::default(),
            j_search: Default::default(),
            j_btns: Default::default(),
            k_btns: Default::default(),
            l_btns: Default::default(),
        }
    }
}

// ── Draw ──────────────────────────────────────────────────────────────────────

pub fn draw_layout_page(
    state: &mut LayoutDemoState,
    focus_system: &mut FocusSystem,
    input: &Input,
    _time: f64,
    win_size: (f32, f32),
    text_system: &mut SampleTextSystem,
    debug_layout: bool,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let pad = 20.0;
    let col_w = (win_w - 2.0 * pad - 30.0) * 0.5;

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
    ctx.debug_layout = debug_layout; // F12 toggles the magenta layout-bounds overlay.

    let theme = ctx.theme;
    let primary = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent = ButtonStyle::accent_from_theme(&theme);
    let mut ghost = ButtonStyle::ghost_from_theme(&theme);
    ghost.disabled_alpha = 1.0; // labels/headings stay readable

    // Root row: two columns side by side.
    let mut root_row = ctx.child_with_layout(
        Rect::new(pad, pad, win_w - 2.0 * pad, win_h - 2.0 * pad),
        RowLayout {
            spacing: 30.0,
            align: CrossAlign::Start,
        },
    );

    // ── Left column: A, B, C, D ───────────────────────────────────────────────
    {
        let mut left = root_row.child_with_layout(
            Vec2::new(col_w, win_h - 2.0 * pad).into(),
            ColumnLayout {
                spacing: 18.0,
                align: CrossAlign::Start,
            },
        );

        heading(&mut left, ghost, "Chrome-less nested layout auto-sizing");

        // A. Auto-height column (no frame).
        subheading(
            &mut left,
            ghost,
            "A. Auto-height column (bare ColumnLayout, height: Auto)",
        );
        {
            // A bare column placed with Auto height fits to its three rows — no
            // frame chrome involved. The following sibling lands right below it.
            let mut auto_col = left.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Auto,
                },
                ColumnLayout {
                    spacing: 6.0,
                    align: CrossAlign::Start,
                },
            );
            for i in 0..3 {
                let text = format!("Auto-column row #{} (clicks: {})", i + 1, state.a_clicks[i]);
                let style = [primary, secondary, accent][i];
                let r = button(
                    &mut auto_col,
                    ButtonSpecBuilder::new().text(&text).style(style),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(34.0),
                    },
                    &mut state.a_btns[i],
                );
                if r.input.clicked {
                    state.a_clicks[i] += 1;
                }
            }
            auto_col.finish();
        }

        // B. Auto-width row (no frame).
        subheading(
            &mut left,
            ghost,
            "B. Auto-width row (bare RowLayout, width: Auto) — hugs its children",
        );
        {
            let mut auto_row = left.child_with_layout(
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Fixed(40.0),
                },
                RowLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );
            for (i, label) in ["One", "Two", "Three"].iter().enumerate() {
                button(
                    &mut auto_row,
                    ButtonSpecBuilder::new().text(label).style(secondary),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fill,
                    },
                    &mut state.b_btns[i],
                );
            }
            auto_row.finish();
        }

        // C. Nested auto-in-auto.
        subheading(
            &mut left,
            ghost,
            "C. Nested auto-in-auto (auto column of auto rows of auto buttons)",
        );
        {
            let mut outer = left.child_with_layout(
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Auto,
                },
                ColumnLayout {
                    spacing: 6.0,
                    align: CrossAlign::Start,
                },
            );
            let labels = [["First", "Second"], ["Third row item", "Fourth"]];
            for (row_idx, pair) in labels.iter().enumerate() {
                let mut inner_row = outer.child_with_layout(
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Auto,
                    },
                    RowLayout {
                        spacing: 6.0,
                        align: CrossAlign::Start,
                    },
                );
                for (col_idx, label) in pair.iter().enumerate() {
                    let idx = row_idx * 2 + col_idx;
                    button(
                        &mut inner_row,
                        ButtonSpecBuilder::new().text(label).style(primary),
                        SizeReq::auto(),
                        &mut state.c_btns[idx],
                    );
                }
                inner_row.finish();
            }
            outer.finish();
        }

        // D. Intrinsic Auto sizing.
        subheading(
            &mut left,
            ghost,
            "D. Intrinsic Auto — each button hugs its own label width",
        );
        {
            let mut row = left.child_with_layout(
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Fixed(40.0),
                },
                RowLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );
            for (i, label) in ["Go", "Cancel", "Save all changes now"].iter().enumerate() {
                let style = [primary, secondary, accent][i];
                button(
                    &mut row,
                    ButtonSpecBuilder::new().text(label).style(style),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fill,
                    },
                    &mut state.d_btns[i],
                );
            }
            row.finish();
        }

        // I. Mixed per-axis: fixed-width icon + intrinsic-width labels in one row.
        subheading(
            &mut left,
            ghost,
            "I. Mixed per-axis — fixed icon + Auto-width labels in one row",
        );
        {
            let mut row = left.child_with_layout(
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Fixed(40.0),
                },
                RowLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );
            // Fixed 40px square "icon" — width imposed, ignores its label extent.
            button(
                &mut row,
                ButtonSpecBuilder::new().text("*").style(accent),
                SizeReq {
                    width: Extent::Fixed(40.0),
                    height: Extent::Fill,
                },
                &mut state.i_btns[0],
            );
            // Two Auto-width labels each hug their own text — different axis policy
            // than the icon, in the same row.
            for (i, label) in ["Intrinsic label", "Another"].iter().enumerate() {
                button(
                    &mut row,
                    ButtonSpecBuilder::new().text(label).style(secondary),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fill,
                    },
                    &mut state.i_btns[i + 1],
                );
            }
            row.finish();
        }

        // L. RowLayout cross-axis alignment (Start / Center / End in a tall row).
        subheading(
            &mut left,
            ghost,
            "L. RowLayout cross-align — differing heights aligned in a 60px row",
        );
        {
            let aligns = [
                ("Start", CrossAlign::Start),
                ("Center", CrossAlign::Center),
                ("End", CrossAlign::End),
            ];
            let heights = [22.0, 36.0, 50.0];
            let styles = [primary, secondary, accent];
            for (row_idx, (name, align)) in aligns.into_iter().enumerate() {
                // Fixed 60px height is the Exact cross axis that alignment requires;
                // each button picks a different height so the alignment is visible.
                let mut row = left.child_with_layout(
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(60.0),
                    },
                    RowLayout {
                        spacing: 6.0,
                        align,
                    },
                );
                for j in 0..3 {
                    let idx = row_idx * 3 + j;
                    button(
                        &mut row,
                        ButtonSpecBuilder::new()
                            .text(&format!("{name} {}", j + 1))
                            .style(styles[j]),
                        SizeReq {
                            width: Extent::Auto,
                            height: Extent::Fixed(heights[j]),
                        },
                        &mut state.l_btns[idx],
                    );
                }
                row.finish();
            }
        }

        left.finish();
    }

    // ── Right column: E, F, G ──────────────────────────────────────────────────
    {
        let mut right = root_row.child_with_layout(
            Vec2::new(col_w, win_h - 2.0 * pad).into(),
            ColumnLayout {
                spacing: 18.0,
                align: CrossAlign::Start,
            },
        );

        heading(&mut right, ghost, "Alignment, equivalence & flow");

        // E. Cross-axis alignment.
        subheading(
            &mut right,
            ghost,
            "E. Cross-axis alignment (Start / Center / End in fit columns)",
        );
        {
            let aligns = [
                ("Start", CrossAlign::Start),
                ("Center", CrossAlign::Center),
                ("End", CrossAlign::End),
            ];
            for (col_idx, (name, align)) in aligns.into_iter().enumerate() {
                subheading(&mut right, ghost, &format!("  CrossAlign::{name}"));
                // Fill width gives the column an Exact cross axis, which alignment
                // requires; Auto height fits the three differently-sized buttons.
                let mut col = right.child_with_layout(
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Auto,
                    },
                    ColumnLayout {
                        spacing: 5.0,
                        align,
                    },
                );
                let widths = [120.0, 220.0, 320.0];
                let styles = [primary, secondary, accent];
                for j in 0..3 {
                    let idx = col_idx * 3 + j;
                    button(
                        &mut col,
                        ButtonSpecBuilder::new()
                            .text(&format!("{name} {}", j + 1))
                            .style(styles[j]),
                        Vec2::new(widths[j], 30.0).into(),
                        &mut state.e_btns[idx],
                    );
                }
                col.finish();
            }
        }

        // F. Fixed vs Auto equivalence.
        subheading(
            &mut right,
            ghost,
            "F. Fixed height ignores child extent; Auto height fits it",
        );
        {
            let mut pair_row = right.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Fixed(150.0),
                },
                RowLayout {
                    spacing: 16.0,
                    align: CrossAlign::Start,
                },
            );

            // Fixed-height column: clips/overflows past its committed 60px slot.
            let mut fixed_col = pair_row.child_with_layout(
                SizeReq {
                    width: Extent::Fixed(col_w * 0.5 - 8.0),
                    height: Extent::Fixed(60.0),
                },
                ColumnLayout {
                    spacing: 4.0,
                    align: CrossAlign::Start,
                },
            );
            for i in 0..3 {
                button(
                    &mut fixed_col,
                    ButtonSpecBuilder::new()
                        .text(&format!("Fixed {}", i + 1))
                        .style(secondary),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(34.0),
                    },
                    &mut state.f_fixed_btns[i],
                );
            }
            fixed_col.finish();

            // Auto-height column: same children, fits all three.
            let mut auto_col = pair_row.child_with_layout(
                SizeReq {
                    width: Extent::Fixed(col_w * 0.5 - 8.0),
                    height: Extent::Auto,
                },
                ColumnLayout {
                    spacing: 4.0,
                    align: CrossAlign::Start,
                },
            );
            for i in 0..3 {
                button(
                    &mut auto_col,
                    ButtonSpecBuilder::new()
                        .text(&format!("Auto {}", i + 1))
                        .style(accent),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(34.0),
                    },
                    &mut state.f_auto_btns[i],
                );
            }
            auto_col.finish();

            pair_row.finish();
        }

        // G. WrapLayout flow.
        subheading(
            &mut right,
            ghost,
            "G. WrapLayout — tags flow onto new lines, height auto-sizes",
        );
        {
            let mut wrap = right.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Auto,
                },
                WrapLayout {
                    spacing: 6.0,
                    line_spacing: 6.0,
                },
            );
            let tags = [
                "rust",
                "layout",
                "intrinsic",
                "auto-size",
                "nested",
                "wrap",
                "demo",
                "framewise",
            ];
            for (i, tag) in tags.iter().enumerate() {
                button(
                    &mut wrap,
                    ButtonSpecBuilder::new().text(tag).style(primary),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fixed(30.0),
                    },
                    &mut state.g_btns[i],
                );
            }
            wrap.finish();
        }

        // H. SplitRow — declared equal thirds (Phase 4).
        subheading(
            &mut right,
            ghost,
            "H. SplitRow — width divided into 3 equal cells (declared count)",
        );
        {
            // count = 3 known up front, so each cell is exactly a third of the
            // (Exact) row width. Children declare only their cross-axis (height).
            let mut split = right.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Fixed(40.0),
                },
                SplitRow {
                    count: 3,
                    spacing: 10.0,
                    align: CrossAlign::Start,
                },
            );
            let styles = [primary, secondary, accent];
            for i in 0..3 {
                let text = format!("Third #{} ({})", i + 1, state.h_clicks[i]);
                let r = button(
                    &mut split,
                    ButtonSpecBuilder::new().text(&text).style(styles[i]),
                    Extent::Fill, // fill the cell height
                    &mut state.h_btns[i],
                );
                if r.input.clicked {
                    state.h_clicks[i] += 1;
                }
            }
            split.finish();
        }

        // J. Toolbar leftover via the emit-reorder trick: the intrinsic right-hand
        // buttons are measured and placed first, the search field fills the gap,
        // then override_next restores logical left→right focus.
        subheading(
            &mut right,
            ghost,
            "J. Toolbar — search fills leftover, buttons stay intrinsic (emit-reorder)",
        );
        {
            let h = 36.0;
            let spacing = 8.0;
            let w = col_w; // Fill width under the Exact right column resolves to col_w.

            // Measure the two intrinsic buttons up front — the reorder trick needs
            // their sizes before the fill child can be placed.
            let measure = |ts: &mut SampleTextSystem, label: &str| {
                let spec = ButtonSpecBuilder::new()
                    .text(label)
                    .style(secondary)
                    .rect(Rect::PLACEHOLDER)
                    .clip_rect(None)
                    .defaults_from_theme(&theme)
                    .build();
                calc_button_intrinsic_size(&spec, ts).preferred.unwrap().x
            };
            let w_filter = measure(right.text_system, "Filter");
            let w_sort = measure(right.text_system, "Sort");
            let x_sort = w - w_sort;
            let x_filter = x_sort - spacing - w_filter;
            let search_w = (x_filter - spacing).max(0.0);

            // ManualLayout: rects are origin-relative to the toolbar's top-left.
            let mut tb = right.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Fixed(h),
                },
                ManualLayout,
            );
            // Emit the intrinsic (right-hand) children first — they depend on no
            // sibling, so their position is known immediately.
            button(
                &mut tb,
                ButtonSpecBuilder::new().text("Filter").style(secondary),
                Rect::new(x_filter, 0.0, w_filter, h),
                &mut state.j_btns[0],
            );
            button(
                &mut tb,
                ButtonSpecBuilder::new().text("Sort").style(secondary),
                Rect::new(x_sort, 0.0, w_sort, h),
                &mut state.j_btns[1],
            );
            // Then the fill child at the computed remainder.
            button(
                &mut tb,
                ButtonSpecBuilder::new().text("Search...").style(primary),
                Rect::new(0.0, 0.0, search_w, h),
                &mut state.j_search,
            );
            tb.finish();

            // Emitted right-first; restore logical left→right focus order.
            right
                .focus_system
                .override_next(state.j_search.focus_id, state.j_btns[0].focus_id);
            right
                .focus_system
                .override_next(state.j_btns[0].focus_id, state.j_btns[1].focus_id);
        }

        // K. AtMost ceiling: a nested Auto-width container is handed AtMost(remaining)
        // by the parent row, so its children shrink-wrap but clamp at that ceiling.
        subheading(
            &mut right,
            ghost,
            "K. AtMost — nested Auto container caps children at the leftover ceiling",
        );
        {
            let mut row = right.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Auto,
                },
                RowLayout {
                    spacing: 12.0,
                    align: CrossAlign::Start,
                },
            );
            // Fixed block eats 55% of the row width.
            button(
                &mut row,
                ButtonSpecBuilder::new().text("Fixed 55%").style(secondary),
                SizeReq {
                    width: Extent::Fixed(col_w * 0.55),
                    height: Extent::Fixed(70.0),
                },
                &mut state.k_btns[0],
            );
            // Nested Auto-width column → receives AtMost(remaining ~45%). Inside, the
            // short label hugs its text; the long one clamps to the AtMost ceiling.
            let mut nested = row.child_with_layout(
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Auto,
                },
                ColumnLayout {
                    spacing: 6.0,
                    align: CrossAlign::Start,
                },
            );
            button(
                &mut nested,
                ButtonSpecBuilder::new().text("Hi").style(primary),
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Fixed(30.0),
                },
                &mut state.k_btns[1],
            );
            button(
                &mut nested,
                ButtonSpecBuilder::new()
                    .text("This long label clamps to the AtMost ceiling")
                    .style(accent),
                SizeReq {
                    width: Extent::Auto,
                    height: Extent::Fixed(30.0),
                },
                &mut state.k_btns[2],
            );
            nested.finish();

            row.finish();
        }

        right.finish();
    }

    root_row.finish();

    cmds
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// A full-width disabled ghost button used as a heading/label. Generic over the
/// column context's `on_finish` closure type, so it works inside any column.
fn label_row<CF: FnOnce(&mut FocusSystem, &mut framewise::DrawCommands, Rect)>(
    col: &mut WidgetContext<SampleTextSystem, framewise::layouts::ColumnState, CF>,
    ghost: ButtonStyle,
    text: &str,
    height: f32,
) {
    button(
        col,
        ButtonSpecBuilder::new()
            .text(text)
            .style(ghost)
            .disabled(true),
        SizeReq {
            width: Extent::Fill,
            height: Extent::Fixed(height),
        },
        &mut ButtonState::default(),
    );
}

fn heading<CF: FnOnce(&mut FocusSystem, &mut framewise::DrawCommands, Rect)>(
    col: &mut WidgetContext<SampleTextSystem, framewise::layouts::ColumnState, CF>,
    ghost: ButtonStyle,
    text: &str,
) {
    label_row(col, ghost, text, 30.0);
}

fn subheading<CF: FnOnce(&mut FocusSystem, &mut framewise::DrawCommands, Rect)>(
    col: &mut WidgetContext<SampleTextSystem, framewise::layouts::ColumnState, CF>,
    ghost: ButtonStyle,
    text: &str,
) {
    label_row(col, ghost, text, 22.0);
}
