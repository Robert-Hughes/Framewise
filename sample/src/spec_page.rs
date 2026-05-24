/// Interactive widget specification page — showcases all Framewise widgets.
///
/// Uses the real Builder, Input and FocusSystem. Placeholder widgets (checkbox,
/// radio, switch, etc.) are called directly; interactive widgets (button,
/// text_edit, slider) go through the Builder.
use framewise::{
    builder::{Builder, BuilderCtx},
    draw::DrawCmd,
    focus::FocusSystem,
    input::Input,
    layout::{Layout, ManualLayout},
    theme::Theme,
    types::{Color, Rect, Vec2},
    widgets::{
        button::{ButtonStyle, ButtonState},
        checkbox::{CheckState, CheckboxSpec, checkbox},
        chip::{ChipSpec, chip},
        color_swatch::{ColorSwatchSpec, color_swatch},
        drag_number::{DragNumberSpec, drag_number},
        keycap::{KeycapSpec, keycap},
        menu::{MenuItem, MenuSpec, menu},
        meter::{MeterSpec, meter},
        progress_bar::{ProgressBarSpec, progress_bar},
        radio::{RadioSpec, radio},
        select::{SelectSpec, select},
        segmented::{SegmentedSpec, segmented},
        slider::SliderState,
        spinner::{SpinnerSpec, spinner},
        status::{StatusSpec, StatusVariant, status},
        switch::{SwitchSpec, switch},
        tabs::{TabsSpec, tabs},
        text_edit::{TextEditState},
        tooltip::{TooltipSpec, TooltipVariant, tooltip},
        tree::{TreeRow, TreeSpec, tree},
        window::{WindowButton, WindowSpec, window},
    },
};
use crate::text::SampleTextSystem;

// ── Page state ────────────────────────────────────────────────────────────────

/// Persistent mutable state for the spec page.
pub struct SpecPageState {
    pub scroll_y: f32,

    // Section 01 – Buttons
    pub btn_secondary: ButtonState,
    pub btn_primary:   ButtonState,
    pub btn_accent:    ButtonState,
    pub btn_ghost:     ButtonState,
    pub btn_disabled:  ButtonState,

    // Section 02 – Text Inputs
    pub te_normal:   TextEditState,
    pub te_error:    TextEditState,
    pub te_disabled: TextEditState,

    // Section 04 – Slider
    pub slider_state: SliderState,
    pub slider_val:   f32,
}

impl Default for SpecPageState {
    fn default() -> Self {
        Self {
            scroll_y: 0.0,
            btn_secondary: ButtonState::default(),
            btn_primary:   ButtonState::default(),
            btn_accent:    ButtonState::default(),
            btn_ghost:     ButtonState::default(),
            btn_disabled:  ButtonState::default(),
            te_normal:   TextEditState::new(""),
            te_error:    TextEditState::new("Invalid value"),
            te_disabled: TextEditState::new("Read only"),
            slider_state: SliderState::default(),
            slider_val:   55.0,
        }
    }
}

// ── Layout constants ──────────────────────────────────────────────────────────

const MARGIN:     f32 = 64.0;
const SEC_GAP:    f32 = 64.0;
const GROUP_GAP:  f32 = 28.0;
const COL_GAP:    f32 = 16.0;

/// Total virtual content height (used by scroll clamping).
pub const CONTENT_HEIGHT: f32 = 4200.0;

// ── Draw helpers ──────────────────────────────────────────────────────────────

fn sec_y(builder: &mut Builder<SampleTextSystem, framewise::layout::ManualState>, t: &Theme, lx: f32, y: f32, w: f32, num: &str, title: &str) {
    // Thin divider line
    builder.append_cmds(vec![DrawCmd::StrokeLine {
        p0: Vec2::new(lx, y + 18.0),
        p1: Vec2::new(lx + w, y + 18.0),
        color: t.line,
        width: 1.0,
    }]);
    builder.label_styled(Rect::new(lx, y, 40.0, 20.0), num, t.text_sm, t.muted, false);
    builder.label_styled(Rect::new(lx + 44.0, y, w - 44.0, 22.0), title, 18.0, t.ink, false);
}

