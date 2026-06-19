use super::{CaretGeom, CaretPosition, TextLayout, WorkingCluster};
use crate::types::Vec2;

impl<G> TextLayout<G> {
    /// Return the visual line index that owns a logical caret position.
    ///
    /// These visual-line caret helpers are intended for keyboard and logical
    /// movement such as Home/End. Pointer coordinate resolution should keep
    /// using [`TextLayout::hit_test_caret`] and [`TextLayout::hit_test_cluster`].
    pub fn visual_line_index_for_caret(&self, caret: CaretPosition) -> usize {
        match caret {
            CaretPosition::BeforeCluster { cluster_byte_start } => self
                .find_exact_cluster(cluster_byte_start)
                .map(|(line_idx, _, _)| line_idx)
                .unwrap_or_else(|| self.visual_line_index_for_insertion_byte(cluster_byte_start)),
            CaretPosition::AfterCluster {
                cluster_byte_start, ..
            } => {
                if let Some((line_idx, _, cluster)) = self.find_exact_cluster(cluster_byte_start) {
                    if cluster.is_hard_break || cluster.is_soft_wrap_boundary {
                        return self
                            .lines
                            .get(line_idx + 1)
                            .map(|_| line_idx + 1)
                            .unwrap_or(line_idx);
                    }
                    return line_idx;
                }
                self.visual_line_index_for_insertion_byte(caret.insertion_byte_hint())
            }
            CaretPosition::EmptyText => 0,
        }
    }

    /// Return the caret at the visual start of a laid-out line.
    ///
    /// Non-empty line starts return the first cluster's `BeforeCluster`
    /// anchor, including mid-word soft-wrap starts that share an insertion byte
    /// with the previous line's end. Empty continuation lines, such as the
    /// editor feedback line after a terminal hard newline, use the preceding
    /// boundary cluster's `AfterCluster` caret because no following cluster
    /// exists.
    pub fn caret_at_visual_line_start(&self, line_index: usize) -> CaretPosition {
        let line_idx = line_index.min(self.lines.len().saturating_sub(1));
        let line = &self.lines[line_idx];

        if let Some(first) = line.clusters.first() {
            return CaretPosition::BeforeCluster {
                cluster_byte_start: first.byte_start,
            };
        }

        self.empty_line_caret_position(line_idx)
    }

    /// Return the caret at the visual end of a laid-out line.
    ///
    /// Hard newline and collapsed soft-wrap-whitespace endings return
    /// `BeforeCluster` for their boundary cluster, keeping the caret visually
    /// on the ending line. Ordinary soft wraps and end-of-text lines return
    /// `AfterCluster` for the last visible cluster on that line.
    pub fn caret_at_visual_line_end(&self, line_index: usize) -> CaretPosition {
        let line_idx = line_index.min(self.lines.len().saturating_sub(1));
        let line = &self.lines[line_idx];

        let Some(last) = line.clusters.last() else {
            return self.empty_line_caret_position(line_idx);
        };

        if last.is_hard_break || last.is_soft_wrap_boundary {
            CaretPosition::BeforeCluster {
                cluster_byte_start: last.byte_start,
            }
        } else {
            CaretPosition::AfterCluster {
                cluster_byte_start: last.byte_start,
                cluster_byte_end: last.byte_end,
            }
        }
    }

