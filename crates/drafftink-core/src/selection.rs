//! Selection and manipulation handle system.

use crate::shapes::{Shape, ShapeId, ShapeTrait};
use kurbo::{Affine, Point, Rect};
use serde::{Deserialize, Serialize};

/// Handle size in screen pixels.
pub const HANDLE_SIZE: f64 = 16.0;
/// Handle hit tolerance in screen pixels.
pub const HANDLE_HIT_TOLERANCE: f64 = 8.0;

/// Type of selection handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandleKind {
    /// Endpoint handle for lines/arrows (index 0 = start, 1 = end).
    Endpoint(usize),
    /// Intermediate point handle for lines/arrows (index into intermediate_points).
    IntermediatePoint(usize),
    /// Virtual midpoint handle for splitting a segment (index = segment index).
    SegmentMidpoint(usize),
    /// Corner handle for rectangles/ellipses.
    Corner(Corner),
    /// Edge midpoint handle for rectangles/ellipses (for resizing).
    Edge(Edge),
    /// Rotation handle (positioned outside the shape).
    Rotate,
}

/// Corner positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Edge positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

/// A selection handle with its position and type.
#[derive(Debug, Clone, Copy)]
pub struct Handle {
    /// Position in world coordinates.
    pub position: Point,
    /// Handle type.
    pub kind: HandleKind,
}

impl Handle {
    /// Create a new handle.
    pub fn new(position: Point, kind: HandleKind) -> Self {
        Self { position, kind }
    }

    /// Check if a point (in world coordinates) hits this handle.
    /// `tolerance` should be adjusted for camera zoom.
    pub fn hit_test(&self, point: Point, tolerance: f64) -> bool {
        let dx = point.x - self.position.x;
        let dy = point.y - self.position.y;
        let dist_sq = dx * dx + dy * dy;
        dist_sq <= tolerance * tolerance
    }
}

/// Get the selection handles for a shape.
pub fn get_handles(shape: &Shape) -> Vec<Handle> {
    match shape {
        Shape::Line(line) => {
            let all_pts = line.all_points();
            let mut handles = Vec::new();
            // Add endpoint and intermediate point handles
            handles.push(Handle::new(line.start, HandleKind::Endpoint(0)));
            for (i, &pt) in line.intermediate_points.iter().enumerate() {
                handles.push(Handle::new(pt, HandleKind::IntermediatePoint(i)));
            }
            handles.push(Handle::new(line.end, HandleKind::Endpoint(1)));
            // Add segment midpoint handles
            for i in 0..all_pts.len() - 1 {
                let mid = Point::new(
                    (all_pts[i].x + all_pts[i + 1].x) / 2.0,
                    (all_pts[i].y + all_pts[i + 1].y) / 2.0,
                );
                handles.push(Handle::new(mid, HandleKind::SegmentMidpoint(i)));
            }
            handles
        }
        Shape::Arrow(arrow) => {
            let all_pts = arrow.all_points();
            let mut handles = Vec::new();
            handles.push(Handle::new(arrow.start, HandleKind::Endpoint(0)));
            for (i, &pt) in arrow.intermediate_points.iter().enumerate() {
                handles.push(Handle::new(pt, HandleKind::IntermediatePoint(i)));
            }
            handles.push(Handle::new(arrow.end, HandleKind::Endpoint(1)));
            // Add segment midpoint handles
            for i in 0..all_pts.len() - 1 {
                let mid = Point::new(
                    (all_pts[i].x + all_pts[i + 1].x) / 2.0,
                    (all_pts[i].y + all_pts[i + 1].y) / 2.0,
                );
                handles.push(Handle::new(mid, HandleKind::SegmentMidpoint(i)));
            }
            handles
        }
        Shape::Rectangle(_) | Shape::Ellipse(_) | Shape::Image(_) => {
            let bounds = shape.bounds();
            let rotation = shape.rotation();
            corner_and_rotate_handles(bounds, rotation)
        }
        Shape::Text(_) | Shape::Math(_) => {
            // Text and Math support nonuniform display scaling, including edge handles.
            let bounds = shape.bounds();
            let rotation = shape.rotation();
            corner_and_rotate_handles(bounds, rotation)
        }
        Shape::Freehand(_) => {
            // Freehand uses bounding box corners (no rotation)
            let bounds = shape.bounds();
            corner_handles(bounds)
        }
        Shape::Group(group) => {
            let bounds = shape.bounds();
            corner_and_rotate_handles(bounds, group.rotation)
        }
    }
}

/// Generate corner handles for a bounding rectangle.
fn corner_handles(bounds: Rect) -> Vec<Handle> {
    let mut handles = vec![
        Handle::new(
            Point::new(bounds.x0, bounds.y0),
            HandleKind::Corner(Corner::TopLeft),
        ),
        Handle::new(
            Point::new(bounds.x1, bounds.y0),
            HandleKind::Corner(Corner::TopRight),
        ),
        Handle::new(
            Point::new(bounds.x0, bounds.y1),
            HandleKind::Corner(Corner::BottomLeft),
        ),
        Handle::new(
            Point::new(bounds.x1, bounds.y1),
            HandleKind::Corner(Corner::BottomRight),
        ),
    ];
    handles.extend(edge_handles(bounds, 0.0));
    handles
}

/// Distance from shape edge to rotation handle (in world units).
pub const ROTATE_HANDLE_OFFSET: f64 = 25.0;

