use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{ColumnLayout, CrossAlign, Extent, RowLayout, SizeReq},
    theme::Theme,
    types::{Rect, Vec2},
    widget::WidgetContext,
    widgets::button::{button, ButtonSpecBuilder, ButtonState, ButtonStyle},
    widgets::frame::{begin_frame, FrameResult, FrameSpecBuilder, FrameStyle},
};

// ── State ──────────────────────────────────────────────────────────────────────

pub struct FrameDemoState {
    // Dynamic List Controls & State
    pub add_btn: ButtonState,
    pub remove_btn: ButtonState,
    pub dynamic_clicks: [u32; 5],
    pub dynamic_btns: [ButtonState; 5],
    pub item_count: usize,

    // Sizing Showcase State
    pub fixed_btn: ButtonState,
    pub width_auto_btn: ButtonState,
    pub height_auto_btn: ButtonState,
    pub height_auto_btn2: ButtonState,
    pub fully_auto_btn: ButtonState,
    pub fixed_clicks: u32,
    pub width_auto_clicks: u32,
    pub height_auto_clicks: u32,
    pub fully_auto_clicks: u32,

    // Nesting & Alignment State
    pub outer_btn: ButtonState,
    pub inner_center_btn: ButtonState,
    pub inner_end_btn: ButtonState,
    pub nested_clicks: [u32; 3],

    // Cross-axis alignment showcase buttons
    pub align_small_btn: ButtonState,
    pub align_med_btn: ButtonState,
    pub align_large_btn: ButtonState,
}

impl Default for FrameDemoState {
    fn default() -> Self {
        Self {
            add_btn: ButtonState::default(),
            remove_btn: ButtonState::default(),
            dynamic_clicks: [0; 5],
            dynamic_btns: [
                ButtonState::default(),
                ButtonState::default(),
                ButtonState::default(),
                ButtonState::default(),
                ButtonState::default(),
            ],
            item_count: 2, // start with 2 items

            fixed_btn: ButtonState::default(),
            width_auto_btn: ButtonState::default(),
            height_auto_btn: ButtonState::default(),
            height_auto_btn2: ButtonState::default(),
            fully_auto_btn: ButtonState::default(),
            fixed_clicks: 0,
            width_auto_clicks: 0,
            height_auto_clicks: 0,
            fully_auto_clicks: 0,

            outer_btn: ButtonState::default(),
            inner_center_btn: ButtonState::default(),
            inner_end_btn: ButtonState::default(),
            nested_clicks: [0; 3],

            align_small_btn: ButtonState::default(),
            align_med_btn: ButtonState::default(),
            align_large_btn: ButtonState::default(),
        }
    }
}

// ── Draw ──────────────────────────────────────────────────────────────────────

