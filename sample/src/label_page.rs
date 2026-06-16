use crate::text::SampleTextBackend;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::Align,
    layouts::linear::{ColumnLayout, ColumnLayoutParams, RowLayout, RowLayoutParams},
    text::{
        EllipsisFallback, OverflowX, OverflowY, TextFlow, TextLineAlign, WrapClusterFallback,
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
    pub page: crate::demo_page::DemoPageState,
}

pub fn draw_label_page(
    _state: &mut LabelPageState,
    focus_system: &mut FocusSystem,
    input: &Input,
    _time: f64,
    win_size: (f32, f32),
    text_backend: &mut SampleTextBackend,
    debug_layout: bool,
) -> framewise::DrawCommands {
    let (win_w, win_h) = win_size;
    let is_unbounded = win_h.is_infinite();

    let mut cmds = framewise::DrawCommands::new();
    let space = if is_unbounded {
        framewise::LayoutSpace::unbounded_height(0.0, 0.0, win_w)
    } else {
        Rect::new(0.0, 0.0, win_w, win_h).into()
    };

    let mut root_ctx = WidgetContext::root(
        Theme::default(),
        text_backend,
        focus_system,
        input,
        ColumnLayout,
        space,
        &mut cmds,
    );

    if is_unbounded {
        let mut outer = crate::demo_page::begin_demo_page_no_scroll(
            &mut root_ctx,
            "Label Demo",
            debug_layout,
            true,
            ColumnLayout,
        );
        draw_label_page_content(&mut outer.ctx);
        outer.ctx.finish();
    } else {
        let mut outer = crate::demo_page::begin_demo_page(
            &mut root_ctx,
            "Label Demo",
            &mut _state.page,
            debug_layout,
            ColumnLayout,
        );
        draw_label_page_content(&mut outer.ctx);
        outer.ctx.finish();
    }

    root_ctx.finish();
    cmds
}