/// Generate corner handles plus a rotation handle for a bounding rectangle.
/// The rotation handle is placed above the top-center, rotated by the shape's rotation.
fn corner_and_rotate_handles(bounds: Rect, rotation: f64) -> Vec<Handle> {
    let center = bounds.center();
    let half_w = bounds.width() / 2.0;
    let half_h = bounds.height() / 2.0;

    // Helper to rotate a point around center
    let rotate_point = |dx: f64, dy: f64| -> Point {
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        Point::new(
            center.x + dx * cos_r - dy * sin_r,
            center.y + dx * sin_r + dy * cos_r,
        )
    };

    let mut handles = vec![
        Handle::new(
            rotate_point(-half_w, -half_h),
            HandleKind::Corner(Corner::TopLeft),
        ),
        Handle::new(
            rotate_point(half_w, -half_h),
            HandleKind::Corner(Corner::TopRight),
        ),
        Handle::new(
            rotate_point(-half_w, half_h),
            HandleKind::Corner(Corner::BottomLeft),
        ),
        Handle::new(
            rotate_point(half_w, half_h),
            HandleKind::Corner(Corner::BottomRight),
        ),
        // Rotation handle: above top-center
        Handle::new(
            rotate_point(0.0, -half_h - ROTATE_HANDLE_OFFSET),
            HandleKind::Rotate,
        ),
    ];
    handles.extend(edge_handles(bounds, rotation));
    handles
}

/// Find which handle (if any) is hit at the given point.
/// Returns the handle kind if hit.
pub fn hit_test_handles(shape: &Shape, point: Point, tolerance: f64) -> Option<HandleKind> {
    let handles = get_handles(shape);
    for handle in handles {
        if handle.hit_test(point, tolerance) {
            return Some(handle.kind);
        }
    }
    // Outer corner annulus: resize at the handle, rotate just outside it.
    if matches!(shape, Shape::Image(_)) {
        let local = Affine::rotate_about(-shape.rotation(), shape.bounds().center()) * point;
        let bounds = shape.bounds();
        for corner in get_handles(shape)
            .into_iter()
            .filter(|h| matches!(h.kind, HandleKind::Corner(_)))
        {
            let c = Affine::rotate_about(-shape.rotation(), bounds.center()) * corner.position;
            let outward = (local.x - c.x) * (c.x - bounds.center().x) >= 0.0
                && (local.y - c.y) * (c.y - bounds.center().y) >= 0.0;
            if outward && point.distance(corner.position) <= tolerance * 2.75 {
                return Some(HandleKind::Rotate);
            }
        }
    }
    None
}

/// Check if a point is on the boundary (edge) of a shape's bounding box.
/// Returns true if the point is within tolerance of any edge but not inside the interior.
pub fn hit_test_boundary(shape: &Shape, point: Point, tolerance: f64) -> bool {
    let bounds = shape.bounds();
    let outer = bounds.inflate(tolerance, tolerance);
    let inner = bounds.inset(
        tolerance
            .min(bounds.width() / 2.0)
            .min(bounds.height() / 2.0),
    );
    outer.contains(point) && !inner.contains(point)
}

/// State of an active manipulation operation (single shape with handle).
#[derive(Debug, Clone)]
pub struct ManipulationState {
    /// The shape being manipulated.
    pub shape_id: ShapeId,
    /// The handle being dragged (None = moving the whole shape).
    pub handle: Option<HandleKind>,
    /// Starting point of the drag.
    pub start_point: Point,
    /// Current point of the drag.
    pub current_point: Point,
    /// Original shape state for preview/undo.
    pub original_shape: Shape,
}

/// State for moving multiple shapes at once.
#[derive(Debug, Clone)]
pub struct MultiMoveState {
    /// Starting point of the drag.
    pub start_point: Point,
    /// Current point of the drag.
    pub current_point: Point,
    /// Original shapes state for preview/undo (shape_id -> original shape).
    pub original_shapes: std::collections::HashMap<ShapeId, Shape>,
    /// Whether this is an alt-drag duplicate operation.
    pub is_duplicate: bool,
    /// IDs of duplicated shapes (only set if is_duplicate is true).
    pub duplicated_ids: Vec<ShapeId>,
}

impl ManipulationState {
    /// Create a new manipulation state.
    pub fn new(
        shape_id: ShapeId,
        handle: Option<HandleKind>,
        start_point: Point,
        original_shape: Shape,
    ) -> Self {
        Self {
            shape_id,
            handle,
            start_point,
            current_point: start_point,
            original_shape,
        }
    }

    /// Get the drag delta.
    pub fn delta(&self) -> kurbo::Vec2 {
        kurbo::Vec2::new(
            self.current_point.x - self.start_point.x,
            self.current_point.y - self.start_point.y,
        )
    }
}

impl MultiMoveState {
    /// Create a new multi-move state.
    pub fn new(
        start_point: Point,
        original_shapes: std::collections::HashMap<ShapeId, Shape>,
    ) -> Self {
        Self {
            start_point,
            current_point: start_point,
            original_shapes,
            is_duplicate: false,
            duplicated_ids: Vec::new(),
        }
    }

    /// Create a new multi-move state for duplication (Alt+drag).
    pub fn new_duplicate(
        start_point: Point,
        original_shapes: std::collections::HashMap<ShapeId, Shape>,
    ) -> Self {
        Self {
            start_point,
            current_point: start_point,
            original_shapes,
            is_duplicate: true,
            duplicated_ids: Vec::new(),
        }
    }

    /// Get the drag delta.
    pub fn delta(&self) -> kurbo::Vec2 {
        kurbo::Vec2::new(
            self.current_point.x - self.start_point.x,
            self.current_point.y - self.start_point.y,
        )
    }

    /// Get the shape IDs being moved.
    pub fn shape_ids(&self) -> Vec<ShapeId> {
        self.original_shapes.keys().copied().collect()
    }
}

