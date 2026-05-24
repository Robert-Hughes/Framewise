use crate::text::SampleTextSystem;
use framewise::text::TextSystem;
/// Interactive widget specification page — mirrors mockups/Framewise Widgets.html.
use framewise::{
    builder::{Builder, BuilderCtx},
    draw::DrawCmd,
    focus::FocusSystem,
    input::Input,
    layout::{Layout, LayoutState, ManualLayout},
    theme::Theme,
    types::{Color, Rect, Vec2},
    widgets::{
        button::{ButtonState, ButtonStyle},
        checkbox::{checkbox, CheckState},
        chip::chip,
        color_swatch::color_swatch,
        drag_number::drag_number,
        frame::FrameSpec,
        frame::{frame, FrameStyle},
        keycap::keycap,
        menu::{menu, MenuItem},
        meter::meter,
        progress_bar::progress_bar,
        radio::radio,
        scroll_area::{begin_scroll_area, ScrollState, ScrollbarVisibility},
        segmented::segmented,
        select::select,
        slider::{Orientation as SliderOrientation, SliderState},
        spinner::spinner,
        status::{status, StatusVariant},
        switch::switch,
        tabs::tabs,
        text_edit::TextEditState,
        tooltip::{tooltip, TooltipVariant},
        tree::{tree, TreeRow},
        window::WindowButton,
        CheckboxSpecBuilder, ProgressBarSpecBuilder, RadioSpecBuilder, SpinnerSpecBuilder,
        SwitchSpecBuilder,
    },
};

// ── Page state ────────────────────────────────────────────────────────────────

pub struct SpecPageState {
    pub page_scroll: ScrollState,

    // 01 Buttons
    pub btn_variants: Vec<ButtonState>, // [secondary, primary, accent, ghost]
    pub btn_matrix: Vec<ButtonState>,   // 4 variants × 5 states = 20
    pub btn_sizes: Vec<ButtonState>,    // [sm, md, lg]
    pub btn_grp1: Vec<ButtonState>,     // [←, Frame 248, →]
    pub btn_grp2: Vec<ButtonState>,     // [Build, Run, Ship]

    // 02 Text Inputs
    pub te_matrix: Vec<TextEditState>, // 2 rows × 5 cols = 10
    pub te_labelled: TextEditState,
    pub te_prefixed: TextEditState,
    pub te_multiline: TextEditState,

    // 04 Sliders
    pub slider1_state: SliderState,
    pub slider1_val: f32,
    pub slider2_state: SliderState,
    pub slider2_val: f32,
    pub slider3_state: SliderState,
    pub slider3_val: f32,
    pub slider4_state: SliderState,
    pub slider4_val: f32, // stepped 0–9

    // 06 Scroll areas
    pub scroll_vert: ScrollState,
    pub scroll_horiz: ScrollState,
    pub scroll_both: ScrollState,

    // 12 In Use
    pub iu_fps_slider: SliderState,
    pub iu_fps_val: f32,
    pub iu_btns: Vec<ButtonState>, // [Reset, Cancel, Apply]
    pub iu_log_scroll: ScrollState,
}

impl Default for SpecPageState {
    fn default() -> Self {
        let mut te_matrix: Vec<TextEditState> = Vec::with_capacity(10);
        for i in 0..10 {
            te_matrix.push(match i {
                3 => TextEditState::new("§ invalid"),
                5 => TextEditState::new("render_pass"),
                6 => TextEditState::new("render_pass"),
                7 => TextEditState::new("render_pass"),
                8 => TextEditState::new("render pass"),
                9 => TextEditState::new("render_pass"),
                _ => TextEditState::new(""),
            });
        }
        Self {
            page_scroll: ScrollState::default(),
            btn_variants: (0..4).map(|_| ButtonState::default()).collect(),
            btn_matrix: (0..20).map(|_| ButtonState::default()).collect(),
            btn_sizes: (0..3).map(|_| ButtonState::default()).collect(),
            btn_grp1: (0..3).map(|_| ButtonState::default()).collect(),
            btn_grp2: (0..3).map(|_| ButtonState::default()).collect(),
            te_matrix,
            te_labelled: TextEditState::new("framewise"),
            te_prefixed: TextEditState::new("0.1.0"),
            te_multiline: TextEditState::new(
                "A small, procedural Rust library for describing GUI elements per frame.",
            ),
            slider1_state: SliderState::default(),
            slider1_val: 0.14,
            slider2_state: SliderState::default(),
            slider2_val: 0.62,
            slider3_state: SliderState::default(),
            slider3_val: 0.88,
            slider4_state: SliderState::default(),
            slider4_val: 3.0,
            scroll_vert: ScrollState::default(),
            scroll_horiz: ScrollState::default(),
            scroll_both: ScrollState::default(),
            iu_fps_slider: SliderState::default(),
            iu_fps_val: 60.0,
            iu_btns: (0..3).map(|_| ButtonState::default()).collect(),
            iu_log_scroll: ScrollState::default(),
        }
    }
}

// ── Layout constants ──────────────────────────────────────────────────────────

const MARGIN: f32 = 64.0;
const SEC_GAP: f32 = 64.0;
const GROUP_GAP: f32 = 28.0;
const COL_GAP: f32 = 16.0;

pub const CONTENT_HEIGHT: f32 = 5600.0;

// ── Draw helpers ──────────────────────────────────────────────────────────────

fn sec_y<S: LayoutState<Params = Rect>>(
    b: &mut Builder<SampleTextSystem, S>,
    t: &Theme,
    lx: f32,
    y: f32,
    w: f32,
    num: &str,
    title: &str,
) {
    b.divider(Rect::new(lx, y, w, 36.0));
    b.label_styled(Rect::new(lx, y, 40.0, 20.0), num, t.text_sm, t.muted, false);
    b.label_styled_font(
        Rect::new(lx + 44.0, y, w - 44.0, 22.0),
        title,
        18.0,
        t.ink,
        false,
        t.sans_font,
    );
}

fn label_styled_sans<S: LayoutState<Params = Rect>>(
    b: &mut Builder<SampleTextSystem, S>,
    t: &Theme,
    rect: Rect,
    text: &str,
    size: f32,
    color: Color,
) {
    b.label_styled_font(rect, text, size, color, false, t.sans_font);
}