pub(crate) fn draw_label_page_content<'a, 'b, CF>(
    ctx: &'b mut WidgetContext<
        'a,
        SampleTextBackend,
        framewise::layouts::OffsetState<framewise::ColumnState>,
        CF,
    >,
) {
    let theme = ctx.theme;

    let box_style = FrameStyle {
        background: Color::from_srgb_u8(240, 240, 243, 255),
        border: Color::from_srgb_u8(210, 210, 215, 255),
        border_width: 1.0,
        padding: 8.0,
    };

    // Section 1: Font Families and Sizes
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            content_placement: framewise::TextContentPlacement::TOP_LEFT,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("1. Font Families & Sizes")
                .style(section_header),
            ColumnLayoutParams::auto(),
        );

        let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

        // Sans Column
        {
            let mut col = row.child_with_layout(RowLayoutParams::auto(), ColumnLayout);
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
                        content_placement: framewise::TextContentPlacement::TOP_LEFT,
                        text_color: theme.rust,
                        rule: false,
                        rule_color: theme.line,
                    }),
                ColumnLayoutParams::auto(),
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
                        content_placement: framewise::TextContentPlacement::TOP_LEFT,
                        text_color: theme.ink,
                        rule: false,
                        rule_color: theme.line,
                    }),
                    ColumnLayoutParams::auto(),
                );
                col.spacer(12.0);
            }
            col.finish();
        }
        row.spacer(40.0);

        // Mono Column
        {
            let mut col = row.child_with_layout(RowLayoutParams::fixed(350.0, 260.0), ColumnLayout);
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
                        content_placement: framewise::TextContentPlacement::TOP_LEFT,
                        text_color: theme.rust,
                        rule: false,
                        rule_color: theme.line,
                    }),
                ColumnLayoutParams::auto(),
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
                        content_placement: framewise::TextContentPlacement::TOP_LEFT,
                        text_color: theme.ink,
                        rule: false,
                        rule_color: theme.line,
                    }),
                    ColumnLayoutParams::auto(),
                );
                col.spacer(12.0);
            }
            col.finish();
        }

        row.finish();
    }
    ctx.spacer(28.0);

    // Section 2: Colors and Underline Rules
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            content_placement: framewise::TextContentPlacement::TOP_LEFT,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("2. Color and Horizontal Rules")
                .style(section_header),
            ColumnLayoutParams::auto(),
        );

        let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

        // Color Showcase Column
        {
            let mut col = row.child_with_layout(RowLayoutParams::auto(), ColumnLayout);
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
                        content_placement: framewise::TextContentPlacement::TOP_LEFT,
                        text_color: color,
                        rule: false,
                        rule_color: theme.line,
                    }),
                    ColumnLayoutParams::auto(),
                );
                col.spacer(10.0);
            }
            col.finish();
        }
        row.spacer(30.0);

        // Rule Showcase Column
        {
            let mut col = row.child_with_layout(RowLayoutParams::auto(), ColumnLayout);

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
                        content_placement: framewise::TextContentPlacement::TOP_LEFT,
                        text_color: text_col,
                        rule: true,
                        rule_color: rule_col,
                    }),
                    ColumnLayoutParams::auto(),
                );
                col.spacer(18.0);
            }
            col.finish();
        }

        row.finish();
    }
    ctx.spacer(28.0);

    // Section 4: Overflow (non-wrapping)
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            content_placement: framewise::TextContentPlacement::TOP_LEFT,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("4. Overflow (non-wrapping)")
                .style(section_header),
            ColumnLayoutParams::auto(),
        );
        label(
            ctx,
            LabelSpecBuilder::new().text(r#"All text is the same "hello\nhello" string"#),
            ColumnLayoutParams::auto(),
        );

        let clip_test_box_style = FrameStyle {
            background: Color::from_srgb_u8(255, 255, 255, 255),
            border: Color::from_srgb_u8(180, 50, 50, 255),
            border_width: 0.0,
            padding: 0.0,
        };

        // Row 1: Cards 1 - 4
        {
            let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

            // Card 1: X: Drop, Y: Drop
            // Keep this card in sync with test_overflow_x_drop_y_drop in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(25.0, 28.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 2: X: Keep, Y: Keep
            // Keep this card in sync with test_overflow_x_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(25.0, 28.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 3: X: Keep, Y: Ellipsis
            // Keep this card in sync with test_overflow_x_keep_y_ellipsis in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(25.0, 28.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 4: X: Keep, Y: Ellipsis (Fallback: Drop)
            // Keep this card in sync with test_overflow_x_keep_y_ellipsis_fallback_drop in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(8.0, 28.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }

        // Row 2: Cards 5 - 8
        {
            let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

            // Card 5: X: Keep, Y: Ellipsis (Fallback: Keep)
            // Keep this card in sync with test_overflow_x_keep_y_ellipsis_fallback_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(8.0, 28.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 6: X: Ellipsis, Y: Keep
            // Keep this card in sync with test_overflow_x_ellipsis_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(23.0, 48.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 7: X: Ellipsis (Fallback: Drop), Y: Keep
            // Keep this card in sync with test_overflow_x_ellipsis_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(8.0, 48.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 8: X: Ellipsis (Fallback: Keep), Y: Keep
            // Keep this card in sync with test_overflow_x_ellipsis_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(8.0, 48.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }
    }
    ctx.spacer(28.0);

    // Section 4.1: Overflow (wrapping)
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            content_placement: framewise::TextContentPlacement::TOP_LEFT,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("4.1 Overflow (wrapping)")
                .style(section_header),
            ColumnLayoutParams::auto(),
        );
        label(
            ctx,
            LabelSpecBuilder::new()
                .text(r#"First row: all text is the same "hello\nhello" string"#),
            ColumnLayoutParams::auto(),
        );

        let clip_test_box_style = FrameStyle {
            background: Color::from_srgb_u8(255, 255, 255, 255),
            border: Color::from_srgb_u8(180, 50, 50, 255),
            border_width: 0.0,
            padding: 0.0,
        };

        // Row 1: Cards 1 - 3
        {
            let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

            // Card 1: X: WrapCluster, Y: Keep
            // Keep this card in sync with test_wrap_cluster_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("1. X: WrapCluster, Y: Keep")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(23.0, 63.0),
                        ColumnLayout,
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapCluster {
                                            fallback: WrapClusterFallback::Drop,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 2: X: WrapCluster (F: Drop), Y: Keep
            // Keep this card in sync with test_wrap_cluster_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("2. X: WrapCluster(F:Drop)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(6.0, 68.0),
                        ColumnLayout,
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapCluster {
                                            fallback: WrapClusterFallback::Drop,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 3: X: WrapCluster (F: Keep), Y: Keep
            // Keep this card in sync with test_wrap_cluster_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 220.0),
                    ColumnLayout,
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("3. X: WrapCluster(F:Keep)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(4.0, 162.0),
                        ColumnLayout,
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello\nhello")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapCluster {
                                            fallback: WrapClusterFallback::Keep,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }

        // Row 2: Cards 4 - 6
        label(
            ctx,
            LabelSpecBuilder::new()
                .text(r#"Remaining rows: all text is the same "hello there\nhello there" string"#),
            ColumnLayoutParams::auto(),
        );
        {
            let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

            // Card 4: X: WrapWord, Y: Keep
            // Keep this card in sync with test_wrap_word_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(48.0, 68.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 5: X: WrapWord (F: WrapCluster), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_wrap_cluster_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 200.0),
                    ColumnLayout,
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("5. X: WrapWord(F:WrapC)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(23.0, 138.0),
                        ColumnLayout,
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::WrapCluster {
                                                fallback: WrapClusterFallback::Drop,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 6: X: WrapWord (F: WrapCluster F: Drop), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_wrap_cluster_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 200.0),
                    ColumnLayout,
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("6. X: WrapWord(F:WC F:Dr)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(6.0, 138.0),
                        ColumnLayout,
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::WrapCluster {
                                                fallback: WrapClusterFallback::Drop,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }

        // Row 3: Cards 7 - 9
        {
            let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

            // Card 7: X: WrapWord (F: WrapCluster F: Keep), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_wrap_cluster_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 380.0),
                    ColumnLayout,
                );
                label(
                    &mut container.ctx,
                    LabelSpecBuilder::new()
                        .text("7. X: WrapWord(F:WC F:Kp)")
                        .style(LabelStyle {
                            text_style: framewise::TextStyle {
                                font: theme.mono_font,
                                size: 13.0,
                                ..(LabelStyle::from_theme(&theme)).text_style
                            },
                            text_color: theme.rust,
                            ..LabelStyle::from_theme(&theme)
                        }),
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(4.0, 318.0),
                        ColumnLayout,
                    );
                    label(
                        &mut clip_box.ctx,
                        LabelSpecBuilder::new()
                            .text("hello there\nhello there")
                            .style(LabelStyle {
                                text_style: framewise::TextStyle {
                                    flow: TextFlow {
                                        overflow_x: OverflowX::WrapWord {
                                            fallback: WrapWordFallback::WrapCluster {
                                                fallback: WrapClusterFallback::Keep,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 8: X: WrapWord (F: Drop), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_drop_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(25.0, 68.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }
            row.spacer(20.0);

            // Card 9: X: WrapWord (F: Keep), Y: Keep
            // Keep this card in sync with test_wrap_word_fallback_keep_y_keep in sample/src/text/tests.rs
            {
                let mut container = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(box_style),
                    RowLayoutParams::fixed(230.0, 140.0),
                    ColumnLayout,
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
                    ColumnLayoutParams::auto(),
                );
                container.ctx.spacer(8.0);
                {
                    let mut clip_box = begin_frame(
                        &mut container.ctx,
                        FrameSpecBuilder::new().style(clip_test_box_style),
                        ColumnLayoutParams::fixed(25.0, 68.0),
                        ColumnLayout,
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
                        ColumnLayoutParams::auto().fill_x().fill_y(),
                    );
                    clip_box.ctx.finish();
                }
                container.ctx.finish();
            }

            row.finish();
        }
    }
    ctx.spacer(28.0);

    // Section 5: Internal Text Alignment
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            content_placement: framewise::TextContentPlacement::TOP_LEFT,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("5. Internal Text Alignment (Fixed-Width)")
                .style(section_header),
            ColumnLayoutParams::auto(),
        );

        let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

        let alignments = [
            ("Start Aligned Text", TextLineAlign::Start),
            ("Center Aligned Text", TextLineAlign::Center),
            ("End Aligned Text", TextLineAlign::End),
        ];

        for (text, text_align) in alignments {
            let mut container = begin_frame(
                &mut row,
                FrameSpecBuilder::new().style(box_style),
                RowLayoutParams::fixed(230.0, 80.0),
                ColumnLayout,
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
                        ..LabelStyle::from_theme(&theme)
                    }),
                ColumnLayoutParams::auto(),
            );
            container.ctx.spacer(6.0);

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
                    ..LabelStyle::from_theme(&theme)
                }),
                ColumnLayoutParams::auto().fill_x(),
            );

            container.ctx.finish();
        }

        row.finish();
    }
    ctx.spacer(28.0);

    // Section 6: Widget Content Placement
    {
        let section_header = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                20.0,
                theme.sans_weight_bold,
                framewise::text::TextFlow::single_line(),
            ),
            content_placement: framewise::TextContentPlacement::TOP_LEFT,
            text_color: theme.ink,
            rule: true,
            rule_color: theme.line,
        };
        label(
            ctx,
            LabelSpecBuilder::new()
                .text("6. Widget Content Placement")
                .style(section_header),
            ColumnLayoutParams::auto(),
        );

        let positions = [
            ("top left", Align::Start, Align::Start),
            ("top center", Align::Center, Align::Start),
            ("top right", Align::End, Align::Start),
            ("middle left", Align::Start, Align::Center),
            ("center", Align::Center, Align::Center),
            ("middle right", Align::End, Align::Center),
            ("bottom left", Align::Start, Align::End),
            ("bottom center", Align::Center, Align::End),
            ("bottom right", Align::End, Align::End),
        ];

        for row_positions in positions.chunks(3) {
            let mut row = ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), RowLayout);

            for (text, x, y) in row_positions {
                let mut cell = begin_frame(
                    &mut row,
                    FrameSpecBuilder::new().style(FrameStyle {
                        background: Color::from_srgb_u8(255, 255, 255, 255),
                        border: theme.line,
                        border_width: 1.0,
                        padding: 0.0,
                    }),
                    RowLayoutParams::fixed(150.0, 72.0),
                    ColumnLayout,
                );
                label(
                    &mut cell.ctx,
                    LabelSpecBuilder::new().text(text).style(LabelStyle {
                        content_placement: framewise::TextContentPlacement::logical(
                            framewise::ContentPlacement::Align(*x),
                            framewise::ContentPlacement::Align(*y),
                        ),
                        text_color: theme.ink,
                        ..LabelStyle::from_theme(&theme)
                    }),
                    ColumnLayoutParams::auto().fill_x().fill_y(),
                );
                cell.ctx.finish();
            }

            row.finish();
        }

        let mut row = ctx.child_with_layout(ColumnLayoutParams::fixed(360.0, 29.0), RowLayout);
        let comparison_label = LabelStyle {
            content_placement: framewise::TextContentPlacement::CENTER,
            ..LabelStyle::from_theme(&theme)
        };
        let icon_flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let icon_label = LabelStyle {
            text_style: framewise::TextStyle::new(
                theme.sans_font,
                30.0,
                theme.sans_weight_regular,
                icon_flow,
            ),
            text_color: theme.rust,
            ..LabelStyle::from_theme(&theme)
        };
        let icon_frame = FrameStyle {
            background: Color::from_srgb_u8(255, 255, 255, 255),
            border: theme.line,
            border_width: 1.0,
            padding: 0.0,
        };

        label(
            &mut row,
            LabelSpecBuilder::new()
                .text("logical center:")
                .style(comparison_label),
            RowLayoutParams::fixed(92.0, 29.0),
        );
        row.spacer(6.0);
        {
            let mut cell = begin_frame(
                &mut row,
                FrameSpecBuilder::new().style(icon_frame),
                RowLayoutParams::fixed(29.0, 29.0),
                ColumnLayout,
            );
            label(
                &mut cell.ctx,
                LabelSpecBuilder::new().text("×").style(LabelStyle {
                    content_placement: framewise::TextContentPlacement::CENTER,
                    ..icon_label
                }),
                ColumnLayoutParams::auto().fill_x().fill_y(),
            );
            cell.ctx.finish();
        }
        row.spacer(18.0);
        label(
            &mut row,
            LabelSpecBuilder::new()
                .text("ink center:")
                .style(comparison_label),
            RowLayoutParams::fixed(72.0, 29.0),
        );
        row.spacer(6.0);
        {
            let mut cell = begin_frame(
                &mut row,
                FrameSpecBuilder::new().style(icon_frame),
                RowLayoutParams::fixed(29.0, 29.0),
                ColumnLayout,
            );
            label(
                &mut cell.ctx,
                LabelSpecBuilder::new().text("×").style(LabelStyle {
                    content_placement: framewise::TextContentPlacement::INK_CENTER,
                    ..icon_label
                }),
                ColumnLayoutParams::auto().fill_x().fill_y(),
            );
            cell.ctx.finish();
        }
        row.finish();
    }

    // Footer info
    label(
        ctx,
        LabelSpecBuilder::new()
            .text("Press F1-F5 to navigate to other showcase pages. (F6 for Labels)")
            .style(LabelStyle {
                text_style: framewise::TextStyle::new(
                    theme.mono_font,
                    12.0,
                    theme.sans_weight_regular,
                    framewise::text::TextFlow::single_line(),
                ),
                content_placement: framewise::TextContentPlacement::TOP_LEFT,
                text_color: Color::from_srgb_u8(120, 120, 130, 255),
                rule: false,
                rule_color: theme.line,
            }),
        ColumnLayoutParams::auto(),
    );
}