/// Get the position of the manipulation target (handle or shape center).
/// This is used for snapping - we snap the target position, not the cursor.
pub fn get_manipulation_target_position(shape: &Shape, handle: Option<HandleKind>) -> Point {
    match handle {
        None => {
            // Moving entire shape - use top-left of bounding box as reference
            let bounds = shape.bounds();
            Point::new(bounds.x0, bounds.y0)
        }
        Some(HandleKind::Endpoint(idx)) => match shape {
            Shape::Line(line) => {
                if idx == 0 {
                    line.start
                } else {
                    line.end
                }
            }
            Shape::Arrow(arrow) => {
                if idx == 0 {
                    arrow.start
                } else {
                    arrow.end
                }
            }
            _ => shape.bounds().center(),
        },
        Some(HandleKind::IntermediatePoint(idx)) => match shape {
            Shape::Line(line) => line
                .intermediate_points
                .get(idx)
                .copied()
                .unwrap_or(line.start),
            Shape::Arrow(arrow) => arrow
                .intermediate_points
                .get(idx)
                .copied()
                .unwrap_or(arrow.start),
            _ => shape.bounds().center(),
        },
        Some(HandleKind::SegmentMidpoint(seg_idx)) => match shape {
            Shape::Line(line) => {
                let pts = line.all_points();
                if seg_idx < pts.len() - 1 {
                    Point::new(
                        (pts[seg_idx].x + pts[seg_idx + 1].x) / 2.0,
                        (pts[seg_idx].y + pts[seg_idx + 1].y) / 2.0,
                    )
                } else {
                    line.start
                }
            }
            Shape::Arrow(arrow) => {
                let pts = arrow.all_points();
                if seg_idx < pts.len() - 1 {
                    Point::new(
                        (pts[seg_idx].x + pts[seg_idx + 1].x) / 2.0,
                        (pts[seg_idx].y + pts[seg_idx + 1].y) / 2.0,
                    )
                } else {
                    arrow.start
                }
            }
            _ => shape.bounds().center(),
        },
        Some(kind @ HandleKind::Corner(_)) => get_handles(shape)
            .iter()
            .find(|h| h.kind == kind)
            .map(|h| h.position)
            .unwrap_or(shape.bounds().center()),
        Some(kind @ HandleKind::Edge(_)) => get_handles(shape)
            .iter()
            .find(|h| h.kind == kind)
            .map(|h| h.position)
            .unwrap_or(shape.bounds().center()),
        Some(HandleKind::Rotate) => {
            // Rotation handle position
            let bounds = shape.bounds();
            let center = bounds.center();
            let rotation = shape.rotation();
            let half_h = bounds.height() / 2.0;
            let cos_r = rotation.cos();
            let sin_r = rotation.sin();
            Point::new(
                center.x - (half_h + ROTATE_HANDLE_OFFSET) * sin_r,
                center.y - (half_h + ROTATE_HANDLE_OFFSET) * cos_r,
            )
        }
    }
}

/// Apply a handle manipulation to a shape.
/// Returns the modified shape.
/// `keep_aspect_ratio`: if true, maintains aspect ratio during corner resize (Shift key).
pub fn apply_manipulation(
    shape: &Shape,
    handle: Option<HandleKind>,
    delta: kurbo::Vec2,
    keep_aspect_ratio: bool,
) -> Shape {
    let mut shape = shape.clone();

    match handle {
        None => {
            // Move the entire shape
            let translation = kurbo::Affine::translate(delta);
            shape.transform(translation);
        }
        Some(HandleKind::Endpoint(idx)) => {
            // Move an endpoint (for lines/arrows)
            match &mut shape {
                Shape::Line(line) => {
                    if idx == 0 {
                        line.start.x += delta.x;
                        line.start.y += delta.y;
                    } else {
                        line.end.x += delta.x;
                        line.end.y += delta.y;
                    }
                }
                Shape::Arrow(arrow) => {
                    if idx == 0 {
                        arrow.start.x += delta.x;
                        arrow.start.y += delta.y;
                    } else {
                        arrow.end.x += delta.x;
                        arrow.end.y += delta.y;
                    }
                }
                _ => {}
            }
        }
        Some(HandleKind::IntermediatePoint(idx)) => {
            // Move an intermediate point (for lines/arrows)
            match &mut shape {
                Shape::Line(line) => {
                    if let Some(pt) = line.intermediate_points.get_mut(idx) {
                        pt.x += delta.x;
                        pt.y += delta.y;
                    }
                }
                Shape::Arrow(arrow) => {
                    if let Some(pt) = arrow.intermediate_points.get_mut(idx) {
                        pt.x += delta.x;
                        pt.y += delta.y;
                    }
                }
                _ => {}
            }
        }
        Some(HandleKind::SegmentMidpoint(seg_idx)) => {
            // Insert a new point at the segment midpoint and move it
            match &mut shape {
                Shape::Line(line) => {
                    let pts = line.all_points();
                    if seg_idx < pts.len() - 1 {
                        let mid = Point::new(
                            (pts[seg_idx].x + pts[seg_idx + 1].x) / 2.0 + delta.x,
                            (pts[seg_idx].y + pts[seg_idx + 1].y) / 2.0 + delta.y,
                        );
                        // seg_idx 0 means between start and first intermediate (or end)
                        // Insert at position seg_idx in intermediate_points
                        line.intermediate_points.insert(seg_idx, mid);
                    }
                }
                Shape::Arrow(arrow) => {
                    let pts = arrow.all_points();
                    if seg_idx < pts.len() - 1 {
                        let mid = Point::new(
                            (pts[seg_idx].x + pts[seg_idx + 1].x) / 2.0 + delta.x,
                            (pts[seg_idx].y + pts[seg_idx + 1].y) / 2.0 + delta.y,
                        );
                        arrow.intermediate_points.insert(seg_idx, mid);
                    }
                }
                _ => {}
            }
        }
        Some(kind @ (HandleKind::Corner(_) | HandleKind::Edge(_))) => {
            apply_box_resize(&mut shape, kind, delta, keep_aspect_ratio);
        }
        Some(HandleKind::Rotate) => {
            // Rotation is handled separately via apply_rotation
        }
    }

    shape
}

