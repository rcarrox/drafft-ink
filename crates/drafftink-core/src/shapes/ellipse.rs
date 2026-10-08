//! Ellipse shape.

use super::{SerializableColor, ShapeId, ShapeStyle, ShapeTrait};
use kurbo::{
    Affine, BezPath, Ellipse as KurboEllipse, ParamCurveNearest, Point, Rect, Shape as KurboShape,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Geometric variants share the same box, handles and document representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GeometryKind {
    #[default]
    Ellipse,
    Triangle,
    Parallelogram,
    Trapezoid,
    Diamond,
}
impl GeometryKind {
    pub const ALL: [Self; 5] = [
        Self::Ellipse,
        Self::Triangle,
        Self::Parallelogram,
        Self::Trapezoid,
        Self::Diamond,
    ];
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ellipse => "Ellipse",
            Self::Triangle => "Triangle",
            Self::Parallelogram => "Parallélogramme",
            Self::Trapezoid => "Trapèze",
            Self::Diamond => "Losange",
        }
    }
}
/// An ellipse or polygon contained in its local box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ellipse {
    pub(crate) id: ShapeId,
    /// Center point.
    pub center: Point,
    #[serde(default)]
    pub geometry: GeometryKind,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    /// Horizontal radius.
    pub radius_x: f64,
    /// Vertical radius.
    pub radius_y: f64,
    /// Rotation angle in radians (around center).
    #[serde(default)]
    pub rotation: f64,
    /// Style properties.
    pub style: ShapeStyle,
}

impl Ellipse {
    /// Create a new ellipse.
    pub fn new(center: Point, radius_x: f64, radius_y: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            center,
            geometry: GeometryKind::Ellipse,
            flip_x: false,
            flip_y: false,
            radius_x,
            radius_y,
            rotation: 0.0,
            style: ShapeStyle::default(),
        }
    }

    /// Reconstruct an ellipse with a specific ID (for CRDT/storage).
    pub(crate) fn reconstruct(
        id: ShapeId,
        center: Point,
        radius_x: f64,
        radius_y: f64,
        rotation: f64,
        style: ShapeStyle,
    ) -> Self {
        Self {
            id,
            center,
            geometry: GeometryKind::Ellipse,
            flip_x: false,
            flip_y: false,
            radius_x,
            radius_y,
            rotation,
            style,
        }
    }

    /// Create a circle.
    pub fn circle(center: Point, radius: f64) -> Self {
        Self::new(center, radius, radius)
    }

    /// Create an ellipse from a bounding rectangle.
    pub fn from_rect(rect: Rect) -> Self {
        Self::new(rect.center(), rect.width() / 2.0, rect.height() / 2.0)
    }

    /// Get as a kurbo Ellipse.
    pub fn as_kurbo(&self) -> KurboEllipse {
        KurboEllipse::new(self.center, (self.radius_x, self.radius_y), 0.0)
    }
}

impl ShapeTrait for Ellipse {
    fn id(&self) -> ShapeId {
        self.id
    }

    fn bounds(&self) -> Rect {
        Rect::new(
            self.center.x - self.radius_x,
            self.center.y - self.radius_y,
            self.center.x + self.radius_x,
            self.center.y + self.radius_y,
        )
    }

    fn hit_test(&self, point: Point, tolerance: f64) -> bool {
        if self.geometry != GeometryKind::Ellipse {
            let path = self.to_path();
            if self.style.fill_color.is_some() && path.winding(point) != 0 {
                return true;
            }
            let limit = tolerance + self.style.stroke_width / 2.0;
            return path
                .segments()
                .any(|s| s.nearest(point, 0.01).distance_sq <= limit * limit);
        }
        let half_sw = self.style.stroke_width / 2.0;
        let dx_outer = (point.x - self.center.x) / (self.radius_x + tolerance + half_sw);
        let dy_outer = (point.y - self.center.y) / (self.radius_y + tolerance + half_sw);
        let outside_outer = dx_outer * dx_outer + dy_outer * dy_outer > 1.0;
        if outside_outer {
            return false;
        }
        if self.style.fill_color.is_some() {
            return true;
        }
        // Outline only: reject if inside inner ellipse
        let inner_rx = (self.radius_x - tolerance - half_sw).max(0.0);
        let inner_ry = (self.radius_y - tolerance - half_sw).max(0.0);
        if inner_rx < f64::EPSILON || inner_ry < f64::EPSILON {
            return true;
        }
        let dx_inner = (point.x - self.center.x) / inner_rx;
        let dy_inner = (point.y - self.center.y) / inner_ry;
        dx_inner * dx_inner + dy_inner * dy_inner > 1.0
    }

