use crate::text::{
    CaretGeom, LineMetrics, TextBounds, TextHandle, TextLayout, TextMetrics, TextSystem,
};
use crate::types::{Rect, Vec2};

/// A dummy text system for unit tests that provides representative text dimensions.
/// Assumes each character is 8px wide and 16px tall, supporting newlines for multi-line layout.
pub struct DummyTextSys {
    pub last_run: Option<(String, TextMetrics)>,
}

#[allow(non_upper_case_globals)]
pub const DummyTextSys: DummyTextSys = DummyTextSys { last_run: None };

impl DummyTextSys {
    fn metrics(text: &str) -> TextMetrics {
        let mut lines = Vec::new();
        let mut y_top = 0.0;
        let mut max_width = 0.0;

        let mut byte_idx = 0;
        let mut lines_iter = text.split('\n').peekable();
        while let Some(line) = lines_iter.next() {
            let line_len = line.len();
            let has_next = lines_iter.peek().is_some();
            let byte_end = byte_idx + line_len + if has_next { 1 } else { 0 };

            lines.push(LineMetrics {
                y_top,
                height: 16.0,
                byte_start: byte_idx,
                byte_end,
            });

            let char_count = line.chars().count();
            let line_width = char_count as f32 * 8.0;
            if line_width > max_width {
                max_width = line_width;
            }

            y_top += 16.0;
            byte_idx = byte_end;
        }

        if lines.is_empty() {
            lines.push(LineMetrics {
                y_top: 0.0,
                height: 16.0,
                byte_start: 0,
                byte_end: 0,
            });
        }

        let line_count = lines.len();
        TextMetrics {
            logical_size: Vec2::new(max_width, line_count as f32 * 16.0),
            ink_bounds: Rect::new(0.0, 0.0, max_width, line_count as f32 * 16.0),
            line_count: line_count as u32,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines,
        }
    }
}

impl TextSystem for DummyTextSys {
    fn measure(
        &mut self,
        text: &str,
        _style: crate::text::TextStyle,
        _bounds: TextBounds,
    ) -> TextMetrics {
        Self::metrics(text)
    }

    fn prepare(&mut self, text: &str, _style: crate::text::TextStyle, _rect: Rect) -> TextLayout {
        let metrics = Self::metrics(text);
        self.last_run = Some((text.to_string(), metrics.clone()));
        TextLayout {
            handle: TextHandle(0),
            metrics,
        }
    }

    fn caret_geom(&self, _handle: TextHandle, byte_index: usize) -> CaretGeom {
        if let Some((ref _text, ref metrics)) = self.last_run {
            let line = metrics
                .lines
                .iter()
                .rev()
                .find(|l| byte_index >= l.byte_start)
                .or_else(|| metrics.lines.first());
            let (x, y_top, height) = if let Some(line) = line {
                let col = byte_index.saturating_sub(line.byte_start);
                let x = col as f32 * 8.0;
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

    fn hit_test_caret(&self, _handle: TextHandle, pos: Vec2) -> usize {
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
                        byte_start: 0,
                        byte_end: text.len(),
                    })
                });

            let col = (pos.x / 8.0).max(0.0).round() as usize;
            let line_len = line.byte_end.saturating_sub(line.byte_start);
            let actual_line_text = &text[line.byte_start..line.byte_end];
            let has_newline = actual_line_text.ends_with('\n');
            let max_col = if has_newline && line_len > 0 {
                line_len - 1
            } else {
                line_len
            };
            let clamped_col = col.min(max_col);
            line.byte_start + clamped_col
        } else {
            (pos.x / 8.0).max(0.0).round() as usize
        }
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
                        byte_start: 0,
                        byte_end: text.len(),
                    })
                });

            let line_len = line.byte_end.saturating_sub(line.byte_start);
            if line_len == 0 {
                return line.byte_start;
            }

            let col = (pos.x / 8.0).max(0.0).floor() as usize;
            let actual_line_text = &text[line.byte_start..line.byte_end];
            let has_newline = actual_line_text.ends_with('\n');
            let max_col = if has_newline {
                line_len - 1
            } else {
                line_len - 1
            };
            let clamped_col = col.min(max_col);
            line.byte_start + clamped_col
        } else {
            (pos.x / 8.0).max(0.0).floor() as usize
        }
    }
}
