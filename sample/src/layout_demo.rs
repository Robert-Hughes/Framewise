//! Layout demo page.
//!
//! Showcases chrome-less nested layout auto-sizing: a bare `child_with_layout`
//! (Column / Row / Wrap — no `frame` wrapper) placed with `Size::Auto` or
//! `Placement::Fill` now fits to its children, instead of falling back /
//! panicking. Fixed placement and `Size::Fixed` slots stay identical to the old
//! eager path.
//!
//! Sections:
//!   A. Auto-height column (no frame) — fits its stacked rows.
//!   B. Auto-width row (no frame)     — hugs the total width of its children.
//!   C. Nested auto-in-auto           — fit-to-children chaining through levels.
//!   D. Size-request `Auto` sizing    — each button hugs its own label width.
//!   E. Cross-axis alignment          — Start / Center / End inside fit columns.
//!   F. Fixed vs Auto equivalence     — Fixed ignores child extent, Auto fits.
//!   G. WrapLayout flow               — tags wrap onto new lines, auto-sizing.
//!   H. SplitRow                      — width divided into equal declared cells.
//!   I. Mixed per-axis                — fixed icon + Auto-width labels in one row.
//!   J. Toolbar (emit-reorder)        — search fills leftover, buttons stay request-sized.
//!   K. `AtMost` ceiling              — nested Auto container clamps to the remainder.
//!   L. RowLayout cross-align         — differing heights aligned in a tall row.
//!   M. RowLayout main-axis End       — trailing action anchored to row end.
//!   N. Overlapping buttons (ManualLayout) — manual layout with overlapping buttons.

use crate::text::SampleTextBackend;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Align, Placement, Placement2D},
    layouts::{
        linear::{
            ColumnLayout, ColumnLayoutParams, ColumnState, MainAxisAlign, RowLayout,
            RowLayoutParams,
        },
        ManualLayout, SplitRow, WrapLayout,
    },
    theme::Theme,
    types::Rect,
    widget::WidgetContext,
    widgets::button::{
        button,
        raw::{pre_layout_button, ButtonPreLayoutSpec},
        ButtonSpec, ButtonState, ButtonStyle,
    },
    widgets::label::{label, LabelSpecBuilder},
    Color, TextContentPlacement,
};

// ── State ──────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct LayoutDemoState {
    pub page: crate::demo_page::DemoPageState,
    pub a_btns: [ButtonState; 3], // A: auto-height column
    pub a_clicks: [u32; 3],
    pub b_btns: [ButtonState; 3],       // B: auto-width row
    pub c_btns: [ButtonState; 4],       // C: nested auto-in-auto (2 rows × 2)
    pub d_btns: [ButtonState; 3],       // D: size-request Auto widths
    pub e_btns: [ButtonState; 9],       // E: 3 align columns × 3 buttons
    pub f_fixed_btns: [ButtonState; 3], // F: fixed-height column
    pub f_auto_btns: [ButtonState; 3],  // F: auto-height column
    pub g_btns: [ButtonState; 8],       // G: wrap tags
    pub h_btns: [ButtonState; 3],       // H: SplitRow equal thirds
    pub h_clicks: [u32; 3],
    pub i_btns: [ButtonState; 3], // I: mixed per-axis (fixed icon + auto labels)
    pub j_search: ButtonState,    // J: toolbar emit-reorder (fill field)
    pub j_btns: [ButtonState; 2], // J: request-sized right-aligned buttons
    pub k_btns: [ButtonState; 3], // K: AtMost shrink-wrap (fixed block + short hugs + long clamps)
    pub l_btns: [ButtonState; 9], // L: RowLayout cross-align (3 rows × 3)
    pub m_btns: [ButtonState; 3], // M: RowLayout main-axis End trailing action
    pub n_btns: [ButtonState; 5], // N: overlapping buttons (ManualLayout)
    pub n_clicks: [u32; 5],
}

// ── Draw ──────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_layout_page(
    state: &mut LayoutDemoState,
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
            "Layout Demo",
            debug_layout,
            true,
            ColumnLayout,
        );
        draw_layout_page_content(&mut outer.ctx, state, win_w);
        outer.ctx.finish();
    } else {
        let mut page_state = std::mem::take(&mut state.page);
        {
            let mut outer = crate::demo_page::begin_demo_page(
                &mut ctx,
                "Layout Demo",
                &mut page_state,
                debug_layout,
                ColumnLayout,
            );
            draw_layout_page_content(&mut outer.ctx, state, win_w);
            outer.ctx.finish();
        }
        state.page = page_state;
    }

    ctx.finish();

    cmds
}

