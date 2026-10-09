//! Arrow shape.

use super::line::PathStyle;
use super::{ShapeId, ShapeStyle, ShapeTrait, StrokeStyle};
use kurbo::{Affine, BezPath, Point, Rect, Vec2};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Visual style of one end of an arrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ArrowHeadStyle {
    None,
    Open,
    Filled,
}

impl Default for ArrowHeadStyle {
    fn default() -> Self {
        Self::Open
    }
}

/// An arrow shape (line with arrowhead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arrow {
    pub(crate) id: ShapeId,
    /// Start point.
    pub start: Point,
    /// End point (where the arrowhead points).
    pub end: Point,
    /// Intermediate points (for polylines/curves).
    #[serde(default)]
    pub intermediate_points: Vec<Point>,
    /// Path style (Direct, Flowing, Angular).
    #[serde(default)]
    pub path_style: PathStyle,
    /// Stroke style (Solid, Dashed, Dotted).
    #[serde(default)]
    pub stroke_style: StrokeStyle,
    /// Size of the arrowhead.
    pub head_size: f64,
    /// Start marker (defaults to none for older document compatibility).
    #[serde(default = "default_start_head")]
    pub start_head: ArrowHeadStyle,
    /// End marker (defaults to the original open arrowhead).
    #[serde(default)]
    pub end_head: ArrowHeadStyle,
    /// Style properties.
    pub style: ShapeStyle,
}

impl Arrow {
    /// Create a new arrow.
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            id: Uuid::new_v4(),
            start,
            end,
            intermediate_points: Vec::new(),
            path_style: PathStyle::Direct,
            stroke_style: StrokeStyle::default(),
            head_size: 15.0,
            start_head: ArrowHeadStyle::None,
            end_head: ArrowHeadStyle::Open,
            style: ShapeStyle::default(),
        }
    }

    /// Reconstruct an arrow with a specific ID (for CRDT/storage).
    pub(crate) fn reconstruct(
        id: ShapeId,
        start: Point,
        end: Point,
        intermediate_points: Vec<Point>,
        path_style: PathStyle,
        stroke_style: StrokeStyle,
        head_size: f64,
        start_head: ArrowHeadStyle,
        end_head: ArrowHeadStyle,
        style: ShapeStyle,
    ) -> Self {
        Self {
            id,
            start,
            end,
            intermediate_points,
            path_style,
            stroke_style,
            head_size,
            start_head,
            end_head,
            style,
        }
    }

    /// Create an arrow from multiple points.
    pub fn from_points(points: Vec<Point>, path_style: PathStyle) -> Self {
        let start = points.first().copied().unwrap_or(Point::ZERO);
        let end = points.last().copied().unwrap_or(Point::ZERO);
        let intermediate_points = if points.len() > 2 {
            points[1..points.len() - 1].to_vec()
        } else {
            Vec::new()
        };
        Self {
            id: Uuid::new_v4(),
            start,
            end,
            intermediate_points,
            path_style,
            stroke_style: StrokeStyle::default(),
            head_size: 15.0,
            start_head: ArrowHeadStyle::None,
            end_head: ArrowHeadStyle::Open,
            style: ShapeStyle::default(),
        }
    }

    /// Get all points including start, intermediate, and end.
    pub fn all_points(&self) -> Vec<Point> {
        let mut pts = vec![self.start];
        pts.extend(&self.intermediate_points);
        pts.push(self.end);
        pts
    }

    /// Get the direction vector (normalized).
    pub fn direction(&self) -> Vec2 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < f64::EPSILON {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(dx / len, dy / len)
        }
    }

    /// Get the length of the arrow shaft.
    pub fn length(&self) -> f64 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn path_points(&self) -> Vec<Point> {
        match self.path_style {
            PathStyle::Angular if self.intermediate_points.is_empty() => {
                let mut points = vec![self.start];
                points.extend(crate::elbow::compute_elbow_path(self.start, self.end));
                points.push(self.end);
                points
            }
            _ => self.all_points(),
        }
    }

    /// Geometry for a head at the requested end. The direction is calculated
    /// from that endpoint's adjacent path segment, so bent arrows stay aligned.
    fn head_points(&self, at_start: bool) -> [Point; 3] {
        let points = self.path_points();
        let (tip, adjacent) = if at_start {
            (points[0], points.get(1).copied().unwrap_or(self.end))
        } else {
            let last = points.len().saturating_sub(1);
            (
                points[last],
                points.get(last.saturating_sub(1)).copied().unwrap_or(self.start),
            )
        };
        let mut dx = tip.x - adjacent.x;
        let mut dy = tip.y - adjacent.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f64::EPSILON {
            dx = if at_start { -1.0 } else { 1.0 };
            dy = 0.0;
        } else {
            dx /= len;
            dy /= len;
        }
        let perp = Vec2::new(-dy, dx);
        let back = Point::new(tip.x - dx * self.head_size, tip.y - dy * self.head_size);
        [
            tip,
            Point::new(
                back.x + perp.x * self.head_size * 0.5,
                back.y + perp.y * self.head_size * 0.5,
            ),
            Point::new(
                back.x - perp.x * self.head_size * 0.5,
                back.y - perp.y * self.head_size * 0.5,
            ),
        ]
    }

    /// Closed arrowhead geometry for filled heads, used by the renderer.
    pub fn filled_head_paths(&self) -> Vec<BezPath> {
        let mut result = Vec::new();
        for (at_start, style) in [(true, self.start_head), (false, self.end_head)] {
            if style == ArrowHeadStyle::Filled {
                let [tip, left, right] = self.head_points(at_start);
                let mut path = BezPath::new();
                path.move_to(tip);
                path.line_to(left);
                path.line_to(right);
                path.close_path();
                result.push(path);
            }
        }
        result
    }
}