    fn to_path(&self) -> BezPath {
        if self.geometry == GeometryKind::Ellipse {
            return self.as_kurbo().to_path(0.1);
        }
        let points: &[(f64, f64)] = match self.geometry {
            GeometryKind::Triangle => &[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)],
            GeometryKind::Parallelogram => &[(0.25, 0.0), (1.0, 0.0), (0.75, 1.0), (0.0, 1.0)],
            GeometryKind::Trapezoid => &[(0.25, 0.0), (0.75, 0.0), (1.0, 1.0), (0.0, 1.0)],
            GeometryKind::Diamond => &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)],
            GeometryKind::Ellipse => unreachable!(),
        };
        let mut path = BezPath::new();
        for (i, &(x, y)) in points.iter().enumerate() {
            let x = if self.flip_x { 1.0 - x } else { x };
            let y = if self.flip_y { 1.0 - y } else { y };
            let p = Point::new(
                self.center.x + (x * 2.0 - 1.0) * self.radius_x,
                self.center.y + (y * 2.0 - 1.0) * self.radius_y,
            );
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path.close_path();
        path
    }

    fn style(&self) -> &ShapeStyle {
        &self.style
    }

    fn style_mut(&mut self) -> &mut ShapeStyle {
        &mut self.style
    }

    fn transform(&mut self, affine: Affine) {
        self.center = affine * self.center;
        let scale = affine.as_coeffs();
        if scale[0] < 0.0 {
            self.flip_x = !self.flip_x;
        }
        if scale[3] < 0.0 {
            self.flip_y = !self.flip_y;
        }
        self.radius_x *= scale[0].abs();
        self.radius_y *= scale[3].abs();
        if scale[1].abs() < 1e-10 && scale[2].abs() < 1e-10 && scale[0] * scale[3] < 0.0 {
            self.rotation = -self.rotation;
        }
    }

    fn clone_box(&self) -> Box<dyn ShapeTrait + Send + Sync> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ellipse_creation() {
        let ellipse = Ellipse::new(Point::new(50.0, 50.0), 30.0, 20.0);
        assert!((ellipse.center.x - 50.0).abs() < f64::EPSILON);
        assert!((ellipse.radius_x - 30.0).abs() < f64::EPSILON);
        assert!((ellipse.radius_y - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_circle() {
        let circle = Ellipse::circle(Point::new(0.0, 0.0), 10.0);
        assert!((circle.radius_x - circle.radius_y).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hit_test_center() {
        let mut ellipse = Ellipse::new(Point::new(50.0, 50.0), 30.0, 20.0);
        ellipse.style.fill_color = Some(SerializableColor::black());
        assert!(ellipse.hit_test(Point::new(50.0, 50.0), 0.0));
    }

    #[test]
    fn test_hit_test_edge() {
        let circle = Ellipse::circle(Point::new(0.0, 0.0), 10.0);
        assert!(circle.hit_test(Point::new(10.0, 0.0), 0.0));
        assert!(!circle.hit_test(Point::new(15.0, 0.0), 0.0));
    }

    #[test]
    fn test_bounds() {
        let ellipse = Ellipse::new(Point::new(50.0, 50.0), 30.0, 20.0);
        let bounds = ellipse.bounds();
        assert!((bounds.x0 - 20.0).abs() < f64::EPSILON);
        assert!((bounds.y0 - 30.0).abs() < f64::EPSILON);
        assert!((bounds.x1 - 80.0).abs() < f64::EPSILON);
        assert!((bounds.y1 - 70.0).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;
    #[test]
    fn geometry_paths_hit_test_and_round_trip() {
        for kind in GeometryKind::ALL {
            let mut shape = Ellipse::new(Point::new(50.0, 50.0), 50.0, 50.0);
            shape.geometry = kind;
            shape.style.fill_color = Some(SerializableColor::black());
            assert!(shape.hit_test(Point::new(50.0, 50.0), 0.0));
            assert!(!shape.hit_test(Point::new(-20.0, -20.0), 0.0));
            let restored: Ellipse =
                serde_json::from_str(&serde_json::to_string(&shape).unwrap()).unwrap();
            assert_eq!(restored.geometry, kind);
            if kind != GeometryKind::Ellipse {
                assert!(!shape.hit_test(Point::new(1.0, 1.0), 0.0));
            }
        }
    }
    #[test]
    fn legacy_ellipse_and_cycle() {
        let shape = Ellipse::new(Point::ZERO, 10.0, 20.0);
        let mut value = serde_json::to_value(shape).unwrap();
        value.as_object_mut().unwrap().remove("geometry");
        value.as_object_mut().unwrap().remove("flip_x");
        value.as_object_mut().unwrap().remove("flip_y");
        assert_eq!(
            serde_json::from_value::<Ellipse>(value).unwrap().geometry,
            GeometryKind::Ellipse
        );
        let mut k = GeometryKind::Ellipse;
        for _ in 0..5 {
            k = k.next();
        }
        assert_eq!(k, GeometryKind::Ellipse);
    }
}
