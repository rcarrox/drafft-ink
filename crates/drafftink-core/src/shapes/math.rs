//! Math shape for LaTeX equations.

use super::{ShapeId, ShapeStyle, ShapeTrait};
use kurbo::{Affine, BezPath, Point, Rect};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use uuid::Uuid;

/// A math equation shape (LaTeX).
#[derive(Debug, Serialize, Deserialize)]
pub struct Math {
    pub(crate) id: ShapeId,
    /// Position (baseline origin).
    pub position: Point,
    /// LaTeX source used by the renderer.
    pub latex: String,
    /// User-facing source. May use the friendly Maple/GeoGebra-like syntax.
    #[serde(default)]
    pub source: String,
    /// Font size in pixels.
    pub font_size: f64,
    /// Rotation angle in radians (around center).
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "super::unit_display_scale")]
    pub display_scale: [f64; 2],
    /// Style properties.
    pub style: ShapeStyle,
    /// Cached layout size (width, height, depth) from renderer.
    #[serde(skip)]
    cached_size: RwLock<Option<(f64, f64, f64)>>,
}

impl Clone for Math {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            position: self.position,
            latex: self.latex.clone(),
            source: self.source.clone(),
            font_size: self.font_size,
            rotation: self.rotation,
            display_scale: self.display_scale,
            style: self.style.clone(),
            cached_size: RwLock::new(self.cached_size.read().ok().and_then(|g| *g)),
        }
    }
}

impl Math {
    pub const DEFAULT_FONT_SIZE: f64 = 20.0;

    pub fn new(position: Point, latex: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            source: latex.clone(),
            latex,
            font_size: Self::DEFAULT_FONT_SIZE,
            rotation: 0.0,
            display_scale: [1.0, 1.0],
            style: ShapeStyle::default(),
            cached_size: RwLock::new(None),
        }
    }

    pub(crate) fn reconstruct(
        id: ShapeId,
        position: Point,
        latex: String,
        font_size: f64,
        rotation: f64,
        style: ShapeStyle,
    ) -> Self {
        Self {
            id,
            position,
            source: latex.clone(),
            latex,
            font_size,
            rotation,
            display_scale: [1.0, 1.0],
            style,
            cached_size: RwLock::new(None),
        }
    }

    pub fn set_cached_size(&self, width: f64, height: f64, depth: f64) {
        if let Ok(mut cache) = self.cached_size.write() {
            *cache = Some((width, height, depth));
        }
    }

    pub fn cached_size(&self) -> Option<(f64, f64, f64)> {
        self.cached_size.read().ok().and_then(|g| *g)
    }

    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cached_size.write() {
            *cache = None;
        }
    }

    pub fn set_latex(&mut self, latex: String) {
        self.source = latex.clone();
        self.latex = latex;
        self.invalidate_cache();
    }

    pub fn set_formula(&mut self, source: String, latex: String) {
        self.source = source;
        self.latex = latex;
        self.invalidate_cache();
    }

    pub fn edit_source(&self) -> &str {
        if self.source.trim().is_empty() {
            &self.latex
        } else {
            &self.source
        }
    }
}

impl ShapeTrait for Math {
    fn id(&self) -> ShapeId {
        self.id
    }

    fn bounds(&self) -> Rect {
        let (width, height, depth) = self.cached_size.read().ok().and_then(|g| *g).unwrap_or((
            self.latex.len() as f64 * self.font_size * 0.5,
            self.font_size,
            self.font_size * 0.3,
        ));

        // Bounds are local, as for other shapes. Rotation is applied once by the
        // renderer and handle system, around this rectangle's center.
        Rect::new(
            self.position.x,
            self.position.y - height * self.display_scale[1],
            self.position.x + width * self.display_scale[0],
            self.position.y - depth * self.display_scale[1],
        )
    }

    fn hit_test(&self, point: Point, tolerance: f64) -> bool {
        self.bounds().inflate(tolerance, tolerance).contains(point)
    }

    fn to_path(&self) -> BezPath {
        let bounds = self.bounds();
        let mut path = BezPath::new();
        path.move_to(Point::new(bounds.x0, bounds.y0));
        path.line_to(Point::new(bounds.x1, bounds.y0));
        path.line_to(Point::new(bounds.x1, bounds.y1));
        path.line_to(Point::new(bounds.x0, bounds.y1));
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
        self.position = affine * self.position;
        let coeffs = affine.as_coeffs();
        self.display_scale[0] *= coeffs[0].hypot(coeffs[1]);
        self.display_scale[1] *= coeffs[2].hypot(coeffs[3]);
        // Extract rotation from affine
        let rotation = coeffs[1].atan2(coeffs[0]);
        if rotation.abs() > 0.001 {
            self.rotation += rotation;
        }
    }

    fn clone_box(&self) -> Box<dyn ShapeTrait + Send + Sync> {
        Box::new(self.clone())
    }
}