fn group_y(builder: &mut Builder<SampleTextSystem, framewise::layout::ManualState>, t: &Theme, lx: f32, y: f32, text: &str) {
    builder.label_styled(Rect::new(lx, y, 300.0, 16.0), text, t.text_sm, t.muted, false);
}

// ── Main function ─────────────────────────────────────────────────────────────

pub fn draw_spec_page(
    ts: &mut SampleTextSystem,
    focus_sys: &mut FocusSystem,
    state: &mut SpecPageState,
    input: &Input,
    time: f64,
    win_w: f32,
    win_h: f32,
) -> Vec<DrawCmd> {
    let t = Theme::framewise();

    let content_w = (win_w - MARGIN * 2.0).min(1100.0);
    let lx = (win_w - content_w) * 0.5;
    let scroll_y = state.scroll_y;

    // Helper: translate content-y to screen-y.
    let sy = |cy: f32| cy - scroll_y;

    // Build ctx to match the Framewise theme.
    let mut ctx = BuilderCtx::default();
    ctx.text_color  = t.ink;
    ctx.bg_color    = t.paper;
    ctx.text_size   = t.text_md;
    ctx.time        = time;
    ctx.button_style = ButtonStyle::default();

    let win_rect = Rect::new(0.0, 0.0, win_w, win_h);
    let mut b = Builder::new(ctx, ts, focus_sys, ManualLayout.begin(win_rect));

    // Background + clip
    b.append_cmds(vec![
        DrawCmd::FillRect  { rect: win_rect, color: t.paper },
        DrawCmd::PushClip  { rect: win_rect },
    ]);

    // ── Page title ────────────────────────────────────────────────────────────
    b.label_styled(Rect::new(lx, sy(MARGIN), content_w, 56.0), "FRAMEWISE", 48.0, t.ink, false);
    b.label_styled(Rect::new(lx, sy(MARGIN + 56.0), content_w, 20.0), "Widget Reference  ·  v0.1", t.text_md, t.muted, true);

    let mut y = MARGIN + 96.0;  // content-space cursor

    // ── 01 · BUTTON ──────────────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "01", "BUTTON");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "VARIANTS");
    y += 20.0;
    {
        let mut bx = lx;
        let h = t.h_md;

        let btn = b.button_styled(
            std::mem::take(&mut state.btn_secondary),
            Rect::new(bx, sy(y), 100.0, h),
            "Secondary", ButtonStyle::default(), false, input,
        );
        state.btn_secondary = btn.state;
        bx += 100.0 + COL_GAP;

        let btn = b.button_styled(
            std::mem::take(&mut state.btn_primary),
            Rect::new(bx, sy(y), 90.0, h),
            "Primary", ButtonStyle::primary(), false, input,
        );
        state.btn_primary = btn.state;
        bx += 90.0 + COL_GAP;

        let btn = b.button_styled(
            std::mem::take(&mut state.btn_accent),
            Rect::new(bx, sy(y), 90.0, h),
            "Accent", ButtonStyle::accent(), false, input,
        );
        state.btn_accent = btn.state;
        bx += 90.0 + COL_GAP;

        let btn = b.button_styled(
            std::mem::take(&mut state.btn_ghost),
            Rect::new(bx, sy(y), 80.0, h),
            "Ghost", ButtonStyle::ghost(), false, input,
        );
        state.btn_ghost = btn.state;
        bx += 80.0 + COL_GAP;

        // Disabled
        let btn = b.button_styled(
            std::mem::take(&mut state.btn_disabled),
            Rect::new(bx, sy(y), 90.0, h),
            "Disabled", ButtonStyle::default(), true, input,
        );
        state.btn_disabled = btn.state;
    }
    y += t.h_md + SEC_GAP;

    // ── 02 · TEXT INPUT ───────────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "02", "TEXT INPUT");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "STATES  (normal · error · disabled)");
    y += 20.0;
    {
        let w = 220.0;
        let h = t.h_md;

        let info = b.text_edit_ext(
            std::mem::take(&mut state.te_normal),
            Rect::new(lx, sy(y), w, h),
            false, false, input,
        );
        state.te_normal = info.state;

        let info = b.text_edit_ext(
            std::mem::take(&mut state.te_error),
            Rect::new(lx + w + COL_GAP, sy(y), w, h),
            true, false, input,
        );
        state.te_error = info.state;

        let info = b.text_edit_ext(
            std::mem::take(&mut state.te_disabled),
            Rect::new(lx + (w + COL_GAP) * 2.0, sy(y), w, h),
            false, true, input,
        );
        state.te_disabled = info.state;
    }
    y += t.h_md + SEC_GAP;

    // ── 03 · CHECK · RADIO · SWITCH ──────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "03", "CHECK · RADIO · SWITCH");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "CHECKBOX");
    y += 20.0;
    {
        let specs: &[(CheckState, bool, bool, &str)] = &[
            (CheckState::Off,           false, false, "Off"),
            (CheckState::On,            false, false, "On"),
            (CheckState::Indeterminate, false, false, "Indeterminate"),
            (CheckState::On,            true,  false, "Focused"),
            (CheckState::Off,           false, true,  "Disabled"),
        ];
        let mut bx = lx;
        for (state_v, focused, disabled, label) in specs {
            let dc = checkbox(CheckboxSpec {
                rect: Rect::new(bx, sy(y), 14.0, 14.0),
                state: *state_v, focused: *focused, disabled: *disabled,
            });
            b.append_cmds(dc.0);
            b.label_styled(Rect::new(bx + 18.0, sy(y), 80.0, 14.0), label, t.text_md, t.ink, false);
            let lw = 18.0 + label.len() as f32 * 7.5 + COL_GAP;
            bx += lw;
        }
    }
    y += 14.0 + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), "RADIO");
    y += 20.0;
    {
        let specs: &[(bool, bool, bool, &str)] = &[
            (false, false, false, "Unselected"),
            (true,  false, false, "Selected"),
            (true,  true,  false, "Focused"),
            (false, false, true,  "Disabled"),
        ];
        let mut bx = lx;
        for (selected, focused, disabled, label) in specs {
            let dc = radio(RadioSpec {
                rect: Rect::new(bx, sy(y), 14.0, 14.0),
                selected: *selected, focused: *focused, disabled: *disabled,
            });
            b.append_cmds(dc.0);
            b.label_styled(Rect::new(bx + 18.0, sy(y), 90.0, 14.0), label, t.text_md, t.ink, false);
            let lw = 18.0 + label.len() as f32 * 7.5 + COL_GAP;
            bx += lw;
        }
    }
    y += 14.0 + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), "SWITCH");
    y += 20.0;
    {
        let specs: &[(bool, bool, bool, &str)] = &[
            (false, false, false, "Off"),
            (true,  false, false, "On"),
            (true,  true,  false, "Focused"),
            (false, false, true,  "Disabled"),
        ];
        let mut bx = lx;
        for (on, focused, disabled, label) in specs {
            let dc = switch(SwitchSpec {
                rect: Rect::new(bx, sy(y), 30.0, 16.0),
                on: *on, focused: *focused, disabled: *disabled,
            });
            b.append_cmds(dc.0);
            b.label_styled(Rect::new(bx + 34.0, sy(y), 80.0, 16.0), label, t.text_md, t.ink, false);
            let lw = 34.0 + label.len() as f32 * 7.5 + COL_GAP;
            bx += lw;
        }
    }
    y += 16.0 + SEC_GAP;

    // ── 04 · SLIDERS · DRAG NUMBER ───────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "04", "SLIDERS · DRAG NUMBER");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "DRAG NUMBER");
    y += 20.0;
    {
        let items: &[(&str, f32, f32, f32, bool)] = &[
            ("Width",  240.0, 0.0, 800.0, false),
            ("Height", 180.0, 0.0, 600.0, false),
            ("Scale",  1.25,  0.0,   4.0, true),
        ];
        let mut bx = lx;
        for (label, val, min, max, active) in items {
            let dc = drag_number(DragNumberSpec {
                rect: Rect::new(bx, sy(y), 168.0, t.h_md),
                label, value: *val, min: *min, max: *max, active: *active,
            }, b.text_system);
            b.append_cmds(dc.0);
            bx += 168.0 + COL_GAP;
        }
    }
    y += t.h_md + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), &format!("SLIDER  ({:.0})", state.slider_val));
    y += 20.0;
    {
        b.slider(
            &mut state.slider_state,
            &mut state.slider_val,
            0.0, 100.0, 10.0,
            framewise::widgets::slider::Orientation::Horizontal,
            Rect::new(lx, sy(y), content_w.min(400.0), t.h_md),
            input,
        );
    }
    y += t.h_md + SEC_GAP;

    // ── 05 · SELECT · SEGMENTED · CHIP ───────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "05", "SELECT · SEGMENTED · CHIP");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "SELECT");
    y += 20.0;
    {
        let opts = ["Option A", "Option B", "Option C"];
        let dc = select(SelectSpec {
            rect: Rect::new(lx, sy(y), 200.0, t.h_md),
            value: "Option A", options: &opts, open: false, focused: false, hovered: None,
        }, b.text_system);
        b.append_cmds(dc.0);
        let dc = select(SelectSpec {
            rect: Rect::new(lx + 216.0, sy(y), 200.0, t.h_md),
            value: "Option B", options: &opts, open: false, focused: true, hovered: None,
        }, b.text_system);
        b.append_cmds(dc.0);
        let dc = select(SelectSpec {
            rect: Rect::new(lx + 432.0, sy(y), 200.0, t.h_md),
            value: "Option A", options: &opts, open: true, focused: true, hovered: Some(1),
        }, b.text_system);
        b.append_cmds(dc.0);
    }
    y += t.h_md + opts_dropdown_h(3) + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), "SEGMENTED");
    y += 20.0;
    {
        let items = ["Grid", "List", "Gallery"];
        let dc = segmented(SegmentedSpec {
            rect: Rect::new(lx, sy(y), 0.0, t.h_md),
            items: &items, active_index: 0, focused: None,
        }, b.text_system);
        b.append_cmds(dc.0);
        let dc = segmented(SegmentedSpec {
            rect: Rect::new(lx + 200.0, sy(y), 0.0, t.h_md),
            items: &items, active_index: 1, focused: Some(2),
        }, b.text_system);
        b.append_cmds(dc.0);
    }
    y += t.h_md + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), "CHIPS");
    y += 20.0;
    {
        let chip_data: &[(&str, bool)] = &[
            ("Design", false), ("Rust", true), ("WGPU", false),
            ("Open Source", true), ("v0.1", false),
        ];
        let mut bx = lx;
        for (label, active) in chip_data {
            let dc = chip(ChipSpec {
                rect: Rect::new(bx, sy(y), 0.0, 22.0),
                label, active: *active, focused: false,
            }, b.text_system);
            let chip_w = dc.0.iter().fold(0.0_f32, |acc, cmd| match cmd {
                DrawCmd::FillRect { rect, .. } | DrawCmd::StrokeRect { rect, .. } => acc.max(rect.x + rect.w - bx),
                _ => acc,
            });
            b.append_cmds(dc.0);
            bx += chip_w + 8.0;
        }
    }
    y += 22.0 + SEC_GAP;

    // ── 06 · TABS ─────────────────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "06", "TABS");
    y += 46.0;
    {
        let items = ["Overview", "Properties", "Inspector", "Logs"];
        let dc = tabs(TabsSpec {
            rect: Rect::new(lx, sy(y), content_w, 36.0),
            items: &items, active_index: 1, focused: None,
        }, b.text_system);
        b.append_cmds(dc.0);
    }
    y += 36.0 + SEC_GAP;

    // ── 07 · PROGRESS · SPINNER · STATUS ─────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "07", "PROGRESS · SPINNER · STATUS");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "PROGRESS BARS");
    y += 20.0;
    {
        let items: &[(f32, bool, &str)] = &[
            (0.0,      false, "0%"),
            (0.35,     false, "35%"),
            (0.75,     false, "75%"),
            (1.0,      false, "100%"),
            (0.6,      true,  "Active 60%"),
            (f32::NAN, true,  "Indeterminate"),
        ];
        let bar_w = 160.0_f32;
        let row_h = 24.0_f32;
        let mut bx = lx;
        let start_y = y;
        let mut col = 0;
        for (val, active, label) in items {
            let row_y = start_y + col as f32 * row_h;
            let dc = progress_bar(ProgressBarSpec {
                rect: Rect::new(bx, sy(row_y) + 10.0, bar_w, 3.0),
                value: *val, phase: 0.4, active: *active,
            });
            b.append_cmds(dc.0);
            b.label_styled(Rect::new(bx + bar_w + 8.0, sy(row_y) + 4.0, 120.0, 16.0), label, t.text_sm, t.muted, false);
            col += 1;
            if col == 3 { col = 0; bx += bar_w + 100.0; }
        }
        y += 3.0 * row_h + GROUP_GAP;
    }

    group_y(&mut b, &t, lx, sy(y), "SPINNERS");
    y += 20.0;
    {
        let dc = spinner(SpinnerSpec { rect: Rect::new(lx, sy(y), 16.0, 16.0), large: false, color: None });
        b.append_cmds(dc.0);
        let dc = spinner(SpinnerSpec { rect: Rect::new(lx + 32.0, sy(y), 24.0, 24.0), large: true, color: None });
        b.append_cmds(dc.0);
        let dc = spinner(SpinnerSpec { rect: Rect::new(lx + 72.0, sy(y), 16.0, 16.0), large: false, color: Some(t.rust) });
        b.append_cmds(dc.0);
    }
    y += 24.0 + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), "STATUS");
    y += 20.0;
    {
        let items: &[(&str, StatusVariant)] = &[
            ("Nominal", StatusVariant::Ok),
            ("Warning", StatusVariant::Warn),
            ("Error",   StatusVariant::Err),
            ("Live",    StatusVariant::Live),
            ("Offline", StatusVariant::Neutral),
        ];
        let mut bx = lx;
        for (label, variant) in items {
            let dc = status(StatusSpec { rect: Rect::new(bx, sy(y), 120.0, 12.0), label, variant: *variant }, b.text_system);
            b.append_cmds(dc.0);
            bx += 124.0;
        }
    }
    y += 12.0 + SEC_GAP;

    // ── 08 · METER ────────────────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "08", "METER");
    y += 46.0;
    {
        let dc = meter(MeterSpec { rect: Rect::new(lx, sy(y), 120.0, 14.0), value: 0.7, peak: Some(0.85), bars: 15 });
        b.append_cmds(dc.draw.0);
        let dc = meter(MeterSpec { rect: Rect::new(lx + 140.0, sy(y), 120.0, 14.0), value: 0.3, peak: None, bars: 15 });
        b.append_cmds(dc.draw.0);
        let dc = meter(MeterSpec { rect: Rect::new(lx + 280.0, sy(y), 120.0, 14.0), value: 1.0, peak: Some(1.0), bars: 15 });
        b.append_cmds(dc.draw.0);
    }
    y += 14.0 + SEC_GAP;

    // ── 09 · TREE ─────────────────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "09", "TREE / LIST");
    y += 46.0;
    {
        let rows = [
            TreeRow { indent: 0, caret: Some(true),  label: "src",         meta: None,         selected: false },
            TreeRow { indent: 1, caret: Some(true),  label: "widgets",     meta: None,         selected: false },
            TreeRow { indent: 2, caret: None,         label: "button.rs",  meta: Some("4 KB"), selected: true  },
            TreeRow { indent: 2, caret: None,         label: "checkbox.rs", meta: Some("2 KB"), selected: false },
            TreeRow { indent: 2, caret: None,         label: "select.rs",  meta: Some("3 KB"), selected: false },
            TreeRow { indent: 1, caret: Some(false),  label: "renderer.rs", meta: None,        selected: false },
            TreeRow { indent: 0, caret: Some(false),  label: "Cargo.toml", meta: None,         selected: false },
        ];
        let dc = tree(TreeSpec { rect: Rect::new(lx, sy(y), 280.0, 0.0), rows: &rows }, b.text_system);
        b.append_cmds(dc.0);
        y += rows.len() as f32 * 20.0 + 12.0;
    }
    y += SEC_GAP;

    // ── 10 · TOOLTIP · MENU ──────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "10", "TOOLTIP · MENU");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "TOOLTIPS");
    y += 20.0;
    {
        let dc = tooltip(TooltipSpec {
            rect: Rect::new(lx, sy(y), 0.0, 0.0),
            text: "Keyboard shortcut: Ctrl+Z",
            variant: TooltipVariant::Dark,
        }, b.text_system);
        b.append_cmds(dc.0);
        let dc = tooltip(TooltipSpec {
            rect: Rect::new(lx + 260.0, sy(y), 0.0, 0.0),
            text: "Destructive — cannot be undone",
            variant: TooltipVariant::Rust,
        }, b.text_system);
        b.append_cmds(dc.0);
    }
    y += 36.0 + GROUP_GAP;

    group_y(&mut b, &t, lx, sy(y), "MENU");
    y += 20.0;
    {
        let items = [
            MenuItem::Group("Edit"),
            MenuItem::Item { label: "Cut",        shortcut: Some("Ctrl+X"), selected: false, disabled: false },
            MenuItem::Item { label: "Copy",       shortcut: Some("Ctrl+C"), selected: false, disabled: false },
            MenuItem::Item { label: "Paste",      shortcut: Some("Ctrl+V"), selected: false, disabled: true  },
            MenuItem::Separator,
            MenuItem::Item { label: "Select All", shortcut: Some("Ctrl+A"), selected: true,  disabled: false },
        ];
        let dc = menu(MenuSpec { rect: Rect::new(lx, sy(y), 220.0, 0.0), items: &items }, b.text_system);
        b.append_cmds(dc.0);
        let menu_h: f32 = items.iter().map(|i| match i {
            MenuItem::Item { .. } => 26.0,
            MenuItem::Separator   =>  9.0,
            MenuItem::Group(_)    => 22.0,
        }).sum::<f32>() + 8.0;
        y += menu_h;
    }
    y += SEC_GAP;

    // ── 11 · WINDOW CHROME ───────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "11", "WINDOW CHROME");
    y += 46.0;
    {
        let buttons = [WindowButton { symbol: "×" }];
        let win_result = window(WindowSpec {
            rect:        Rect::new(lx, sy(y), 360.0, 180.0),
            title:       "Properties",
            buttons:     &buttons,
            status_bar:  true,
            status_text: "3 items selected",
        }, b.text_system);
        b.append_cmds(win_result.draw.0);
        y += 180.0 + SEC_GAP;
    }

    // ── 12 · COLOUR PALETTE ──────────────────────────────────────────────────
    sec_y(&mut b, &t, lx, sy(y), content_w, "12", "COLOUR PALETTE");
    y += 46.0;

    group_y(&mut b, &t, lx, sy(y), "THEME TOKENS");
    y += 20.0;
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
            let swatch_w = 120.0_f32;
            let dc = color_swatch(ColorSwatchSpec {
                rect:   Rect::new(bx, sy(y), 18.0, t.h_md),
                color:  *color,
                border: t.line,
            });
            b.append_cmds(dc.draw.0);
            b.label_styled(Rect::new(bx + 22.0, sy(y) + 4.0, swatch_w - 22.0, 20.0), hex, t.text_sm, t.ink, false);
            bx += swatch_w + 8.0;
        }
        y += t.h_md + GROUP_GAP;
    }

    group_y(&mut b, &t, lx, sy(y), "KEYCAPS");
    y += 20.0;
    {
        let keys = ["Ctrl", "Alt", "Shift", "Tab", "Enter", "Esc", "F2", "↑", "↓"];
        let mut bx = lx;
        for key in keys {
            let kw = key.len() as f32 * 7.5 + 12.0;
            let kw = kw.max(24.0);
            let dc = keycap(KeycapSpec {
                rect: Rect::new(bx, sy(y), kw, 22.0),
                label: key,
                ..Default::default()
            }, b.text_system);
            b.append_cmds(dc.draw.0);
            bx += kw + 6.0;
        }
    }
    y += 22.0 + SEC_GAP;

    let _ = y; // suppress unused warning

    b.append_cmds(vec![DrawCmd::PopClip]);
    b.finish()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn opts_dropdown_h(n: usize) -> f32 {
    n as f32 * 26.0 + 8.0
}
