use super::{CaretGeom, CaretPosition, TextCluster, TextLayout};
use crate::types::Vec2;

impl<G> TextLayout<G> {
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

    pub fn previous_caret_position(&self, position: CaretPosition) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(position);
        let Some(target_byte) = self.previous_insertion_boundary(byte_index) else {
            return self.caret_position_at_insertion_byte(0);
        };
        self.caret_position_for_movement_target(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }

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
