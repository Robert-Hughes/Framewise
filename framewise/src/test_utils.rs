use crate::text::{
    CaretGeom, CaretPosition, LineEndKind, LineMetrics, OverflowX, PrepareGlyphRequest,
    ShapedCluster, ShapedGlyph, ShapedText, TextBackend, TextBounds, TextHandle, TextLayout,
    TextLineAlign, TextMetrics, TextStyle, TextSystem,
};
use crate::types::{Rect, Vec2};
use crate::{DrawGlyph, PreparedGlyphHandle};

/// A dummy text system for unit tests that provides representative text dimensions.
/// Assumes each character is 8px wide and 16px tall, supporting newlines for multi-line layout.
pub struct DummyTextSys {
    pub last_run: Option<(String, TextMetrics)>,
    pub last_rect_width: f32,
    pub last_line_align: TextLineAlign,
}

#[allow(non_upper_case_globals)]
pub const DummyTextSys: DummyTextSys = DummyTextSys {
    last_run: None,
    last_rect_width: 0.0,
    last_line_align: TextLineAlign::Start,
};

impl DummyTextSys {
    fn metrics(text: &str, style: TextStyle, max_width: Option<f32>) -> TextMetrics {
        let mut lines = Vec::new();
        let mut y_top = 0.0;
        let mut max_line_width = 0.0;

        let wrap = matches!(
            style.flow.overflow_x,
            OverflowX::WrapWord { .. } | OverflowX::WrapCluster { .. }
        );

        let max_chars = if let Some(w) = max_width {
            ((w / 8.0).floor() as usize).max(1)
        } else {
            usize::MAX
        };

        let line_align_offset_for = |line_width: f32| -> f32 {
            if let Some(w) = max_width {
                match style.flow.line_align {
                    TextLineAlign::Start => 0.0,
                    TextLineAlign::Center => ((w - line_width) * 0.5).max(0.0),
                    TextLineAlign::End => (w - line_width).max(0.0),
                }
            } else {
                0.0
            }
        };

        let mut byte_idx = 0;
        let mut lines_iter = text.split('\n').peekable();
        while let Some(line) = lines_iter.next() {
            let line_len = line.len();
            let has_next = lines_iter.peek().is_some();
            let byte_end = byte_idx + line_len + if has_next { 1 } else { 0 };

            if wrap && line.chars().count() > max_chars {
                // Soft wrap this line at character boundary
                let char_indices: Vec<(usize, char)> = line.char_indices().collect();
                let mut start_idx = 0;
                while start_idx < char_indices.len() {
                    let end_idx = (start_idx + max_chars).min(char_indices.len());

                    let byte_start = byte_idx + char_indices[start_idx].0;
                    let byte_end_sub = if end_idx < char_indices.len() {
                        byte_idx + char_indices[end_idx].0
                    } else {
                        byte_idx + line_len
                    };

                    let sub_char_count = end_idx - start_idx;
                    let line_width = sub_char_count as f32 * 8.0;
                    if line_width > max_line_width {
                        max_line_width = line_width;
                    }

                    let logical_x = line_align_offset_for(line_width);
                    lines.push(LineMetrics {
                        y_top,
                        height: 16.0,
                        logical_width: line_width,
                        ink_width: line_width,
                        logical_x,
                        ink_x: logical_x,
                        byte_start,
                        byte_end: byte_end_sub,
                        end_kind: if end_idx < char_indices.len() {
                            LineEndKind::SoftWrapNonWhitespace
                        } else if has_next {
                            LineEndKind::HardNewline
                        } else {
                            LineEndKind::EndOfText
                        },
                    });

                    y_top += 16.0;
                    start_idx = end_idx;
                }
            } else {
                let char_count = line.chars().count();
                let line_width = char_count as f32 * 8.0;
                if line_width > max_line_width {
                    max_line_width = line_width;
                }

                let logical_x = line_align_offset_for(line_width);
                lines.push(LineMetrics {
                    y_top,
                    height: 16.0,
                    logical_width: line_width,
                    ink_width: line_width,
                    logical_x,
                    ink_x: logical_x,
                    byte_start: byte_idx,
                    byte_end,
                    end_kind: if has_next {
                        LineEndKind::HardNewline
                    } else {
                        LineEndKind::EndOfText
                    },
                });

                y_top += 16.0;
            }

            byte_idx = byte_end;
        }

        if lines.is_empty() {
            let logical_x = line_align_offset_for(0.0);
            lines.push(LineMetrics {
                y_top: 0.0,
                height: 16.0,
                logical_width: 0.0,
                ink_width: 0.0,
                logical_x,
                ink_x: logical_x,
                byte_start: 0,
                byte_end: 0,
                end_kind: LineEndKind::EndOfText,
            });
        }

        let line_count = lines.len();
        TextMetrics {
            logical_size: Vec2::new(max_line_width, line_count as f32 * 16.0),
            ink_bounds: Rect::new(0.0, 0.0, max_line_width, line_count as f32 * 16.0),
            line_count: line_count as u32,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines,
        }
    }
}

