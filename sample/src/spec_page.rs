/// Widget specification page — static visual reference of all widget states.
///
/// No interaction. Calls placeholder widget functions directly at fixed positions.
/// `scroll_y` is subtracted from every widget's y coordinate so the page scrolls.
use framewise::{
    draw::DrawCmd,
    text::TextSystem as _,
    theme::Theme,
    types::{Color, Rect, Vec2},
    widgets::{
        checkbox::{CheckState, CheckboxSpec, checkbox},
        chip::{ChipSpec, chip},
        drag_number::{DragNumberSpec, drag_number},
        menu::{MenuItem, MenuSpec, menu},
        progress_bar::{ProgressBarSpec, progress_bar},
        radio::{RadioSpec, radio},
        select::{SelectSpec, select},
        segmented::{SegmentedSpec, segmented},
        spinner::{SpinnerSpec, spinner},
        status::{StatusSpec, StatusVariant, status},
        switch::{SwitchSpec, switch},
        tabs::{TabsSpec, tabs},
        tooltip::{TooltipSpec, TooltipVariant, tooltip},
        tree::{TreeRow, TreeSpec, tree},
        window::{WindowButton, WindowSpec, window},
    },
};
use crate::text::SampleTextSystem;

const MARGIN: f32 = 60.0;
const COL_GAP: f32 = 24.0;
const SEC_GAP: f32 = 72.0;
const GROUP_GAP: f32 = 32.0;

// ── text helpers ──────────────────────────────────────────────────────────────

fn txt(ts: &mut SampleTextSystem, s: &str, x: f32, y: f32, size: f32, color: Color) -> DrawCmd {
    let layout = ts.prepare(s, size);
    DrawCmd::Text {
        rect:   Rect::new(x, y, layout.size.x, layout.size.y),
        color,
        handle: layout.handle,
    }
}

/// Section divider: horizontal line + number + title.
fn section_header(
    ts: &mut SampleTextSystem,
    cmds: &mut Vec<DrawCmd>,
    x: f32,
    y: f32,
    w: f32,
    num: &str,
    title: &str,
) -> f32 {
    let t = Theme::framewise();
    // Divider line.
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(x, y + 18.0),
        p1:    Vec2::new(x + w, y + 18.0),
        color: t.line,
        width: 1.0,
    });
    cmds.push(txt(ts, num, x, y, t.text_sm, t.muted));
    let num_layout = ts.prepare(num, t.text_sm);
    cmds.push(txt(ts, title, x + num_layout.size.x + 20.0, y, 22.0, t.ink));
    46.0 // height consumed
}

/// Mono uppercase group label.
fn group_label(
    ts: &mut SampleTextSystem,
    cmds: &mut Vec<DrawCmd>,
    x: f32,
    y: f32,
    text: &str,
) -> f32 {
    let t = Theme::framewise();
    cmds.push(txt(ts, text, x, y, t.text_sm, t.muted));
    24.0
}

// ── button visuals (drawn manually since button() needs state+input) ──────────

#[derive(Clone, Copy)]
enum BtnState { Normal, Hovered, Pressed, Focused, Disabled }

fn draw_button_visual(
    ts: &mut SampleTextSystem,
    cmds: &mut Vec<DrawCmd>,
    rect: Rect,
    label: &str,
    style: &framewise::widgets::button::ButtonStyle,
    state: BtnState,
) {
    let _t = Theme::framewise();
    let alpha = if matches!(state, BtnState::Disabled) { 0.32_f32 } else { 1.0 };
    let tint = |c: Color| Color::new(c.r, c.g, c.b, c.a * alpha);

    // Focus ring.
    if matches!(state, BtnState::Focused) {
        cmds.push(DrawCmd::StrokeRect {
            rect:  rect.inset(-(style.border_width + 2.0)),
            color: style.focus_border,
            width: 2.0,
        });
    }

    let fill = match state {
        BtnState::Pressed  => style.pressed,
        BtnState::Hovered  => style.hovered,
        _                  => style.background,
    };
    cmds.push(DrawCmd::FillRect { rect, color: tint(fill) });
    if style.border_width > 0.0 {
        cmds.push(DrawCmd::StrokeRect { rect, color: tint(style.border), width: style.border_width });
    }

    let lyt = ts.prepare(label, style.text_size);
    let tx = rect.x + (rect.w - lyt.size.x) * 0.5;
    let ty = rect.y + (rect.h - lyt.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(tx, ty, lyt.size.x, lyt.size.y),
        color: tint(style.text_color),
        handle: lyt.handle,
    });
}

