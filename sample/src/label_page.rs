use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Placement, Placement2D},
    layouts::{ColumnLayout, RowLayout},
    text::{
        EllipsisFallback, OverflowX, OverflowY, TextFlow, TextLineAlign, WrapGlyphFallback,
        WrapWordFallback,
    },
    theme::Theme,
    types::{Color, Rect},
    widget::WidgetContext,
    widgets::frame::{begin_frame, FrameSpecBuilder, FrameStyle},
    widgets::label::{label, LabelSpecBuilder, LabelStyle},
};

#[derive(Default)]
pub struct LabelPageState {
    #[cfg(feature = "scroll_area")]
    pub scroll: framewise::widgets::scroll_area::ScrollState,
}

pub fn draw_label_page(
    _state: &mut LabelPageState,
    focus_system: &mut FocusSystem,
    input: &Input,
    _time: f64,
    win_size: (f32, f32),
    text_system: &mut SampleTextSystem,
    debug_layout: bool,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let _pad = 20.0;

    let mut cmds = framewise::DrawCommands::new();
    #[allow(unused_mut)]
    let mut root_ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        #[cfg(feature = "scroll_area")]
        framewise::layouts::ManualLayout,
        #[cfg(not(feature = "scroll_area"))]
        ColumnLayout { spacing: 28.0 },
        #[cfg(feature = "scroll_area")]
        Rect::new(0.0, 0.0, win_w, win_h),
        #[cfg(not(feature = "scroll_area"))]
        Rect::new(_pad, _pad, win_w - 2.0 * _pad, win_h - 2.0 * _pad),
        &mut cmds,
    );

    #[cfg(feature = "scroll_area")]
    let framewise::widgets::scroll_area::ScrollAreaResult { layout: _, mut ctx } =
        framewise::widgets::scroll_area::begin_scroll_area(
            &mut root_ctx,
            framewise::widgets::scroll_area::ScrollAreaSpecBuilder::new().vertical(
                framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::Unbounded,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                },
            ),
            Rect::new(0.0, 0.0, win_w, win_h),
            &mut _state.scroll,
            ColumnLayout { spacing: 28.0 },
        );
    #[cfg(not(feature = "scroll_area"))]
    let mut ctx = root_ctx;

    ctx.debug_layout = debug_layout;
    ctx.layout_policy = framewise::LayoutViolationPolicy::Highlight;

    let theme = ctx.theme;

    let box_style = FrameStyle {
        background: Color::from_srgb_u8(240, 240, 243, 255),
        border: Color::from_srgb_u8(210, 210, 215, 255),
        border_width: 1.0,
        padding: 8.0,
    };

    // Page Title
    let title_style = LabelStyle {
        text_style: framewise::TextStyle::new(
            theme.sans_font,
            32.0,
            theme.sans_weight_bold,
            framewise::text::TextFlow::single_line(),
        ),
        text_color: theme.rust,
        rule: true,
        rule_color: theme.rust,
    };
    label(
        &mut ctx,
        LabelSpecBuilder::new()
            .text("Label Widget Showcase")
            .style(title_style),
        Placement2D::auto(),
    );

    // Section 1: Font Families and Sizes
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("1. Font Families & Sizes")
                .style(section_header),
            Placement2D::auto(),
        );

        let mut row = ctx.child_with_layout(
            Placement2D {
                width: Placement::fill(),
                height: Placement::auto(),
            },
            RowLayout { spacing: 40.0 },
        );

        // Sans Column
        {
            let mut col =
                row.child_with_layout(Placement2D::auto(), ColumnLayout { spacing: 12.0 });
            label(
                &mut col,
                LabelSpecBuilder::new()
                    .text("Sans Serif (Inter Tight)")
                    .style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.sans_font,
                            16.0,
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: theme.rust,
                        rule: false,
                        rule_color: theme.line,
                    }),
                Placement2D::auto(),
            );

            let sizes = [12.0, 16.0, 24.0, 36.0, 48.0];
            let labels = [
                "Small Size (12px)",
                "Medium Size (16px)",
                "Large Size (24px)",
                "X-Large Size (36px)",
                "Display Size (48px)",
            ];
            for i in 0..5 {
                label(
                    &mut col,
                    LabelSpecBuilder::new().text(labels[i]).style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.sans_font,
                            sizes[i],
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: theme.ink,
                        rule: false,
                        rule_color: theme.line,
                    }),
                    Placement2D::auto(),
                );
            }
            col.finish();
        }

        // Mono Column
        {
            let mut col = row.child_with_layout(
                Placement2D::fixed(350.0, 260.0),
                ColumnLayout { spacing: 12.0 },
            );
            label(
                &mut col,
                LabelSpecBuilder::new()
                    .text("Monospace (JetBrains Mono)")
                    .style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.mono_font,
                            16.0,
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: theme.rust,
                        rule: false,
                        rule_color: theme.line,
                    }),
                Placement2D::auto(),
            );

            let sizes = [12.0, 16.0, 24.0, 36.0, 48.0];
            let labels = [
                "mono 12px",
                "mono 16px",
                "mono 24px",
                "mono 36px",
                "mono 48px",
            ];
            for i in 0..5 {
                label(
                    &mut col,
                    LabelSpecBuilder::new().text(labels[i]).style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.mono_font,
                            sizes[i],
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: theme.ink,
                        rule: false,
                        rule_color: theme.line,
                    }),
                    Placement2D::auto(),
                );
            }
            col.finish();
        }

        row.finish();
    }

    // Section 2: Colors and Underline Rules
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("2. Color and Horizontal Rules")
                .style(section_header),
            Placement2D::auto(),
        );

        let mut row = ctx.child_with_layout(
            Placement2D {
                width: Placement::fill(),
                height: Placement::auto(),
            },
            RowLayout { spacing: 30.0 },
        );

        // Color Showcase Column
        {
            let mut col =
                row.child_with_layout(Placement2D::auto(), ColumnLayout { spacing: 10.0 });
            let colors = [
                ("Default Ink Color", theme.ink),
                ("Accent Rust Color", theme.rust),
                ("Subtle Gray Color", Color::from_srgb_u8(120, 120, 130, 255)),
                (
                    "Vibrant Emerald Green",
                    Color::from_srgb_u8(16, 185, 129, 255),
                ),
                (
                    "Sunset Orange Color",
                    Color::from_srgb_u8(249, 115, 22, 255),
                ),
            ];
            for (text, color) in colors {
                label(
                    &mut col,
                    LabelSpecBuilder::new().text(text).style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.sans_font,
                            16.0,
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: color,
                        rule: false,
                        rule_color: theme.line,
                    }),
                    Placement2D::auto(),
                );
            }
            col.finish();
        }

        // Rule Showcase Column
        {
            let mut col =
                row.child_with_layout(Placement2D::auto(), ColumnLayout { spacing: 18.0 });

            let rule_styles = [
                ("Underlined Heading", theme.ink, theme.line),
                ("Accent Underline Heading", theme.ink, theme.rust),
                ("Colored Heading & Colored Rule", theme.rust, theme.rust),
                (
                    "Green Underline Heading",
                    Color::from_srgb_u8(16, 185, 129, 255),
                    Color::from_srgb_u8(16, 185, 129, 255),
                ),
            ];

            for (text, text_col, rule_col) in rule_styles {
                label(
                    &mut col,
                    LabelSpecBuilder::new().text(text).style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.sans_font,
                            16.0,
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: text_col,
                        rule: true,
                        rule_color: rule_col,
                    }),
                    Placement2D::auto(),
                );
            }
            col.finish();
        }

        row.finish();
    }
    // Section 4: Overflow (non-wrapping)
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("4. Overflow (non-wrapping)")
                .style(section_header),
            Placement2D::auto(),
        );
        label(
            &mut ctx,
            LabelSpecBuilder::new().text(r#"All text is the same "hello\nhello" string"#),
            Placement2D::auto(),
        );

        let clip_test_box_style = FrameStyle {
            background: Color::from_srgb_u8(255, 255, 255, 255),
            border: Color::from_srgb_u8(180, 50, 50, 255),
            border_width: 0.0,
            padding: 0.0,
        };

        // Row 1: Cards 1 - 4
        {
            let mut row = ctx.child_with_layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
                RowLayout { spacing: 20.0 },
            );

            // Card 1: X: Drop, Y: Drop (hello\nhello inside 27x30)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("1. X: Drop, Y: Drop")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 28.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Drop,
                                        overflow_y: OverflowY::Drop,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 2: X: Keep, Y: Keep (hello\nhello inside 27x30)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("2. X: Keep, Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 28.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Keep,
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 3: X: Keep, Y: Ellipsis (hello\nhello inside 27x30)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("3. X: Keep, Y: Ellipsis")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 28.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Keep,
                                        overflow_y: OverflowY::Ellipsis {
                                            fallback: EllipsisFallback::Drop,
                                        },
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 4: X: Keep, Y: Ellipsis (Fallback: Drop) (hello\nhello inside 10x30)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("4. X: Keep, Y: Ell(F:Drop)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(8.0, 28.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Keep,
                                        overflow_y: OverflowY::Ellipsis {
                                            fallback: EllipsisFallback::Drop,
                                        },
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }

        // Row 2: Cards 5 - 8
        {
            let mut row = ctx.child_with_layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
                RowLayout { spacing: 20.0 },
            );

            // Card 5: X: Keep, Y: Ellipsis (Fallback: Keep) (hello\nhello inside 10x30)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("5. X: Keep, Y: Ell(F:Keep)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(8.0, 28.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Keep,
                                        overflow_y: OverflowY::Ellipsis {
                                            fallback: EllipsisFallback::Keep,
                                        },
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 6: X: Ellipsis, Y: Keep (hello\nhello inside 25x50)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("6. X: Ellipsis, Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(23.0, 48.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Ellipsis {
                                            fallback: EllipsisFallback::Drop,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 7: X: Ellipsis (Fallback: Drop), Y: Keep (hello\nhello inside 10x50)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("7. X: Ell(F:Drop), Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(8.0, 48.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Ellipsis {
                                            fallback: EllipsisFallback::Drop,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 8: X: Ellipsis (Fallback: Keep), Y: Keep (hello\nhello inside 10x50)
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("8. X: Ell(F:Keep), Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(8.0, 48.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::Ellipsis {
                                            fallback: EllipsisFallback::Keep,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }
    }

    // Section 4.1: Overflow (wrapping)
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("4.1 Overflow (wrapping)")
                .style(section_header),
            Placement2D::auto(),
        );
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text(r#"First row: all text is the same "hello\nhello" string"#),
            Placement2D::auto(),
        );

        let clip_test_box_style = FrameStyle {
            background: Color::from_srgb_u8(255, 255, 255, 255),
            border: Color::from_srgb_u8(180, 50, 50, 255),
            border_width: 0.0,
            padding: 0.0,
        };

        // Row 1: Cards 1 - 3
        {
            let mut row = ctx.child_with_layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
                RowLayout { spacing: 20.0 },
            );

            // Card 1: X: WrapGlyph, Y: Keep
            // Keep this card in sync with test_wrap_glyph_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("1. X: WrapGlyph, Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(23.0, 63.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapGlyph {
                                            fallback: WrapGlyphFallback::Drop,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 2: X: WrapGlyph (F: Drop), Y: Keep
            // Keep this card in sync with test_wrap_glyph_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("2. X: WrapGlyph(F:Drop)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(6.0, 68.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapGlyph {
                                            fallback: WrapGlyphFallback::Drop,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 3: X: WrapGlyph (F: Keep), Y: Keep
            // Keep this card in sync with test_wrap_glyph_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 220.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("3. X: WrapGlyph(F:Keep)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(4.0, 162.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapGlyph {
                                            fallback: WrapGlyphFallback::Keep,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }

        // Row 2: Cards 4 - 6
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text(r#"Remaining rows: all text is the same "hello there\nhello there" string"#),
            Placement2D::auto(),
        );
        {
            let mut row = ctx.child_with_layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
                RowLayout { spacing: 20.0 },
            );

            // Card 4: X: WrapWord, Y: Keep
            // Keep this card in sync with test_wrap_word_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("4. X: WrapWord, Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(48.0, 68.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::Drop,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 5: X: WrapWord (F: WrapGlyph), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_wrap_glyph_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 200.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("5. X: WrapWord(F:WrapG)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(23.0, 138.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::WrapGlyph {
                                                fallback: WrapGlyphFallback::Drop,
                                            },
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 6: X: WrapWord (F: WrapGlyph F: Drop), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_wrap_glyph_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 200.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("6. X: WrapWord(F:WG F:Dr)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(6.0, 138.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::WrapGlyph {
                                                fallback: WrapGlyphFallback::Drop,
                                            },
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }

        // Row 3: Cards 7 - 9
        {
            let mut row = ctx.child_with_layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
                RowLayout { spacing: 20.0 },
            );

            // Card 7: X: WrapWord (F: WrapGlyph F: Keep), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_wrap_glyph_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 380.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("7. X: WrapWord(F:WG F:Kp)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(4.0, 318.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::WrapGlyph {
                                                fallback: WrapGlyphFallback::Keep,
                                            },
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 8: X: WrapWord (F: Drop), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("8. X: WrapWord(F:Drop)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 68.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::Drop,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            // Card 9: X: WrapWord (F: Keep), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    Placement2D::fixed(230.0, 140.0),
                    ColumnLayout { spacing: 8.0 },
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("9. X: WrapWord(F:Keep)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 68.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::Keep,
                                        },
                                        overflow_y: OverflowY::Keep,
                                        line_align: TextLineAlign::Start,
                                    },
                                    font: theme.sans_font,
                                    size: 14.0,
                                    ..(LabelStyle::from_theme(&theme)).text_style
                                },
                                text_color: theme.ink,
                                ..LabelStyle::from_theme(&theme)
                            }),
                        Placement2D {
                            width: Placement::fill(),
                            height: Placement::fill(),
                        },
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }
    }

    // Section 5: Internal Text Alignment
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("5. Internal Text Alignment (Fixed-Width)")
                .style(section_header),
            Placement2D::auto(),
        );

        let mut row = ctx.child_with_layout(
            Placement2D {
                width: Placement::fill(),
                height: Placement::auto(),
            },
            RowLayout { spacing: 20.0 },
        );

        let alignments = [
            ("Start Aligned Text", TextLineAlign::Start),
            ("Center Aligned Text", TextLineAlign::Center),
            ("End Aligned Text", TextLineAlign::End),
        ];

        for (text, text_align) in alignments {
            let mut container = begin_frame(
                &mut row,
                FrameSpecBuilder::new().style(box_style),
                Placement2D::fixed(230.0, 80.0),
                ColumnLayout { spacing: 6.0 },
            );

            label(
                &mut container.ctx,
                LabelSpecBuilder::new()
                    .text(match text_align {
                        TextLineAlign::Start => "TextFlow::line_align(Start)",
                        TextLineAlign::Center => "TextFlow::line_align(Center)",
                        TextLineAlign::End => "TextFlow::line_align(End)",
                    })
                    .style(LabelStyle {
                        text_style: framewise::TextStyle::new(
                            theme.mono_font,
                            11.0,
                            theme.sans_weight_regular,
                            framewise::text::TextFlow::single_line(),
                        ),
                        text_color: theme.rust,
                        rule: false,
                        rule_color: theme.line,
                    }),
                Placement2D::auto(),
            );

            label(
                &mut container.ctx,
                LabelSpecBuilder::new().text(text).style(LabelStyle {
                    text_style: framewise::TextStyle::new(
                        theme.sans_font,
                        14.0,
                        theme.sans_weight_regular,
                        TextFlow {
                            overflow_x: OverflowX::Drop,
                            overflow_y: OverflowY::Drop,
                            line_align: text_align,
                        },
                    ),
                    text_color: theme.ink,
                    rule: false,
                    rule_color: theme.line,
                }),
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
            );

            container.ctx.finish();
        }

        row.finish();
    }

    // Footer info
    label(
        &mut ctx,
        LabelSpecBuilder::new()
            .text("Press F1-F5 to navigate to other showcase pages. (F6 for Labels)")
            .style(LabelStyle {
                text_style: framewise::TextStyle::new(
                    theme.mono_font,
                    12.0,
                    theme.sans_weight_regular,
                    framewise::text::TextFlow::single_line(),
                ),
                text_color: Color::from_srgb_u8(120, 120, 130, 255),
                rule: false,
                rule_color: theme.line,
            }),
        Placement2D::auto(),
    );

    #[cfg(feature = "scroll_area")]
    {
        ctx.finish();
        root_ctx.finish();
    }
    #[cfg(not(feature = "scroll_area"))]
    {
        ctx.finish();
    }
    cmds
}