    /// Caret geometry for a visual caret position in block-local coordinates.
    ///
    /// Caret positions follow shaped advances and line metrics, not the tight
    /// ink box of the surrounding text. `EmptyText` returns the start of the
    /// single empty line with a positive height.
    ///
    /// Mid-word soft wraps can have two visual anchors for one insertion byte:
    /// `AfterCluster` on the previous visual line and `BeforeCluster` on the
    /// following visual line. Hard newline clusters and collapsed
    /// soft-wrap-boundary whitespace are source-distinct boundary clusters:
    /// `BeforeCluster` is on the previous visual line, while `AfterCluster` is
    /// at the start of the following visual line. If boundary whitespace is
    /// terminal, `AfterCluster` is on the following empty visual line created
    /// for editor feedback.
    pub fn caret_geom(&self, position: CaretPosition) -> CaretGeom {
        let Some((line_idx, _, cluster)) = self.find_caret_cluster(position) else {
            let line = self
                .lines
                .first()
                .expect("a text layout always has at least one line");
            return CaretGeom {
                x: line.logical_x,
                y_top: line.y_top,
                height: line.height,
            };
        };

        let line = &self.lines[line_idx];
        let (x, y_top, height) = match position {
            CaretPosition::BeforeCluster { .. } => (cluster.x, line.y_top, line.height),
            CaretPosition::AfterCluster { .. }
                if cluster.is_hard_break || cluster.is_soft_wrap_boundary =>
            {
                let next_line = self.lines.get(line_idx + 1).unwrap_or(line);
                let next_x = next_line
                    .clusters
                    .first()
                    .map(|cluster| cluster.x)
                    .unwrap_or(next_line.logical_x);
                (next_x, next_line.y_top, next_line.height)
            }
            CaretPosition::AfterCluster { .. } => {
                (cluster.x + cluster.advance, line.y_top, line.height)
            }
            CaretPosition::EmptyText => unreachable!("handled by missing-cluster branch"),
        };

        CaretGeom { x, y_top, height }
    }

    /// Return the caret position on the selected visual line closest to the layout-space x coordinate.
    ///
    /// This is intended for keyboard vertical movement (such as Up, Down, PageUp, PageDown),
    /// not for pointer coordinate hit-testing.
    pub fn caret_at_visual_line_x(&self, line_index: usize, x: f32) -> CaretPosition {
        let line_idx = line_index.min(self.lines.len().saturating_sub(1));
        let line = &self.lines[line_idx];
        let clusters = &line.clusters;
        if clusters.is_empty() {
            return self.empty_line_caret_position(line_idx);
        }

        let start_caret = self.caret_at_visual_line_start(line_idx);
        let end_caret = self.caret_at_visual_line_end(line_idx);

        let start_x = self.caret_geom(start_caret).x;
        let end_x = self.caret_geom(end_caret).x;

        let is_ltr = start_x <= end_x;
        if is_ltr {
            if x <= start_x {
                return start_caret;
            }
            if x >= end_x {
                return end_caret;
            }
        } else {
            if x >= start_x {
                return start_caret;
            }
            if x <= end_x {
                return end_caret;
            }
        }

        for cluster in clusters {
            let mid = cluster.x + cluster.advance * 0.5;
            if x < mid {
                return CaretPosition::BeforeCluster {
                    cluster_byte_start: cluster.byte_start,
                };
            }
        }

        end_caret
    }

    /// Hit-test a block-local point to the nearest character boundary.
    ///
    /// The point is resolved to a visual line by y, then to the nearest gap
    /// between clusters by x:
    ///
    /// - Points above the block clamp to the first line; points below clamp to
    ///   the last line.
    /// - Points to the left of a non-empty line return `BeforeCluster` for that
    ///   line's first cluster.
    /// - Points to the right of a line clamp to the end of the visible content
    ///   on that line. If the line ends with a hard newline or collapsed
    ///   soft-wrap boundary, this returns `BeforeCluster` for that source
    ///   boundary cluster so the previous visual line is preserved. At
    ///   mid-word soft wraps, this returns `AfterCluster` for the previous
    ///   line's last cluster, which may have the same insertion byte as the
    ///   following line's `BeforeCluster`.
    /// - Points on an empty line return the visual position for that empty line:
    ///   `EmptyText` for empty input, or `AfterCluster` for the previous hard
    ///   newline or terminal collapsed soft-wrap boundary when the empty line
    ///   exists because of such a boundary.
    /// - Points anywhere in an empty text layout return `EmptyText`.
    pub fn hit_test_caret(&self, pos: Vec2) -> CaretPosition {
        let line_idx = self
            .lines
            .iter()
            .position(|line| pos.y < line.y_top + line.height)
            .unwrap_or_else(|| self.lines.len().saturating_sub(1));
        self.caret_at_visual_line_x(line_idx, pos.x)
    }