impl TextBackend for DummyTextSys {
    type ShapedGlyphId = u32;

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        16.0
    }

    fn shape_text(&mut self, text: &str, _style: TextStyle) -> ShapedText<Self::ShapedGlyphId> {
        let mut clusters = Vec::new();
        for (byte_start, ch) in text.char_indices() {
            let byte_end = byte_start + ch.len_utf8();
            let advance = 8.0;
            let is_whitespace = ch.is_whitespace();
            let glyphs = vec![ShapedGlyph {
                id: ch as u32,
                x: 0.0,
                y: 0.0,
                advance,
            }];
            clusters.push(ShapedCluster {
                byte_start,
                byte_end,
                advance,
                is_whitespace,
                glyphs,
            });
        }

        ShapedText { clusters }
    }

    fn shape_ellipsis(&mut self, _style: TextStyle) -> ShapedText<Self::ShapedGlyphId> {
        ShapedText {
            clusters: vec![ShapedCluster {
                byte_start: 0,
                byte_end: 0,
                advance: 8.0,
                is_whitespace: false,
                glyphs: vec![ShapedGlyph {
                    id: '.' as u32,
                    x: 0.0,
                    y: 0.0,
                    advance: 8.0,
                }],
            }],
        }
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphId>,
    ) -> Option<DrawGlyph> {
        if request.glyph == ' ' as u32 {
            return None;
        }

        Some(DrawGlyph {
            handle: PreparedGlyphHandle(request.glyph),
            top_left: request.glyph_origin,
        })
    }
}

impl TextSystem for DummyTextSys {
    fn measure(&mut self, text: &str, style: TextStyle, bounds: TextBounds) -> TextMetrics {
        Self::metrics(text, style, bounds.max_width)
    }

    fn prepare(&mut self, text: &str, style: TextStyle, rect: Rect) -> TextLayout {
        let metrics = Self::metrics(text, style, Some(rect.w));
        self.last_run = Some((text.to_string(), metrics.clone()));
        self.last_rect_width = rect.w;
        self.last_line_align = style.flow.line_align;
        TextLayout {
            handle: TextHandle(0),
            metrics,
            lines: Vec::new(),
            clusters: Vec::new(),
            glyphs: Vec::new(),
        }
    }

    fn caret_geom(&self, _handle: TextHandle, position: CaretPosition) -> CaretGeom {
        let byte_index = self.caret_insertion_byte(TextHandle(0), position);
        let anchor_byte = match position {
            CaretPosition::BeforeCluster { cluster_byte_index }
            | CaretPosition::AfterCluster { cluster_byte_index } => cluster_byte_index,
            CaretPosition::EmptyText => 0,
        };
        if let Some((ref _text, ref metrics)) = self.last_run {
            let line = metrics
                .lines
                .iter()
                .rev()
                .find(|l| anchor_byte >= l.byte_start && anchor_byte < l.byte_end)
                .or_else(|| {
                    metrics
                        .lines
                        .iter()
                        .rev()
                        .find(|l| byte_index >= l.byte_start)
                })
                .or_else(|| metrics.lines.first());
            let (x, y_top, height) = if let Some(line) = line {
                let col = byte_index.saturating_sub(line.byte_start);
                let x = line.logical_x + col as f32 * 8.0;
                (x, line.y_top, line.height)
            } else {
                (byte_index as f32 * 8.0, 0.0, 16.0)
            };

            CaretGeom { x, y_top, height }
        } else {
            CaretGeom {
                x: byte_index as f32 * 8.0,
                y_top: 0.0,
                height: 16.0,
            }
        }
    }

