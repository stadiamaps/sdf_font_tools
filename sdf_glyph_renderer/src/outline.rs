/// A 2D point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A line segment between two points.
#[derive(Clone, Copy, Debug)]
pub struct LineSegment {
    pub p0: Point,
    pub p1: Point,
}

impl LineSegment {
    /// Computes the squared distance from a point to this line segment,
    /// along with the closest point on the segment.
    pub fn distance_sq_to_point(&self, p: Point) -> f64 {
        let dx = self.p1.x - self.p0.x;
        let dy = self.p1.y - self.p0.y;
        let len_sq = dx * dx + dy * dy;

        if len_sq < 1e-12 {
            // Degenerate segment (zero length)
            let ex = p.x - self.p0.x;
            let ey = p.y - self.p0.y;
            return ex * ex + ey * ey;
        }

        // Project point onto the line, clamping to [0, 1]
        let t = ((p.x - self.p0.x) * dx + (p.y - self.p0.y) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let closest_x = self.p0.x + t * dx;
        let closest_y = self.p0.y + t * dy;

        let ex = p.x - closest_x;
        let ey = p.y - closest_y;
        ex * ex + ey * ey
    }
}

/// A contour is a closed path made of line segments (after flattening bezier curves).
#[derive(Clone, Debug, Default)]
pub struct Contour {
    pub segments: Vec<LineSegment>,
}

/// A glyph outline consisting of one or more contours.
#[derive(Clone, Debug, Default)]
pub struct GlyphOutline {
    pub contours: Vec<Contour>,
}

/// Builder for constructing a `GlyphOutline` from move/line/curve commands.
///
/// Bezier curves are flattened to line segments using recursive subdivision.
#[derive(Debug)]
pub struct OutlineBuilder {
    outline: GlyphOutline,
    current_contour: Contour,
    current_point: Point,
    first_point: Point,
    has_move: bool,
    /// Scale factor to convert from font units to pixels.
    pub scale: f64,
    /// X offset to apply (for positioning the glyph in the SDF grid).
    pub offset_x: f64,
    /// Y offset to apply.
    pub offset_y: f64,
}

impl OutlineBuilder {
    pub fn new(scale: f64, offset_x: f64, offset_y: f64) -> Self {
        Self {
            outline: GlyphOutline::default(),
            current_contour: Contour::default(),
            current_point: Point::new(0.0, 0.0),
            first_point: Point::new(0.0, 0.0),
            has_move: false,
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Transform a point from font units to pixel coordinates.
    fn transform(&self, x: f64, y: f64) -> Point {
        Point::new(x * self.scale + self.offset_x, y * self.scale + self.offset_y)
    }

    fn close_contour(&mut self) {
        if self.has_move && !self.current_contour.segments.is_empty() {
            // Close the contour by adding a segment back to the start if needed
            let last = self.current_point;
            let first = self.first_point;
            if (last.x - first.x).abs() > 1e-6 || (last.y - first.y).abs() > 1e-6 {
                self.current_contour.segments.push(LineSegment {
                    p0: last,
                    p1: first,
                });
            }
            let contour = std::mem::take(&mut self.current_contour);
            self.outline.contours.push(contour);
        }
        self.has_move = false;
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.close_contour();
        let p = self.transform(x, y);
        self.current_point = p;
        self.first_point = p;
        self.has_move = true;
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        let p = self.transform(x, y);
        self.current_contour.segments.push(LineSegment {
            p0: self.current_point,
            p1: p,
        });
        self.current_point = p;
    }

    /// Flatten a quadratic bezier curve (p0, p1, p2) into line segments.
    pub fn quad_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let p0 = self.current_point;
        let p1 = self.transform(x1, y1);
        let p2 = self.transform(x2, y2);
        self.flatten_quad(p0, p1, p2, 0);
        self.current_point = p2;
    }

    /// Flatten a cubic bezier curve (p0, p1, p2, p3) into line segments.
    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        let p0 = self.current_point;
        let p1 = self.transform(x1, y1);
        let p2 = self.transform(x2, y2);
        let p3 = self.transform(x3, y3);
        self.flatten_cubic(p0, p1, p2, p3, 0);
        self.current_point = p3;
    }

    pub fn finish(mut self) -> GlyphOutline {
        self.close_contour();
        self.outline
    }

    /// Maximum recursion depth for curve flattening.
    const MAX_DEPTH: u32 = 8;

    /// Flatness tolerance in pixels squared.
    /// Controls how closely line segments approximate curves.
    const FLATNESS_SQ: f64 = 0.25;