/// Apply rotation to a shape.
/// `cursor_point`: current cursor position in world coordinates.
/// `snap_to_15deg`: if true, snap rotation to 15° increments.
/// Returns the new rotation angle in radians.
pub fn apply_rotation(shape: &mut Shape, cursor_point: Point, snap_to_15deg: bool) -> f64 {
    let bounds = shape.bounds();
    let center = bounds.center();

    // Calculate angle from center to cursor
    let dx = cursor_point.x - center.x;
    let dy = cursor_point.y - center.y;
    let mut angle = dy.atan2(dx) + std::f64::consts::FRAC_PI_2; // Offset so 0° is up

    // Snap to 15° increments if requested
    if snap_to_15deg {
        let snap_angle = std::f64::consts::PI / 12.0; // 15°
        angle = (angle / snap_angle).round() * snap_angle;
    }

    shape.set_rotation(angle);
    angle
}

/// Reset rotation to a specific angle (0° or 90°).
pub fn reset_rotation(shape: &mut Shape, angle_degrees: f64) {
    let angle_radians = angle_degrees.to_radians();
    shape.set_rotation(angle_radians);
}

fn edge_handles(bounds: Rect, rotation: f64) -> Vec<Handle> {
    let rotate = Affine::rotate_about(rotation, bounds.center());
    vec![
        Handle::new(
            rotate * Point::new(bounds.center().x, bounds.y0),
            HandleKind::Edge(Edge::Top),
        ),
        Handle::new(
            rotate * Point::new(bounds.x1, bounds.center().y),
            HandleKind::Edge(Edge::Right),
        ),
        Handle::new(
            rotate * Point::new(bounds.center().x, bounds.y1),
            HandleKind::Edge(Edge::Bottom),
        ),
        Handle::new(
            rotate * Point::new(bounds.x0, bounds.center().y),
            HandleKind::Edge(Edge::Left),
        ),
    ]
}
fn rotate_delta(delta: kurbo::Vec2, angle: f64) -> kurbo::Vec2 {
    let (sin, cos) = angle.sin_cos();
    kurbo::Vec2::new(cos * delta.x - sin * delta.y, sin * delta.x + cos * delta.y)
}
fn apply_box_resize(shape: &mut Shape, kind: HandleKind, delta: kurbo::Vec2, aspect: bool) {
    let old = shape.bounds();
    let w = old.width().max(1.0);
    let h = old.height().max(1.0);
    let rotation = shape.rotation();
    let d = rotate_delta(delta, -rotation);
    let left = matches!(
        kind,
        HandleKind::Corner(Corner::TopLeft | Corner::BottomLeft) | HandleKind::Edge(Edge::Left)
    );
    let right = matches!(
        kind,
        HandleKind::Corner(Corner::TopRight | Corner::BottomRight) | HandleKind::Edge(Edge::Right)
    );
    let top = matches!(
        kind,
        HandleKind::Corner(Corner::TopLeft | Corner::TopRight) | HandleKind::Edge(Edge::Top)
    );
    let bottom = matches!(
        kind,
        HandleKind::Corner(Corner::BottomLeft | Corner::BottomRight)
            | HandleKind::Edge(Edge::Bottom)
    );
    if let Shape::Image(image) = shape {
        let mut signed_w = w + if left {
            -d.x
        } else if right {
            d.x
        } else {
            0.0
        };
        let mut signed_h = h + if top {
            -d.y
        } else if bottom {
            d.y
        } else {
            0.0
        };
        if aspect && matches!(kind, HandleKind::Corner(_)) {
            let magnitude = (signed_w.abs() / w).max(signed_h.abs() / h);
            signed_w = signed_w.signum() * w * magnitude;
            signed_h = signed_h.signum() * h * magnitude;
        }
        let shift = rotate_delta(
            kurbo::Vec2::new(
                (signed_w - w)
                    * if left {
                        -0.5
                    } else if right {
                        0.5
                    } else {
                        0.0
                    },
                (signed_h - h)
                    * if top {
                        -0.5
                    } else if bottom {
                        0.5
                    } else {
                        0.0
                    },
            ),
            rotation,
        );
        let center = old.center() + shift;
        image.width = signed_w.abs().max(0.001);
        image.height = signed_h.abs().max(0.001);
        image.position = Point::new(center.x - image.width / 2.0, center.y - image.height / 2.0);
        image.flip_x ^= signed_w < 0.0;
        image.flip_y ^= signed_h < 0.0;
        return;
    }
    let mut nw = (w + if left {
        -d.x
    } else if right {
        d.x
    } else {
        0.0
    })
    .max(1.0);
    let mut nh = (h + if top {
        -d.y
    } else if bottom {
        d.y
    } else {
        0.0
    })
    .max(1.0);
    if aspect && matches!(kind, HandleKind::Corner(_)) {
        let scale = (nw / w).max(nh / h);
        nw = w * scale;
        nh = h * scale;
    }
    let shift = rotate_delta(
        kurbo::Vec2::new(
            (nw - w)
                * if left {
                    -0.5
                } else if right {
                    0.5
                } else {
                    0.0
                },
            (nh - h)
                * if top {
                    -0.5
                } else if bottom {
                    0.5
                } else {
                    0.0
                },
        ),
        rotation,
    );
    let center = old.center() + shift;
    let next = Rect::from_center_size(center, kurbo::Size::new(nw, nh));
    let scale = Affine::translate((next.x0, next.y0))
        * Affine::scale_non_uniform(nw / w, nh / h)
        * Affine::translate((-old.x0, -old.y0));
    match shape {
        Shape::Rectangle(rect) => {
            rect.position = Point::new(next.x0, next.y0);
            rect.width = nw;
            rect.height = nh;
        }
        Shape::Ellipse(ellipse) => {
            ellipse.center = center;
            ellipse.radius_x = nw / 2.0;
            ellipse.radius_y = nh / 2.0;
        }
        Shape::Image(image) => {
            image.position = Point::new(next.x0, next.y0);
            image.width = nw;
            image.height = nh;
        }
        Shape::Freehand(freehand) => {
            for p in &mut freehand.points {
                *p = scale * *p;
            }
        }
        Shape::Group(group) => {
            for child in group.children_mut() {
                child.transform(scale);
            }
        }
        Shape::Text(text) => {
            text.display_scale[0] *= nw / w;
            text.display_scale[1] *= nh / h;
            text.position = Point::new(next.x0, next.y0);
        }
        Shape::Math(math) => {
            let baseline = (math.position.y - old.y0) * nh / h;
            math.display_scale[0] *= nw / w;
            math.display_scale[1] *= nh / h;
            math.position = Point::new(next.x0, next.y0 + baseline);
        }
        _ => {}
    }
}

