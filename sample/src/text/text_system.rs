//! # Framewise Text System (Swash Migration)
//!
//! ## Rationale for choosing Swash
//!
//! Originally, Framewise used `fontdue` for text rendering. While `fontdue` is extremely fast and
//! lightweight, it has several major limitations that prevent a high-end, premium UI look:
//! - **No Variable Font Support:** It cannot dynamically adapt font weights, widths, or optical sizes,
//!   which is critical for clean typography across headings and micro-labels.
//! - **No Subpixel Positioning:** It does not support rendering glyphs at fractional offsets, causing
//!   either blurry text (with bilinear filtering) or uneven letter spacing/kerning (with pixel snapping).
//! - **No Autohinting:** It lacks sophisticated hinting routines, making text shapes look slightly soft
//!   and uneven at small pixel heights.
//!
//! We evaluated alternative pure-Rust options:
//! - **`ab_glyph`:** Good for basic GUIs, but has limited shaping capabilities (no ligatures or OpenType features)
//!   and a simpler scaler.
//! - **`cosmic-text`:** A massive library that handles layout, shaping, bi-directional text, and fallbacks.
//!   However, it is too heavy and would require discarding our custom line-wrapping, spacing, and layout policies,
//!   which are optimized specifically for Framewise.
//! - **`swash` (Selected):** Provides state-of-the-art OpenType shaping, variable font axis matching,
//!   and a premium scaler with autohinting and fractional subpixel rendering. It lets us retain full control
//!   over Framewise's custom line-wrapping and manual/wrapped layout algorithms.
//!
//! ## Design Selection
//! - **Grayscale Anti-Aliasing (Alpha coverage):** We use grayscale AA rather than RGB subpixel rendering.
//!   Grayscale is standard across modern operating systems (iOS, Android, macOS, and modern Windows Store apps)
//!   because it is robust under high-DPI scaling, transformations, subpixel layouts, and OLED/rotation changes.
//! - **Horizontal Subpixel Snapping (4 Bins):** Snaps glyphs to 4 horizontal fractional bins (0.0, 0.25, 0.5, 0.75).
//!   Since vertical baselines in standard UIs are always snapped to integer pixels, we do not perform vertical
//!   subpixel positioning, conserving texture atlas space.
//! - **Nearest-Neighbor Filtering:** In `renderer.rs`, we sample the font atlas using Nearest-neighbor filtering.
//!   By rendering pixel-aligned quads with pre-shifted subpixel glyphs, we map every font pixel 1-to-1 with screen
//!   pixels for maximum crispness.

