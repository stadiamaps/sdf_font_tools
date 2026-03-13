use rstar::{RTree, RTreeObject, AABB};

use crate::outline::{GlyphOutline, LineSegment, Point};

/// Wrapper for inserting line segments into an R-tree with bounding boxes.
#[derive(Clone, Debug)]
struct IndexedSegment {
    segment: LineSegment,
}

impl RTreeObject for IndexedSegment {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let min_x = self.segment.p0.x.min(self.segment.p1.x);
        let min_y = self.segment.p0.y.min(self.segment.p1.y);
        let max_x = self.segment.p0.x.max(self.segment.p1.x);
        let max_y = self.segment.p0.y.max(self.segment.p1.y);
        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

impl GlyphOutline {
    /// Render a signed distance field from this glyph outline.
    ///
    /// The output grid has dimensions `(width + 2*buffer) x (height + 2*buffer)`.
    /// Distances are normalized by `radius`, producing values in the range [-1.0, 1.0].
    /// Positive values are outside the glyph, negative values are inside.
    ///
    /// This matches the output contract of [`BitmapGlyph::render_sdf`](crate::BitmapGlyph::render_sdf).
    pub fn render_sdf(
        &self,
        width: usize,
        height: usize,
        buffer: usize,
        radius: usize,
    ) -> Vec<f64> {
        let buffered_width = width + buffer * 2;
        let buffered_height = height + buffer * 2;
        let total_pixels = buffered_width * buffered_height;

        if total_pixels == 0 || self.contours.is_empty() {
            return vec![1.0; total_pixels];
        }

        // Collect all segments and build spatial index
        let indexed_segments: Vec<IndexedSegment> = self
            .contours
            .iter()
            .flat_map(|c| c.segments.iter())
            .map(|s| IndexedSegment { segment: *s })
            .collect();

        let tree = RTree::bulk_load(indexed_segments);

        let radius_f64 = radius as f64;
        let search_radius = radius_f64 + 1.0; // Slightly larger to avoid boundary issues

        let mut result = Vec::with_capacity(total_pixels);

        for y in 0..buffered_height {
            for x in 0..buffered_width {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                let point = Point::new(px, py);

                // Find minimum squared distance using R-tree spatial query
                let search_envelope = AABB::from_corners(
                    [px - search_radius, py - search_radius],
                    [px + search_radius, py + search_radius],
                );

                let mut min_dist_sq = f64::MAX;
                for indexed in tree.locate_in_envelope(&search_envelope) {
                    let dist_sq = indexed.segment.distance_sq_to_point(point);
                    if dist_sq < min_dist_sq {
                        min_dist_sq = dist_sq;
                    }
                }

                // If no segments found within search radius, search all segments
                if min_dist_sq == f64::MAX {
                    for contour in &self.contours {
                        for segment in &contour.segments {
                            let dist_sq = segment.distance_sq_to_point(point);
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                            }
                        }
                    }
                }

                let min_dist = min_dist_sq.sqrt();

                // Determine inside/outside using winding number
                let inside = self.winding_number(point) != 0;

                let signed_dist = if inside { -min_dist } else { min_dist };

                // Normalize by radius and clamp to [-1, 1]
                result.push((signed_dist / radius_f64).clamp(-1.0, 1.0));
            }
        }

        result
    }

    /// Compute the winding number of a point relative to the outline.
    ///
    /// Non-zero winding number means the point is inside the glyph.
    /// Uses the standard ray-casting approach (horizontal ray to the right).
    fn winding_number(&self, point: Point) -> i32 {
        let mut winding = 0i32;

        for contour in &self.contours {
            for seg in &contour.segments {
                let p0 = seg.p0;
                let p1 = seg.p1;

                if p0.y <= point.y {
                    if p1.y > point.y {
                        // Upward crossing
                        let cross = (p1.x - p0.x) * (point.y - p0.y)
                            - (point.x - p0.x) * (p1.y - p0.y);
                        if cross > 0.0 {
                            winding += 1;
                        }
                    }
                } else if p1.y <= point.y {
                    // Downward crossing
                    let cross = (p1.x - p0.x) * (point.y - p0.y)
                        - (point.x - p0.x) * (p1.y - p0.y);
                    if cross < 0.0 {
                        winding -= 1;
                    }
                }
            }
        }

        winding
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::{Contour, LineSegment, Point};

    /// Create a simple square outline for testing.
    fn square_outline(x: f64, y: f64, size: f64) -> GlyphOutline {
        let p0 = Point::new(x, y);
        let p1 = Point::new(x + size, y);
        let p2 = Point::new(x + size, y + size);
        let p3 = Point::new(x, y + size);

        GlyphOutline {
            contours: vec![Contour {
                segments: vec![
                    LineSegment { p0, p1 },
                    LineSegment { p0: p1, p1: p2 },
                    LineSegment { p0: p2, p1: p3 },
                    LineSegment { p0: p3, p1: p0 },
                ],
            }],
        }
    }

    #[test]
    fn test_winding_number_inside() {
        let outline = square_outline(2.0, 2.0, 6.0);
        assert_ne!(outline.winding_number(Point::new(5.0, 5.0)), 0);
    }

    #[test]
    fn test_winding_number_outside() {
        let outline = square_outline(2.0, 2.0, 6.0);
        assert_eq!(outline.winding_number(Point::new(0.0, 0.0)), 0);
    }

    #[test]
    fn test_render_sdf_basic() {
        // 6x6 square at position (3,3) with buffer=3, radius=8
        let outline = square_outline(3.0, 3.0, 6.0);
        let sdf = outline.render_sdf(6, 6, 3, 8);

        let buffered_width = 12;
        let buffered_height = 12;
        assert_eq!(sdf.len(), buffered_width * buffered_height);

        // Point at center (6, 6) should be inside (negative)
        let center_idx = 6 * buffered_width + 6;
        assert!(sdf[center_idx] < 0.0, "Center should be inside (negative)");

        // Corner at (0, 0) should be outside (positive)
        assert!(sdf[0] > 0.0, "Corner should be outside (positive)");
    }

    #[test]
    fn test_render_sdf_empty() {
        let outline = GlyphOutline::default();
        let sdf = outline.render_sdf(4, 4, 2, 8);
        // All outside
        assert!(sdf.iter().all(|&v| v == 1.0));
    }

    #[test]
    fn test_render_sdf_values_normalized() {
        let outline = square_outline(3.0, 3.0, 6.0);
        let sdf = outline.render_sdf(6, 6, 3, 8);

        // All values should be in [-1, 1]
        assert!(sdf.iter().all(|&v| (-1.0..=1.0).contains(&v)));
    }
}