/// Manipulate image source boundaries without resampling or stretching pixels.
/// All deltas are measured from the original drag snapshot, as for resizing.
pub fn apply_image_crop(shape: &Shape, handle: Option<HandleKind>, delta: kurbo::Vec2) -> Shape {
    let (Shape::Image(original), Some(kind @ (HandleKind::Corner(_) | HandleKind::Edge(_)))) =
        (shape, handle)
    else {
        return shape.clone();
    };
    let mut image = original.clone();
    let mut local = rotate_delta(delta, -image.rotation);
    if image.flip_x {
        local.x = -local.x;
    }
    if image.flip_y {
        local.y = -local.y;
    }
    let left = matches!(
        kind,
        HandleKind::Corner(Corner::TopLeft | Corner::BottomLeft) | HandleKind::Edge(Edge::Left)
    );
    let left = left ^ image.flip_x;
    let horizontal = !matches!(kind, HandleKind::Edge(Edge::Top | Edge::Bottom));
    let vertical = !matches!(kind, HandleKind::Edge(Edge::Left | Edge::Right));
    let top = matches!(
        kind,
        HandleKind::Corner(Corner::TopLeft | Corner::TopRight) | HandleKind::Edge(Edge::Top)
    );
    let top = top ^ image.flip_y;
    let sx = image.width / image.crop.width();
    let sy = image.height / image.crop.height();
    let min_x = (1.0 / image.source_width.max(1) as f64).min(image.crop.width());
    let min_y = (1.0 / image.source_height.max(1) as f64).min(image.crop.height());
    let mut crop = image.crop;
    if horizontal && left {
        crop.x0 = (crop.x0 + local.x / sx).clamp(0.0, crop.x1 - min_x);
    } else if horizontal {
        crop.x1 = (crop.x1 + local.x / sx).clamp(crop.x0 + min_x, 1.0);
    }
    if vertical && top {
        crop.y0 = (crop.y0 + local.y / sy).clamp(0.0, crop.y1 - min_y);
    } else if vertical {
        crop.y1 = (crop.y1 + local.y / sy).clamp(crop.y0 + min_y, 1.0);
    }
    let shift = kurbo::Vec2::new(
        (crop.center().x - image.crop.center().x) * sx * if image.flip_x { -1.0 } else { 1.0 },
        (crop.center().y - image.crop.center().y) * sy * if image.flip_y { -1.0 } else { 1.0 },
    );
    let center = image.as_rect().center() + rotate_delta(shift, image.rotation);
    image.crop = crop;
    image.width = crop.width() * sx;
    image.height = crop.height() * sy;
    image.position = Point::new(center.x - image.width / 2.0, center.y - image.height / 2.0);
    Shape::Image(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::{Line, Rectangle};

    #[test]
    fn test_line_handles() {
        let line = Line::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let handles = get_handles(&Shape::Line(line));

        // 2 endpoints + 1 segment midpoint
        assert_eq!(handles.len(), 3);
        assert!(matches!(handles[0].kind, HandleKind::Endpoint(0)));
        assert!(matches!(handles[1].kind, HandleKind::Endpoint(1)));
        assert!(matches!(handles[2].kind, HandleKind::SegmentMidpoint(0)));
    }

    #[test]
    fn test_rectangle_handles() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), 100.0, 50.0);
        let handles = get_handles(&Shape::Rectangle(rect));

        // 4 corners + rotation + 4 edge centers
        assert_eq!(handles.len(), 9);
        assert!(matches!(
            handles[0].kind,
            HandleKind::Corner(Corner::TopLeft)
        ));
        assert!(matches!(handles[4].kind, HandleKind::Rotate));
    }

    #[test]
    fn test_handle_hit_test() {
        let handle = Handle::new(Point::new(50.0, 50.0), HandleKind::Endpoint(0));

        assert!(handle.hit_test(Point::new(50.0, 50.0), 10.0));
        assert!(handle.hit_test(Point::new(55.0, 55.0), 10.0));
        assert!(!handle.hit_test(Point::new(70.0, 70.0), 10.0));
    }

    #[test]
    fn test_apply_endpoint_manipulation() {
        let line = Line::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let shape = Shape::Line(line);

        let result = apply_manipulation(
            &shape,
            Some(HandleKind::Endpoint(1)),
            kurbo::Vec2::new(10.0, 20.0),
            false,
        );

        if let Shape::Line(line) = result {
            assert!((line.end.x - 110.0).abs() < f64::EPSILON);
            assert!((line.end.y - 120.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Line shape");
        }
    }

    #[test]
    fn test_apply_corner_manipulation() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), 100.0, 100.0);
        let shape = Shape::Rectangle(rect);

        let result = apply_manipulation(
            &shape,
            Some(HandleKind::Corner(Corner::BottomRight)),
            kurbo::Vec2::new(50.0, 50.0),
            false,
        );

        if let Shape::Rectangle(rect) = result {
            assert!((rect.width - 150.0).abs() < f64::EPSILON);
            assert!((rect.height - 150.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Rectangle shape");
        }
    }

    #[test]
    fn test_freehand_resize() {
        use crate::shapes::Freehand;

        let freehand = Freehand::from_points(vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(0.0, 50.0),
        ]);
        let shape = Shape::Freehand(freehand);

        let result = apply_manipulation(
            &shape,
            Some(HandleKind::Corner(Corner::BottomRight)),
            kurbo::Vec2::new(50.0, 50.0),
            false,
        );

        if let Shape::Freehand(freehand) = result {
            let bounds = freehand.bounds();
            assert!((bounds.width() - 100.0).abs() < 0.1);
            assert!((bounds.height() - 100.0).abs() < 0.1);
        } else {
            panic!("Expected Freehand shape");
        }
    }

    #[test]
    fn test_image_resize() {
        use crate::shapes::{Image, ImageFormat, ShapeStyle};

        let image = Image {
            id: uuid::Uuid::new_v4(),
            position: Point::new(0.0, 0.0),
            width: 100.0,
            height: 100.0,
            source_width: 200,
            source_height: 200,
            format: ImageFormat::Png,
            data_base64: String::new().into(),
            rotation: 0.0,
            crop: Rect::new(0.0, 0.0, 1.0, 1.0),
            flip_x: false,
            flip_y: false,
            style: ShapeStyle::default(),
        };
        let shape = Shape::Image(image);

        let result = apply_manipulation(
            &shape,
            Some(HandleKind::Corner(Corner::BottomRight)),
            kurbo::Vec2::new(50.0, 50.0),
            false,
        );

        if let Shape::Image(image) = result {
            assert!((image.width - 150.0).abs() < f64::EPSILON);
            assert!((image.height - 150.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Image shape");
        }
    }

    #[test]
    fn test_aspect_ratio_resize() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), 100.0, 50.0);
        let shape = Shape::Rectangle(rect);

        let result = apply_manipulation(
            &shape,
            Some(HandleKind::Corner(Corner::BottomRight)),
            kurbo::Vec2::new(100.0, 100.0),
            true, // keep aspect ratio
        );

        if let Shape::Rectangle(rect) = result {
            // With aspect ratio 2:1, resizing should maintain that ratio
            let aspect = rect.width / rect.height;
            assert!((aspect - 2.0).abs() < 0.1);
        } else {
            panic!("Expected Rectangle shape");
        }
    }
}