pub fn draw_frame_page(
    state: &mut FrameDemoState,
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
        framewise::layout::ManualLayout,
        Rect::new(0.0, 0.0, win_w, win_h),
        &mut cmds,
    );
    ctx.debug_layout = debug_layout;

    // Root Row — two columns side-by-side (Left column: Dynamic list & sizes; Right column: Nesting & alignments)
    let mut root_row = ctx.child_with_layout(
        Rect::new(pad, pad, win_w - 2.0 * pad, win_h - 2.0 * pad),
        RowLayout {
            spacing: 30.0,
            align: CrossAlign::Start,
        },
    );

    let theme = root_row.theme;
    let primary = ButtonStyle::primary_from_theme(&theme);
    let secondary = ButtonStyle::secondary_from_theme(&theme);
    let accent = ButtonStyle::accent_from_theme(&theme);
    let mut ghost = ButtonStyle::ghost_from_theme(&theme);
    ghost.disabled_alpha = 1.0; // So buttons used as labels are more visible

    let frame_style = FrameStyle::from_theme(&theme);

    // ── Left Column: Dynamic List & Sizing Dimensions ─────────────────────────
    {
        let mut left_col = root_row.child_with_layout(
            Vec2::new((win_w - 2.0 * pad - 30.0) * 0.5, win_h - 2.0 * pad).into(),
            ColumnLayout {
                spacing: 20.0,
                align: CrossAlign::Start,
            },
        );

        // Heading: Left Column Title
        button(
            &mut left_col,
            ButtonSpecBuilder::new()
                .text("1. Dynamic & Axis Sizing Showcase")
                .style(ghost)
                .disabled(true),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(30.0),
            },
            &mut ButtonState::default(),
        );

        // Sub-row: Add / Remove Controls
        {
            let mut control_row = left_col.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Fixed(40.0),
                },
                RowLayout {
                    spacing: 12.0,
                    align: CrossAlign::Start,
                },
            );

            let add_r = button(
                &mut control_row,
                ButtonSpecBuilder::new()
                    .text("Add Dynamic Button")
                    .style(primary),
                Vec2::new(200.0, 40.0).into(),
                &mut state.add_btn,
            );
            if add_r.input.clicked && state.item_count < 5 {
                state.item_count += 1;
            }

            let rem_r = button(
                &mut control_row,
                ButtonSpecBuilder::new()
                    .text("Remove Dynamic Button")
                    .style(accent),
                Vec2::new(200.0, 40.0).into(),
                &mut state.remove_btn,
            );
            if rem_r.input.clicked && state.item_count > 0 {
                state.item_count -= 1;
            }

            control_row.finish();
        }

        // Subheading: Dynamic Frame
        let label_style = format!("Auto-Sizing Frame (Current Items: {})", state.item_count);
        button(
            &mut left_col,
            ButtonSpecBuilder::new()
                .text(&label_style)
                .style(ghost)
                .disabled(true),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(24.0),
            },
            &mut ButtonState::default(),
        );

        // ── DYNAMIC FRAME ──
        // Sizes height to children, fills width
        {
            let FrameResult {
                layout: _,
                ctx: mut dynamic_frame,
            } = begin_frame(
                &mut left_col,
                FrameSpecBuilder::new().style(frame_style),
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Auto, // This is the magic auto-sizing dimension that grows/contracts with the number of items in the frame!
                },
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::Start,
                },
            );

            if state.item_count == 0 {
                button(
                    &mut dynamic_frame,
                    ButtonSpecBuilder::new()
                        .text("Frame is empty! Use buttons above to add items.")
                        .style(ghost)
                        .disabled(true),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(32.0),
                    },
                    &mut ButtonState::default(),
                );
            } else {
                for i in 0..state.item_count {
                    let text = format!(
                        "Dynamic Button #{} (Clicks: {})",
                        i + 1,
                        state.dynamic_clicks[i]
                    );
                    let r = button(
                        &mut dynamic_frame,
                        ButtonSpecBuilder::new().text(&text).style(secondary),
                        SizeReq {
                            width: Extent::Fill,
                            height: Extent::Fixed(36.0),
                        },
                        &mut state.dynamic_btns[i],
                    );
                    if r.input.clicked {
                        state.dynamic_clicks[i] += 1;
                    }
                }
            }

            dynamic_frame.finish();
        }

        // Subheading: Axis Sizing Dimensions
        button(
            &mut left_col,
            ButtonSpecBuilder::new()
                .text("Comparison of Sizing Dimensions")
                .style(ghost)
                .disabled(true),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(24.0),
            },
            &mut ButtonState::default(),
        );

        // Row of 4 different Frame Dimension constraints
        {
            let mut dimensions_row = left_col.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Fixed(180.0),
                },
                RowLayout {
                    spacing: 16.0,
                    align: CrossAlign::Start,
                },
            );

            // 1. Fixed-Size Frame (200x120)
            {
                let FrameResult {
                    layout: _,
                    ctx: mut sub_frame,
                } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    SizeReq::fixed(120.0, 120.0),
                    ColumnLayout {
                        spacing: 4.0,
                        align: CrossAlign::Start,
                    },
                );

                button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new()
                        .text("Fixed frame")
                        .style(ghost)
                        .disabled(true),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(20.0),
                    },
                    &mut ButtonState::default(),
                );

                let text = format!("Cl: {}", state.fixed_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(primary),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(36.0),
                    },
                    &mut state.fixed_btn,
                );
                if r.input.clicked {
                    state.fixed_clicks += 1;
                }

                sub_frame.finish();
            }

            // 2. Width Auto, Height Fixed (Auto width wraps to child intrinsic text width!)
            {
                let FrameResult {
                    layout: _,
                    ctx: mut sub_frame,
                } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fixed(120.0),
                    },
                    ColumnLayout {
                        spacing: 4.0,
                        align: CrossAlign::Start,
                    },
                );

                button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new()
                        .text("Auto Width")
                        .style(ghost)
                        .disabled(true),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fixed(20.0),
                    },
                    &mut ButtonState::default(),
                );

                let text = format!("Auto Width for me! {}", state.width_auto_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(secondary),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fixed(36.0),
                    },
                    &mut state.width_auto_btn,
                );
                if r.input.clicked {
                    state.width_auto_clicks += 1;
                }

                sub_frame.finish();
            }

            // 3. Width Fixed, Height Auto
            {
                let FrameResult {
                    layout: _,
                    ctx: mut sub_frame,
                } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    SizeReq {
                        width: Extent::Fixed(130.0),
                        height: Extent::Auto,
                    },
                    ColumnLayout {
                        spacing: 4.0,
                        align: CrossAlign::Start,
                    },
                );

                button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new()
                        .text("Auto Height")
                        .style(ghost)
                        .disabled(true),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(20.0),
                    },
                    &mut ButtonState::default(),
                );

                let text = format!("Auto H: {}", state.height_auto_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(accent),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Fixed(36.0),
                    },
                    &mut state.height_auto_btn,
                );
                if r.input.clicked {
                    state.height_auto_clicks += 1;
                }

                button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text("Height").style(accent),
                    SizeReq {
                        width: Extent::Fill,
                        height: Extent::Auto,
                    },
                    &mut state.height_auto_btn2,
                );

                sub_frame.finish();
            }

            // 4. Fully Auto (Both Width & Height Auto)
            {
                let FrameResult {
                    layout: _,
                    ctx: mut sub_frame,
                } = begin_frame(
                    &mut dimensions_row,
                    FrameSpecBuilder::new().style(frame_style),
                    SizeReq::auto(),
                    ColumnLayout {
                        spacing: 4.0,
                        align: CrossAlign::Start,
                    },
                );

                button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new()
                        .text("Fully Auto")
                        .style(ghost)
                        .disabled(true),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Fixed(20.0),
                    },
                    &mut ButtonState::default(),
                );

                let text = format!("Fully Auto: {}", state.fully_auto_clicks);
                let r = button(
                    &mut sub_frame,
                    ButtonSpecBuilder::new().text(&text).style(secondary),
                    SizeReq {
                        width: Extent::Auto,
                        height: Extent::Auto,
                    },
                    &mut state.fully_auto_btn,
                );
                if r.input.clicked {
                    state.fully_auto_clicks += 1;
                }

                sub_frame.finish();
            }

            dimensions_row.finish();
        }

        left_col.finish();
    }

    // ── Right Column: Nesting & Alignments ────────────────────────────────────
    {
        let mut right_col = root_row.child_with_layout(
            Vec2::new((win_w - 2.0 * pad - 30.0) * 0.5, win_h - 2.0 * pad).into(),
            ColumnLayout {
                spacing: 20.0,
                align: CrossAlign::Start,
            },
        );

        // Heading: Right Column Title
        button(
            &mut right_col,
            ButtonSpecBuilder::new()
                .text("2. Complex Nesting & Cross-Alignments")
                .style(ghost)
                .disabled(true),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(30.0),
            },
            &mut ButtonState::default(),
        );

        // Showcase 1: Symmetrical Nesting Cases (Fixed Panel centered in Fixed Outer)
        button(
            &mut right_col,
            ButtonSpecBuilder::new()
                .text("Nesting Showcase (Fixed Panel centered in Fixed Outer)")
                .style(ghost)
                .disabled(true),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(24.0),
            },
            &mut ButtonState::default(),
        );

        // Outer Fixed frame (450x180) containing centered inner fixed frame
        {
            let FrameResult {
                layout: _,
                ctx: mut outer_fixed,
            } = begin_frame(
                &mut right_col,
                FrameSpecBuilder::new().style(frame_style),
                SizeReq::fixed(450.0, 180.0),
                ColumnLayout {
                    spacing: 8.0,
                    align: CrossAlign::Center, // Centers children horizontally!
                },
            );

            let text = format!("Outer Fixed Frame (Clicks: {})", state.nested_clicks[0]);
            let r = button(
                &mut outer_fixed,
                ButtonSpecBuilder::new().text(&text).style(secondary),
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Fixed(36.0),
                },
                &mut state.outer_btn,
            );
            if r.input.clicked {
                state.nested_clicks[0] += 1;
            }

            // Inner Fixed-Width Frame nested inside Fixed outer!
            {
                let FrameResult {
                    layout: _,
                    ctx: mut inner_auto,
                } = begin_frame(
                    &mut outer_fixed,
                    FrameSpecBuilder::new().style(FrameStyle {
                        background: theme.paper, // distinct dark background
                        ..frame_style
                    }),
                    SizeReq {
                        width: Extent::Fixed(350.0),
                        height: Extent::Auto,
                    },
                    RowLayout {
                        spacing: 12.0,
                        align: CrossAlign::Start,
                    },
                );

                let text_c = format!("Inner Center (Clicks: {})", state.nested_clicks[1]);
                let r1 = button(
                    &mut inner_auto,
                    ButtonSpecBuilder::new().text(&text_c).style(accent),
                    SizeReq::auto(),
                    &mut state.inner_center_btn,
                );
                if r1.input.clicked {
                    state.nested_clicks[1] += 1;
                }

                let text_e = format!("Inner End (Clicks: {})", state.nested_clicks[2]);
                let r2 = button(
                    &mut inner_auto,
                    ButtonSpecBuilder::new().text(&text_e).style(primary),
                    SizeReq::auto(),
                    &mut state.inner_end_btn,
                );
                if r2.input.clicked {
                    state.nested_clicks[2] += 1;
                }

                inner_auto.finish();
            }

            outer_fixed.finish();
        }

        // Showcase 2: Cross-Axis Alignment Demonstration inside a Fit Frame
        button(
            &mut right_col,
            ButtonSpecBuilder::new()
                .text("Cross-Axis Alignment within Fit Frame (Auto height, Centered)")
                .style(ghost)
                .disabled(true),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(24.0),
            },
            &mut ButtonState::default(),
        );

        // Center-aligned column layout inside an auto-height frame
        {
            let FrameResult {
                layout: _,
                ctx: mut fit_centered,
            } = begin_frame(
                &mut right_col,
                FrameSpecBuilder::new().style(frame_style),
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Auto,
                },
                ColumnLayout {
                    spacing: 10.0,
                    align: CrossAlign::Center, // Centered cross-axis layout inside a fit frame!
                },
            );

            // Three buttons of varying widths visually demonstrating centered alignment within dynamic frame
            button(
                &mut fit_centered,
                ButtonSpecBuilder::new().text("Small Width").style(primary),
                Vec2::new(120.0, 36.0).into(),
                &mut state.align_small_btn,
            );

            button(
                &mut fit_centered,
                ButtonSpecBuilder::new()
                    .text("Medium Width Button")
                    .style(secondary),
                Vec2::new(240.0, 36.0).into(),
                &mut state.align_med_btn,
            );

            button(
                &mut fit_centered,
                ButtonSpecBuilder::new()
                    .text("Large Width Content Button")
                    .style(accent),
                Vec2::new(360.0, 36.0).into(),
                &mut state.align_large_btn,
            );

            fit_centered.finish();
        }

        right_col.finish();
    }

    root_row.finish();

    cmds
}
