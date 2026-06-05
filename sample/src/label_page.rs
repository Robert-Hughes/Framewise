use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Placement, Placement2D},
    layouts::{ColumnLayout, RowLayout},
    text::{
        EllipsisFallback, HorizontalAlign, OverflowX, OverflowY, TextFlow, WrapGlyphFallback,
        WrapWordFallback,
    },
    theme::Theme,
    types::{Color, Rect},
    widget::WidgetContext,
    widgets::frame::{begin_frame, FrameSpecBuilder, FrameStyle},
    widgets::label::{label, LabelSpecBuilder, LabelStyle},
};

#[derive(Default)]
pub struct LabelPageState {}

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
    let pad = 20.0;

    let mut cmds = framewise::DrawCommands::new();
    let mut ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        ColumnLayout { spacing: 28.0 },
        Rect::new(pad, pad, win_w - 2.0 * pad, win_h - 2.0 * pad),
        &mut cmds,
    );
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
        size: 32.0,
        font: theme.sans_font,
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
            size: 20.0,
            font: theme.sans_font,
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
                        size: 16.0,
                        font: theme.sans_font,
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
                        size: sizes[i],
                        font: theme.sans_font,
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
                        size: 16.0,
                        font: theme.mono_font,
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
                        size: sizes[i],
                        font: theme.mono_font,
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
            size: 20.0,
            font: theme.sans_font,
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
            let mut col = row.child_with_layout(
                Placement2D::fixed(350.0, 200.0),
                ColumnLayout { spacing: 10.0 },
            );
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
                        size: 16.0,
                        font: theme.sans_font,
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
            let mut col = row.child_with_layout(
                Placement2D::fixed(350.0, 200.0),
                ColumnLayout { spacing: 18.0 },
            );

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
                        size: 16.0,
                        font: theme.sans_font,
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
            size: 20.0,
            font: theme.sans_font,
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
            border_width: 1.0,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(27.0, 30.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Drop,
                                overflow_y: OverflowY::Drop,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(27.0, 30.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Keep,
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(27.0, 30.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Keep,
                                overflow_y: OverflowY::Ellipsis {
                                    fallback: EllipsisFallback::Drop,
                                },
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(10.0, 30.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Keep,
                                overflow_y: OverflowY::Ellipsis {
                                    fallback: EllipsisFallback::Drop,
                                },
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(10.0, 30.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Keep,
                                overflow_y: OverflowY::Ellipsis {
                                    fallback: EllipsisFallback::Keep,
                                },
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 50.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Ellipsis {
                                    fallback: EllipsisFallback::Drop,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(10.0, 50.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Ellipsis {
                                    fallback: EllipsisFallback::Drop,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(10.0, 50.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::Ellipsis {
                                    fallback: EllipsisFallback::Keep,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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

    // Section 4.1: Systematic Multi-line Wrapping & Fallbacks
    {
        let section_header = LabelStyle {
            size: 20.0,
            font: theme.sans_font,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            &mut ctx,
            LabelSpecBuilder::new()
                .text("4.1 Systematic Multi-line Wrapping & Fallbacks")
                .style(section_header),
            Placement2D::auto(),
        );

        let clip_test_box_style = FrameStyle {
            background: Color::from_srgb_u8(255, 255, 255, 255),
            border: Color::from_srgb_u8(180, 50, 50, 255),
            border_width: 1.0,
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
            // Keep this card in sync with test_wrap_glyph_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 70.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapGlyph {
                                    fallback: WrapGlyphFallback::Drop,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_glyph_fallback_drop_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(5.0, 70.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapGlyph {
                                    fallback: WrapGlyphFallback::Drop,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_glyph_fallback_keep_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(5.0, 160.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapGlyph {
                                    fallback: WrapGlyphFallback::Keep,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
        {
            let mut row = ctx.child_with_layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::auto(),
                },
                RowLayout { spacing: 20.0 },
            );

            // Card 4: X: WrapWord, Y: Keep
            // Keep this card in sync with test_wrap_word_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(50.0, 70.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapWord {
                                    fallback: WrapWordFallback::Drop,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_word_fallback_wrap_glyph_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 140.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapWord {
                                    fallback: WrapWordFallback::WrapGlyph {
                                        fallback: WrapGlyphFallback::Drop,
                                    },
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_word_fallback_wrap_glyph_fallback_drop_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(5.0, 140.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapWord {
                                    fallback: WrapWordFallback::WrapGlyph {
                                        fallback: WrapGlyphFallback::Drop,
                                    },
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_word_fallback_wrap_glyph_fallback_keep_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(5.0, 320.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapWord {
                                    fallback: WrapWordFallback::WrapGlyph {
                                        fallback: WrapGlyphFallback::Keep,
                                    },
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_word_fallback_drop_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 70.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapWord {
                                    fallback: WrapWordFallback::Drop,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            // Keep this card in sync with test_wrap_word_fallback_keep_y_keep in sample/src/text.rs
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
                            size: 13.0,
                            font: theme.mono_font,
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    Placement2D::auto(),
                );
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        Placement2D::fixed(25.0, 70.0),
                        ColumnLayout { spacing: 0.0 },
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .text_flow(TextFlow {
                                overflow_x: OverflowX::WrapWord {
                                    fallback: WrapWordFallback::Keep,
                                },
                                overflow_y: OverflowY::Keep,
                                horizontal_align: HorizontalAlign::Start,
                            })
                            .style(LabelStyle {
                                size: 14.0,
                                font: theme.sans_font,
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
            size: 20.0,
            font: theme.sans_font,
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
            ("Start Aligned Text", HorizontalAlign::Start),
            ("Center Aligned Text", HorizontalAlign::Center),
            ("End Aligned Text", HorizontalAlign::End),
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
                        HorizontalAlign::Start => "TextFlow::horizontal_align(Start)",
                        HorizontalAlign::Center => "TextFlow::horizontal_align(Center)",
                        HorizontalAlign::End => "TextFlow::horizontal_align(End)",
                    })
                    .style(LabelStyle {
                        size: 11.0,
                        font: theme.mono_font,
                        text_color: theme.rust,
                        rule: false,
                        rule_color: theme.line,
                    }),
                Placement2D::auto(),
            );

            label(
                &mut container.ctx,
                LabelSpecBuilder::new()
                    .text(text)
                    .text_flow(TextFlow {
                        overflow_x: OverflowX::Drop,
                        overflow_y: OverflowY::Drop,
                        horizontal_align: text_align,
                    })
                    .style(LabelStyle {
                        size: 14.0,
                        font: theme.sans_font,
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
                size: 12.0,
                font: theme.mono_font,
                text_color: Color::from_srgb_u8(120, 120, 130, 255),
                rule: false,
                rule_color: theme.line,
            }),
        Placement2D::auto(),
    );

    ctx.finish();
    cmds
}