    fn flatten_quad(&mut self, p0: Point, p1: Point, p2: Point, depth: u32) {
        // Check if the curve is flat enough by measuring the distance from the
        // control point to the midpoint of the line from p0 to p2.
        let mid_x = (p0.x + p2.x) * 0.5;
        let mid_y = (p0.y + p2.y) * 0.5;
        let dx = p1.x - mid_x;
        let dy = p1.y - mid_y;

        if depth >= Self::MAX_DEPTH || dx * dx + dy * dy <= Self::FLATNESS_SQ {
            self.current_contour.segments.push(LineSegment {
                p0: p0,
                p1: p2,
            });
            return;
        }

        // Subdivide at t=0.5
        let p01 = Point::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
        let p12 = Point::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
        let p012 = Point::new((p01.x + p12.x) * 0.5, (p01.y + p12.y) * 0.5);

        self.flatten_quad(p0, p01, p012, depth + 1);
        self.flatten_quad(p012, p12, p2, depth + 1);
    }

    fn flatten_cubic(&mut self, p0: Point, p1: Point, p2: Point, p3: Point, depth: u32) {
        // Check flatness: max distance of control points from the line p0→p3
        let dx = p3.x - p0.x;
        let dy = p3.y - p0.y;
        let len_sq = dx * dx + dy * dy;

        let flat_enough = if len_sq < 1e-12 {
            // Degenerate: all points close together
            let d1 = (p1.x - p0.x).powi(2) + (p1.y - p0.y).powi(2);
            let d2 = (p2.x - p0.x).powi(2) + (p2.y - p0.y).powi(2);
            d1 <= Self::FLATNESS_SQ && d2 <= Self::FLATNESS_SQ
        } else {
            // Distance of p1 and p2 from line p0→p3
            let inv_len = 1.0 / len_sq.sqrt();
            let nx = -dy * inv_len;
            let ny = dx * inv_len;
            let d1 = ((p1.x - p0.x) * nx + (p1.y - p0.y) * ny).abs();
            let d2 = ((p2.x - p0.x) * nx + (p2.y - p0.y) * ny).abs();
            d1 * d1 <= Self::FLATNESS_SQ && d2 * d2 <= Self::FLATNESS_SQ
        };

        if depth >= Self::MAX_DEPTH || flat_enough {
            self.current_contour.segments.push(LineSegment {
                p0: p0,
                p1: p3,
            });
            return;
        }

        // De Casteljau subdivision at t=0.5
        let p01 = Point::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
        let p12 = Point::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
        let p23 = Point::new((p2.x + p3.x) * 0.5, (p2.y + p3.y) * 0.5);
        let p012 = Point::new((p01.x + p12.x) * 0.5, (p01.y + p12.y) * 0.5);
        let p123 = Point::new((p12.x + p23.x) * 0.5, (p12.y + p23.y) * 0.5);
        let p0123 = Point::new((p012.x + p123.x) * 0.5, (p012.y + p123.y) * 0.5);

        self.flatten_cubic(p0, p01, p012, p0123, depth + 1);
        self.flatten_cubic(p0123, p123, p23, p3, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_segment_distance() {
        let seg = LineSegment {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(10.0, 0.0),
        };

        // Point on the segment
        assert!((seg.distance_sq_to_point(Point::new(5.0, 0.0))).abs() < 1e-10);

        // Point above the middle
        assert!((seg.distance_sq_to_point(Point::new(5.0, 3.0)) - 9.0).abs() < 1e-10);

        // Point beyond the end
        assert!((seg.distance_sq_to_point(Point::new(12.0, 0.0)) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_outline_builder_square() {
        let mut builder = OutlineBuilder::new(1.0, 0.0, 0.0);
        builder.move_to(0.0, 0.0);
        builder.line_to(10.0, 0.0);
        builder.line_to(10.0, 10.0);
        builder.line_to(0.0, 10.0);
        let outline = builder.finish();

        assert_eq!(outline.contours.len(), 1);
        // 3 explicit line_to + 1 closing segment = 4
        assert_eq!(outline.contours[0].segments.len(), 4);
    }

    #[test]
    fn test_outline_builder_quad_flattening() {
        let mut builder = OutlineBuilder::new(1.0, 0.0, 0.0);
        builder.move_to(0.0, 0.0);
        builder.quad_to(50.0, 100.0, 100.0, 0.0);
        let outline = builder.finish();

        assert_eq!(outline.contours.len(), 1);
        // Should produce multiple segments from flattening
        assert!(outline.contours[0].segments.len() > 1);
    }
}