    /// Hit-test a block-local point to a shaped cluster start byte.
    ///
    /// The point is resolved to a visual line by y, then to the cluster
    /// containing x:
    ///
    /// - Points above the block clamp to the first line; points below clamp to
    ///   the last line.
    /// - Points to the left of a line clamp to the first cluster of that line.
    /// - Points to the right of a line clamp to the last cluster of that line.
    /// - For multi-byte characters or complex clusters, this returns the
    ///   starting byte index of the cluster.
    /// - If the line ends with a boundary cluster that has no visual advance,
    ///   such as a hard newline or collapsed soft-wrap-boundary whitespace, a
    ///   hit to the right of the line or on that boundary returns the boundary
    ///   cluster's start byte index.
    /// - Empty text has no clusters, so every hit returns byte `0`.
    pub fn hit_test_cluster(&self, pos: Vec2) -> usize {
        let line_idx = self
            .lines
            .iter()
            .position(|line| pos.y < line.y_top + line.height)
            .unwrap_or_else(|| self.lines.len().saturating_sub(1));
        let line = &self.lines[line_idx];
        let clusters = &line.clusters;
        if clusters.is_empty() {
            return 0;
        }

        for cluster in clusters {
            if pos.x <= cluster.x + cluster.advance {
                return cluster.byte_start;
            }
        }

        clusters
            .last()
            .map(|cluster| cluster.byte_start)
            .unwrap_or(0)
    }

    /// Choose a canonical visual caret anchor for a programmatic insertion byte
    /// index.
    ///
    /// This is intended for non-hit-tested movement such as "go to byte 0",
    /// "go to end", or adapting existing byte-oriented editor state. It returns
    /// `BeforeCluster` for the first cluster at or after the byte, and
    /// `AfterCluster` for the last cluster when the byte is at or beyond the
    /// prepared text's end. Empty text returns `EmptyText` for every requested
    /// byte index.
    pub fn caret_position_at_insertion_byte(&self, byte_index: usize) -> CaretPosition {
        if self.first_cluster().is_none() {
            return CaretPosition::EmptyText;
        }

        let mut clusters = self.iter_clusters().peekable();
        while let Some((_, _, cluster)) = clusters.next() {
            if byte_index <= cluster.byte_start || byte_index < cluster.byte_end {
                return CaretPosition::BeforeCluster {
                    cluster_byte_start: cluster.byte_start,
                };
            }

            if byte_index == cluster.byte_end {
                if let Some((_, _, next)) = clusters.peek() {
                    if next.byte_start == byte_index {
                        return CaretPosition::BeforeCluster {
                            cluster_byte_start: next.byte_start,
                        };
                    }
                }
                return CaretPosition::AfterCluster {
                    cluster_byte_start: cluster.byte_start,
                    cluster_byte_end: cluster.byte_end,
                };
            }
        }

        let last = self.last_cluster().expect("clusters is non-empty");
        CaretPosition::AfterCluster {
            cluster_byte_start: last.byte_start,
            cluster_byte_end: last.byte_end,
        }
    }

    /// Return an `AfterCluster` caret anchored to a cluster start in this
    /// already-built layout.
    pub fn caret_after_cluster_start(&self, cluster_byte_start: usize) -> Option<CaretPosition> {
        self.iter_clusters()
            .find(|(_, _, cluster)| cluster.byte_start == cluster_byte_start)
            .map(|(_, _, cluster)| CaretPosition::AfterCluster {
                cluster_byte_start: cluster.byte_start,
                cluster_byte_end: cluster.byte_end,
            })
    }