#[cfg(test)]
mod image_geometry_regressions {
    use super::*;
    use crate::canvas::CanvasDocument;
    use crate::shapes::{Image, ImageFormat, StrokeStyle};

    fn image(angle: f64) -> Shape {
        let mut image = Image::new(
            Point::new(20.0, 30.0),
            &[1, 2, 3],
            400,
            200,
            ImageFormat::Png,
        )
        .with_size(200.0, 100.0);
        image.rotation = angle;
        Shape::Image(image)
    }
    fn corner(shape: &Shape, kind: Corner) -> Point {
        get_handles(shape)
            .iter()
            .find(|h| h.kind == HandleKind::Corner(kind))
            .unwrap()
            .position
    }
    fn opposite(c: Corner) -> Corner {
        match c {
            Corner::TopLeft => Corner::BottomRight,
            Corner::TopRight => Corner::BottomLeft,
            Corner::BottomLeft => Corner::TopRight,
            Corner::BottomRight => Corner::TopLeft,
        }
    }
    #[test]
    fn rotated_resize_tracks_mouse_and_anchors_opposite_corner() {
        for angle in [0.0, 0.7, std::f64::consts::FRAC_PI_2, 2.6] {
            for c in [
                Corner::TopLeft,
                Corner::TopRight,
                Corner::BottomLeft,
                Corner::BottomRight,
            ] {
                let original = image(angle);
                let delta = rotate_delta(kurbo::Vec2::new(12.0, 9.0), angle);
                let resized =
                    apply_manipulation(&original, Some(HandleKind::Corner(c)), delta, false);
                assert!(corner(&resized, c).distance(corner(&original, c) + delta) < 1e-8);
                assert!(
                    corner(&resized, opposite(c)).distance(corner(&original, opposite(c))) < 1e-8
                );
            }
        }
    }
    #[test]
    fn crop_keeps_pixel_scale_and_rotated_anchor() {
        for angle in [0.0, 0.7, std::f64::consts::FRAC_PI_2, 2.6] {
            for c in [
                Corner::TopLeft,
                Corner::TopRight,
                Corner::BottomLeft,
                Corner::BottomRight,
            ] {
                let original = image(angle);
                let dx = if matches!(c, Corner::TopLeft | Corner::BottomLeft) {
                    20.0
                } else {
                    -20.0
                };
                let dy = if matches!(c, Corner::TopLeft | Corner::TopRight) {
                    10.0
                } else {
                    -10.0
                };
                let delta = rotate_delta(kurbo::Vec2::new(dx, dy), angle);
                let cropped = apply_image_crop(&original, Some(HandleKind::Corner(c)), delta);
                assert!(corner(&cropped, c).distance(corner(&original, c) + delta) < 1e-8);
                assert!(
                    corner(&cropped, opposite(c)).distance(corner(&original, opposite(c))) < 1e-8
                );
                if let Shape::Image(i) = cropped {
                    assert!((i.width / i.crop.width() - 200.0).abs() < 1e-8);
                    assert!((i.height / i.crop.height() - 100.0).abs() < 1e-8);
                    assert_eq!(i.data(), Some(vec![1, 2, 3]));
                }
            }
        }
    }
    #[test]
    fn crop_json_undo_redo_and_resize_preserve_source() {
        let original = image(0.7);
        let id = original.id();
        let mut doc = CanvasDocument::new();
        doc.add_shape(original.clone());
        doc.push_undo();
        let cropped = apply_image_crop(
            &original,
            Some(HandleKind::Corner(Corner::TopLeft)),
            rotate_delta(kurbo::Vec2::new(30.0, 20.0), 0.7),
        );
        *doc.get_shape_mut(id).unwrap() = cropped.clone();
        assert!(doc.undo());
        assert_eq!(doc.get_shape(id).unwrap().bounds(), original.bounds());
        assert!(doc.redo());
        let restored = CanvasDocument::from_json(&doc.to_json().unwrap()).unwrap();
        let resized = apply_manipulation(
            restored.get_shape(id).unwrap(),
            Some(HandleKind::Corner(Corner::BottomRight)),
            kurbo::Vec2::new(10.0, 20.0),
            false,
        );
        if let (Shape::Image(c), Shape::Image(r)) = (cropped, resized) {
            assert_eq!(c.crop, r.crop);
            assert_eq!(c.data_base64, r.data_base64);
        } else {
            panic!("image expected");
        }
    }
    #[test]
    fn stroke_styles_round_trip_and_old_documents_default_to_solid() {
        for pattern in [
            StrokeStyle::Solid,
            StrokeStyle::Dashed,
            StrokeStyle::DashedShort,
            StrokeStyle::Dotted,
        ] {
            let mut shape =
                Shape::Rectangle(crate::shapes::Rectangle::new(Point::ZERO, 40.0, 20.0));
            shape.style_mut().stroke_style = pattern;
            let mut doc = CanvasDocument::new();
            let id = shape.id();
            doc.add_shape(shape);
            let json = doc.to_json().unwrap();
            let loaded = CanvasDocument::from_json(&json).unwrap();
            assert_eq!(loaded.get_shape(id).unwrap().style().stroke_style, pattern);
        }
        let mut value = serde_json::to_value(image(0.0)).unwrap();
        let i = value.get_mut("Image").unwrap();
        i.as_object_mut().unwrap().remove("crop");
        i.get_mut("style")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("stroke_style");
        let Shape::Image(i) = serde_json::from_value::<Shape>(value).unwrap() else {
            panic!()
        };
        assert_eq!(i.crop, Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(i.style.stroke_style, StrokeStyle::Solid);
    }
}

#[cfg(test)]
mod edge_handle_regressions {
    use super::*;
    use crate::shapes::{Ellipse, Image, ImageFormat, Math, Rectangle, Text};
    fn shapes(angle: f64) -> Vec<Shape> {
        let text = Text::new(Point::new(20.0, 30.0), "Example".into());
        text.set_cached_size(100.0, 40.0);
        let math = Math::new(Point::new(20.0, 60.0), "x^{3}".into());
        math.set_cached_size(100.0, 30.0, -10.0);
        let mut shapes = vec![
            Shape::Rectangle(Rectangle::new(Point::new(20.0, 30.0), 100.0, 40.0)),
            Shape::Ellipse(Ellipse::new(Point::new(70.0, 50.0), 50.0, 20.0)),
            Shape::Image(
                Image::new(
                    Point::new(20.0, 30.0),
                    &[1, 2, 3],
                    400,
                    200,
                    ImageFormat::Png,
                )
                .with_size(100.0, 40.0),
            ),
            Shape::Text(text),
            Shape::Math(math),
        ];
        for shape in &mut shapes {
            shape.set_rotation(angle);
        }
        shapes
    }
    fn edge(shape: &Shape, kind: Edge) -> Point {
        get_handles(shape)
            .iter()
            .find(|h| h.kind == HandleKind::Edge(kind))
            .unwrap()
            .position
    }
    fn opposite(kind: Edge) -> Edge {
        match kind {
            Edge::Left => Edge::Right,
            Edge::Right => Edge::Left,
            Edge::Top => Edge::Bottom,
            Edge::Bottom => Edge::Top,
        }
    }
    #[test]
    fn edge_resize_is_one_axis_with_fixed_opposite_edge_after_rotation() {
        for angle in [0.0, 0.73, std::f64::consts::FRAC_PI_2] {
            for shape in shapes(angle) {
                assert_eq!(
                    get_handles(&shape)
                        .iter()
                        .filter(|h| matches!(h.kind, HandleKind::Edge(_)))
                        .count(),
                    4
                );
                for kind in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
                    let delta = rotate_delta(kurbo::Vec2::new(12.0, 8.0), angle);
                    let next =
                        apply_manipulation(&shape, Some(HandleKind::Edge(kind)), delta, false);
                    let expected = if matches!(kind, Edge::Left | Edge::Right) {
                        kurbo::Vec2::new(12.0, 0.0)
                    } else {
                        kurbo::Vec2::new(0.0, 8.0)
                    };
                    assert!(
                        edge(&next, kind)
                            .distance(edge(&shape, kind) + rotate_delta(expected, angle))
                            < 1e-8
                    );
                    assert!(
                        edge(&next, opposite(kind)).distance(edge(&shape, opposite(kind))) < 1e-8
                    );
                    if matches!(kind, Edge::Left | Edge::Right) {
                        assert!((next.bounds().height() - shape.bounds().height()).abs() < 1e-8);
                    } else {
                        assert!((next.bounds().width() - shape.bounds().width()).abs() < 1e-8);
                    }
                }
            }
        }
    }
    #[test]
    fn edge_crop_preserves_other_axis_pixels_and_source() {
        for angle in [0.0, 0.73] {
            for kind in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
                let mut original = Image::new(Point::ZERO, &[1, 2, 3], 400, 200, ImageFormat::Png)
                    .with_size(200.0, 100.0);
                original.rotation = angle;
                let shape = Shape::Image(original.clone());
                let local = match kind {
                    Edge::Left => kurbo::Vec2::new(20.0, 15.0),
                    Edge::Right => kurbo::Vec2::new(-20.0, 15.0),
                    Edge::Top => kurbo::Vec2::new(20.0, 10.0),
                    Edge::Bottom => kurbo::Vec2::new(20.0, -10.0),
                };
                let next = apply_image_crop(
                    &shape,
                    Some(HandleKind::Edge(kind)),
                    rotate_delta(local, angle),
                );
                assert!(edge(&next, opposite(kind)).distance(edge(&shape, opposite(kind))) < 1e-8);
                let Shape::Image(image) = next else { panic!() };
                assert!(image.data_base64.shares_storage(&original.data_base64));
                assert!((image.width / image.crop.width() - 200.0).abs() < 1e-8);
                assert!((image.height / image.crop.height() - 100.0).abs() < 1e-8);
                if matches!(kind, Edge::Left | Edge::Right) {
                    assert_eq!(image.height, original.height);
                } else {
                    assert_eq!(image.width, original.width);
                }
            }
        }
    }
    #[test]
    fn text_math_display_scale_round_trips_and_keeps_content() {
        for shape in shapes(0.0)
            .into_iter()
            .filter(|s| matches!(s, Shape::Text(_) | Shape::Math(_)))
        {
            let next = apply_manipulation(
                &shape,
                Some(HandleKind::Edge(Edge::Right)),
                kurbo::Vec2::new(50.0, 0.0),
                false,
            );
            let value = serde_json::to_value(&next).unwrap();
            let restored: Shape = serde_json::from_value(value).unwrap();
            match (&next, &restored) {
                (Shape::Text(a), Shape::Text(b)) => {
                    assert_eq!(a.display_scale, b.display_scale);
                    assert_eq!(a.content, b.content);
                    assert_eq!(a.font_size, b.font_size);
                }
                (Shape::Math(a), Shape::Math(b)) => {
                    assert_eq!(a.display_scale, b.display_scale);
                    assert_eq!(a.latex, b.latex);
                    assert_eq!(a.font_size, b.font_size);
                }
                _ => panic!(),
            }
        }
    }
}