    fn hit_test_caret(&self, _handle: TextHandle, pos: Vec2) -> CaretPosition {
        if let Some((ref text, ref metrics)) = self.last_run {
            let line = metrics
                .lines
                .iter()
                .find(|l| pos.y < l.y_top + l.height)
                .copied()
                .unwrap_or_else(|| {
                    metrics.lines.last().copied().unwrap_or(LineMetrics {
                        y_top: 0.0,
                        height: 16.0,
                        logical_width: 0.0,
                        ink_width: 0.0,
                        logical_x: 0.0,
                        ink_x: 0.0,
                        byte_start: 0,
                        byte_end: text.len(),
                        end_kind: LineEndKind::EndOfText,
                    })
                });

            let pos_x = pos.x - line.logical_x;
            let col = (pos_x / 8.0).max(0.0).round() as usize;
            let line_len = line.byte_end.saturating_sub(line.byte_start);
            let actual_line_text = &text[line.byte_start..line.byte_end];
            let has_newline = actual_line_text.ends_with('\n');
            let max_col = if has_newline && line_len > 0 {
                line_len - 1
            } else {
                line_len
            };
            let clamped_col = col.min(max_col);
            self.caret_position_at_insertion_byte(TextHandle(0), line.byte_start + clamped_col)
        } else {
            CaretPosition::BeforeCluster {
                cluster_byte_index: (pos.x / 8.0).max(0.0).round() as usize,
            }
        }
    }

    fn caret_insertion_byte(&self, _handle: TextHandle, position: CaretPosition) -> usize {
        match position {
            CaretPosition::BeforeCluster { cluster_byte_index }
            | CaretPosition::AfterCluster { cluster_byte_index } => {
                let after_offset =
                    usize::from(matches!(position, CaretPosition::AfterCluster { .. }));
                cluster_byte_index + after_offset
            }
            CaretPosition::EmptyText => 0,
        }
    }

    fn caret_position_at_insertion_byte(
        &self,
        _handle: TextHandle,
        byte_index: usize,
    ) -> CaretPosition {
        if let Some((ref text, _)) = self.last_run {
            if text.is_empty() {
                return CaretPosition::EmptyText;
            }
            if byte_index >= text.len() {
                return CaretPosition::AfterCluster {
                    cluster_byte_index: text.len().saturating_sub(1),
                };
            }
        }
        CaretPosition::BeforeCluster {
            cluster_byte_index: byte_index,
        }
    }

    fn previous_caret_position(
        &self,
        handle: TextHandle,
        position: CaretPosition,
    ) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(handle, position);
        let Some((ref text, _)) = self.last_run else {
            return position;
        };
        if byte_index == 0 {
            return self.caret_position_at_insertion_byte(handle, 0);
        }

        let previous = text[..byte_index]
            .char_indices()
            .next_back()
            .map_or(0, |(idx, _)| idx);
        self.caret_position_at_insertion_byte(handle, previous)
    }

    fn next_caret_position(&self, handle: TextHandle, position: CaretPosition) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(handle, position);
        let Some((ref text, _)) = self.last_run else {
            return position;
        };
        if byte_index >= text.len() {
            return self.caret_position_at_insertion_byte(handle, text.len());
        }

        let next = text[byte_index..]
            .chars()
            .next()
            .map_or(text.len(), |ch| byte_index + ch.len_utf8());
        self.caret_position_at_insertion_byte(handle, next)
    }

    fn hit_test_cluster(&self, _handle: TextHandle, pos: Vec2) -> usize {
        if let Some((ref text, ref metrics)) = self.last_run {
            let line = metrics
                .lines
                .iter()
                .find(|l| pos.y < l.y_top + l.height)
                .copied()
                .unwrap_or_else(|| {
                    metrics.lines.last().copied().unwrap_or(LineMetrics {
                        y_top: 0.0,
                        height: 16.0,
                        logical_width: 0.0,
                        ink_width: 0.0,
                        logical_x: 0.0,
                        ink_x: 0.0,
                        byte_start: 0,
                        byte_end: text.len(),
                        end_kind: LineEndKind::EndOfText,
                    })
                });

            let line_len = line.byte_end.saturating_sub(line.byte_start);
            if line_len == 0 {
                return line.byte_start;
            }

            let pos_x = pos.x - line.logical_x;
            let col = (pos_x / 8.0).max(0.0).floor() as usize;
            let max_col = line_len.saturating_sub(1);
            let clamped_col = col.min(max_col);
            line.byte_start + clamped_col
        } else {
            (pos.x / 8.0).max(0.0).floor() as usize
        }
    }
}