pub(crate) fn draw_layout_page_content<'a, 'b, CF>(
    outer: &'b mut WidgetContext<
        'a,
        SampleTextBackend,
        framewise::layouts::OffsetState<framewise::ColumnState>,
        CF,
    >,
    state: &mut LayoutDemoState,
    win_w: f32,
) {
    let pad = 20.0;
    let col_w = (win_w - 2.0 * pad - 30.0) * 0.5;

    let theme = outer.theme;
    let primary = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent = ButtonStyle::accent_from_theme(&theme);

    // Root row: two columns side by side.
    let mut root_row = outer.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

    // ── Left column: A, B, C, D ───────────────────────────────────────────────
    {
        let mut left =
            root_row.child_with_layout(RowLayoutParams::auto().fixed_x(col_w), ColumnLayout);

        heading(&mut left, "Chrome-less nested layout auto-sizing");
        left.spacer(18.0);

        // A. Auto-height column (no frame).
        subheading(
            &mut left,
            "A. Auto-height column (bare ColumnLayout, height: Auto)",
        );
        left.spacer(18.0);
        {
            // A bare column placed with Auto height fits to its three rows — no
            // frame chrome involved. The following sibling lands right below it.
            let mut auto_col =
                left.child_with_layout(ColumnLayoutParams::auto().fill_x(), ColumnLayout);
            for i in 0..3 {
                let text = format!("Auto-column row #{} (clicks: {})", i + 1, state.a_clicks[i]);
                let style = [primary, secondary, accent][i];
                let r = button(
                    ButtonSpec::new(&text).style(style),
                    ColumnLayoutParams::auto().fill_x().fixed_y(34.0),
                    &mut state.a_btns[i],
                    &mut auto_col,
                );
                if r.input.clicked {
                    state.a_clicks[i] += 1;
                }
                auto_col.spacer(6.0);
            }
            auto_col.finish();
        }
        left.spacer(18.0);

        // B. Auto-width row (no frame).
        subheading(
            &mut left,
            "B. Auto-width row (bare RowLayout, width: Auto) — hugs its children",
        );
        left.spacer(18.0);
        {
            let mut auto_row =
                left.child_with_layout(ColumnLayoutParams::auto().fixed_y(40.0), RowLayout);
            for (i, label) in ["One", "Two", "Three"].iter().enumerate() {
                button(
                    ButtonSpec::new(label).style(secondary),
                    RowLayoutParams::auto().fill_y(),
                    &mut state.b_btns[i],
                    &mut auto_row,
                );
                auto_row.spacer(8.0);
            }
            auto_row.finish();
        }
        left.spacer(18.0);

        // C. Nested auto-in-auto.
        subheading(
            &mut left,
            "C. Nested auto-in-auto (auto column of auto rows of auto buttons)",
        );
        left.spacer(18.0);
        {
            let mut outer = left.child_with_layout(ColumnLayoutParams::auto(), ColumnLayout);
            let labels = [["First", "Second"], ["Third row item", "Fourth"]];
            for (row_idx, pair) in labels.iter().enumerate() {
                let mut inner_row = outer.child_with_layout(ColumnLayoutParams::auto(), RowLayout);
                for (col_idx, label) in pair.iter().enumerate() {
                    let idx = row_idx * 2 + col_idx;
                    button(
                        ButtonSpec::new(label).style(primary),
                        RowLayoutParams::auto(),
                        &mut state.c_btns[idx],
                        &mut inner_row,
                    );
                    inner_row.spacer(6.0);
                }
                inner_row.finish();
                outer.spacer(6.0);
            }
            outer.finish();
        }
        left.spacer(18.0);

        // D. Size-request Auto sizing.
        subheading(
            &mut left,
            "D. Intrinsic Auto — each button hugs its own label width",
        );
        left.spacer(18.0);
        {
            let mut row =
                left.child_with_layout(ColumnLayoutParams::auto().fixed_y(40.0), RowLayout);
            for (i, label) in ["Go", "Cancel", "Save all changes now"].iter().enumerate() {
                let style = [primary, secondary, accent][i];
                button(
                    ButtonSpec::new(label).style(style),
                    RowLayoutParams::auto().fill_y(),
                    &mut state.d_btns[i],
                    &mut row,
                );
                row.spacer(8.0);
            }
            row.finish();
        }
        left.spacer(18.0);

        // I. Mixed per-axis: fixed-width icon + request-sized labels in one row.
        subheading(
            &mut left,
            "I. Mixed per-axis — fixed icon + Auto-width labels in one row",
        );
        left.spacer(18.0);
        {
            let mut row =
                left.child_with_layout(ColumnLayoutParams::auto().fixed_y(40.0), RowLayout);
            // Fixed 40px square "icon" — width imposed, ignores its label extent.
            button(
                ButtonSpec::new("*").style(accent),
                RowLayoutParams::auto().fixed_x(40.0).fill_y(),
                &mut state.i_btns[0],
                &mut row,
            );
            row.spacer(8.0);
            // Two Auto-width labels each hug their own text — different axis policy
            // than the icon, in the same row.
            for (i, label) in ["Intrinsic label", "Another"].iter().enumerate() {
                button(
                    ButtonSpec::new(label).style(secondary),
                    RowLayoutParams::auto().fill_y(),
                    &mut state.i_btns[i + 1],
                    &mut row,
                );
                row.spacer(8.0);
            }
            row.finish();
        }
        left.spacer(18.0);

        // L. RowLayout cross-axis alignment (Start / Center / End in a tall row).
        subheading(
            &mut left,
            "L. RowLayout cross-align — differing heights aligned in a 60px row",
        );
        left.spacer(18.0);
        {
            {
                let aligns = [
                    ("Start", Align::Start),
                    ("Center", Align::Center),
                    ("End", Align::End),
                ];
                let heights = [22.0, 36.0, 50.0];
                let mut compact_primary = primary;
                compact_primary.pad_y = 2.0;
                let styles = [compact_primary, secondary, accent];
                for (row_idx, (name, align)) in aligns.into_iter().enumerate() {
                    // Fixed 60px height is the Exact cross axis that alignment requires;
                    // each button picks a different height so the alignment is visible.
                    let mut row = left.child_with_layout(
                        ColumnLayoutParams::auto().fill_x().fixed_y(60.0),
                        RowLayout,
                    );
                    for j in 0..3 {
                        let idx = row_idx * 3 + j;
                        button(
                            ButtonSpec::new(&format!("{name} {}", j + 1)).style(styles[j]),
                            RowLayoutParams::auto().fixed_y(heights[j]).align_y(align),
                            &mut state.l_btns[idx],
                            &mut row,
                        );
                        row.spacer(6.0);
                    }
                    row.finish();
                    left.spacer(18.0);
                }
            }
        }
        left.spacer(18.0);

        // M. Main-axis End: append normal children, then close the row with a
        // trailing action anchored to the committed right edge.
        subheading(&mut left, "M. RowLayout main-axis End — trailing action");
        left.spacer(18.0);
        {
            let mut row = left
                .child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(36.0), RowLayout);
            button(
                ButtonSpec::new("Back").style(secondary),
                RowLayoutParams::auto().fill_y(),
                &mut state.m_btns[0],
                &mut row,
            );
            row.spacer(8.0);
            button(
                ButtonSpec::new("Edit").style(secondary),
                RowLayoutParams::auto().fill_y(),
                &mut state.m_btns[1],
                &mut row,
            );
            button(
                ButtonSpec::new("Save").style(primary),
                RowLayoutParams::auto()
                    .fixed_x(96.0)
                    .fill_y()
                    .align_x(MainAxisAlign::End),
                &mut state.m_btns[2],
                &mut row,
            );
            row.finish();
        }
        left.spacer(18.0);

        // N. Overlapping buttons (ManualLayout)
        subheading(&mut left, "N. Overlapping buttons (ManualLayout)");
        left.spacer(18.0);
        {
            let h = 140.0;
            let mut manual = left
                .child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(h), ManualLayout);

            // Button 0: Outer (Strictly encompasses Inner)
            let mut outer_style = primary;
            outer_style.content_placement = TextContentPlacement::logical(
                framewise::ContentPlacement::Align(Align::Center),
                framewise::ContentPlacement::Align(Align::Start),
            );
            outer_style.hovered = Color::from_srgb_u8(130, 50, 200, 255); // Custom vibrant purple

            let text_0 = format!("Clicks: {}", state.n_clicks[0]);
            let r0 = button(
                ButtonSpec::new(&text_0).style(outer_style),
                Rect::new(10.0, 10.0, 130.0, 120.0),
                &mut state.n_btns[0],
                &mut manual,
            );
            if r0.input.clicked {
                state.n_clicks[0] = (state.n_clicks[0] + 1) % 10;
            }

            // Button 1: Inner (Strictly encompassed by Outer)
            let mut inner_style = accent;
            inner_style.hovered = Color::from_srgb_u8(30, 140, 240, 255); // High-contrast bright blue

            let text_1 = format!("Clicks: {}", state.n_clicks[1]);
            let r1 = button(
                ButtonSpec::new(&text_1).style(inner_style),
                Rect::new(25.0, 35.0, 100.0, 70.0),
                &mut state.n_btns[1],
                &mut manual,
            );
            if r1.input.clicked {
                state.n_clicks[1] = (state.n_clicks[1] + 1) % 10;
            }

            // Button 2: Side A (Overlaps B/C slightly)
            let mut side_style = secondary;
            side_style.hovered = Color::from_srgb_u8(250, 215, 200, 255); // Light peach/orange fill

            let text_2 = format!("Clicks: {}", state.n_clicks[2]);
            let r2 = button(
                ButtonSpec::new(&text_2).style(side_style),
                Rect::new(135.0, 10.0, 85.0, 40.0),
                &mut state.n_btns[2],
                &mut manual,
            );
            if r2.input.clicked {
                state.n_clicks[2] = (state.n_clicks[2] + 1) % 10;
            }

            // Button 3: Mid (3) (Overlaps A slightly, overlaps D corner-wise)
            let mut mid_style = primary;
            mid_style.hovered = Color::from_srgb_u8(130, 50, 200, 255); // Same custom vibrant purple

            let text_3 = format!("Clicks: {}", state.n_clicks[3]);
            let r3 = button(
                ButtonSpec::new(&text_3).style(mid_style),
                Rect::new(200.0, 25.0, 85.0, 40.0),
                &mut state.n_btns[3],
                &mut manual,
            );
            if r3.input.clicked {
                state.n_clicks[3] = (state.n_clicks[3] + 1) % 10;
            }

            // Button 4: Corner D (4) (Overlaps 3 corner-wise)
            let mut corner_style = accent;
            corner_style.hovered = Color::from_srgb_u8(30, 140, 240, 255); // Same as button 1 (bright blue)

            let text_4 = format!("Clicks: {}", state.n_clicks[4]);
            let r4 = button(
                ButtonSpec::new(&text_4).style(corner_style),
                Rect::new(265.0, 50.0, 85.0, 40.0),
                &mut state.n_btns[4],
                &mut manual,
            );
            if r4.input.clicked {
                state.n_clicks[4] = (state.n_clicks[4] + 1) % 10;
            }

            manual.finish();
        }

        left.finish();
    }

    root_row.spacer(30.0);

    // ── Right column: E, F, G ──────────────────────────────────────────────────
    {
        let mut right =
            root_row.child_with_layout(RowLayoutParams::auto().fixed_x(col_w), ColumnLayout);

        heading(&mut right, "Alignment, equivalence & flow");
        right.spacer(18.0);

        // E. Cross-axis alignment.
        subheading(
            &mut right,
            "E. Cross-axis alignment (Start / Center / End in fit columns)",
        );
        right.spacer(18.0);
        {
            {
                let aligns = [
                    ("Start", Align::Start),
                    ("Center", Align::Center),
                    ("End", Align::End),
                ];
                for (col_idx, (name, align)) in aligns.into_iter().enumerate() {
                    subheading(&mut right, &format!("  Align::{name:?}"));
                    right.spacer(18.0);
                    // Fill width gives the column an Exact cross axis, which alignment
                    // requires; Auto height fits the three differently-sized buttons.
                    let mut col =
                        right.child_with_layout(ColumnLayoutParams::auto().fill_x(), ColumnLayout);
                    let widths = [120.0, 220.0, 320.0];
                    let styles = [primary, secondary, accent];
                    for j in 0..3 {
                        let idx = col_idx * 3 + j;
                        button(
                            ButtonSpec::new(&format!("{name} {}", j + 1)).style(styles[j]),
                            ColumnLayoutParams::fixed(widths[j], 30.0).align_x(align),
                            &mut state.e_btns[idx],
                            &mut col,
                        );
                        col.spacer(5.0);
                    }
                    col.finish();
                    right.spacer(18.0);
                }
            }
        }

        // F. Fixed vs Auto equivalence.
        subheading(
            &mut right,
            "F. Fixed height ignores child extent; Auto height fits it",
        );
        right.spacer(18.0);
        {
            let mut pair_row = right.child_with_layout(
                ColumnLayoutParams::auto().fill_x().fixed_y(150.0),
                RowLayout,
            );

            // Fixed-height column: clips/overflows past its committed 60px slot.
            let mut fixed_col = pair_row.child_with_layout(
                RowLayoutParams::fixed(col_w * 0.5 - 8.0, 60.0),
                ColumnLayout,
            );
            for i in 0..3 {
                button(
                    ButtonSpec::new(&format!("Fixed {}", i + 1)).style(secondary),
                    ColumnLayoutParams::auto().fill_x().fixed_y(34.0),
                    &mut state.f_fixed_btns[i],
                    &mut fixed_col,
                );
                fixed_col.spacer(4.0);
            }
            fixed_col.finish();

            pair_row.spacer(16.0);

            // Auto-height column: same children, fits all three.
            let mut auto_col = pair_row.child_with_layout(
                RowLayoutParams::auto().fixed_x(col_w * 0.5 - 8.0),
                ColumnLayout,
            );
            for i in 0..3 {
                button(
                    ButtonSpec::new(&format!("Auto {}", i + 1)).style(accent),
                    ColumnLayoutParams::auto().fill_x().fixed_y(34.0),
                    &mut state.f_auto_btns[i],
                    &mut auto_col,
                );
                auto_col.spacer(4.0);
            }
            auto_col.finish();

            pair_row.finish();
        }
        right.spacer(18.0);

        // G. WrapLayout flow.
        subheading(
            &mut right,
            "G. WrapLayout — tags flow onto new lines, height auto-sizes",
        );
        right.spacer(18.0);
        {
            let mut wrap = right.child_with_layout(
                ColumnLayoutParams::auto().fill_x(),
                WrapLayout {
                    spacing: 6.0,
                    line_spacing: 6.0,
                },
            );
            let tags = [
                "rust",
                "layout",
                "request",
                "auto-size",
                "nested",
                "wrap",
                "demo",
                "framewise",
            ];
            for (i, tag) in tags.iter().enumerate() {
                button(
                    ButtonSpec::new(tag).style(primary),
                    Placement2D {
                        width: Placement::auto(),
                        height: Placement::fixed(30.0),
                    },
                    &mut state.g_btns[i],
                    &mut wrap,
                );
            }
            wrap.finish();
        }
        right.spacer(18.0);

        // H. SplitRow — declared equal thirds (Phase 4).
        subheading(
            &mut right,
            "H. SplitRow — width divided into 3 equal cells (declared count)",
        );
        right.spacer(18.0);
        {
            // count = 3 known up front, so each cell is exactly a third of the
            // (Exact) row width. Children declare only their cross-axis (height).
            let mut split = right.child_with_layout(
                ColumnLayoutParams::auto().fill_x().fixed_y(40.0),
                SplitRow {
                    count: 3,
                    spacing: 10.0,
                },
            );
            let styles = [primary, secondary, accent];
            for (i, &style) in styles.iter().enumerate() {
                let text = format!("Third #{} ({})", i + 1, state.h_clicks[i]);
                let r = button(
                    ButtonSpec::new(&text).style(style),
                    Placement::fill(), // fill the cell height
                    &mut state.h_btns[i],
                    &mut split,
                );
                if r.input.clicked {
                    state.h_clicks[i] += 1;
                }
            }
            split.finish();
        }
        right.spacer(18.0);

        // J. Toolbar leftover via the emit-reorder trick: the request-sized right-hand
        // buttons are measured and placed first, the search field fills the gap,
        // then override_keyboard_next restores logical left→right focus.
        subheading(
            &mut right,
            "J. Toolbar — search fills leftover, buttons stay request-sized (emit-reorder)",
        );
        right.spacer(18.0);
        {
            let h = 36.0;
            let spacing = 8.0;
            let w = col_w; // Fill width under the Exact right column resolves to col_w.

            // Query the two button size requests up front — the reorder trick needs
            // their sizes before the fill child can be placed.
            let measure = |ts: &mut SampleTextBackend, label: &str| {
                let spec = ButtonSpec::new_from_theme(label, &theme);
                let spec = ButtonPreLayoutSpec {
                    text: spec.text,
                    style: spec.style,
                };
                // These buttons are manually placed in a fixed toolbar slot, so
                // there is no layout offer to peek for this raw size query.
                pre_layout_button(&spec, framewise::layout::SizeOffer::UNBOUNDED, ts)
                    .size_request
                    .preferred
                    .unwrap()
                    .x
            };
            let w_filter = measure(right.text_backend, "Filter");
            let w_sort = measure(right.text_backend, "Sort");
            let x_sort = w - w_sort;
            let x_filter = x_sort - spacing - w_filter;
            let search_w = (x_filter - spacing).max(0.0);

            // ManualLayout: rects are origin-relative to the toolbar's top-left.
            let mut tb = right
                .child_with_layout(ColumnLayoutParams::auto().fill_x().fixed_y(h), ManualLayout);
            // Emit the request-sized (right-hand) children first — they depend on no
            // sibling, so their position is known immediately.
            button(
                ButtonSpec::new("Filter").style(secondary),
                Rect::new(x_filter, 0.0, w_filter, h),
                &mut state.j_btns[0],
                &mut tb,
            );
            button(
                ButtonSpec::new("Sort").style(secondary),
                Rect::new(x_sort, 0.0, w_sort, h),
                &mut state.j_btns[1],
                &mut tb,
            );
            // Then the fill child at the computed remainder.
            button(
                ButtonSpec::new("Search...").style(primary),
                Rect::new(0.0, 0.0, search_w, h),
                &mut state.j_search,
                &mut tb,
            );
            tb.finish();

            // Emitted right-first; restore logical left→right focus order.
            right
                .focus_system
                .override_keyboard_next(state.j_search.focus_id, state.j_btns[0].focus_id);
            right
                .focus_system
                .override_keyboard_next(state.j_btns[0].focus_id, state.j_btns[1].focus_id);
        }
        right.spacer(18.0);

        // K. AtMost ceiling: a nested Auto-width container is handed AtMost(remaining)
        // by the parent row, so its children shrink-wrap but clamp at that ceiling.
        subheading(
            &mut right,
            "K. AtMost — nested Auto container caps children at the leftover ceiling",
        );
        right.spacer(18.0);
        {
            let mut row = right.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);
            // Fixed block eats 55% of the row width.
            button(
                ButtonSpec::new("Fixed 55%").style(secondary),
                RowLayoutParams::fixed(col_w * 0.55, 70.0),
                &mut state.k_btns[0],
                &mut row,
            );
            row.spacer(12.0);
            // Nested Auto-width column → receives AtMost(remaining ~45%). Inside, the
            // short label hugs its text; the long one clamps to the AtMost ceiling.
            let mut nested = row.child_with_layout(RowLayoutParams::auto(), ColumnLayout);
            button(
                ButtonSpec::new("Hi").style(primary),
                ColumnLayoutParams::auto().fixed_y(30.0),
                &mut state.k_btns[1],
                &mut nested,
            );
            nested.spacer(6.0);
            button(
                ButtonSpec::new("This long label clamps to the AtMost ceiling").style(accent),
                ColumnLayoutParams::auto().fixed_y(30.0),
                &mut state.k_btns[2],
                &mut nested,
            );
            nested.finish();

            row.finish();
        }

        right.finish();
    }

    root_row.finish();
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// A full-width label used as a heading/label. Generic over the
/// column context's `on_finish` closure type, so it works inside any column.
fn label_row<
    CF: FnOnce(
        &mut FocusSystem,
        &mut SampleTextBackend,
        &mut framewise::DrawCommands,
        &mut framewise::Output,
        Rect,
    ),
>(
    col: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    text: &str,
    height: f32,
) {
    label(
        col,
        LabelSpecBuilder::new().text(text),
        ColumnLayoutParams::auto().fill_x().fixed_y(height),
    );
}

fn heading<
    CF: FnOnce(
        &mut FocusSystem,
        &mut SampleTextBackend,
        &mut framewise::DrawCommands,
        &mut framewise::Output,
        Rect,
    ),
>(
    col: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    text: &str,
) {
    label_row(col, text, 30.0);
}

fn subheading<
    CF: FnOnce(
        &mut FocusSystem,
        &mut SampleTextBackend,
        &mut framewise::DrawCommands,
        &mut framewise::Output,
        Rect,
    ),
>(
    col: &mut WidgetContext<SampleTextBackend, ColumnState, CF>,
    text: &str,
) {
    label_row(col, text, 22.0);
}
