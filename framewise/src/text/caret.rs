use super::{CaretGeom, CaretPosition, TextCluster, TextLayout};
use crate::types::Vec2;

impl<G> TextLayout<G> {
    /// Caret geometry for a visual caret position in block-local coordinates.
    ///
    /// Caret positions follow shaped advances and line metrics, not the tight
    /// ink box of the surrounding text. `EmptyText` returns the start of the
    /// single empty line with a positive height.
    ///
    /// Hard newline clusters and collapsed soft-wrap-boundary whitespace have
    /// two distinct visual anchors: `BeforeCluster` is on the previous visual
    /// line, while `AfterCluster` is at the start of the following visual line.
    /// If the boundary whitespace is terminal, `AfterCluster` is on the
    /// following empty visual line created for editor feedback.
    pub fn caret_geom(&self, position: CaretPosition) -> CaretGeom {
        let Some((cluster_idx, cluster)) = self.find_caret_cluster(position) else {
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

        let line_idx = self.line_index_for_cluster(cluster_idx).unwrap_or(0);
        let line = &self.lines[line_idx];
        let (x, y_top, height) = match position {
            CaretPosition::BeforeCluster { .. } => (cluster.x, line.y_top, line.height),
            CaretPosition::AfterCluster { .. }
                if cluster.is_hard_break || cluster.is_soft_wrap_boundary =>
            {
                let next_line = self.lines.get(line_idx + 1).unwrap_or(line);
                let next_clusters = &self.clusters[next_line.cluster_start..next_line.cluster_end];
                let next_x = next_clusters
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
    ///   soft-wrap boundary, this returns a caret anchored to that boundary
    ///   cluster so the visual line is preserved.
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
        let line = &self.lines[line_idx];
        let clusters = &self.clusters[line.cluster_start..line.cluster_end];
        if clusters.is_empty() {
            return self.empty_line_caret_position(line_idx);
        }
        for cluster in clusters {
            let mid = cluster.x + cluster.advance * 0.5;
            if pos.x < mid {
                return CaretPosition::BeforeCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }
        }
        match clusters.last() {
            Some(last) if last.is_hard_break || last.is_soft_wrap_boundary => {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: last.byte_start,
                }
            }
            Some(last) => CaretPosition::AfterCluster {
                cluster_byte_index: last.byte_start,
            },
            None => self.empty_line_caret_position(line_idx),
        }
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
        let clusters = &self.clusters[line.cluster_start..line.cluster_end];
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

    /// Convert a visual caret position into the insertion byte index used by
    /// text editing operations.
    ///
    /// `BeforeCluster` returns the anchored cluster's `byte_start`;
    /// `AfterCluster` returns its `byte_end`; `EmptyText` returns `0`.
    pub fn caret_insertion_byte(&self, position: CaretPosition) -> usize {
        match self.find_caret_cluster(position) {
            Some((_, cluster)) => match position {
                CaretPosition::BeforeCluster { .. } => cluster.byte_start,
                CaretPosition::AfterCluster { .. } => cluster.byte_end,
                CaretPosition::EmptyText => 0,
            },
            None => 0,
        }
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
        if self.clusters.is_empty() {
            return CaretPosition::EmptyText;
        }

        for (idx, cluster) in self.clusters.iter().enumerate() {
            if byte_index <= cluster.byte_start || byte_index < cluster.byte_end {
                return CaretPosition::BeforeCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }

            if byte_index == cluster.byte_end {
                if let Some(next) = self.clusters.get(idx + 1) {
                    if next.byte_start == byte_index {
                        return CaretPosition::BeforeCluster {
                            cluster_byte_index: next.byte_start,
                        };
                    }
                }
                return CaretPosition::AfterCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }
        }

        let last = self.clusters.last().expect("clusters is non-empty");
        CaretPosition::AfterCluster {
            cluster_byte_index: last.byte_start,
        }
    }

    /// Move one shaped cluster boundary to the left.
    ///
    /// Movement follows the prepared text's cluster model, not UTF-8 scalar
    /// boundaries. When movement is possible, the returned caret maps to a
    /// different insertion byte from `position`. At hard newlines and collapsed
    /// soft-wrap-boundary whitespace, the returned [`CaretPosition`] preserves
    /// the visual side reached by moving from the right, such as `AfterCluster`
    /// for the boundary character when landing immediately after it.
    pub fn previous_caret_position(&self, position: CaretPosition) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(position);
        if let CaretPosition::BeforeCluster { cluster_byte_index } = position {
            if let Some(cluster) = self
                .clusters
                .iter()
                .find(|cluster| cluster.byte_start == cluster_byte_index)
            {
                if let Some(previous) = self.clusters.iter().rev().find(|previous| {
                    previous.byte_end == cluster.byte_start
                        && (previous.is_hard_break || previous.is_soft_wrap_boundary)
                }) {
                    return CaretPosition::AfterCluster {
                        cluster_byte_index: previous.byte_start,
                    };
                }
            }
        }
        let Some(target_byte) = self.previous_insertion_boundary(byte_index) else {
            return self.caret_position_at_insertion_byte(0);
        };
        self.caret_position_for_movement_target(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }

    /// Move one shaped cluster boundary to the right.
    ///
    /// Movement follows the prepared text's cluster model, not UTF-8 scalar
    /// boundaries. When movement is possible, the returned caret maps to a
    /// different insertion byte from `position`. At hard newlines and collapsed
    /// soft-wrap-boundary whitespace, the returned [`CaretPosition`] preserves
    /// the visual side reached by moving from the left, such as `BeforeCluster`
    /// for the boundary character when landing immediately before it.
    pub fn next_caret_position(&self, position: CaretPosition) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(position);
        let Some(target_byte) = self.next_insertion_boundary(byte_index) else {
            return self
                .clusters
                .last()
                .map(|cluster| CaretPosition::AfterCluster {
                    cluster_byte_index: cluster.byte_start,
                })
                .unwrap_or(CaretPosition::EmptyText);
        };
        self.caret_position_for_movement_target(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }
    fn find_caret_cluster(&self, position: CaretPosition) -> Option<(usize, &TextCluster)> {
        let cluster_byte_index = match position {
            CaretPosition::BeforeCluster { cluster_byte_index }
            | CaretPosition::AfterCluster { cluster_byte_index } => cluster_byte_index,
            CaretPosition::EmptyText => return None,
        };

        self.clusters
            .iter()
            .enumerate()
            .find(|(_, cluster)| cluster.byte_start == cluster_byte_index)
            .or_else(|| {
                self.clusters.iter().enumerate().find(|(_, cluster)| {
                    cluster_byte_index <= cluster.byte_start
                        || cluster_byte_index < cluster.byte_end
                })
            })
            .or_else(|| self.clusters.iter().enumerate().next_back())
    }

    fn line_index_for_cluster(&self, cluster_idx: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| cluster_idx >= line.cluster_start && cluster_idx < line.cluster_end)
    }

    fn empty_line_caret_position(&self, line_idx: usize) -> CaretPosition {
        if self.clusters.is_empty() {
            return CaretPosition::EmptyText;
        }

        self.lines
            .get(..line_idx)
            .and_then(|lines| {
                lines
                    .iter()
                    .rev()
                    .find(|line| line.cluster_end > line.cluster_start)
            })
            .and_then(|line| self.clusters.get(line.cluster_end - 1))
            .map(|cluster| CaretPosition::AfterCluster {
                cluster_byte_index: cluster.byte_start,
            })
            .unwrap_or(CaretPosition::EmptyText)
    }

    fn previous_insertion_boundary(&self, byte_index: usize) -> Option<usize> {
        self.clusters
            .iter()
            .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
            .filter(|byte| *byte < byte_index)
            .max()
    }

    fn next_insertion_boundary(&self, byte_index: usize) -> Option<usize> {
        self.clusters
            .iter()
            .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
            .filter(|byte| *byte > byte_index)
            .min()
    }

    fn caret_position_for_movement_target(&self, target_byte: usize) -> Option<CaretPosition> {
        if let Some(cluster) = self.clusters.iter().find(|cluster| {
            (cluster.is_hard_break || cluster.is_soft_wrap_boundary)
                && cluster.byte_end == target_byte
        }) {
            return Some(CaretPosition::AfterCluster {
                cluster_byte_index: cluster.byte_start,
            });
        }

        if let Some(cluster) = self.clusters.iter().find(|cluster| {
            (cluster.is_hard_break || cluster.is_soft_wrap_boundary)
                && cluster.byte_start == target_byte
        }) {
            return Some(CaretPosition::BeforeCluster {
                cluster_byte_index: cluster.byte_start,
            });
        }

        None
    }
}