fn default_start_head() -> ArrowHeadStyle {
    ArrowHeadStyle::None
}

impl ShapeTrait for Arrow {
    fn id(&self) -> ShapeId {
        self.id
    }

    fn bounds(&self) -> Rect {
        let points = self.path_points();
        let mut min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let mut min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let mut max_x = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let mut max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        for (at_start, style) in [(true, self.start_head), (false, self.end_head)] {
            if style != ArrowHeadStyle::None {
                for point in self.head_points(at_start) {
                    min_x = min_x.min(point.x);
                    min_y = min_y.min(point.y);
                    max_x = max_x.max(point.x);
                    max_y = max_y.max(point.y);
                }
            }
        }

        Rect::new(min_x, min_y, max_x, max_y)
    }

    fn hit_test(&self, point: Point, tolerance: f64) -> bool {
        // Check all line segments
        let points = self.path_points();
        if points.len() >= 2 {
            let dist = super::point_to_polyline_dist(point, &points);
            if dist <= tolerance + self.style.stroke_width / 2.0 {
                return true;
            }
        }

        // Point in triangle test
        fn sign(p1: Point, p2: Point, p3: Point) -> f64 {
            (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
        }

        [(true, self.start_head), (false, self.end_head)]
            .into_iter()
            .any(|(at_start, style)| {
                if style == ArrowHeadStyle::None {
                    return false;
                }
                let [tip, left, right] = self.head_points(at_start);
                let d1 = sign(point, tip, left);
                let d2 = sign(point, left, right);
                let d3 = sign(point, right, tip);
                let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
                let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
                !(has_neg && has_pos)
            })
    }

    fn to_path(&self) -> BezPath {
        let mut path = BezPath::new();

        if self.start == self.end {
            return path;
        }

        // Get points to draw
        let points = self.path_points();

        if points.len() < 2 {
            return path;
        }

        // Shaft
        path.move_to(points[0]);

        match self.path_style {
            PathStyle::Direct | PathStyle::Angular => {
                for p in &points[1..] {
                    path.line_to(*p);
                }
            }
            PathStyle::Flowing => {
                // Catmull-Rom spline
                let tension = 0.5;
                for i in 0..points.len() - 1 {
                    let p0 = points[if i == 0 { 0 } else { i - 1 }];
                    let p1 = points[i];
                    let p2 = points[i + 1];
                    let p3 = points[if i + 2 >= points.len() {
                        points.len() - 1
                    } else {
                        i + 2
                    }];

                    let t1x = (p2.x - p0.x) * tension;
                    let t1y = (p2.y - p0.y) * tension;
                    let t2x = (p3.x - p1.x) * tension;
                    let t2y = (p3.y - p1.y) * tension;

                    let cp1 = Point::new(p1.x + t1x / 3.0, p1.y + t1y / 3.0);
                    let cp2 = Point::new(p2.x - t2x / 3.0, p2.y - t2y / 3.0);

                    path.curve_to(cp1, cp2, p2);
                }
            }
        }

        for (at_start, style) in [(true, self.start_head), (false, self.end_head)] {
            if style == ArrowHeadStyle::Open {
                let [tip, left, right] = self.head_points(at_start);
                path.move_to(tip);
                path.line_to(left);
                path.move_to(tip);
                path.line_to(right);
            }
        }

        path
    }

    fn style(&self) -> &ShapeStyle {
        &self.style
    }

    fn style_mut(&mut self) -> &mut ShapeStyle {
        &mut self.style
    }

    fn transform(&mut self, affine: Affine) {
        self.start = affine * self.start;
        self.end = affine * self.end;
        for p in &mut self.intermediate_points {
            *p = affine * *p;
        }
        // Scale head size based on transform
        let scale = affine.as_coeffs();
        // Use each transformed basis vector's length so pure rotations keep
        // the arrowhead size unchanged.
        let x_scale = scale[0].hypot(scale[1]);
        let y_scale = scale[2].hypot(scale[3]);
        self.head_size *= (x_scale + y_scale) / 2.0;
    }

    fn clone_box(&self) -> Box<dyn ShapeTrait + Send + Sync> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_creation() {
        let arrow = Arrow::new(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        assert!((arrow.length() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_direction() {
        let arrow = Arrow::new(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        let dir = arrow.direction();
        assert!((dir.x - 1.0).abs() < f64::EPSILON);
        assert!(dir.y.abs() < f64::EPSILON);
    }

    #[test]
    fn test_hit_test_shaft() {
        let arrow = Arrow::new(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        assert!(arrow.hit_test(Point::new(50.0, 0.0), 5.0));
    }

    #[test]
    fn test_hit_test_head() {
        let arrow = Arrow::new(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        assert!(arrow.hit_test(Point::new(100.0, 0.0), 1.0));
    }

    #[test]
    fn arrow_heads_round_trip_and_old_documents_keep_open_end() {
        let mut arrow = Arrow::new(Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        arrow.start_head = ArrowHeadStyle::Filled;
        arrow.end_head = ArrowHeadStyle::None;
        let json = serde_json::to_string(&arrow).expect("serialize arrow");
        let decoded: Arrow = serde_json::from_str(&json).expect("deserialize arrow");
        assert_eq!(decoded.start_head, ArrowHeadStyle::Filled);
        assert_eq!(decoded.end_head, ArrowHeadStyle::None);
        assert_eq!(decoded.filled_head_paths().len(), 1);

        let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
        old.as_object_mut().unwrap().remove("start_head");
        old.as_object_mut().unwrap().remove("end_head");
        let decoded: Arrow = serde_json::from_value(old).expect("old arrow JSON");
        assert_eq!(decoded.start_head, ArrowHeadStyle::None);
        assert_eq!(decoded.end_head, ArrowHeadStyle::Open);
    }
}