// ── text-input visual (drawn manually) ───────────────────────────────────────

#[derive(Clone, Copy)]
enum InputState { Normal, Focused, Error, Disabled }

fn draw_input_visual(
    ts: &mut SampleTextSystem,
    cmds: &mut Vec<DrawCmd>,
    rect: Rect,
    value: &str,
    placeholder: &str,
    state: InputState,
) {
    let t = Theme::framewise();
    let alpha = if matches!(state, InputState::Disabled) { 0.55_f32 } else { 1.0 };
    let tint = |c: Color| Color::new(c.r, c.g, c.b, c.a * alpha);

    let bg = match state {
        InputState::Error => t.rust_soft,
        _                 => t.paper_elev,
    };
    cmds.push(DrawCmd::FillRect { rect, color: tint(bg) });

    // Error: 4px left accent bar.
    if matches!(state, InputState::Error) {
        cmds.push(DrawCmd::FillRect {
            rect:  Rect::new(rect.x, rect.y, 4.0, rect.h),
            color: t.rust,
        });
    }

    // Border.
    cmds.push(DrawCmd::StrokeRect { rect, color: tint(t.ink), width: 1.0 });

    // Focus ring.
    if matches!(state, InputState::Focused) {
        cmds.push(DrawCmd::StrokeRect {
            rect:  rect.inset(-1.0),
            color: t.rust,
            width: 2.0,
        });
    }

    let has_value = !value.is_empty();
    let display = if has_value { value } else { placeholder };
    let color   = if has_value { tint(t.ink) } else { tint(t.muted) };
    let pad = if matches!(state, InputState::Error) { 17.0 } else { 10.0 };
    let lyt = ts.prepare(display, t.text_md);
    let ty = rect.y + (rect.h - lyt.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(rect.x + pad, ty, lyt.size.x, lyt.size.y),
        color,
        handle: lyt.handle,
    });

    // Simulated caret for focused state.
    if matches!(state, InputState::Focused) {
        let caret_x = rect.x + pad + lyt.size.x + 2.0;
        cmds.push(DrawCmd::FillRect {
            rect:  Rect::new(caret_x, rect.y + (rect.h - 14.0) * 0.5, 2.0, 14.0),
            color: t.rust,
        });
    }
}

// ── color swatch helper ───────────────────────────────────────────────────────

pub fn color_swatch(
    ts: &mut SampleTextSystem,
    cmds: &mut Vec<DrawCmd>,
    x: f32,
    y: f32,
    chip_color: Color,
    hex: &str,
) {
    let t = Theme::framewise();
    let h = t.h_md;
    let chip_w = 26.0_f32;
    let hex_lyt = ts.prepare(hex, t.text_md);
    let total_w = chip_w + hex_lyt.size.x + 20.0;
    let outer = Rect::new(x, y, total_w, h);

    cmds.push(DrawCmd::FillRect { rect: outer, color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: outer, color: t.ink, width: 1.0 });

    // Colour chip.
    let chip_rect = Rect::new(x, y, chip_w, h);
    cmds.push(DrawCmd::FillRect { rect: chip_rect, color: chip_color });
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(x + chip_w, y),
        p1:    Vec2::new(x + chip_w, y + h),
        color: t.ink,
        width: 1.0,
    });

    let ty = y + (h - hex_lyt.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(x + chip_w + 10.0, ty, hex_lyt.size.x, hex_lyt.size.y),
        color:  t.ink,
        handle: hex_lyt.handle,
    });
}

// ── keycap helper ─────────────────────────────────────────────────────────────

pub fn keycap(
    ts: &mut SampleTextSystem,
    cmds: &mut Vec<DrawCmd>,
    x: f32,
    y: f32,
    key: &str,
) -> f32 {
    let t = Theme::framewise();
    let lyt = ts.prepare(key, t.text_sm);
    let w = (lyt.size.x + 10.0).max(18.0);
    let h = 18.0_f32;
    cmds.push(DrawCmd::FillRect { rect: Rect::new(x, y, w, h), color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: Rect::new(x, y, w, h), color: t.ink, width: 1.0 });
    let ty = y + (h - lyt.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(x + (w - lyt.size.x) * 0.5, ty, lyt.size.x, lyt.size.y),
        color: t.ink,
        handle: lyt.handle,
    });
    w
}