#[cfg(test)]
mod image_flip_tests {
    use super::*;
    use crate::shapes::{Image, ImageFormat};
    #[test]
    fn crossing_edges_mirrors_rotated_image_and_keeps_anchor() {
        for angle in [0.0, 0.7, 1.8] {
            let mut image = Image::new(
                Point::new(20.0, 30.0),
                &[1, 2, 3],
                100,
                80,
                ImageFormat::Png,
            );
            image.rotation = angle;
            let original = Shape::Image(image);
            let anchor = get_handles(&original)[0].position;
            let delta = rotate_delta(kurbo::Vec2::new(-140.0, -100.0), angle);
            let next = apply_manipulation(
                &original,
                Some(HandleKind::Corner(Corner::BottomRight)),
                delta,
                false,
            );
            let Shape::Image(image) = &next else { panic!() };
            assert!(image.flip_x && image.flip_y);
            assert!((image.width - 40.0).abs() < 1e-9 && (image.height - 20.0).abs() < 1e-9);
            assert!(get_handles(&next)[3].position.distance(anchor) < 1e-8);
            let restored: Shape =
                serde_json::from_str(&serde_json::to_string(&next).unwrap()).unwrap();
            let Shape::Image(restored) = restored else {
                panic!()
            };
            assert!(restored.flip_x && restored.flip_y);
            assert_eq!(restored.data(), image.data());
        }
    }
    #[test]
    fn mirrored_crop_moves_visible_edge_without_stretching() {
        let mut image = Image::new(Point::ZERO, &[1], 100, 80, ImageFormat::Png);
        image.flip_x = true;
        let cropped = apply_image_crop(
            &Shape::Image(image),
            Some(HandleKind::Edge(Edge::Left)),
            kurbo::Vec2::new(20.0, 0.0),
        );
        let Shape::Image(image) = cropped else {
            panic!()
        };
        assert!((image.crop.x1 - 0.8).abs() < 1e-9);
        assert!((image.position.x - 20.0).abs() < 1e-9);
        assert!((image.width - 80.0).abs() < 1e-9);
    }
}