fn group_y<S: LayoutState<Params = Rect>>(
    b: &mut Builder<SampleTextSystem, S>,
    t: &Theme,
    lx: f32,
    y: f32,
    text: &str,
) {
    b.label_styled(
        Rect::new(lx, y, 400.0, 16.0),
        text,
        t.text_sm,
        t.muted,
        false,
    );
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

    let mut ctx = BuilderCtx::default();
    ctx.text_color = t.ink;
    ctx.bg_color = t.paper;
    ctx.text_size = t.text_md;
    ctx.text_font = t.mono_font;
    ctx.time = time;
    ctx.button_style = ButtonStyle::default();

    let win_rect = Rect::new(0.0, 0.0, win_w, win_h);
    let mut b = Builder::new(ctx, ts, focus_sys, ManualLayout.begin(win_rect));

    // Background fill (outside clip so it covers the whole viewport).
    let bg = frame(FrameSpec {
        rect: win_rect,
        style: FrameStyle {
            background: t.paper,
            border: t.paper,
            border_width: 0.0,
            padding: 0.0,
        },
    });
    b.append_cmds(bg.draw.0);

    // Scroll area provides clip + scroll offset for all page content.
    let page_cmds = {
        let mut page = b.scroll_area(
            win_rect,
            Vec2::new(content_w, CONTENT_HEIGHT),
            ScrollbarVisibility::None,
            ScrollbarVisibility::Auto,
            &mut state.page_scroll,
            ManualLayout,
            input,
        );
        {
            let mut b = &mut page;

            // ── HERO ─────────────────────────────────────────────────────────────────
            {
                b.custom(Rect::new(lx, MARGIN, 96.0, 96.0), |rect| {
                    hero_logo(&t, rect.x, rect.y)
                });

                let tx = lx + 116.0;
                b.label_styled(
                    Rect::new(tx, MARGIN, content_w - 116.0, 16.0),
                    "framewise · widget specification · v0.1",
                    t.text_sm,
                    t.muted,
                    false,
                );
                label_styled_sans(
                    b,
                    &t,
                    Rect::new(tx, MARGIN + 20.0, content_w - 116.0, 36.0),
                    "A widget set that explains itself.",
                    28.0,
                    t.ink,
                );
                label_styled_sans(
                    b,
                    &t,
                    Rect::new(tx, MARGIN + 60.0, (content_w - 116.0).min(600.0), 32.0),
                    "Sharp corners, hairline borders, monospaced numerics. One accent — rust — reserved for focus, drag, and primary action.",
                    t.text_md,
                    t.muted,
                );

                // color meta row
                let meta_items: &[(&str, &str)] = &[
                    ("ink", "#15130f"),
                    ("paper", "#f4f1ea"),
                    ("rust", "#c25a2c"),
                    ("type", "Inter Tight · JetBrains Mono"),
                ];
                let mut mx = tx;
                for (key, val) in meta_items {
                    b.label_styled(
                        Rect::new(mx, MARGIN + 96.0, 20.0, 14.0),
                        key,
                        t.text_sm,
                        t.ink,
                        false,
                    );
                    let key_w = key.len() as f32 * 7.5 + 4.0;
                    b.label_styled(
                        Rect::new(mx + key_w, MARGIN + 96.0, 200.0, 14.0),
                        val,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    mx += key_w + val.len() as f32 * 6.5 + 24.0;
                }
            }

            let mut y = MARGIN + 136.0;

            // ── 01 · BUTTONS ─────────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "01", "Buttons");
            y += 46.0;

            // variants row
            group_y(&mut b, &t, lx, y, "variants");
            y += 20.0;
            {
                let styles: &[(&str, ButtonStyle, bool)] = &[
                    ("Apply changes", ButtonStyle::primary(), false),
                    ("Cancel", ButtonStyle::default(), false),
                    ("Reset", ButtonStyle::ghost(), false),
                    ("Publish v0.2", ButtonStyle::accent(), false),
                ];
                let mut bx = lx;
                for (i, (label, style, _)) in styles.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 24.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_variants[i]),
                        Rect::new(bx, y, w, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_variants[i] = btn.state;
                    bx += w + COL_GAP;
                }
            }
            y += t.h_md + GROUP_GAP;

            // state matrix
            group_y(&mut b, &t, lx, y, "states · default button");
            y += 20.0;
            {
                let col_labels = ["default", "hover", "pressed", "focused", "disabled"];
                let row_labels = ["secondary", "primary", "accent", "ghost"];
                let row_styles: &[ButtonStyle] = &[
                    ButtonStyle::default(),
                    ButtonStyle::primary(),
                    ButtonStyle::accent(),
                    ButtonStyle::ghost(),
                ];
                let label_w = 80.0_f32;
                let cell_w = 88.0_f32;

                // column headers
                for (ci, col) in col_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx + label_w + ci as f32 * cell_w, y, cell_w - 8.0, 16.0),
                        col,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                }
                y += 20.0;

                for (ri, row_label) in row_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx, y, label_w - 8.0, t.h_md),
                        row_label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    for ci in 0..5 {
                        let idx = ri * 5 + ci;
                        let disabled = ci == 4;
                        let btn = b.button_styled(
                            std::mem::take(&mut state.btn_matrix[idx]),
                            Rect::new(lx + label_w + ci as f32 * cell_w, y, cell_w - 8.0, t.h_md),
                            "Action",
                            row_styles[ri].clone(),
                            disabled,
                            input,
                        );
                        state.btn_matrix[idx] = btn.state;
                    }
                    y += t.h_md + 4.0;
                }
            }
            y += GROUP_GAP;

            // sizes & groups
            group_y(&mut b, &t, lx, y, "sizes  ·  groups");
            y += 20.0;
            {
                let size_defs: &[(&str, f32, ButtonStyle)] = &[
                    ("22 px", t.h_sm, ButtonStyle::default()),
                    ("28 px", t.h_md, ButtonStyle::default()),
                    ("36 px", t.h_lg, ButtonStyle::default()),
                ];
                let mut bx = lx;
                for (i, (label, h, style)) in size_defs.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 20.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_sizes[i]),
                        Rect::new(bx, y, w, *h),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_sizes[i] = btn.state;
                    bx += w + COL_GAP;
                }
                bx += 24.0;

                // button group 1: ← | Frame 248 | →
                let grp1: &[(&str, ButtonStyle)] = &[
                    ("←", ButtonStyle::default()),
                    ("Frame 248", ButtonStyle::default()),
                    ("→", ButtonStyle::default()),
                ];
                // draw group border
                for (i, (label, style)) in grp1.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 20.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_grp1[i]),
                        Rect::new(bx, y, w, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_grp1[i] = btn.state;
                    bx += w;
                }
                bx += COL_GAP;

                // button group 2: Build | Run | Ship
                let grp2: &[(&str, ButtonStyle)] = &[
                    ("Build", ButtonStyle::default()),
                    ("Run", ButtonStyle::default()),
                    ("Ship", ButtonStyle::primary()),
                ];
                for (i, (label, style)) in grp2.iter().enumerate() {
                    let w = label.len() as f32 * 7.0 + 20.0;
                    let btn = b.button_styled(
                        std::mem::take(&mut state.btn_grp2[i]),
                        Rect::new(bx, y, w, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.btn_grp2[i] = btn.state;
                    bx += w;
                }
                let _ = bx;
            }
            y += t.h_md + SEC_GAP;

            // ── 02 · TEXT INPUTS ─────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "02", "Text inputs");
            y += 46.0;

            group_y(&mut b, &t, lx, y, "states · single-line");
            y += 20.0;
            {
                let col_labels = ["default", "hover", "focused", "error", "disabled"];
                let row_labels = ["empty", "filled"];
                let cell_w = 160.0_f32;
                let label_w = 60.0_f32;

                for (ci, col) in col_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx + label_w + ci as f32 * (cell_w + 8.0), y, cell_w, 16.0),
                        col,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                }
                y += 20.0;

                for (ri, row_label) in row_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx, y, label_w - 4.0, t.h_md),
                        row_label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    for ci in 0..5 {
                        let idx = ri * 5 + ci;
                        let error = ci == 3;
                        let disabled = ci == 4;
                        let info = b.text_edit_ext(
                            std::mem::take(&mut state.te_matrix[idx]),
                            Rect::new(lx + label_w + ci as f32 * (cell_w + 8.0), y, cell_w, t.h_md),
                            error,
                            disabled,
                            input,
                        );
                        state.te_matrix[idx] = info.state;
                    }
                    y += t.h_md + 8.0;
                }
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "labelled  ·  prefixed  ·  multiline");
            y += 20.0;
            {
                // Labelled field
                let field_x = lx;
                b.label_styled(
                    Rect::new(field_x, y, 120.0, 14.0),
                    "crate name",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let info = b.text_edit_ext(
                    std::mem::take(&mut state.te_labelled),
                    Rect::new(field_x, y + 18.0, 160.0, t.h_md),
                    false,
                    false,
                    input,
                );
                state.te_labelled = info.state;
                b.label_styled(
                    Rect::new(field_x, y + 18.0 + t.h_md + 4.0, 200.0, 14.0),
                    "a–z, 0–9, hyphen; max 64",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Prefixed field (draw prefix addon manually)
                let pf_x = lx + 200.0;
                b.label_styled(
                    Rect::new(pf_x, y, 120.0, 14.0),
                    "version",
                    t.text_sm,
                    t.muted,
                    false,
                );
                b.custom(Rect::new(pf_x, y + 18.0, 24.0, t.h_md), |rect| {
                    vec![
                        DrawCmd::FillRect { rect, color: t.ink },
                        DrawCmd::StrokeRect {
                            rect,
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(pf_x + 6.0, y + 18.0 + 7.0, 16.0, 14.0),
                    "v",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let info = b.text_edit_ext(
                    std::mem::take(&mut state.te_prefixed),
                    Rect::new(pf_x + 24.0, y + 18.0, 120.0, t.h_md),
                    false,
                    false,
                    input,
                );
                state.te_prefixed = info.state;
                b.label_styled(
                    Rect::new(pf_x, y + 18.0 + t.h_md + 4.0, 200.0, 14.0),
                    "semver mismatch — bump minor",
                    t.text_sm,
                    t.rust,
                    false,
                );

                // Multiline field
                let ml_x = lx + 420.0;
                b.label_styled(
                    Rect::new(ml_x, y, 120.0, 14.0),
                    "description",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let info = b.text_edit_ext(
                    std::mem::take(&mut state.te_multiline),
                    Rect::new(ml_x, y + 18.0, 280.0, 68.0),
                    false,
                    false,
                    input,
                );
                state.te_multiline = info.state;
            }
            y += 18.0 + 68.0 + 4.0 + 14.0 + SEC_GAP;

            // ── 03 · CHECK · RADIO · SWITCH ──────────────────────────────────────────
            sec_y(
                &mut b,
                &t,
                lx,
                y,
                content_w,
                "03",
                "Checkboxes, radios & switches",
            );
            y += 46.0;

            group_y(&mut b, &t, lx, y, "checkbox");
            y += 20.0;
            {
                let col_labels = ["off", "on", "mixed", "focused", "disabled"];
                let label_w = 80.0_f32;
                let cell_w = 100.0_f32;
                for (ci, col) in col_labels.iter().enumerate() {
                    b.label_styled(
                        Rect::new(lx + label_w + ci as f32 * cell_w, y, cell_w - 4.0, 14.0),
                        col,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                }
                y += 18.0;

                // Row 1: box only
                b.label_styled(
                    Rect::new(lx, y, label_w - 4.0, 14.0),
                    "box",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let box_specs: &[(CheckState, bool, bool)] = &[
                    (CheckState::Off, false, false),
                    (CheckState::On, false, false),
                    (CheckState::Indeterminate, false, false),
                    (CheckState::On, true, false),
                    (CheckState::On, false, true),
                ];
                for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
                    b.add(
                        Rect::new(lx + label_w + ci as f32 * cell_w, y, 14.0, 14.0),
                        checkbox,
                        CheckboxSpecBuilder::new(*cs)
                            .focused(*focused)
                            .disabled(*disabled),
                    );
                }
                y += 14.0 + 12.0;

                // Row 2: with label
                b.label_styled(
                    Rect::new(lx, y, label_w - 4.0, 14.0),
                    "with label",
                    t.text_sm,
                    t.muted,
                    false,
                );
                for (ci, (cs, focused, disabled)) in box_specs.iter().enumerate() {
                    let cx = lx + label_w + ci as f32 * cell_w;
                    b.add(
                        Rect::new(cx, y, 14.0, 14.0),
                        checkbox,
                        CheckboxSpecBuilder::new(*cs)
                            .focused(*focused)
                            .disabled(*disabled),
                    );

                    let label_alpha = if *disabled { t.muted } else { t.ink };
                    b.label_styled(
                        Rect::new(cx + 18.0, y, 60.0, 14.0),
                        "vsync",
                        t.text_sm,
                        label_alpha,
                        false,
                    );
                }
                y += 14.0;
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "radio  ·  switch");
            y += 20.0;
            {
                let radio_items: &[(bool, bool, bool, &str)] = &[
                    (true, false, false, "immediate-mode"),
                    (false, false, false, "retained-mode"),
                    (false, false, false, "hybrid"),
                    (false, true, false, "deferred"),
                ];
                let switch_items: &[(bool, bool, bool, &str)] = &[
                    (false, false, false, "debug overlay"),
                    (true, false, false, "show layout grid"),
                    (true, true, false, "vsync"),
                    (false, false, true, "multisampling"),
                ];
                for (i, (selected, focused, disabled, label)) in radio_items.iter().enumerate() {
                    let ry = y + i as f32 * 22.0;
                    b.add(
                        Rect::new(lx, ry, 14.0, 14.0),
                        radio,
                        RadioSpecBuilder::new()
                            .selected(*selected)
                            .focused(*focused)
                            .disabled(*disabled),
                    );
                    b.label_styled(
                        Rect::new(lx + 18.0, ry, 140.0, 14.0),
                        label,
                        t.text_md,
                        t.ink,
                        false,
                    );
                }
                let sw_x = lx + 220.0;
                for (i, (on, focused, disabled, label)) in switch_items.iter().enumerate() {
                    let ry = y + i as f32 * 22.0;
                    b.add(
                        Rect::new(sw_x, ry, 30.0, 16.0),
                        switch,
                        SwitchSpecBuilder::new()
                            .on(*on)
                            .focused(*focused)
                            .disabled(*disabled),
                    );
                    let label_color = if *disabled { t.muted } else { t.ink };
                    b.label_styled(
                        Rect::new(sw_x + 36.0, ry, 140.0, 16.0),
                        label,
                        t.text_md,
                        label_color,
                        false,
                    );
                }
            }
            y += 4.0 * 22.0 + SEC_GAP;

            // ── 04 · SLIDERS · DRAGS ─────────────────────────────────────────────────
            sec_y(
                &mut b,
                &t,
                lx,
                y,
                content_w,
                "04",
                "Sliders & numeric drags",
            );
            y += 46.0;

            group_y(&mut b, &t, lx, y, "slider · single value");
            y += 20.0;
            {
                let slider_w = 360.0_f32;
                let row_gap = 14.0_f32;

                b.slider(
                    &mut state.slider1_state,
                    &mut state.slider1_val,
                    0.0,
                    1.0,
                    0.1,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.2}", state.slider1_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                y += t.h_md + row_gap;

                b.slider(
                    &mut state.slider2_state,
                    &mut state.slider2_val,
                    0.0,
                    1.0,
                    0.1,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.2}", state.slider2_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                y += t.h_md + row_gap;

                b.slider(
                    &mut state.slider3_state,
                    &mut state.slider3_val,
                    0.0,
                    1.0,
                    0.1,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.2}", state.slider3_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                y += t.h_md + row_gap;

                // Stepped slider (0–9) with tick marks
                b.slider(
                    &mut state.slider4_state,
                    &mut state.slider4_val,
                    0.0,
                    9.0,
                    1.0,
                    SliderOrientation::Horizontal,
                    Rect::new(lx, y, slider_w, t.h_md),
                    input,
                );
                b.label_styled(
                    Rect::new(lx + slider_w + 12.0, y + 6.0, 80.0, 14.0),
                    &format!("{:.0} / 9", state.slider4_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                // tick marks below track
                let tick_y = y + t.h_md + 2.0;
                let tick_h = 4.0;
                let usable = slider_w - 12.0;
                for i in 0..=9usize {
                    let tx = lx + 6.0 + (i as f32 / 9.0) * usable;
                    b.custom(Rect::new(tx - 0.5, tick_y, 1.0, tick_h), |rect| {
                        vec![DrawCmd::FillRect {
                            rect,
                            color: t.line,
                        }]
                    });
                }
                y += t.h_md + 8.0;
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "range slider");
            y += 20.0;
            {
                let track_w = 360.0_f32;
                let mid_y = y + t.h_md * 0.5;
                b.custom(Rect::new(lx, mid_y - 0.75, track_w, 12.0), |rect| {
                    let lx = rect.x;
                    let track_w = rect.w;
                    let mid_y = rect.y + 0.75;
                    let t1 = 0.24_f32;
                    let t2 = 0.76_f32;
                    let fill_x1 = lx + track_w * t1;
                    let fill_x2 = lx + track_w * t2;
                    let ts = 12.0_f32; // thumb size
                    let half_ts = ts * 0.5;

                    vec![
                        // full track
                        DrawCmd::FillRect {
                            rect: Rect::new(lx, mid_y - 0.75, track_w, 1.5),
                            color: t.line,
                        },
                        // fill bar
                        DrawCmd::FillRect {
                            rect: Rect::new(fill_x1, mid_y - 0.75, fill_x2 - fill_x1, 1.5),
                            color: t.ink,
                        },
                        // thumb 1
                        DrawCmd::FillRect {
                            rect: Rect::new(fill_x1 - half_ts, mid_y - half_ts, ts, ts),
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(fill_x1 - half_ts, mid_y - half_ts, ts, ts),
                            color: t.ink,
                            width: 1.5,
                        },
                        // thumb 2
                        DrawCmd::FillRect {
                            rect: Rect::new(fill_x2 - half_ts, mid_y - half_ts, ts, ts),
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(fill_x2 - half_ts, mid_y - half_ts, ts, ts),
                            color: t.ink,
                            width: 1.5,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(lx + track_w + 12.0, y + 6.0, 80.0, 14.0),
                    ".24–.76",
                    t.text_sm,
                    t.ink,
                    false,
                );
            }
            y += t.h_md + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "drag-number (imgui-style)");
            y += 20.0;
            {
                let drag_items: &[(&str, f32, f32, f32, bool)] = &[
                    ("X", 320.0, 0.0, 800.0, false),
                    ("Y", 144.0, 0.0, 600.0, false),
                    ("W", 576.0, 0.0, 800.0, true),
                    ("H", 400.0, 0.0, 600.0, false),
                ];
                let mut bx = lx;
                for (label, val, min, max, active) in drag_items {
                    b.add(
                        Rect::new(bx, y, 100.0, t.h_md),
                        drag_number,
                        framewise::widgets::DragNumberSpecBuilder::new()
                            .label(label)
                            .value(*val)
                            .min(*min)
                            .max(*max)
                            .active(*active),
                    );
                    bx += 100.0 + 8.0;
                }
            }
            y += t.h_md + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "numeric stepper  ·  colour swatch");
            y += 20.0;
            {
                // prefix + value display
                let stepper_x = lx;
                b.custom(Rect::new(stepper_x, y, 64.0, t.h_md), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: t.hover,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(stepper_x + 6.0, y + 7.0, 56.0, 14.0),
                    "padding",
                    t.text_sm,
                    t.muted,
                    false,
                );
                b.custom(Rect::new(stepper_x + 64.0, y, 40.0, t.h_md), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(stepper_x + 72.0, y + 7.0, 24.0, 14.0),
                    "12",
                    t.text_sm,
                    t.ink,
                    false,
                );

                // +/- buttons as text
                let sx = stepper_x + 120.0;
                b.custom(Rect::new(sx, y, 84.0, t.h_sm), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect: Rect::new(rect.x, rect.y, 22.0, t.h_sm),
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(rect.x, rect.y, 22.0, t.h_sm),
                            color: t.line,
                            width: 1.0,
                        },
                        DrawCmd::FillRect {
                            rect: Rect::new(rect.x + 22., rect.y, 40.0, t.h_sm),
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(rect.x + 22., rect.y, 40.0, t.h_sm),
                            color: t.line,
                            width: 1.0,
                        },
                        DrawCmd::FillRect {
                            rect: Rect::new(rect.x + 62., rect.y, 22.0, t.h_sm),
                            color: t.paper_elev,
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(rect.x + 62., rect.y, 22.0, t.h_sm),
                            color: t.line,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(sx + 6.0, y + 4.0, 10.0, 14.0),
                    "−",
                    t.text_sm,
                    t.ink,
                    false,
                );
                b.label_styled(
                    Rect::new(sx + 28.0, y + 4.0, 28.0, 14.0),
                    "12",
                    t.text_sm,
                    t.ink,
                    false,
                );
                b.label_styled(
                    Rect::new(sx + 68.0, y + 4.0, 10.0, 14.0),
                    "+",
                    t.text_sm,
                    t.ink,
                    false,
                );

                // color swatches
                let sw_x = sx + 100.0;
                let swatches: &[(Color, &str)] = &[(t.ink, "#15130f"), (t.rust, "#c25a2c")];
                let mut bx = sw_x;
                for (color, hex) in swatches {
                    b.add(
                        Rect::new(bx, y, 18.0, t.h_md),
                        color_swatch,
                        framewise::widgets::ColorSwatchSpecBuilder::new()
                            .color(*color)
                            .border(t.line),
                    );
                    b.label_styled(
                        Rect::new(bx + 22.0, y + 7.0, 60.0, 14.0),
                        hex,
                        t.text_sm,
                        t.ink,
                        false,
                    );
                    bx += 86.0;
                }
            }
            y += t.h_md + SEC_GAP;

            // ── 05 · SELECTION ───────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "05", "Selection");
            y += 46.0;

            group_y(&mut b, &t, lx, y, "select  ·  segmented  ·  chips");
            y += 20.0;
            {
                // Select widgets
                let opts = ["Layout: row", "Layout: column", "Layout: grid"];
                b.add(
                    Rect::new(lx, y, 160.0, t.h_md),
                    select,
                    framewise::widgets::SelectSpecBuilder::new()
                        .value("Layout row")
                        .options(&opts)
                        .open(false)
                        .focused(false)
                        .hovered(None),
                );
                b.add(
                    Rect::new(lx, y + t.h_md + 4.0, 160.0, t.h_md),
                    select,
                    framewise::widgets::SelectSpecBuilder::new()
                        .value("Layout row")
                        .options(&opts)
                        .open(true)
                        .focused(true)
                        .hovered(Some(0)),
                );

                // Segmented controls
                let seg_x = lx + 200.0;
                let segs1 = ["row", "column", "grid", "flex"];
                b.add(
                    Rect::new(seg_x, y, 0.0, t.h_md),
                    segmented,
                    framewise::widgets::SegmentedSpecBuilder::new()
                        .items(&segs1)
                        .active_index(0)
                        .focused(None),
                );
                let segs2 = ["start", "center", "end"];
                b.add(
                    Rect::new(seg_x, y + t.h_md + 4.0, 0.0, t.h_md),
                    segmented,
                    framewise::widgets::SegmentedSpecBuilder::new()
                        .items(&segs2)
                        .active_index(1)
                        .focused(None),
                );

                // Chips
                let chip_data: &[(&str, bool)] = &[
                    ("opengl", true),
                    ("vulkan", false),
                    ("metal", false),
                    ("wgpu", false),
                ];
                let chip_y = y;
                let mut chip_x = lx + 560.0;
                for (label, active) in chip_data {
                    let layout = b.text_system.prepare(label, t.text_sm, t.mono_font);
                    let chip_w = (layout.size.x + 16.0).max(32.0);
                    b.add(
                        Rect::new(chip_x, chip_y, chip_w, 22.0),
                        chip,
                        framewise::widgets::ChipSpecBuilder::new()
                            .label(label)
                            .active(*active)
                            .focused(false),
                    );
                    chip_x += chip_w + 6.0;
                }
                let add_layout = b
                    .text_system
                    .prepare("+ add backend", t.text_sm, t.mono_font);
                let add_w = (add_layout.size.x + 16.0).max(32.0);
                b.add(
                    Rect::new(lx + 560.0, y + 28.0, add_w, 22.0),
                    chip,
                    framewise::widgets::ChipSpecBuilder::new()
                        .label("+ add backend")
                        .active(false)
                        .focused(false),
                );
            }
            let select_open_h = 3.0 * 26.0 + 8.0;
            y += t.h_md + 4.0 + t.h_md + select_open_h + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "dropdown menu (open)");
            y += 20.0;
            {
                let items1 = [
                    MenuItem::Group("Frame"),
                    MenuItem::Item {
                        label: "New panel",
                        shortcut: Some("⌘ N"),
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "Duplicate",
                        shortcut: Some("⌘ D"),
                        selected: true,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "Detach",
                        shortcut: Some("⌘ ⇧ D"),
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Separator,
                    MenuItem::Group("Inspect"),
                    MenuItem::Item {
                        label: "Show layout grid",
                        shortcut: Some("G"),
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "Show id tree",
                        shortcut: Some("⌘ ⇧ I"),
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Separator,
                    MenuItem::Item {
                        label: "Replay last frame",
                        shortcut: Some("F2"),
                        selected: false,
                        disabled: true,
                    },
                ];
                b.add(
                    Rect::new(lx, y, 240.0, 0.0),
                    menu,
                    framewise::widgets::MenuSpecBuilder::new().items(&items1),
                );

                let items2 = [
                    MenuItem::Group("Theme"),
                    MenuItem::Item {
                        label: "framewise · default",
                        shortcut: None,
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "framewise · ink",
                        shortcut: None,
                        selected: true,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "framewise · paper",
                        shortcut: None,
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "custom…",
                        shortcut: None,
                        selected: false,
                        disabled: false,
                    },
                ];
                b.add(
                    Rect::new(lx + 264.0, y, 200.0, 0.0),
                    menu,
                    framewise::widgets::MenuSpecBuilder::new().items(&items2),
                );

                let menu1_h: f32 = items1
                    .iter()
                    .map(|i| match i {
                        MenuItem::Item { .. } => 26.0,
                        MenuItem::Separator => 9.0,
                        MenuItem::Group(_) => 22.0,
                    })
                    .sum::<f32>()
                    + 8.0;
                y += menu1_h;
            }
            y += SEC_GAP;

            // ── 06 · SCROLLBARS ──────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "06", "Scrollbars");
            y += 46.0;
            {
                let box_gap = 24.0_f32;
                let cap_h = 20.0_f32;

                // Box 1: vertical, idle
                let b1 = Rect::new(lx, y, 180.0, 130.0);
                let b1_content = Vec2::new(180.0, 320.0);
                b.custom(b1, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let (pre, scope, cb, _) = begin_scroll_area(
                        b1,
                        b1_content,
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut state.scroll_vert,
                        input,
                        b.focus_sys,
                        None,
                        time,
                    );
                    b.append_cmds(pre);
                    let code_lines = [
                        "fn frame(ctx: &mut Ctx) {",
                        "  ctx.window(\"Inspector\", |w| {",
                        "    w.label(\"position\");",
                        "    w.drag(\"x\", &mut pos.x);",
                        "    w.drag(\"y\", &mut pos.y);",
                        "    w.separator();",
                        "    w.label(\"size\");",
                        "    w.drag(\"w\", &mut size.w);",
                        "    w.drag(\"h\", &mut size.h);",
                        "    w.slider(\"alpha\", &mut a, 0..1);",
                        "  });",
                        "}",
                    ];
                    let oy = cb.y - state.scroll_vert.offset.y;
                    for (i, line) in code_lines.iter().enumerate() {
                        b.label_styled(
                            Rect::new(cb.x + 6.0, oy + i as f32 * 18.0 + 6.0, cb.w - 8.0, 14.0),
                            line,
                            t.text_sm,
                            t.muted,
                            false,
                        );
                    }
                    let post = scope.finish(b.focus_sys);
                    b.append_cmds(post);
                }
                b.label_styled(
                    Rect::new(b1.x, y + b1.h + 4.0, b1.w, cap_h),
                    "vertical · idle",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Box 2: vertical, dragging (same implementation, user can drag)
                let b2_x = b1.x + b1.w + box_gap;
                let b2 = Rect::new(b2_x, y, 180.0, 130.0);
                let b2_content = Vec2::new(180.0, 300.0);
                b.custom(b2, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let (pre, scope, cb, _) = begin_scroll_area(
                        b2,
                        b2_content,
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut state.scroll_horiz,
                        input,
                        b.focus_sys,
                        None,
                        time,
                    );
                    b.append_cmds(pre);
                    let oy = cb.y - state.scroll_horiz.offset.y;
                    for i in 0..15usize {
                        b.label_styled(
                            Rect::new(cb.x + 6.0, oy + i as f32 * 18.0 + 6.0, cb.w - 8.0, 14.0),
                            &format!("// entry {:02}/24 — frame state", i + 1),
                            t.text_sm,
                            t.muted,
                            false,
                        );
                    }
                    let post = scope.finish(b.focus_sys);
                    b.append_cmds(post);
                }
                b.label_styled(
                    Rect::new(b2.x, y + b2.h + 4.0, b2.w, cap_h),
                    "vertical · dragging (rust)",
                    t.text_sm,
                    t.muted,
                    false,
                );

                // Box 3: horizontal
                let b3_x = b2_x + b2.w + box_gap;
                let b3 = Rect::new(b3_x, y + 15.0, 300.0, 100.0);
                let b3_content = Vec2::new(700.0, 100.0);
                b.custom(b3, |rect| {
                    vec![DrawCmd::StrokeRect {
                        rect,
                        color: t.line,
                        width: 1.0,
                    }]
                });
                {
                    let (pre, scope, cb, _) = begin_scroll_area(
                        b3,
                        b3_content,
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut state.scroll_both,
                        input,
                        b.focus_sys,
                        None,
                        time,
                    );
                    b.append_cmds(pre);
                    let ox = cb.x - state.scroll_both.offset.x;
                    b.label_styled(
                Rect::new(ox + 6.0, cb.y + 6.0, 680.0, 14.0),
                "frame.draw_rect( … )  frame.draw_text( \"hello, framewise\" )  frame.draw_image( logo )  frame.layout.push( Row )",
                t.text_sm, t.muted, false,
            );
                    let post = scope.finish(b.focus_sys);
                    b.append_cmds(post);
                }
                b.label_styled(
                    Rect::new(b3.x, y + b3.h + 15.0 + 4.0, b3.w, cap_h),
                    "horizontal",
                    t.text_sm,
                    t.muted,
                    false,
                );

                y += 130.0 + cap_h + 8.0;
            }
            y += SEC_GAP;

            // ── 07 · TABS ────────────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "07", "Tabs");
            y += 46.0;
            {
                let tabs1 = ["Inspector", "Layout", "Timing", "Logs", "Replay"];
                b.add(
                    Rect::new(lx, y, content_w.min(640.0), 36.0),
                    tabs,
                    framewise::widgets::TabsSpecBuilder::new()
                        .items(&tabs1)
                        .active_index(0)
                        .focused(None),
                );
                y += 36.0 + 20.0;

                let tabs2 = ["frame.rs", "layout.rs", "theme.rs", "state.rs"];
                b.add(
                    Rect::new(lx, y, content_w.min(480.0), 36.0),
                    tabs,
                    framewise::widgets::TabsSpecBuilder::new()
                        .items(&tabs2)
                        .active_index(1)
                        .focused(None),
                );
                y += 36.0;
            }
            y += SEC_GAP;

            // ── 08 · PROGRESS · METERS · STATUS ──────────────────────────────────────
            sec_y(
                &mut b,
                &t,
                lx,
                y,
                content_w,
                "08",
                "Progress, meters & status",
            );
            y += 46.0;

            group_y(&mut b, &t, lx, y, "progress");
            y += 20.0;
            {
                let bar_items: &[(f32, bool, &str)] = &[
                    (0.12, false, "12% · compiling"),
                    (0.68, false, "68% · linking"),
                    (0.94, true, "94% · uploading textures"),
                    (f32::NAN, true, "indeterminate"),
                ];
                let bar_w = 240.0_f32;
                for (val, active, label) in bar_items {
                    b.add(
                        Rect::new(lx, y + 8.0, bar_w, 3.0),
                        progress_bar,
                        ProgressBarSpecBuilder::new(*val)
                            .phase((time as f32) * 0.5)
                            .active(*active),
                    );
                    b.label_styled(
                        Rect::new(lx + bar_w + 12.0, y + 2.0, 180.0, 14.0),
                        label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    y += 22.0;
                }
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "meters");
            y += 20.0;
            {
                let meters: &[(&str, f32, Option<f32>)] = &[
                    ("CPU", 0.6, None),
                    ("GPU", 0.8, Some(0.9)),
                    ("FRAME", 1.0, None),
                ];
                let mut bx = lx;
                for (label, val, peak) in meters {
                    b.label_styled(
                        Rect::new(bx, y, 36.0, 14.0),
                        label,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    bx += 40.0;
                    if *label == "FRAME" {
                        b.label_styled(
                            Rect::new(bx, y - 1.0, 60.0, 16.0),
                            "2.4 ms",
                            t.text_sm,
                            t.ink,
                            false,
                        );
                        bx += 70.0;
                    } else {
                        b.add(
                            Rect::new(bx, y, 100.0, 12.0),
                            meter,
                            framewise::widgets::MeterSpecBuilder::new()
                                .value(*val)
                                .peak(*peak)
                                .bars(10),
                        );
                        bx += 108.0;
                    }
                }
            }
            y += 14.0 + GROUP_GAP;

            group_y(&mut b, &t, lx, y, "spinners  ·  status");
            y += 20.0;
            {
                b.add(
                    Rect::new(lx, y, 16.0, 16.0),
                    spinner,
                    SpinnerSpecBuilder::new(),
                );
                b.label_styled(
                    Rect::new(lx + 20.0, y + 1.0, 60.0, 14.0),
                    "loading",
                    t.text_sm,
                    t.muted,
                    false,
                );

                b.add(
                    Rect::new(lx + 90.0, y - 4.0, 24.0, 24.0),
                    spinner,
                    SpinnerSpecBuilder::new().large(true),
                );
                b.label_styled(
                    Rect::new(lx + 118.0, y + 1.0, 50.0, 14.0),
                    "large",
                    t.text_sm,
                    t.muted,
                    false,
                );

                let status_items: &[(&str, StatusVariant)] = &[
                    ("idle", StatusVariant::Neutral),
                    ("ready", StatusVariant::Ok),
                    ("frame drop", StatusVariant::Warn),
                    ("panic", StatusVariant::Err),
                    ("rendering", StatusVariant::Live),
                ];
                let mut sx = lx + 180.0;
                for (label, variant) in status_items {
                    b.add(
                        Rect::new(sx, y + 1.0, 120.0, 12.0),
                        status,
                        framewise::widgets::StatusSpecBuilder::new()
                            .label(label)
                            .variant(*variant),
                    );
                    sx += 110.0;
                }
            }
            y += 16.0 + SEC_GAP;

            // ── 09 · TREE / LIST ─────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "09", "Tree & list");
            y += 46.0;
            {
                let widget_tree = [
                    TreeRow {
                        indent: 0,
                        caret: Some(true),
                        label: "App",
                        meta: Some("#0001"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 1,
                        caret: Some(true),
                        label: "MenuBar",
                        meta: Some("#0002"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 2,
                        caret: None,
                        label: "File",
                        meta: Some("#0003"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 2,
                        caret: None,
                        label: "Edit",
                        meta: Some("#0004"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 1,
                        caret: Some(true),
                        label: "Workspace",
                        meta: Some("#0010"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 2,
                        caret: Some(true),
                        label: "Canvas",
                        meta: Some("#0011"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 3,
                        caret: None,
                        label: "Layer \"frame\"",
                        meta: Some("#0014"),
                        selected: true,
                    },
                    TreeRow {
                        indent: 3,
                        caret: None,
                        label: "Layer \"ui\"",
                        meta: Some("#0015"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 2,
                        caret: Some(false),
                        label: "Inspector",
                        meta: Some("#0020"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 1,
                        caret: Some(false),
                        label: "StatusBar",
                        meta: Some("#0030"),
                        selected: false,
                    },
                ];
                b.add(
                    Rect::new(lx, y, 320.0, 0.0),
                    tree,
                    framewise::widgets::TreeSpecBuilder::new().rows(&widget_tree),
                );

                let file_list = [
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "frame_buffer.rs",
                        meta: Some("2.1 kb"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "layout.rs",
                        meta: Some("5.4 kb"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "renderer.rs",
                        meta: Some("12.0 kb"),
                        selected: true,
                    },
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "state.rs",
                        meta: Some("3.8 kb"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "theme.rs",
                        meta: Some("1.6 kb"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "widget/",
                        meta: Some("11 files"),
                        selected: false,
                    },
                    TreeRow {
                        indent: 0,
                        caret: None,
                        label: "main.rs",
                        meta: Some("0.4 kb"),
                        selected: false,
                    },
                ];
                b.add(
                    Rect::new(lx + 360.0, y, 240.0, 0.0),
                    tree,
                    framewise::widgets::TreeSpecBuilder::new().rows(&file_list),
                );

                y += widget_tree.len().max(file_list.len()) as f32 * 20.0 + 12.0;
            }
            y += SEC_GAP;

            // ── 10 · TOOLTIPS · KEYCAPS ──────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "10", "Tooltips & keycaps");
            y += 46.0;

            group_y(&mut b, &t, lx, y, "tooltips");
            y += 20.0;
            {
                b.add(
                    Rect::new(lx, y, 0.0, 0.0),
                    tooltip,
                    framewise::widgets::TooltipSpecBuilder::new()
                        .text("Drag to scrub — hold ⌥ for fine.")
                        .variant(TooltipVariant::Dark),
                );
                y += 28.0 + 8.0;

                b.add(Rect::new(lx, y, 0.0, 0.0), tooltip, framewise::widgets::TooltipSpecBuilder::new().text("Re-described every frame from current application state. No retained nodes.").variant(TooltipVariant::Dark));
                y += 28.0 + 8.0;

                b.add(
                    Rect::new(lx, y, 0.0, 0.0),
                    tooltip,
                    framewise::widgets::TooltipSpecBuilder::new()
                        .text("⚠ shader recompiled this frame (12 ms)")
                        .variant(TooltipVariant::Rust),
                );
                y += 28.0;
            }
            y += GROUP_GAP;

            group_y(&mut b, &t, lx, y, "keycaps");
            y += 20.0;
            {
                let key_rows: &[(&[&str], &str)] = &[
                    (&["⌘", "⇧", "P"], "command palette"),
                    (&["G"], "toggle layout grid"),
                    (&["F2"], "replay last frame"),
                    (&["⌥", "drag"], "fine scrub"),
                ];
                for (keys, desc) in key_rows {
                    let mut kx = lx;
                    for key in *keys {
                        let kw = (key.len() as f32 * 7.0 + 12.0).max(24.0);
                        b.add(
                            Rect::new(kx, y, kw, 22.0),
                            keycap,
                            framewise::widgets::KeycapSpecBuilder::new()
                                .label(key)
                                .bg(t.paper_elev)
                                .border(t.line)
                                .text_color(t.ink)
                                .text_size(t.text_sm),
                        );
                        kx += kw + 4.0;
                    }
                    b.label_styled(
                        Rect::new(kx + 4.0, y + 3.0, 200.0, 14.0),
                        desc,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    y += 28.0;
                }
            }
            y += SEC_GAP;

            // ── 11 · WINDOW CHROME ───────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "11", "Window & panel chrome");
            y += 46.0;
            {
                // Light window: Inspector with content
                let win_buttons = [
                    WindowButton { symbol: "−" },
                    WindowButton { symbol: "▢" },
                    WindowButton { symbol: "×" },
                ];
                let win_rect = Rect::new(lx, y, 360.0, 280.0);
                let mut win = b.window(
                    win_rect,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Inspector")
                        .buttons(&win_buttons)
                        .status_bar(true)
                        .status_text("rendering  frame #00248  2.4 ms"),
                    ManualLayout,
                );

                // Inner content: drag numbers + checkboxes
                let mut iy = 0.0;
                let drag_items: &[(&str, f32, f32, f32)] =
                    &[("X", 320.0, 0.0, 800.0), ("Y", 144.0, 0.0, 600.0)];
                let mut drx = 0.0;
                let cr_w = win_rect.w - 32.0;
                for (label, val, min, max) in drag_items {
                    win.add(
                        Rect::new(drx, iy, (cr_w / 2.0) - 4.0, t.h_md),
                        drag_number,
                        framewise::widgets::DragNumberSpecBuilder::new()
                            .label(label)
                            .value(*val)
                            .min(*min)
                            .max(*max)
                            .active(false),
                    );
                    drx += (cr_w / 2.0) + 4.0;
                }
                iy += t.h_md + 6.0;
                drx = 0.0;
                let drag_items2: &[(&str, f32, f32, f32)] =
                    &[("W", 576.0, 0.0, 800.0), ("H", 400.0, 0.0, 600.0)];
                for (label, val, min, max) in drag_items2 {
                    win.add(
                        Rect::new(drx, iy, (cr_w / 2.0) - 4.0, t.h_md),
                        drag_number,
                        framewise::widgets::DragNumberSpecBuilder::new()
                            .label(label)
                            .value(*val)
                            .min(*min)
                            .max(*max)
                            .active(false),
                    );
                    drx += (cr_w / 2.0) + 4.0;
                }
                iy += t.h_md + 10.0;
                win.divider(Rect::new(0.0, iy, cr_w, 1.0));
                iy += 10.0;
                let check_items: &[(CheckState, &str)] = &[
                    (CheckState::On, "clip to parent"),
                    (CheckState::Off, "debug overlay"),
                ];
                for (cs, label) in check_items {
                    win.add(
                        Rect::new(0.0, iy, 14.0, 14.0),
                        checkbox,
                        CheckboxSpecBuilder::new(*cs),
                    );

                    win.label_styled(
                        Rect::new(18.0, iy, cr_w - 18.0, 14.0),
                        label,
                        t.text_md,
                        t.ink,
                        false,
                    );
                    iy += 22.0;
                }
                let cmds = win.finish();
                b.append_cmds(cmds);

                // Dark variant window (drawn with DrawCmds)
                let dw = Rect::new(lx + 388.0, y, 300.0, 240.0);
                let dark_bg = Color::from_srgb_u8(26, 24, 20, 255);
                let darker = Color::from_srgb_u8(12, 11, 9, 255);
                let dark_bdr = Color::from_srgb_u8(58, 53, 45, 255);
                let light = t.paper;
                let muted_l = t.muted;

                b.custom(dw, |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: dark_bg,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: dark_bdr,
                            width: 1.0,
                        },
                        DrawCmd::FillRect {
                            rect: Rect::new(rect.x, rect.y, rect.w, 26.0),
                            color: darker,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(dw.x + 10.0, y + 6.0, 180.0, 14.0),
                    "framewise · dark",
                    t.text_sm,
                    light,
                    false,
                );
                b.label_styled(
                    Rect::new(dw.x + dw.w - 28.0, y + 6.0, 20.0, 14.0),
                    "✕",
                    t.text_sm,
                    light,
                    false,
                );

                let cx = dw.x + 16.0;
                let cyw = y + 26.0 + 16.0;
                // keycap row
                b.custom(Rect::new(cx, cyw, 50.0, 22.0), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect: Rect::new(rect.x, rect.y, 24.0, 22.0),
                            color: Color::from_srgb_u8(42, 37, 32, 255),
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(rect.x, rect.y, 24.0, 22.0),
                            color: dark_bdr,
                            width: 1.0,
                        },
                        DrawCmd::FillRect {
                            rect: Rect::new(rect.x + 28.0, rect.y, 22.0, 22.0),
                            color: Color::from_srgb_u8(42, 37, 32, 255),
                        },
                        DrawCmd::StrokeRect {
                            rect: Rect::new(rect.x + 28.0, rect.y, 22.0, 22.0),
                            color: dark_bdr,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(cx + 7.0, cyw + 5.0, 12.0, 12.0),
                    "⌘",
                    t.text_sm,
                    light,
                    false,
                );
                b.label_styled(
                    Rect::new(cx + 35.0, cyw + 5.0, 12.0, 12.0),
                    "K",
                    t.text_sm,
                    light,
                    false,
                );
                b.label_styled(
                    Rect::new(cx + 56.0, cyw + 5.0, 140.0, 12.0),
                    "search everything",
                    t.text_sm,
                    muted_l,
                    false,
                );

                // fake dark input
                let inp_y = cyw + 28.0;
                b.custom(Rect::new(cx, inp_y, dw.w - 32.0, 26.0), |rect| {
                    vec![
                        DrawCmd::FillRect {
                            rect,
                            color: darker,
                        },
                        DrawCmd::StrokeRect {
                            rect,
                            color: dark_bdr,
                            width: 1.0,
                        },
                    ]
                });
                b.label_styled(
                    Rect::new(cx + 8.0, inp_y + 7.0, dw.w - 48.0, 12.0),
                    "type a command…",
                    t.text_sm,
                    muted_l,
                    false,
                );

                // fake dark tabs
                let tab_y = inp_y + 30.0;
                b.custom(Rect::new(cx, tab_y + 26.0, dw.w - 16.0, 1.0), |rect| {
                    vec![DrawCmd::StrokeLine {
                        p0: Vec2::new(rect.x, rect.y),
                        p1: Vec2::new(rect.x + rect.w, rect.y),
                        color: dark_bdr,
                        width: 1.0,
                    }]
                });
                let tab_items = ["Files", "Symbols", "Frames"];
                let mut tab_x = cx;
                for (i, item) in tab_items.iter().enumerate() {
                    b.label_styled(
                        Rect::new(tab_x, tab_y + 5.0, 60.0, 14.0),
                        item,
                        t.text_sm,
                        if i == 0 { light } else { muted_l },
                        false,
                    );
                    if i == 0 {
                        b.custom(Rect::new(tab_x, tab_y + 24.0, 40.0, 2.0), |rect| {
                            vec![DrawCmd::FillRect {
                                rect,
                                color: t.rust,
                            }]
                        });
                    }
                    tab_x += 60.0;
                }
                let file_y = tab_y + 32.0;
                for (i, file) in ["▸ renderer.rs", "▸ layout.rs", "▸ widget/button.rs"]
                    .iter()
                    .enumerate()
                {
                    b.label_styled(
                        Rect::new(cx, file_y + i as f32 * 18.0, 200.0, 14.0),
                        file,
                        t.text_sm,
                        muted_l,
                        false,
                    );
                }

                y += 280.0 + SEC_GAP;
            }

            // ── 12 · IN USE ──────────────────────────────────────────────────────────
            sec_y(&mut b, &t, lx, y, content_w, "12", "In use");
            y += 46.0;
            {
                // Left: Renderer Settings window
                let win_w_left = 440.0_f32;
                let win_h_full = 480.0_f32;
                let win_buttons = [
                    WindowButton { symbol: "−" },
                    WindowButton { symbol: "▢" },
                    WindowButton { symbol: "×" },
                ];
                let wr = Rect::new(lx, y, win_w_left, win_h_full);
                let mut win = b.window(
                    wr,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Renderer Settings")
                        .buttons(&win_buttons)
                        .status_bar(true)
                        .status_text("rendering  frame #00248  2.4 ms  Vulkan 1.3 · 4× msaa"),
                    ManualLayout,
                );
                let cr_w = win_w_left - 32.0;

                // Tabs inside window
                let tabs_items = ["General", "Frame", "Output", "Debug"];
                win.add(
                    Rect::new(0.0, 0.0, cr_w, 28.0),
                    tabs,
                    framewise::widgets::TabsSpecBuilder::new()
                        .items(&tabs_items)
                        .active_index(0)
                        .focused(None),
                );

                // Form rows
                let form_y_start = 38.0;
                let label_w = 84.0_f32;
                let widget_x = label_w + 8.0;
                let widget_w = cr_w - label_w - 8.0;
                let row_h = 28.0_f32;
                let row_gap = 8.0_f32;
                let mut fy = form_y_start;

                // backend (segmented)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "backend",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let backends = ["OpenGL", "Vulkan", "Metal", "wgpu"];
                win.add(
                    Rect::new(widget_x, fy, 0.0, row_h),
                    segmented,
                    framewise::widgets::SegmentedSpecBuilder::new()
                        .items(&backends)
                        .active_index(1)
                        .focused(None),
                );
                fy += row_h + row_gap;

                // target fps (slider)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "target fps",
                    t.text_sm,
                    t.muted,
                    false,
                );
                win.slider(
                    &mut state.iu_fps_slider,
                    &mut state.iu_fps_val,
                    24.0,
                    240.0,
                    10.0,
                    SliderOrientation::Horizontal,
                    Rect::new(widget_x, fy, widget_w - 40.0, row_h),
                    input,
                );
                win.label_styled(
                    Rect::new(widget_x + widget_w - 34.0, fy + 7.0, 34.0, 14.0),
                    &format!("{:.0}", state.iu_fps_val),
                    t.text_sm,
                    t.ink,
                    false,
                );
                fy += row_h + row_gap;

                // vsync (switch)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "vsync",
                    t.text_sm,
                    t.muted,
                    false,
                );
                win.add(
                    Rect::new(widget_x, fy + 6.0, 30.0, 16.0),
                    switch,
                    SwitchSpecBuilder::new()
                        .on(true)
                        .focused(false)
                        .disabled(false),
                );
                win.label_styled(
                    Rect::new(widget_x + 36.0, fy + 7.0, 120.0, 14.0),
                    "match display",
                    t.text_sm,
                    t.ink,
                    false,
                );
                fy += row_h + row_gap;

                // msaa (segmented)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "msaa",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let msaa_opts = ["off", "2×", "4×", "8×"];
                win.add(
                    Rect::new(widget_x, fy, 0.0, row_h),
                    segmented,
                    framewise::widgets::SegmentedSpecBuilder::new()
                        .items(&msaa_opts)
                        .active_index(2)
                        .focused(None),
                );
                fy += row_h + row_gap;

                // viewport (drag numbers)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "viewport",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let vp_items: &[(&str, f32)] = &[("W", 1920.0), ("H", 1080.0)];
                let mut vpx = widget_x;
                for (label, val) in vp_items {
                    win.add(
                        Rect::new(vpx, fy, (widget_w / 2.0) - 4.0, row_h),
                        drag_number,
                        framewise::widgets::DragNumberSpecBuilder::new()
                            .label(label)
                            .value(*val)
                            .min(0.0)
                            .max(7680.0)
                            .active(false),
                    );
                    vpx += (widget_w / 2.0) + 4.0;
                }
                fy += row_h + row_gap;

                // accent (color swatch + button)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "accent",
                    t.text_sm,
                    t.muted,
                    false,
                );
                win.add(
                    Rect::new(widget_x, fy + 4.0, 18.0, 20.0),
                    color_swatch,
                    framewise::widgets::ColorSwatchSpecBuilder::new()
                        .color(t.rust)
                        .border(t.line),
                );
                win.label_styled(
                    Rect::new(widget_x + 22.0, fy + 7.0, 60.0, 14.0),
                    "#c25a2c",
                    t.text_sm,
                    t.ink,
                    false,
                );
                fy += row_h + row_gap;

                // options (checkboxes)
                win.label_styled(
                    Rect::new(0.0, fy + 7.0, label_w, 14.0),
                    "options",
                    t.text_sm,
                    t.muted,
                    false,
                );
                let opt_items: &[(CheckState, &str)] = &[
                    (CheckState::On, "show layout grid"),
                    (CheckState::Off, "log every frame"),
                    (CheckState::Indeterminate, "tessellate (per-mesh)"),
                ];
                for (i, (cs, label)) in opt_items.iter().enumerate() {
                    let opt_y = fy + i as f32 * 22.0;
                    win.add(
                        Rect::new(widget_x, opt_y + 4.0, 14.0, 14.0),
                        checkbox,
                        CheckboxSpecBuilder::new(*cs),
                    );

                    win.label_styled(
                        Rect::new(widget_x + 18.0, opt_y + 4.0, widget_w - 18.0, 14.0),
                        label,
                        t.text_md,
                        t.ink,
                        false,
                    );
                }
                fy += 3.0 * 22.0 + 4.0;

                win.divider(Rect::new(0.0, fy, cr_w, 1.0));
                fy += 10.0;

                // button row
                let mut btn_x = cr_w;
                let btns: &[(&str, ButtonStyle)] = &[
                    ("Apply", ButtonStyle::primary()),
                    ("Cancel", ButtonStyle::default()),
                    ("Reset", ButtonStyle::ghost()),
                ];
                for (i, (label, style)) in btns.iter().enumerate() {
                    let bw = label.len() as f32 * 7.0 + 20.0;
                    btn_x -= bw;
                    let btn = win.button_styled(
                        std::mem::take(&mut state.iu_btns[i]),
                        Rect::new(btn_x, fy, bw, t.h_md),
                        *label,
                        style.clone(),
                        false,
                        input,
                    );
                    state.iu_btns[i] = btn.state;
                    btn_x -= 8.0;
                }
                let cmds = win.finish();
                b.append_cmds(cmds);

                // Right column
                let rcol_x = lx + win_w_left + 24.0;
                let rcol_w = (content_w - win_w_left - 24.0).max(0.0);

                // Frame Log window
                let fl_h = 310.0_f32;
                let fl_buttons = [
                    WindowButton { symbol: "⌕" },
                    WindowButton { symbol: "⏸" },
                    WindowButton { symbol: "×" },
                ];
                let fl_rect = Rect::new(rcol_x, y, rcol_w, fl_h);
                let mut fl_win = b.window(
                    fl_rect,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Frame Log")
                        .buttons(&fl_buttons)
                        .status_bar(true)
                        .status_text("recording  248 frames  2.6 ms avg"),
                    ManualLayout,
                );
                let fl_cr_w = rcol_w - 32.0;
                let fl_cr_h = fl_h - 80.0; // 26 title + 22 status + 32 padding

                // Scroll area for log content
                let fl_scroll_rect = Rect::new(0.0, 0.0, fl_cr_w, fl_cr_h);
                let log_lines: &[(&str, &str, bool)] = &[
                    ("00248 · 2.40ms", "frame begin", false),
                    ("00248 · 2.41ms", "layout(row) · 14 nodes", false),
                    ("00248 · 2.45ms", "draw_rect( inspector )", false),
                    ("00248 · 2.48ms", "draw_text( \"Inspector\", 14px )", false),
                    ("00248 · 2.61ms", "drag_started( \"X\", 320.00 )", true),
                    ("00248 · 2.74ms", "state.x ← 322.00", false),
                    ("00248 · 2.89ms", "invalidate( panel#0011 )", false),
                    ("00248 · 3.10ms", "frame end · uploaded 14 commands", false),
                    ("00249 · 2.36ms", "frame begin", false),
                    ("00249 · 2.40ms", "layout(row) · 14 nodes", false),
                    ("00249 · 2.50ms", "draw_rect( inspector )", false),
                    ("00249 · 2.52ms", "state.x ← 324.00", false),
                ];
                let log_content_h = log_lines.len() as f32 * 18.0 + 8.0;
                {
                    let mut log_page = fl_win.scroll_area(
                        fl_scroll_rect,
                        Vec2::new(fl_scroll_rect.w, log_content_h),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Auto,
                        &mut state.iu_log_scroll,
                        framewise::layout::ManualLayout,
                        input,
                    );
                    let loy = 4.0;
                    for (i, (ts_str, msg, highlight)) in log_lines.iter().enumerate() {
                        let row_y = loy + i as f32 * 18.0;
                        let ts_w = 100.0_f32;
                        log_page.label_styled(
                            Rect::new(6.0, row_y, ts_w, 14.0),
                            ts_str,
                            t.text_sm,
                            t.muted,
                            false,
                        );
                        let msg_color = if *highlight { t.rust } else { t.ink };
                        log_page.label_styled(
                            Rect::new(
                                6.0 + ts_w + 8.0,
                                row_y,
                                fl_scroll_rect.w - ts_w - 14.0,
                                14.0,
                            ),
                            msg,
                            t.text_sm,
                            msg_color,
                            false,
                        );
                    }
                    let log_cmds = log_page.finish();
                    fl_win.append_cmds(log_cmds);
                }
                let cmds = fl_win.finish();
                b.append_cmds(cmds);

                // Quick Actions window
                let qa_y = y + fl_h + 16.0;
                let qa_buttons = [WindowButton { symbol: "×" }];
                let qa_rect = Rect::new(rcol_x, qa_y, rcol_w, 174.0);
                let mut qa_win = b.window(
                    qa_rect,
                    framewise::widgets::WindowSpecBuilder::new()
                        .title("Quick actions")
                        .buttons(&qa_buttons)
                        .status_bar(false)
                        .status_text(""),
                    ManualLayout,
                );
                let qa_cr_w = rcol_w - 32.0;

                let qa_items = [
                    MenuItem::Item {
                        label: "Render frame",
                        shortcut: Some("F1"),
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "Replay last frame",
                        shortcut: Some("F2"),
                        selected: true,
                        disabled: false,
                    },
                    MenuItem::Item {
                        label: "Show id tree",
                        shortcut: Some("⌘ ⇧ I"),
                        selected: false,
                        disabled: false,
                    },
                    MenuItem::Separator,
                    MenuItem::Item {
                        label: "Dump state to clipboard",
                        shortcut: Some("⌘ ⇧ C"),
                        selected: false,
                        disabled: false,
                    },
                ];
                qa_win.add(
                    Rect::new(0.0, -8.0, qa_cr_w, 0.0),
                    menu,
                    framewise::widgets::MenuSpecBuilder::new().items(&qa_items),
                );
                let cmds = qa_win.finish();
                b.append_cmds(cmds);

                y += win_h_full;
            }
            y += SEC_GAP;

            // ── FOOTER ───────────────────────────────────────────────────────────────
            {
                b.divider(Rect::new(lx, y, content_w, 1.0));
                y += 10.0;
                let foot_items: &[(&str, &str)] = &[
                    ("spec", "v0.1 · 12 sections"),
                    ("radius", "0 px"),
                    ("borders", "1 px ink"),
                    ("focus", "2 px rust outset"),
                    ("density", "28 px row · 14 px label · 12 px mono"),
                ];
                let mut fx = lx;
                for (key, val) in foot_items {
                    b.label_styled(Rect::new(fx, y, 32.0, 14.0), key, t.text_sm, t.ink, false);
                    let kw = key.len() as f32 * 7.0 + 8.0;
                    b.label_styled(
                        Rect::new(fx + kw, y, 220.0, 14.0),
                        val,
                        t.text_sm,
                        t.muted,
                        false,
                    );
                    fx += kw + val.len() as f32 * 6.5 + 24.0;
                }
                b.label_styled(
                    Rect::new(lx + content_w - 200.0, y, 200.0, 14.0),
                    "framewise · widget specification",
                    t.text_sm,
                    t.ink,
                    false,
                );
            }
            let _ = (y, b);
        } // end content block (drops `b` alias, releases borrow on `page`)
        page.finish()
    }; // end page_cmds block
    b.append_cmds(page_cmds);
    b.finish()
}

fn hero_logo(t: &Theme, lx: f32, y0: f32) -> Vec<DrawCmd> {
    let mut cmds = vec![];

    // Logo (Framewise mark), scaled from 200×200 viewBox → 96×96 px
    let ls = 0.48_f32;
    let lx0 = lx;
    let lw = 4.8_f32;
    cmds.extend(vec![
        // left bracket
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 56. * ls, y0 + 40. * ls),
            p1: Vec2::new(lx0 + 40. * ls, y0 + 40. * ls),
            color: t.ink,
            width: lw,
        },
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 40. * ls, y0 + 40. * ls),
            p1: Vec2::new(lx0 + 40. * ls, y0 + 160. * ls),
            color: t.ink,
            width: lw,
        },
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 40. * ls, y0 + 160. * ls),
            p1: Vec2::new(lx0 + 56. * ls, y0 + 160. * ls),
            color: t.ink,
            width: lw,
        },
        // top horizontal
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 78. * ls, y0 + 40. * ls),
            p1: Vec2::new(lx0 + 140. * ls, y0 + 40. * ls),
            color: t.ink,
            width: lw,
        },
        // middle horizontal (rust)
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 78. * ls, y0 + 96. * ls),
            p1: Vec2::new(lx0 + 120. * ls, y0 + 96. * ls),
            color: t.rust,
            width: lw,
        },
        // vertical
        DrawCmd::StrokeLine {
            p0: Vec2::new(lx0 + 78. * ls, y0 + 40. * ls),
            p1: Vec2::new(lx0 + 78. * ls, y0 + 160. * ls),
            color: t.ink,
            width: lw,
        },
    ]);

    cmds
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn opts_dropdown_h(n: usize) -> f32 {
    n as f32 * 26.0 + 8.0
}