// ── main page function ────────────────────────────────────────────────────────

pub fn draw_spec_page(
    ts: &mut SampleTextSystem,
    win_w: f32,
    win_h: f32,
    scroll_y: f32,
) -> Vec<DrawCmd> {
    let t = Theme::framewise();
    let mut cmds: Vec<DrawCmd> = Vec::new();

    // Clip to window.
    cmds.push(DrawCmd::PushClip { rect: Rect::new(0.0, 0.0, win_w, win_h) });

    // Paper background.
    cmds.push(DrawCmd::FillRect {
        rect:  Rect::new(0.0, 0.0, win_w, win_h),
        color: t.paper,
    });

    let content_w = (win_w - MARGIN * 2.0).min(1100.0);
    let lx = (win_w - content_w) * 0.5; // left edge, centered
    let mut y = MARGIN - scroll_y;

    // ── Page title ────────────────────────────────────────────────────────────
    {
        let lyt = ts.prepare("FRAMEWISE", 56.0);
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(lx, y, lyt.size.x, lyt.size.y),
            color:  t.ink,
            handle: lyt.handle,
        });
        y += lyt.size.y + 8.0;
        let sub_lyt = ts.prepare("Widget Reference  ·  v0.1", t.text_md);
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(lx, y, sub_lyt.size.x, sub_lyt.size.y),
            color:  t.muted,
            handle: sub_lyt.handle,
        });
        y += sub_lyt.size.y + 40.0;
        // Horizontal rule.
        cmds.push(DrawCmd::StrokeLine {
            p0:    Vec2::new(lx, y),
            p1:    Vec2::new(lx + content_w, y),
            color: t.line,
            width: 1.0,
        });
        y += SEC_GAP * 0.5;
    }

    // ── 1 · BUTTON ───────────────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "01", "BUTTON");

    // Row 1: variants
    y += group_label(ts, &mut cmds, lx, y, "VARIANTS");
    {
        let btn_h = t.h_md;
        let states_info: &[(&str, &framewise::widgets::button::ButtonStyle, BtnState)] = &[
            ("Secondary", &framewise::widgets::button::ButtonStyle::default(), BtnState::Normal),
            ("Primary",   &framewise::widgets::button::ButtonStyle::primary(),  BtnState::Normal),
            ("Accent",    &framewise::widgets::button::ButtonStyle::accent(),    BtnState::Normal),
            ("Ghost",     &framewise::widgets::button::ButtonStyle::ghost(),     BtnState::Normal),
        ];
        let btn_styles: Vec<(_, _, _)> = states_info.iter().map(|(l, s, st)| (*l, (*s).clone(), *st)).collect();
        let mut bx = lx;
        for (label, style, state) in &btn_styles {
            let lyt = ts.prepare(label, style.text_size);
            let w = lyt.size.x + 28.0;
            draw_button_visual(ts, &mut cmds, Rect::new(bx, y, w, btn_h), label, style, *state);
            bx += w + COL_GAP;
        }
        y += btn_h + GROUP_GAP;
    }

    // Row 2: states (secondary style)
    y += group_label(ts, &mut cmds, lx, y, "STATES");
    {
        let btn_h = t.h_md;
        let sec = framewise::widgets::button::ButtonStyle::default();
        let pri = framewise::widgets::button::ButtonStyle::primary();
        let states_info: &[(&str, &framewise::widgets::button::ButtonStyle, BtnState)] = &[
            ("Normal",   &sec, BtnState::Normal),
            ("Hovered",  &sec, BtnState::Hovered),
            ("Pressed",  &pri, BtnState::Pressed),
            ("Focused",  &sec, BtnState::Focused),
            ("Disabled", &sec, BtnState::Disabled),
        ];
        let btn_data: Vec<_> = states_info.iter().map(|(l, s, st)| (*l, (*s).clone(), *st)).collect();
        let mut bx = lx;
        for (label, style, state) in &btn_data {
            let lyt = ts.prepare(label, style.text_size);
            let w = lyt.size.x + 28.0;
            draw_button_visual(ts, &mut cmds, Rect::new(bx, y, w, btn_h), label, style, *state);
            bx += w + COL_GAP;
        }
        y += btn_h + SEC_GAP;
    }

    // ── 2 · TEXT INPUT ───────────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "02", "TEXT INPUT");
    y += group_label(ts, &mut cmds, lx, y, "STATES");
    {
        let input_w = 220.0_f32;
        let input_h = t.h_md;
        let states: &[(&str, &str, InputState)] = &[
            ("",         "Placeholder…",  InputState::Normal),
            ("Editing",  "Placeholder…",  InputState::Focused),
            ("Error value", "Placeholder…", InputState::Error),
            ("",         "Placeholder…",  InputState::Disabled),
        ];
        let mut bx = lx;
        for (val, ph, state) in states {
            draw_input_visual(ts, &mut cmds, Rect::new(bx, y, input_w, input_h), val, ph, *state);
            bx += input_w + COL_GAP;
        }
        y += input_h + SEC_GAP;
    }

    // ── 3 · CHECK / RADIO / SWITCH ───────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "03", "CHECK · RADIO · SWITCH");
    y += group_label(ts, &mut cmds, lx, y, "CHECKBOX");
    {
        let specs: &[(CheckState, bool, bool, &str)] = &[
            (CheckState::Off,           false, false, "Off"),
            (CheckState::On,            false, false, "On"),
            (CheckState::Indeterminate, false, false, "Indeterminate"),
            (CheckState::On,            true,  false, "Focused"),
            (CheckState::Off,           false, true,  "Disabled"),
        ];
        let mut bx = lx;
        for (state, focused, disabled, label) in specs {
            let mut dc = checkbox(CheckboxSpec {
                rect: Rect::new(bx, y, 14.0, 14.0),
                state: *state,
                focused: *focused,
                disabled: *disabled,
            });
            cmds.extend(dc.0.drain(..));
            let lyt = ts.prepare(label, t.text_md);
            cmds.push(DrawCmd::Text {
                rect:   Rect::new(bx + 18.0, y + (14.0 - lyt.size.y) * 0.5, lyt.size.x, lyt.size.y),
                color:  t.ink,
                handle: lyt.handle,
            });
            bx += 18.0 + lyt.size.x + COL_GAP + 8.0;
        }
        y += 14.0 + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "RADIO");
    {
        let specs: &[(bool, bool, bool, &str)] = &[
            (false, false, false, "Unselected"),
            (true,  false, false, "Selected"),
            (true,  true,  false, "Focused"),
            (false, false, true,  "Disabled"),
        ];
        let mut bx = lx;
        for (selected, focused, disabled, label) in specs {
            let mut dc = radio(RadioSpec {
                rect: Rect::new(bx, y, 14.0, 14.0),
                selected: *selected,
                focused: *focused,
                disabled: *disabled,
            });
            cmds.extend(dc.0.drain(..));
            let lyt = ts.prepare(label, t.text_md);
            cmds.push(DrawCmd::Text {
                rect:   Rect::new(bx + 18.0, y + (14.0 - lyt.size.y) * 0.5, lyt.size.x, lyt.size.y),
                color:  t.ink,
                handle: lyt.handle,
            });
            bx += 18.0 + lyt.size.x + COL_GAP + 8.0;
        }
        y += 14.0 + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "SWITCH");
    {
        let specs: &[(bool, bool, bool, &str)] = &[
            (false, false, false, "Off"),
            (true,  false, false, "On"),
            (true,  true,  false, "Focused"),
            (false, false, true,  "Disabled"),
        ];
        let mut bx = lx;
        for (on, focused, disabled, label) in specs {
            let mut dc = switch(SwitchSpec {
                rect: Rect::new(bx, y, 30.0, 16.0),
                on: *on,
                focused: *focused,
                disabled: *disabled,
            });
            cmds.extend(dc.0.drain(..));
            let lyt = ts.prepare(label, t.text_md);
            cmds.push(DrawCmd::Text {
                rect:   Rect::new(bx + 34.0, y + (16.0 - lyt.size.y) * 0.5, lyt.size.x, lyt.size.y),
                color:  t.ink,
                handle: lyt.handle,
            });
            bx += 34.0 + lyt.size.x + COL_GAP + 8.0;
        }
        y += 16.0 + SEC_GAP;
    }

    // ── 4 · SLIDERS + DRAG NUMBER ────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "04", "SLIDERS · DRAG NUMBER");
    y += group_label(ts, &mut cmds, lx, y, "DRAG NUMBER");
    {
        let items: &[(&str, f32, f32, f32, bool)] = &[
            ("Width",  240.0, 0.0, 800.0, false),
            ("Height", 180.0, 0.0, 600.0, false),
            ("Scale",  1.25,  0.0,   4.0, true),
        ];
        let mut bx = lx;
        for (label, val, min, max, active) in items {
            let mut dc = drag_number(DragNumberSpec {
                rect: Rect::new(bx, y, 168.0, t.h_md),
                label,
                value: *val,
                min: *min,
                max: *max,
                active: *active,
            }, ts);
            cmds.extend(dc.0.drain(..));
            bx += 168.0 + COL_GAP;
        }
        y += t.h_md + SEC_GAP;
    }

    // ── 5 · SELECT / SEGMENTED / CHIP ────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "05", "SELECT · SEGMENTED · CHIP");
    y += group_label(ts, &mut cmds, lx, y, "SELECT");
    {
        let opts = ["Option A", "Option B", "Option C"];
        // Closed.
        let mut dc = select(SelectSpec {
            rect: Rect::new(lx, y, 200.0, t.h_md),
            value: "Option A",
            options: &opts,
            open: false,
            focused: false,
            hovered: None,
        }, ts);
        cmds.extend(dc.0.drain(..));
        // Focused.
        let mut dc2 = select(SelectSpec {
            rect: Rect::new(lx + 220.0, y, 200.0, t.h_md),
            value: "Option B",
            options: &opts,
            open: false,
            focused: true,
            hovered: None,
        }, ts);
        cmds.extend(dc2.0.drain(..));
        // Open — rendered below its row, allow space.
        let open_y = y;
        let mut dc3 = select(SelectSpec {
            rect: Rect::new(lx + 440.0, open_y, 200.0, t.h_md),
            value: "Option A",
            options: &opts,
            open: true,
            focused: true,
            hovered: Some(1),
        }, ts);
        cmds.extend(dc3.0.drain(..));
        y += t.h_md + opts.len() as f32 * 26.0 + 16.0 + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "SEGMENTED");
    {
        let items = ["Grid", "List", "Gallery"];
        let mut dc = segmented(SegmentedSpec {
            rect: Rect::new(lx, y, 0.0, t.h_md), // width auto-computed
            items: &items,
            active_index: 0,
            focused: None,
        }, ts);
        let w = dc.0.iter().fold(0.0_f32, |acc, cmd| match cmd {
            DrawCmd::FillRect { rect, .. } | DrawCmd::StrokeRect { rect, .. } => acc.max(rect.x + rect.w - lx),
            _ => acc,
        });
        cmds.extend(dc.0.drain(..));
        // Second variant with second item active.
        let mut dc2 = segmented(SegmentedSpec {
            rect: Rect::new(lx + w + COL_GAP * 2.0, y, 0.0, t.h_md),
            items: &items,
            active_index: 1,
            focused: Some(2),
        }, ts);
        cmds.extend(dc2.0.drain(..));
        y += t.h_md + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "CHIPS");
    {
        let chip_labels = ["Design", "Rust", "WGPU", "Open Source", "v0.1"];
        let active = [false, true, false, true, false];
        let mut bx = lx;
        for (label, is_active) in chip_labels.iter().zip(active.iter()) {
            let mut dc = chip(ChipSpec {
                rect: Rect::new(bx, y, 0.0, 22.0),
                label,
                active: *is_active,
                focused: false,
            }, ts);
            // Measure actual width.
            let chip_w = dc.0.iter().fold(0.0_f32, |acc, cmd| match cmd {
                DrawCmd::FillRect { rect, .. } | DrawCmd::StrokeRect { rect, .. } => acc.max(rect.x + rect.w - bx),
                _ => acc,
            });
            cmds.extend(dc.0.drain(..));
            bx += chip_w + 8.0;
        }
        y += 22.0 + SEC_GAP;
    }

    // ── 6 · TABS ─────────────────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "06", "TABS");
    {
        let items = ["Overview", "Properties", "Inspector", "Logs"];
        let mut dc = tabs(TabsSpec {
            rect: Rect::new(lx, y, content_w, 36.0),
            items: &items,
            active_index: 1,
            focused: None,
        }, ts);
        cmds.extend(dc.0.drain(..));
        y += 36.0 + SEC_GAP;
    }

    // ── 7 · PROGRESS · SPINNER · STATUS ──────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "07", "PROGRESS · SPINNER · STATUS");
    y += group_label(ts, &mut cmds, lx, y, "PROGRESS BARS");
    {
        let items: &[(f32, bool, &str)] = &[
            (0.0,  false, "0%"),
            (0.35, false, "35%"),
            (0.75, false, "75%"),
            (1.0,  false, "100%"),
            (0.6,  true,  "Active 60%"),
            (f32::NAN, true, "Indeterminate"),
        ];
        let bar_w = 160.0_f32;
        let row_h  = 24.0_f32;
        let mut bx = lx;
        let start_y = y;
        let mut col = 0;
        for (val, active, label) in items {
            let row_y = start_y + col as f32 * row_h;
            let mut dc = progress_bar(ProgressBarSpec {
                rect: Rect::new(bx, row_y + 10.0, bar_w, 3.0),
                value: *val,
                phase: 0.4,
                active: *active,
            });
            cmds.extend(dc.0.drain(..));
            let lyt = ts.prepare(label, t.text_sm);
            cmds.push(DrawCmd::Text {
                rect:   Rect::new(bx + bar_w + 8.0, row_y + 4.0, lyt.size.x, lyt.size.y),
                color:  t.muted,
                handle: lyt.handle,
            });
            col += 1;
            if col == 3 {
                col = 0;
                bx += bar_w + 100.0;
            }
        }
        y += 3.0 * row_h + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "SPINNERS");
    {
        let mut dc = spinner(SpinnerSpec { rect: Rect::new(lx, y, 16.0, 16.0), large: false, color: None });
        cmds.extend(dc.0.drain(..));
        let mut dc2 = spinner(SpinnerSpec { rect: Rect::new(lx + 32.0, y, 24.0, 24.0), large: true, color: None });
        cmds.extend(dc2.0.drain(..));
        let mut dc3 = spinner(SpinnerSpec { rect: Rect::new(lx + 72.0, y, 16.0, 16.0), large: false, color: Some(t.rust) });
        cmds.extend(dc3.0.drain(..));
        y += 24.0 + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "STATUS");
    {
        let items: &[(&str, StatusVariant)] = &[
            ("Nominal",  StatusVariant::Ok),
            ("Warning",  StatusVariant::Warn),
            ("Error",    StatusVariant::Err),
            ("Live",     StatusVariant::Live),
            ("Offline",  StatusVariant::Neutral),
        ];
        let mut bx = lx;
        for (label, variant) in items {
            let mut dc = status(StatusSpec {
                rect: Rect::new(bx, y, 140.0, 12.0),
                label,
                variant: *variant,
            }, ts);
            let w = dc.0.iter().fold(0.0_f32, |acc, cmd| match cmd {
                DrawCmd::FillRect { rect, .. } | DrawCmd::StrokeRect { rect, .. } => acc.max(rect.x + rect.w - bx),
                DrawCmd::Text { rect, .. } => acc.max(rect.x + rect.w - bx),
                _ => acc,
            });
            cmds.extend(dc.0.drain(..));
            bx += w.max(100.0) + 16.0;
        }
        y += 12.0 + SEC_GAP;
    }

    // ── 8 · TREE ─────────────────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "08", "TREE / LIST");
    {
        let rows = [
            TreeRow { indent: 0, caret: Some(true),  label: "src",            meta: None,      selected: false },
            TreeRow { indent: 1, caret: Some(true),  label: "widgets",        meta: None,      selected: false },
            TreeRow { indent: 2, caret: None,         label: "button.rs",      meta: Some("4 KB"), selected: true },
            TreeRow { indent: 2, caret: None,         label: "checkbox.rs",    meta: Some("2 KB"), selected: false },
            TreeRow { indent: 2, caret: None,         label: "select.rs",      meta: Some("3 KB"), selected: false },
            TreeRow { indent: 1, caret: Some(false), label: "renderer.rs",    meta: None,      selected: false },
            TreeRow { indent: 0, caret: Some(false), label: "Cargo.toml",     meta: None,      selected: false },
        ];
        let mut dc = tree(TreeSpec {
            rect: Rect::new(lx, y, 280.0, 0.0),
            rows: &rows,
        }, ts);
        cmds.extend(dc.0.drain(..));
        y += rows.len() as f32 * 20.0 + 8.0 + 4.0 + SEC_GAP;
    }

    // ── 9 · TOOLTIP + MENU ───────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "09", "TOOLTIP · MENU");
    y += group_label(ts, &mut cmds, lx, y, "TOOLTIPS");
    {
        let mut dc1 = tooltip(TooltipSpec {
            rect: Rect::new(lx, y, 0.0, 0.0),
            text: "Keyboard shortcut: Ctrl+Z",
            variant: TooltipVariant::Dark,
        }, ts);
        cmds.extend(dc1.0.drain(..));
        let mut dc2 = tooltip(TooltipSpec {
            rect: Rect::new(lx + 240.0, y, 0.0, 0.0),
            text: "Destructive — cannot be undone",
            variant: TooltipVariant::Rust,
        }, ts);
        cmds.extend(dc2.0.drain(..));
        y += 32.0 + GROUP_GAP;
    }

    y += group_label(ts, &mut cmds, lx, y, "MENU");
    {
        let items = [
            MenuItem::Group("Edit"),
            MenuItem::Item { label: "Cut",   shortcut: Some("Ctrl+X"), selected: false, disabled: false },
            MenuItem::Item { label: "Copy",  shortcut: Some("Ctrl+C"), selected: false, disabled: false },
            MenuItem::Item { label: "Paste", shortcut: Some("Ctrl+V"), selected: false, disabled: true  },
            MenuItem::Separator,
            MenuItem::Item { label: "Select All", shortcut: Some("Ctrl+A"), selected: true, disabled: false },
        ];
        let mut dc = menu(MenuSpec {
            rect: Rect::new(lx, y, 220.0, 0.0),
            items: &items,
        }, ts);
        cmds.extend(dc.0.drain(..));
        let menu_h = items.iter().map(|i| match i {
            MenuItem::Item { .. } => 26.0,
            MenuItem::Separator   => 9.0,
            MenuItem::Group(_)    => 22.0,
        }).sum::<f32>() + 8.0;
        y += menu_h + SEC_GAP;
    }

    // ── 10 · WINDOW CHROME ───────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "10", "WINDOW CHROME");
    {
        let buttons = [WindowButton { symbol: "×" }];
        let win_result = window(WindowSpec {
            rect:        Rect::new(lx, y, 360.0, 200.0),
            title:       "Properties",
            buttons:     &buttons,
            status_bar:  true,
            status_text: "3 items selected",
        }, ts);
        cmds.extend(win_result.draw.0);
        y += 200.0 + SEC_GAP;
    }

    // ── 11 · COLOUR PALETTE ──────────────────────────────────────────────────
    y += section_header(ts, &mut cmds, lx, y, content_w, "11", "COLOUR PALETTE");
    y += group_label(ts, &mut cmds, lx, y, "THEME TOKENS");
    {
        let swatches: &[(Color, &str)] = &[
            (t.ink,        "#15130f  ink"),
            (t.paper,      "#f4f1ea  paper"),
            (t.paper_elev, "#fbf9f4  paper-elev"),
            (t.rust,       "#c25a2c  rust"),
            (t.muted,      "#8a8378  muted"),
        ];
        let mut bx = lx;
        for (color, hex) in swatches {
            color_swatch(ts, &mut cmds, bx, y, *color, hex);
            let lyt = ts.prepare(hex, t.text_md);
            bx += lyt.size.x + 26.0 + 20.0 + 12.0;
        }
        y += t.h_md + 80.0;
    }

    cmds.push(DrawCmd::PopClip);

    cmds
}

/// Total virtual content height (used by scroll clamping).
pub const CONTENT_HEIGHT: f32 = 3200.0;
