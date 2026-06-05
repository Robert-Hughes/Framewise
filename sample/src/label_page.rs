use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::{Align, Placement, Placement2D},
    layouts::{ColumnLayout, RowLayout},
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
            let mut col = row.child_with_layout(
                Placement2D::fixed(350.0, 260.0),
                ColumnLayout { spacing: 12.0 },
            );
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

    // Section 3: Alignment Showcase
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
                .text("3. Alignment in Bounded Space")
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

        // Alignment boxes
        let alignments = [
            ("Start Aligned", Align::Start),
            ("Center Aligned", Align::Center),
            ("End Aligned", Align::End),
        ];

        let box_style = FrameStyle {
            background: Color::from_srgb_u8(240, 240, 243, 255),
            border: Color::from_srgb_u8(210, 210, 215, 255),
            border_width: 1.0,
            padding: 8.0,
        };

        for (text, align) in alignments {
            // Draw a framed box to visualize the alignment bounds
            let mut container = begin_frame(
                &mut row,
                FrameSpecBuilder::new().style(box_style),
                Placement2D::fixed(230.0, 60.0),
                ColumnLayout { spacing: 0.0 },
            );

            label(
                &mut container.ctx,
                LabelSpecBuilder::new().text(text).style(LabelStyle {
                    size: 14.0,
                    font: theme.sans_font,
                    text_color: theme.ink,
                    rule: false,
                    rule_color: theme.line,
                }),
                // The frame's interior padding reduces the usable space, so we
                // align the label using fill on cross-axis and auto on the main axis
                // with the desired alignment.
                Placement2D::fixed(200.0, 40.0)
                    .align_x(align)
                    .align_y(Align::Center),
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