    /// Move one shaped cluster boundary to the left.
    ///
    /// Movement follows the prepared text's cluster model, not UTF-8 scalar
    /// boundaries. When movement crosses a visual line boundary, the returned
    /// [`CaretPosition`] is the nearest visually distinct caret position to the
    /// left, not a same-geometry intermediate anchor.
    pub fn previous_caret_position(&self, position: CaretPosition) -> CaretPosition {
        if let CaretPosition::BeforeCluster { cluster_byte_start } = position {
            if let Some((line_idx, cluster_idx, cluster)) =
                self.find_exact_cluster(cluster_byte_start)
            {
                if cluster_idx == 0 {
                    if let Some(previous) = self
                        .lines
                        .get(..line_idx)
                        .and_then(|lines| lines.iter().rev().find_map(|line| line.clusters.last()))
                        .filter(|previous| previous.byte_end == cluster.byte_start)
                    {
                        if previous.is_hard_break || previous.is_soft_wrap_boundary {
                            return CaretPosition::BeforeCluster {
                                cluster_byte_start: previous.byte_start,
                            };
                        }
                    }
                }
            }
        }

        let byte_index = position.insertion_byte_hint();
        let Some(target_byte) = self.previous_insertion_boundary(byte_index) else {
            return self.caret_position_at_insertion_byte(0);
        };
        self.caret_position_before_insertion_boundary(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }

    /// Move one shaped cluster boundary to the right.
    ///
    /// Movement follows the prepared text's cluster model, not UTF-8 scalar
    /// boundaries. When movement crosses a visual line boundary, the returned
    /// [`CaretPosition`] is the nearest visually distinct caret position to the
    /// right, not a same-geometry intermediate anchor.
    pub fn next_caret_position(&self, position: CaretPosition) -> CaretPosition {
        match position {
            CaretPosition::BeforeCluster { cluster_byte_start } => {
                if let Some((line_idx, cluster_idx, cluster)) =
                    self.find_exact_cluster(cluster_byte_start)
                {
                    if cluster.is_hard_break || cluster.is_soft_wrap_boundary {
                        return CaretPosition::AfterCluster {
                            cluster_byte_start: cluster.byte_start,
                            cluster_byte_end: cluster.byte_end,
                        };
                    }

                    if self.next_cluster_after(line_idx, cluster_idx).is_none() {
                        return CaretPosition::AfterCluster {
                            cluster_byte_start: cluster.byte_start,
                            cluster_byte_end: cluster.byte_end,
                        };
                    }
                }
            }
            CaretPosition::AfterCluster { .. } => {}
            CaretPosition::EmptyText => {}
        }

        let byte_index = position.insertion_byte_hint();
        let Some(target_byte) = self.next_insertion_boundary(byte_index) else {
            return self
                .last_cluster()
                .map(|cluster| CaretPosition::AfterCluster {
                    cluster_byte_start: cluster.byte_start,
                    cluster_byte_end: cluster.byte_end,
                })
                .unwrap_or(CaretPosition::EmptyText);
        };
        self.caret_position_after_insertion_boundary(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }

    fn find_caret_cluster(
        &self,
        position: CaretPosition,
    ) -> Option<(usize, usize, &WorkingCluster)> {
        let cluster_byte_start = match position {
            CaretPosition::BeforeCluster { cluster_byte_start }
            | CaretPosition::AfterCluster {
                cluster_byte_start, ..
            } => cluster_byte_start,
            CaretPosition::EmptyText => return None,
        };

        self.find_exact_cluster(cluster_byte_start)
            .or_else(|| {
                self.iter_clusters().find(|(_, _, cluster)| {
                    cluster_byte_start <= cluster.byte_start
                        || cluster_byte_start < cluster.byte_end
                })
            })
            .or_else(|| self.iter_clusters().next_back())
    }

    fn find_exact_cluster(
        &self,
        cluster_byte_start: usize,
    ) -> Option<(usize, usize, &WorkingCluster)> {
        self.iter_clusters()
            .find(|(_, _, cluster)| cluster.byte_start == cluster_byte_start)
    }

    fn next_cluster_after(
        &self,
        line_idx: usize,
        cluster_idx: usize,
    ) -> Option<(usize, usize, &WorkingCluster)> {
        self.lines
            .iter()
            .enumerate()
            .skip(line_idx)
            .flat_map(|(candidate_line_idx, line)| {
                let start_idx = if candidate_line_idx == line_idx {
                    cluster_idx + 1
                } else {
                    0
                };
                line.clusters.iter().enumerate().skip(start_idx).map(
                    move |(candidate_cluster_idx, cluster)| {
                        (candidate_line_idx, candidate_cluster_idx, cluster)
                    },
                )
            })
            .next()
    }

    fn caret_position_before_insertion_boundary(
        &self,
        target_byte: usize,
    ) -> Option<CaretPosition> {
        if let Some((_, _, cluster)) = self
            .iter_clusters()
            .find(|(_, _, cluster)| cluster.byte_start == target_byte)
        {
            return Some(CaretPosition::BeforeCluster {
                cluster_byte_start: cluster.byte_start,
            });
        }

        self.iter_clusters()
            .find(|(_, _, cluster)| cluster.byte_end == target_byte)
            .map(|(_, _, cluster)| CaretPosition::AfterCluster {
                cluster_byte_start: cluster.byte_start,
                cluster_byte_end: cluster.byte_end,
            })
    }

    fn caret_position_after_insertion_boundary(&self, target_byte: usize) -> Option<CaretPosition> {
        if let Some((_, _, cluster)) = self
            .iter_clusters()
            .find(|(_, _, cluster)| cluster.byte_start == target_byte)
        {
            return Some(CaretPosition::BeforeCluster {
                cluster_byte_start: cluster.byte_start,
            });
        }

        self.iter_clusters()
            .find(|(_, _, cluster)| cluster.byte_end == target_byte)
            .map(|(_, _, cluster)| CaretPosition::AfterCluster {
                cluster_byte_start: cluster.byte_start,
                cluster_byte_end: cluster.byte_end,
            })
    }

    fn empty_line_caret_position(&self, line_idx: usize) -> CaretPosition {
        if self.first_cluster().is_none() {
            return CaretPosition::EmptyText;
        }

        self.lines
            .get(..line_idx)
            .and_then(|lines| lines.iter().rev().find(|line| !line.clusters.is_empty()))
            .and_then(|line| line.clusters.last())
            .map(|cluster| CaretPosition::AfterCluster {
                cluster_byte_start: cluster.byte_start,
                cluster_byte_end: cluster.byte_end,
            })
            .unwrap_or(CaretPosition::EmptyText)
    }

    fn previous_insertion_boundary(&self, byte_index: usize) -> Option<usize> {
        self.iter_clusters()
            .map(|(_, _, cluster)| cluster)
            .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
            .filter(|byte| *byte < byte_index)
            .max()
    }

    fn next_insertion_boundary(&self, byte_index: usize) -> Option<usize> {
        self.iter_clusters()
            .map(|(_, _, cluster)| cluster)
            .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
            .filter(|byte| *byte > byte_index)
            .min()
    }

    fn visual_line_index_for_insertion_byte(&self, byte_index: usize) -> usize {
        self.lines
            .iter()
            .enumerate()
            .find(|(_, line)| byte_index >= line.byte_start && byte_index <= line.byte_end)
            .map(|(line_idx, _)| line_idx)
            .unwrap_or_else(|| self.lines.len().saturating_sub(1))
    }

    fn iter_clusters(&self) -> impl DoubleEndedIterator<Item = (usize, usize, &WorkingCluster)> {
        self.lines.iter().enumerate().flat_map(|(line_idx, line)| {
            line.clusters
                .iter()
                .enumerate()
                .map(move |(cluster_idx, cluster)| (line_idx, cluster_idx, cluster))
        })
    }

    fn first_cluster(&self) -> Option<&WorkingCluster> {
        self.lines.iter().find_map(|line| line.clusters.first())
    }

    fn last_cluster(&self) -> Option<&WorkingCluster> {
        self.lines
            .iter()
            .rev()
            .find_map(|line| line.clusters.last())
    }
}