use crate::text::types::{CachedLayout, GlyphInfo, GlyphKey, GlyphPosition, LineRec, TextCluster};
use framewise::{
    CaretGeom, CaretPosition, FontId, LineHeight, Rect, TextBounds, TextFlow, TextLayout,
    TextMetrics, TextSystem, Vec2,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutKey {
    pub text: String,
    pub size_bits: u32,
    pub font_id: u16,
    pub weight: u16,
    pub flow: TextFlow,
    pub max_w_bits: Option<u32>,
    pub max_h_bits: Option<u32>,
    pub absolute_x_bits: Option<u32>,
    pub letter_spacing_bits: i32,
    pub line_height_val: Option<u32>,
}

type LayoutCacheValue = (
    Vec<GlyphPosition>,
    Vec<TextCluster>,
    Vec<LineRec>,
    TextMetrics,
);
use swash::scale::ScaleContext;
use swash::shape::ShapeContext;
use swash::FontRef;

pub struct SampleTextSystem {
    pub fonts: Vec<FontRef<'static>>,
    pub font_opsz_ranges: Vec<(f32, f32)>, // (min, max) for each font's opsz axis
    pub font_weights: Vec<u16>, // Current weight setting for each font (for variable fonts)
    pub font_has_wght: Vec<bool>, // Whether each font has a wght axis
    pub font_has_opsz: Vec<bool>, // Whether each font has an opsz axis
    pub shape_context: ShapeContext,
    pub scale_context: ScaleContext,
    pub runs: Vec<CachedLayout>,
    pub layout_cache: HashMap<LayoutKey, LayoutCacheValue>,

    // Atlas data
    pub glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    pub atlas_data: Vec<u8>,
    pub atlas_size: u32,

    // Simple shelf allocator
    pub current_x: u32,
    pub current_y: u32,
    pub row_height: u32,

    pub atlas_dirty: bool,
}

impl SampleTextSystem {
    pub fn new() -> Self {
        let jetbrains_mono_data =
            include_bytes!("../../assets/JetBrains_Mono/JetBrainsMono-VariableFont_wght.ttf")
                as &[u8];
        let jetbrains_mono = FontRef::from_index(jetbrains_mono_data, 0)
            .expect("failed to load JetBrainsMono variable font");

        // Load Inter variable font
        let inter_data =
            include_bytes!("../../assets/Inter/Inter-VariableFont_opsz,wght.ttf") as &[u8];
        let inter = FontRef::from_index(inter_data, 0).expect("failed to load Inter variable font");

        // Inter Tight - specialised for "Hero headings, landing pages, article titles". Slightly different!
        let inter_tight_data =
            include_bytes!("../../assets/Inter_Tight/InterTight-VariableFont_wght.ttf") as &[u8];
        let inter_tight = FontRef::from_index(inter_tight_data, 0)
            .expect("failed to load Inter Tight variable font");

        let fonts = vec![jetbrains_mono, inter, inter_tight];

        // Detect supported variation axes for each font
        let mut font_has_wght = Vec::new();
        let mut font_has_opsz = Vec::new();
        for font in &fonts {
            let mut has_wght = false;
            let mut has_opsz = false;
            for var in font.variations() {
                let tag = var.tag();
                if tag == 0x77676874 {
                    // 'wght'
                    has_wght = true;
                }
                if tag == 0x6F70737A {
                    // 'opsz'
                    has_opsz = true;
                }
            }
            font_has_wght.push(has_wght);
            font_has_opsz.push(has_opsz);
        }

        // Extract opsz range from each font
        let mut font_opsz_ranges = Vec::new();

        // JetBrainsMono has no opsz axis
        font_opsz_ranges.push((0.0, 0.0));

        // Inter has opsz axis - extract its range
        let opsz_range = {
            let variations = fonts[1].variations();
            let mut range = (14.0, 32.0); // fallback to documented range
            for var in variations {
                let tag = var.tag();
                // 'opsz' = 0x6F70737A
                if tag == 0x6F70737A {
                    // Note: swash Variation doesn't expose min/max directly in all versions
                    // Use documented Inter range for now
                    range = (14.0, 32.0);
                    break;
                }
            }
            range
        };
        font_opsz_ranges.push(opsz_range);

        // Inter Tight has no opsz axis
        font_opsz_ranges.push((0.0, 0.0));

        let atlas_size = 1024;
        Self {
            fonts,
            font_opsz_ranges,
            font_weights: vec![400, 400, 400], // Default weights for each font
            font_has_wght,
            font_has_opsz,
            shape_context: ShapeContext::new(),
            scale_context: ScaleContext::new(),
            runs: Vec::new(),
            layout_cache: HashMap::new(),
            glyph_cache: HashMap::new(),
            atlas_data: vec![0; (atlas_size * atlas_size) as usize],
            atlas_size,
            current_x: 0,
            current_y: 0,
            row_height: 0,
            atlas_dirty: false,
        }
    }

    /// Set the weight to use for a specific font. Used to control variable font weight axis.
    pub fn set_font_weight(&mut self, font_id: FontId, weight: u16) {
        if (font_id.0 as usize) < self.font_weights.len() {
            self.font_weights[font_id.0 as usize] = weight;
        }
    }

    pub fn begin_frame(&mut self) {
        self.runs.clear();
        self.atlas_dirty = false;
    }
}

impl TextSystem for SampleTextSystem {
    fn measure(
        &mut self,
        text: &str,
        style: framewise::TextStyle,
        bounds: TextBounds,
    ) -> TextMetrics {
        let key = LayoutKey {
            text: text.to_string(),
            size_bits: (style.size * 100.0) as u32,
            font_id: style.font.0,
            weight: style.weight,
            flow: style.flow,
            max_w_bits: bounds.max_width.map(|w| (w * 100.0) as u32),
            max_h_bits: bounds.max_height.map(|h| (h * 100.0) as u32),
            absolute_x_bits: None,
            letter_spacing_bits: (style.letter_spacing * 10000.0) as i32,
            line_height_val: match style.line_height {
                LineHeight::Normal => None,
                LineHeight::Relative(mult) => Some((mult * 1000.0) as u32),
            },
        };

        if let Some((_, _, _, metrics)) = self.layout_cache.get(&key) {
            return metrics.clone();
        }

        // Temporarily set the weight for this font before shaping
        let old_weight = self
            .font_weights
            .get(style.font.0 as usize)
            .copied()
            .unwrap_or(400);
        self.set_font_weight(style.font, style.weight);

        let (glyphs, clusters, lines, metrics) =
            self.shape_internal(text, style, bounds.max_width, bounds.max_height, None);

        // Restore old weight
        self.set_font_weight(style.font, old_weight);

        // Insert into cache, preventing unbounded growth
        if self.layout_cache.len() >= 2000 {
            self.layout_cache.clear();
        }
        self.layout_cache
            .insert(key, (glyphs, clusters, lines, metrics.clone()));

        metrics
    }

    fn prepare(&mut self, text: &str, style: framewise::TextStyle, rect: Rect) -> TextLayout {
        let key = LayoutKey {
            text: text.to_string(),
            size_bits: (style.size * 100.0) as u32,
            font_id: style.font.0,
            weight: style.weight,
            flow: style.flow,
            max_w_bits: Some((rect.w * 100.0) as u32),
            max_h_bits: Some((rect.h * 100.0) as u32),
            absolute_x_bits: Some((rect.x * 100.0) as u32),
            letter_spacing_bits: (style.letter_spacing * 10000.0) as i32,
            line_height_val: match style.line_height {
                LineHeight::Normal => None,
                LineHeight::Relative(mult) => Some((mult * 1000.0) as u32),
            },
        };

        let cached = self
            .layout_cache
            .get(&key)
            .map(|(glyphs, clusters, lines, metrics)| {
                (
                    glyphs.clone(),
                    clusters.clone(),
                    lines.clone(),
                    metrics.clone(),
                )
            });

        if let Some((glyphs, clusters, lines, metrics)) = cached {
            // Populate atlas for any cached glyphs that are needed
            let opsz = self.opsz_for_size(style.size, style.font);
            for g in &glyphs {
                let key = GlyphKey {
                    font_id: style.font.0,
                    glyph_index: g.key.glyph_index,
                    size: (g.key.px * 10.0) as u32,
                    subpixel_x: g.subpixel_x,
                    weight: style.weight,
                    opsz: opsz as u16,
                };
                self.ensure_glyph(key);
            }

            let handle_id = self.runs.len();
            self.runs.push(CachedLayout {
                font_id: style.font,
                glyphs,
                clusters,
                lines,
            });

            return TextLayout {
                handle: framewise::TextHandle(handle_id),
                metrics,
            };
        }

        // Temporarily set the weight for this font before shaping
        let old_weight = self
            .font_weights
            .get(style.font.0 as usize)
            .copied()
            .unwrap_or(400);
        self.set_font_weight(style.font, style.weight);

        // Pass the absolute X coordinate (rect.x) to internal shaper to compute correct subpixel offsets
        let (glyphs, clusters, lines, metrics) =
            self.shape_internal(text, style, Some(rect.w), Some(rect.h), Some(rect.x));

        let opsz = self.opsz_for_size(style.size, style.font);

        for g in &glyphs {
            let key = GlyphKey {
                font_id: style.font.0,
                glyph_index: g.key.glyph_index,
                size: (g.key.px * 10.0) as u32,
                subpixel_x: g.subpixel_x,
                weight: style.weight,
                opsz: opsz as u16,
            };
            self.ensure_glyph(key);
        }

        let handle_id = self.runs.len();
        self.runs.push(CachedLayout {
            font_id: style.font,
            glyphs: glyphs.clone(),
            clusters: clusters.clone(),
            lines: lines.clone(),
        });

        // Restore old weight
        self.set_font_weight(style.font, old_weight);

        // Insert into cache, preventing unbounded growth
        if self.layout_cache.len() >= 2000 {
            self.layout_cache.clear();
        }
        self.layout_cache
            .insert(key, (glyphs, clusters, lines, metrics.clone()));

        TextLayout {
            handle: framewise::TextHandle(handle_id),
            metrics,
        }
    }

    fn caret_geom(&self, handle: framewise::TextHandle, position: CaretPosition) -> CaretGeom {
        let run = &self.runs[handle.0];
        let Some((cluster_idx, cluster)) = find_caret_cluster(run, position) else {
            let line = run
                .lines
                .first()
                .expect("a prepared run always has at least one line");
            return CaretGeom {
                x: line.logical_x,
                y_top: line.y_top,
                height: line.height,
            };
        };

        let line_idx = line_index_for_cluster(run, cluster_idx).unwrap_or(0);
        let line = &run.lines[line_idx];
        let (x, y_top, height) = match position {
            CaretPosition::BeforeCluster { .. } => (cluster.x, line.y_top, line.height),
            CaretPosition::AfterCluster { .. }
                if cluster.is_hard_break || cluster.is_soft_wrap_boundary =>
            {
                let next_line = run.lines.get(line_idx + 1).unwrap_or(line);
                let next_clusters = &run.clusters[next_line.cluster_start..next_line.cluster_end];
                let next_x = if next_clusters.is_empty() {
                    next_line.logical_x
                } else {
                    line_start_x(next_clusters)
                };
                (next_x, next_line.y_top, next_line.height)
            }
            CaretPosition::AfterCluster { .. } => {
                (cluster.x + cluster.advance, line.y_top, line.height)
            }
            CaretPosition::EmptyText => unreachable!("handled by missing-cluster branch"),
        };

        CaretGeom { x, y_top, height }
    }

    fn hit_test_caret(&self, handle: framewise::TextHandle, pos: Vec2) -> CaretPosition {
        let run = &self.runs[handle.0];

        // Resolve the line by Y (clamp above/below to first/last).
        let line_idx = run
            .lines
            .iter()
            .position(|l| pos.y < l.y_top + l.height)
            .unwrap_or_else(|| run.lines.len().saturating_sub(1));
        let line = &run.lines[line_idx];

        let clusters = &run.clusters[line.cluster_start..line.cluster_end];
        if clusters.is_empty() {
            return empty_line_caret_position(run, line_idx);
        }
        for cluster in clusters {
            let mid = cluster.x + cluster.advance * 0.5;
            if pos.x < mid {
                return CaretPosition::BeforeCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }
        }
        // The point is to the right of every cluster. If the line ends with a
        // hard newline or collapsed soft-wrap boundary space, clamp to that
        // source character so clicking in the right margin never jumps the
        // visual caret anchor to the next line.
        match clusters.last() {
            Some(last) if last.is_hard_break || last.is_soft_wrap_boundary => {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: last.byte_start,
                }
            }
            Some(last) => CaretPosition::AfterCluster {
                cluster_byte_index: last.byte_start,
            },
            None => empty_line_caret_position(run, line_idx),
        }
    }

    fn caret_insertion_byte(
        &self,
        handle: framewise::TextHandle,
        position: CaretPosition,
    ) -> usize {
        let run = &self.runs[handle.0];
        match find_caret_cluster(run, position) {
            Some((_, cluster)) => match position {
                CaretPosition::BeforeCluster { .. } => cluster.byte_start,
                CaretPosition::AfterCluster { .. } => cluster.byte_end,
                CaretPosition::EmptyText => 0,
            },
            None => 0,
        }
    }

    fn caret_position_at_insertion_byte(
        &self,
        handle: framewise::TextHandle,
        byte_index: usize,
    ) -> CaretPosition {
        let run = &self.runs[handle.0];
        if run.clusters.is_empty() {
            return CaretPosition::EmptyText;
        }

        for (idx, cluster) in run.clusters.iter().enumerate() {
            if byte_index <= cluster.byte_start || byte_index < cluster.byte_end {
                return CaretPosition::BeforeCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }

            if byte_index == cluster.byte_end {
                if let Some(next) = run.clusters.get(idx + 1) {
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

        let last = run.clusters.last().expect("clusters is non-empty");
        CaretPosition::AfterCluster {
            cluster_byte_index: last.byte_start,
        }
    }

    fn previous_caret_position(
        &self,
        handle: framewise::TextHandle,
        position: CaretPosition,
    ) -> CaretPosition {
        let run = &self.runs[handle.0];
        let byte_index = self.caret_insertion_byte(handle, position);
        let Some(target_byte) = previous_insertion_boundary(run, byte_index) else {
            return self.caret_position_at_insertion_byte(handle, 0);
        };
        caret_position_for_movement_target(run, target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(handle, target_byte))
    }

    fn next_caret_position(
        &self,
        handle: framewise::TextHandle,
        position: CaretPosition,
    ) -> CaretPosition {
        let run = &self.runs[handle.0];
        let byte_index = self.caret_insertion_byte(handle, position);
        let Some(target_byte) = next_insertion_boundary(run, byte_index) else {
            return run
                .clusters
                .last()
                .map(|cluster| CaretPosition::AfterCluster {
                    cluster_byte_index: cluster.byte_start,
                })
                .unwrap_or(CaretPosition::EmptyText);
        };
        caret_position_for_movement_target(run, target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(handle, target_byte))
    }

    fn hit_test_cluster(&self, handle: framewise::TextHandle, pos: Vec2) -> usize {
        let run = &self.runs[handle.0];

        // Resolve the line by Y (clamp above/below to first/last).
        let line = run
            .lines
            .iter()
            .find(|l| pos.y < l.y_top + l.height)
            .unwrap_or_else(|| run.lines.last().expect("at least one line"));

        let clusters = &run.clusters[line.cluster_start..line.cluster_end];
        if clusters.is_empty() {
            return line.byte_start;
        }
        for cluster in clusters {
            if pos.x < cluster.x + cluster.advance {
                return cluster.byte_start;
            }
        }
        clusters
            .last()
            .map(|c| c.byte_start)
            .unwrap_or(line.byte_start)
    }
}

fn line_start_x(clusters: &[crate::text::types::TextCluster]) -> f32 {
    clusters.first().map(|cluster| cluster.x).unwrap_or(0.0)
}

fn find_caret_cluster(
    run: &CachedLayout,
    position: CaretPosition,
) -> Option<(usize, &TextCluster)> {
    let cluster_byte_index = match position {
        CaretPosition::BeforeCluster { cluster_byte_index }
        | CaretPosition::AfterCluster { cluster_byte_index } => cluster_byte_index,
        CaretPosition::EmptyText => return None,
    };

    run.clusters
        .iter()
        .enumerate()
        .find(|(_, cluster)| cluster.byte_start == cluster_byte_index)
        .or_else(|| {
            run.clusters.iter().enumerate().find(|(_, cluster)| {
                cluster_byte_index <= cluster.byte_start || cluster_byte_index < cluster.byte_end
            })
        })
        .or_else(|| run.clusters.iter().enumerate().next_back())
}

fn line_index_for_cluster(run: &CachedLayout, cluster_idx: usize) -> Option<usize> {
    run.lines
        .iter()
        .position(|line| cluster_idx >= line.cluster_start && cluster_idx < line.cluster_end)
}

fn empty_line_caret_position(run: &CachedLayout, line_idx: usize) -> CaretPosition {
    if run.clusters.is_empty() {
        return CaretPosition::EmptyText;
    }

    run.lines
        .get(..line_idx)
        .and_then(|lines| {
            lines
                .iter()
                .rev()
                .find(|line| line.cluster_end > line.cluster_start)
        })
        .and_then(|line| run.clusters.get(line.cluster_end - 1))
        .map(|cluster| CaretPosition::AfterCluster {
            cluster_byte_index: cluster.byte_start,
        })
        .unwrap_or(CaretPosition::EmptyText)
}

fn previous_insertion_boundary(run: &CachedLayout, byte_index: usize) -> Option<usize> {
    run.clusters
        .iter()
        .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
        .filter(|byte| *byte < byte_index)
        .max()
}

fn next_insertion_boundary(run: &CachedLayout, byte_index: usize) -> Option<usize> {
    run.clusters
        .iter()
        .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
        .filter(|byte| *byte > byte_index)
        .min()
}

fn caret_position_for_movement_target(
    run: &CachedLayout,
    target_byte: usize,
) -> Option<CaretPosition> {
    if let Some(cluster) = run
        .clusters
        .iter()
        .find(|cluster| is_visual_boundary_cluster(cluster) && cluster.byte_end == target_byte)
    {
        return Some(CaretPosition::AfterCluster {
            cluster_byte_index: cluster.byte_start,
        });
    }

    if let Some(cluster) = run
        .clusters
        .iter()
        .find(|cluster| is_visual_boundary_cluster(cluster) && cluster.byte_start == target_byte)
    {
        return Some(CaretPosition::BeforeCluster {
            cluster_byte_index: cluster.byte_start,
        });
    }

    None
}

fn is_visual_boundary_cluster(cluster: &TextCluster) -> bool {
    cluster.is_hard_break || cluster.is_soft_wrap_boundary
}
