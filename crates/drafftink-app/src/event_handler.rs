//! Event handling for tool interactions.

use drafftink_core::canvas::Canvas;
use drafftink_core::input::InputState;
use drafftink_core::selection::{Corner, HandleKind};
use drafftink_core::selection::{
    HANDLE_HIT_TOLERANCE, ManipulationState, MultiMoveState, apply_image_crop, apply_manipulation,
    apply_rotation_from_drag, get_handles, get_manipulation_target_position, hit_test_boundary,
    hit_test_handles,
};
use drafftink_core::shapes::{
    Freehand, Math, Shape, ShapeId, ShapeStyle, ShapeTrait, Sloppiness, Text,
};
use drafftink_core::snap::{
    AngleSnapResult, ENDPOINT_SNAP_RADIUS, GRID_SIZE, MULTI_MOVE_SNAP_RADIUS,
    SMART_GUIDE_THRESHOLD, SmartGuide, SnapResult, detect_smart_guides,
    detect_smart_guides_for_point, snap_line_endpoint_isometric, snap_ray_to_smart_guides,
    snap_to_grid,
};
use drafftink_core::tools::{EraserMode, ToolKind};
use kurbo::{Point, Rect, Size};

/// Maximum number of snap candidates (like Inkscape's limit of 200).
const MAX_SNAP_CANDIDATES: usize = 200;

fn constrain_square_endpoint(start: Point, end: Point) -> Point {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let side = dx.abs().max(dy.abs());
    let sx = if dx < 0.0 { -1.0 } else { 1.0 };
    let sy = if dy < 0.0 { -1.0 } else { 1.0 };
    Point::new(start.x + sx * side, start.y + sy * side)
}

fn point_segment_distance(point: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len_sq = ab.hypot2();
    if len_sq <= f64::EPSILON {
        return (point - a).hypot();
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    (point - projection).hypot()
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    const EPS: f64 = 1e-9;
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);

    if ((o1 > EPS && o2 < -EPS) || (o1 < -EPS && o2 > EPS))
        && ((o3 > EPS && o4 < -EPS) || (o3 < -EPS && o4 > EPS))
    {
        return true;
    }

    // Collinear/touching cases are covered by a tiny point-to-segment tolerance.
    point_segment_distance(c, a, b) <= EPS
        || point_segment_distance(d, a, b) <= EPS
        || point_segment_distance(a, c, d) <= EPS
        || point_segment_distance(b, c, d) <= EPS
}

fn segment_segment_distance(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if segments_intersect(a, b, c, d) {
        0.0
    } else {
        point_segment_distance(a, c, d)
            .min(point_segment_distance(b, c, d))
            .min(point_segment_distance(c, a, b))
            .min(point_segment_distance(d, a, b))
    }
}

/// Cut a freehand/highlighter stroke where the eraser capsule crosses it.
/// Returns None when the stroke is untouched; otherwise returns the remaining fragments.
fn split_freehand_by_eraser(
    freehand: &Freehand,
    erase_start: Point,
    erase_end: Point,
    radius: f64,
) -> Option<Vec<Shape>> {
    if freehand.closed || freehand.points.len() < 2 {
        return None;
    }

    let threshold = radius + freehand.style.stroke_width / 2.0;
    let has_pressure = freehand.pressures.len() == freehand.points.len();
    let mut changed = false;
    let mut fragments: Vec<(Vec<Point>, Vec<f64>)> = Vec::new();
    let mut points: Vec<Point> = Vec::new();
    let mut pressures: Vec<f64> = Vec::new();

    let mut flush = |points: &mut Vec<Point>, pressures: &mut Vec<f64>| {
        if points.len() >= 2 {
            fragments.push((std::mem::take(points), std::mem::take(pressures)));
        } else {
            points.clear();
            pressures.clear();
        }
    };

    for i in 0..freehand.points.len() - 1 {
        let a = freehand.points[i];
        let b = freehand.points[i + 1];
        let erased = segment_segment_distance(a, b, erase_start, erase_end) <= threshold;

        if erased {
            changed = true;
            flush(&mut points, &mut pressures);
            continue;
        }

        if points.is_empty() {
            points.push(a);
            if has_pressure {
                pressures.push(freehand.pressures[i]);
            }
        }
        points.push(b);
        if has_pressure {
            pressures.push(freehand.pressures[i + 1]);
        }
    }
    flush(&mut points, &mut pressures);

    if !changed {
        return None;
    }

    let remaining = fragments
        .into_iter()
        .map(|(pts, prs)| {
            let mut fragment = if has_pressure {
                Freehand::from_points_with_pressure(pts, prs)
            } else {
                Freehand::from_points(pts)
            };
            fragment.style = freehand.style.clone();
            Shape::Freehand(fragment)
        })
        .collect();

    Some(remaining)
}

/// Turn only a touched geometric contour into editable vector fragments.
/// Subdivide long straight edges too: erasing their middle must not delete the whole edge.
fn split_contour_by_eraser(shape: &Shape, a: Point, b: Point, radius: f64) -> Option<Vec<Shape>> {
    if !matches!(
        shape,
        Shape::Line(_) | Shape::Arrow(_) | Shape::Rectangle(_) | Shape::Ellipse(_)
    ) && !matches!(shape, Shape::Freehand(f) if f.closed)
    {
        return None;
    }
    let eraser_bounds = Rect::from_points(a, b).inflate(
        radius + shape.style().stroke_width,
        radius + shape.style().stroke_width,
    );
    let bounds = shape.bounds();
    let diagonal = bounds.width().hypot(bounds.height()) / 2.0;
    if bounds
        .inflate(diagonal, diagonal)
        .intersect(eraser_bounds)
        .area()
        <= 0.0
    {
        return None;
    }
    let center = shape.bounds().center().to_vec2();
    let transform = kurbo::Affine::translate(center)
        * kurbo::Affine::rotate(shape.rotation())
        * kurbo::Affine::translate(-center);
    let path = transform * shape.to_path();
    let mut contours: Vec<Vec<Point>> = Vec::new();
    let mut points = Vec::new();
    let mut start = Point::ZERO;
    kurbo::flatten(path.iter(), 0.2, |element| {
        let endpoint = match element {
            kurbo::PathEl::MoveTo(p) => {
                if points.len() > 1 {
                    contours.push(std::mem::take(&mut points));
                } else {
                    points.clear();
                }
                start = p;
                points.push(p);
                return;
            }
            kurbo::PathEl::LineTo(p) => p,
            kurbo::PathEl::ClosePath => start,
            _ => return,
        };
        if let Some(&previous) = points.last() {
            let steps = ((endpoint - previous).hypot() / (radius * 0.2).clamp(0.5, 2.0))
                .ceil()
                .max(1.0) as usize;
            for i in 1..=steps {
                points.push(previous + (endpoint - previous) * (i as f64 / steps as f64));
            }
        }
    });
    if points.len() > 1 {
        contours.push(points);
    }
    let mut changed = false;
    let mut result = Vec::new();
    for points in contours {
        if shape.style().fill_color.is_some()
            && (matches!(shape, Shape::Rectangle(_) | Shape::Ellipse(_))
                || matches!(shape, Shape::Freehand(f) if f.closed))
        {
            let (cut, polygons) = subtract_eraser_capsule(&points, a, b, radius);
            changed |= cut;
            for polygon in polygons {
                let mut fill = Freehand::from_points(polygon);
                fill.closed = true;
                fill.style = shape.style().clone();
                fill.style.stroke_width = 0.0;
                fill.style.stroke_color.a = 0;
                result.push(Shape::Freehand(fill));
            }
        }
        let mut contour = Freehand::from_points(points);
        contour.style = shape.style().clone();
        contour.style.fill_color = None;
        if let Some(parts) = split_freehand_by_eraser(&contour, a, b, radius) {
            changed = true;
            result.extend(parts);
        } else {
            result.push(Shape::Freehand(contour));
        }
    }
    changed.then_some(result)
}

/// Subtract a convex capsule from a filled convex geometric contour. Keep the
/// remaining fill as closed vector pieces, without inventing strokes along cuts.
fn subtract_eraser_capsule(
    points: &[Point],
    a: Point,
    b: Point,
    radius: f64,
) -> (bool, Vec<Vec<Point>>) {
    fn half_plane(points: &[Point], a: Point, b: Point, inside: bool) -> Vec<Point> {
        let mut out = Vec::new();
        let Some(&mut_previous) = points.last() else {
            return out;
        };
        let mut previous = mut_previous;
        for &current in points {
            let dp = orientation(a, b, previous);
            let dc = orientation(a, b, current);
            let kp = if inside { dp >= 0.0 } else { dp <= 0.0 };
            let kc = if inside { dc >= 0.0 } else { dc <= 0.0 };
            if kp != kc && (dp - dc).abs() > 1e-10 {
                out.push(previous + (current - previous) * (dp / (dp - dc)));
            }
            if kc {
                out.push(current);
            }
            previous = current;
        }
        out
    }
    let angle = (b.y - a.y).atan2(b.x - a.x);
    let mut capsule = Vec::new();
    for (center, start) in [
        (b, angle - std::f64::consts::FRAC_PI_2),
        (a, angle + std::f64::consts::FRAC_PI_2),
    ] {
        for i in 0..=16 {
            let angle = start + std::f64::consts::PI * i as f64 / 16.0;
            capsule.push(center + kurbo::Vec2::new(angle.cos(), angle.sin()) * radius);
        }
    }
    let mut pending = points.to_vec();
    let mut remaining = Vec::new();
    for i in 0..capsule.len() {
        let x = capsule[i];
        let y = capsule[(i + 1) % capsule.len()];
        if (y - x).hypot() < 1e-8 {
            continue;
        }
        let outside = half_plane(&pending, x, y, false);
        if outside.len() >= 3 {
            remaining.push(outside);
        }
        pending = half_plane(&pending, x, y, true);
        if pending.len() < 3 {
            return (false, vec![points.to_vec()]);
        }
    }
    (true, remaining)
}

/// Get snap points from a line/arrow's own endpoints, excluding the one being dragged.
fn self_snap_rects(shape: &Shape, handle: Option<HandleKind>) -> Vec<Rect> {
    let points = shape.snap_points();
    if points.is_empty() {
        return Vec::new();
    }
    let dragged_idx = match handle {
        Some(HandleKind::Endpoint(idx)) => Some(idx),
        Some(HandleKind::IntermediatePoint(idx)) => Some(idx + 1), // offset by 1 for start
        _ => None,
    };
    points
        .iter()
        .enumerate()
        .filter(|(i, _)| dragged_idx.is_none_or(|d| *i != d))
        .map(|(_, pt)| Rect::new(pt.x, pt.y, pt.x, pt.y))
        .collect()
}

/// Collect snap candidate bounds with viewport culling, proximity filtering, and limit.
/// `exclude_ids` - shape IDs to exclude (e.g., the shape being dragged)
/// `snap_zone` - expanded bounds around the dragged object for proximity filtering
fn collect_snap_candidates(
    canvas: &Canvas,
    exclude_ids: &[ShapeId],
    snap_zone: Option<Rect>,
) -> Vec<Rect> {
    let viewport = canvas.visible_world_bounds();
    let mut rects: Vec<Rect> = Vec::new();
    for shape in canvas
        .document
        .shapes_ordered()
        .filter(|s| !exclude_ids.contains(&s.id()))
        .filter(|s| {
            !viewport
                .intersect(s.bounds().inflate(1.0, 1.0))
                .is_zero_area()
        })
        .filter(|s| {
            snap_zone.is_none_or(|z| !z.intersect(s.bounds().inflate(1.0, 1.0)).is_zero_area())
        })
        .take(MAX_SNAP_CANDIDATES)
    {
        rects.push(shape.bounds());
        // Add individual segment endpoints for lines/arrows
        for pt in shape.snap_points() {
            rects.push(Rect::new(pt.x, pt.y, pt.x, pt.y));
        }
    }
    rects
}

/// Get the other endpoint of a line/arrow given the handle being manipulated.
/// Returns the start point if manipulating the end, and vice versa.
fn get_line_other_endpoint(shape: &Shape, handle: Option<HandleKind>) -> Point {
    match shape {
        Shape::Line(line) => {
            match handle {
                Some(HandleKind::Endpoint(0)) => line.end, // Manipulating start -> return end
                Some(HandleKind::Endpoint(1)) => line.start, // Manipulating end -> return start
                _ => line.start,                           // Default to start
            }
        }
        Shape::Arrow(arrow) => {
            match handle {
                Some(HandleKind::Endpoint(0)) => arrow.end, // Manipulating start -> return end
                Some(HandleKind::Endpoint(1)) => arrow.start, // Manipulating end -> return start
                _ => arrow.start,                           // Default to start
            }
        }
        _ => Point::ZERO, // Not a line/arrow
    }
}

/// Selection rectangle state for marquee selection.
#[derive(Debug, Clone)]
pub struct SelectionRect {
    /// Starting point in world coordinates.
    pub start: Point,
    /// Current point in world coordinates.
    pub current: Point,
}

impl SelectionRect {
    /// Get the selection rectangle as a Rect.
    pub fn to_rect(&self) -> Rect {
        Rect::new(
            self.start.x.min(self.current.x),
            self.start.y.min(self.current.y),
            self.start.x.max(self.current.x),
            self.start.y.max(self.current.y),
        )
    }
}

/// Handles high-level events and translates them to canvas operations.
pub struct EventHandler {
    /// Current manipulation state (when dragging a handle on a single shape).
    manipulation: Option<ManipulationState>,
    /// Current multi-move state (when moving multiple selected shapes).
    multi_move: Option<MultiMoveState>,
    /// Selection rectangle for marquee selection.
    selection_rect: Option<SelectionRect>,
    /// Shape ID being edited (for text editing).
    pub editing_text: Option<ShapeId>,
    pub text_font: drafftink_core::shapes::TextFont,
    /// Original top-left handle position when editing started.
    pub text_edit_anchor: Option<Point>,
    /// Original size (width, height) when editing started.
    text_edit_size: Option<(f64, f64)>,
    /// Last snap result (for rendering snap guides).
    pub last_snap: Option<SnapResult>,
    /// Last angle snap result (for rendering angle guides).
    pub last_angle_snap: Option<AngleSnapResult>,
    /// Start point for line/arrow drawing (for angle snap visualization).
    pub line_start_point: Option<Point>,
    /// Start point for rectangle/ellipse Shift-constrained drawing.
    shape_start_point: Option<Point>,
    /// Current smart guides (for rendering).
    pub smart_guides: Vec<SmartGuide>,
    /// Current rotation angle during rotation drag (for helper line rendering).
    pub rotation_state: Option<RotationState>,
    /// Eraser path points for current stroke.
    eraser_points: Vec<Point>,
    /// Eraser radius for hit detection.
    pub eraser_radius: f64,
    /// Current eraser behavior.
    pub eraser_mode: EraserMode,
    /// Whether an undo snapshot has already been created for the current eraser stroke.
    eraser_undo_started: bool,
    /// Laser pointer position (for rendering).
    pub laser_position: Option<Point>,
    /// Laser pointer trail for fading effect.
    pub laser_trail: Vec<(Point, f64)>,
    /// Math shape ID to open editor for (set on double-click).
    pub pending_math_edit: Option<ShapeId>,
}

/// State for rotation drag operation.
#[derive(Debug, Clone)]
pub struct RotationState {
    /// Center of the shape being rotated.
    pub center: Point,
    /// Current rotation angle in radians.
    pub angle: f64,
    /// Whether snapping to 15° increments.
    pub snapped: bool,
}

impl EventHandler {
    /// Create a new event handler.
    pub fn new() -> Self {
        Self {
            manipulation: None,
            multi_move: None,
            selection_rect: None,
            editing_text: None,
            text_font: Default::default(),
            text_edit_anchor: None,
            text_edit_size: None,
            last_snap: None,
            last_angle_snap: None,
            line_start_point: None,
            shape_start_point: None,
            smart_guides: Vec::new(),
            rotation_state: None,
            eraser_points: Vec::new(),
            eraser_radius: 10.0,
            eraser_mode: EraserMode::Classic,
            eraser_undo_started: false,
            laser_position: None,
            laser_trail: Vec::new(),
            pending_math_edit: None,
        }
    }

    /// Check if a manipulation is in progress.
    pub fn is_moving_shapes(&self) -> bool {
        self.multi_move.is_some()
            || self
                .manipulation
                .as_ref()
                .is_some_and(|m| m.handle.is_none())
    }
    pub fn is_manipulating(&self) -> bool {
        self.manipulation.is_some() || self.multi_move.is_some()
    }

    /// Check if a selection rectangle is active.
    pub fn is_selecting(&self) -> bool {
        self.selection_rect.is_some()
    }

    /// Cancel any ongoing operation.
    pub fn cancel(&mut self, canvas: &mut Canvas) {
        // Restore original shapes if manipulating
        if let Some(manip) = self.manipulation.take() {
            if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                *shape = manip.original_shape;
            }
        }
        if let Some(mm) = self.multi_move.take() {
            for (id, original) in mm.original_shapes {
                if let Some(shape) = canvas.document.get_shape_mut(id) {
                    *shape = original;
                }
            }
        }
        self.selection_rect = None;
        self.last_snap = None;
        self.last_angle_snap = None;
        self.shape_start_point = None;
        self.rotation_state = None;
        canvas.tool_manager.cancel();
    }

    /// Get the current selection rectangle (for rendering).
    pub fn selection_rect(&self) -> Option<&SelectionRect> {
        self.selection_rect.as_ref()
    }

    /// Get the current manipulation state.
    #[allow(dead_code)]
    pub fn manipulation(&self) -> Option<&ManipulationState> {
        self.manipulation.as_ref()
    }

    /// Enter text editing mode for a shape.
    /// This updates both the local state and the Canvas's WidgetManager.
    /// Stores the top-left handle position and size to preserve anchor after editing.
    pub fn enter_text_edit(&mut self, canvas: &mut Canvas, id: ShapeId) {
        if let Some(shape) = canvas.document.get_shape(id) {
            // Store the actual top-left handle position
            self.text_edit_anchor = get_handles(shape)
                .into_iter()
                .find(|h| matches!(h.kind, HandleKind::Corner(Corner::TopLeft)))
                .map(|h| h.position);
            // Store the current bounds size
            let bounds = shape.bounds();
            self.text_edit_size = Some((bounds.width(), bounds.height()));
        }
        self.editing_text = Some(id);
        canvas.enter_text_editing(id);
    }

    /// Exit text editing mode.
    /// This updates both the local state and the Canvas's WidgetManager.
    /// If the text is empty, the shape is deleted.
    /// Preserves top-left handle position using current size.
    pub fn exit_text_edit(&mut self, canvas: &mut Canvas) {
        if let Some(id) = self.editing_text {
            let should_delete = canvas
                .document
                .get_shape(id)
                .map(|shape| {
                    if let Shape::Text(text) = shape {
                        text.content.trim().is_empty()
                    } else {
                        false
                    }
                })
                .unwrap_or(false);

            if should_delete {
                canvas.remove_shape(id);
            } else if let Some(anchor) = self.text_edit_anchor {
                // Update position to keep anchor fixed with current size
                if let Some(Shape::Text(text)) = canvas.document.get_shape_mut(id) {
                    let bounds = text.bounds();
                    let half_w = bounds.width() / 2.0;
                    let half_h = bounds.height() / 2.0;
                    let rotation = text.rotation;

                    if rotation.abs() > 0.001 {
                        let cos_r = rotation.cos();
                        let sin_r = rotation.sin();
                        let rot_x = -half_w * cos_r + half_h * sin_r;
                        let rot_y = -half_w * sin_r - half_h * cos_r;
                        text.position =
                            Point::new(anchor.x - half_w - rot_x, anchor.y - half_h - rot_y);
                    } else {
                        text.position = anchor;
                    }
                }
            }
        }

        self.editing_text = None;
        self.text_edit_anchor = None;
        self.text_edit_size = None;
        canvas.exit_text_editing();
    }

    /// Check if currently editing text.
    #[allow(dead_code)]
    pub fn is_editing_text(&self) -> bool {
        self.editing_text.is_some()
    }

    /// Determine the cursor type based on hover position.
    /// Returns: None = default, Some(None) = move, Some(Some(handle)) = resize with direction
    pub fn get_cursor_for_position(
        &self,
        canvas: &Canvas,
        world_point: Point,
    ) -> Option<Option<HandleKind>> {
        let handle_tolerance = HANDLE_HIT_TOLERANCE / canvas.camera.zoom;
        let boundary_tolerance = 8.0 / canvas.camera.zoom;

        match canvas.tool_manager.current_tool {
            ToolKind::Text => {
                let hits = canvas
                    .document
                    .shapes_at_point(world_point, 5.0 / canvas.camera.zoom);
                if let Some(&id) = hits.first() {
                    if let Some(shape @ Shape::Text(_)) = canvas.document.get_shape(id) {
                        if let Some(handle) = hit_test_handles(shape, world_point, handle_tolerance)
                        {
                            return Some(Some(handle));
                        }
                        if hit_test_boundary(shape, world_point, boundary_tolerance) {
                            return Some(None); // move
                        }
                    }
                }
            }
            ToolKind::Select => {
                for &shape_id in &canvas.selection {
                    if let Some(shape) = canvas.document.get_shape(shape_id) {
                        if let Some(handle) = hit_test_handles(shape, world_point, handle_tolerance)
                        {
                            return Some(Some(handle));
                        }
                    }
                }
                let hits = canvas
                    .document
                    .shapes_at_point(world_point, 5.0 / canvas.camera.zoom);
                if !hits.is_empty() {
                    return Some(None); // move
                }
            }
            _ => {}
        }
        None
    }

    /// Handle a press event (mouse down).
    /// `grid_snap_enabled` controls whether the start point should snap to grid.
    pub fn handle_press(
        &mut self,
        canvas: &mut Canvas,
        world_point: Point,
        input: &InputState,
        grid_snap_enabled: bool,
    ) {
        let tool = canvas.tool_manager.current_tool;
        if matches!(
            tool,
            ToolKind::Freehand
                | ToolKind::Highlighter
                | ToolKind::Rectangle
                | ToolKind::Ellipse
                | ToolKind::Line
                | ToolKind::Arrow
        ) {
            let selected_hit = canvas.selection.iter().any(|id| {
                canvas.document.get_shape(*id).is_some_and(|shape| {
                    hit_test_handles(
                        shape,
                        world_point,
                        HANDLE_HIT_TOLERANCE / canvas.camera.zoom,
                    )
                    .is_some()
                        || shape.hit_test(world_point, 3.0 / canvas.camera.zoom)
                })
            });
            if selected_hit {
                canvas.tool_manager.current_tool = ToolKind::Select;
                self.handle_press(canvas, world_point, input, grid_snap_enabled);
                canvas.tool_manager.current_tool = tool;
                return;
            }
        }
        // If we're editing text and click elsewhere, stop editing
        if self.editing_text.is_some() {
            let hits = canvas
                .document
                .shapes_at_point(world_point, 5.0 / canvas.camera.zoom);
            let clicked_on_editing = hits
                .first()
                .map(|&id| Some(id) == self.editing_text)
                .unwrap_or(false);
            if !clicked_on_editing {
                self.exit_text_edit(canvas);
            } else {
                // Clicked on the text we're editing - stay in edit mode
                return;
            }
        }

        match canvas.tool_manager.current_tool {
            ToolKind::Text => {
                // Text tool: check if clicking on existing text
                let hits = canvas
                    .document
                    .shapes_at_point(world_point, 5.0 / canvas.camera.zoom);
                if let Some(&id) = hits.first() {
                    if let Some(shape @ Shape::Text(_)) = canvas.document.get_shape(id) {
                        let boundary_tolerance = 8.0 / canvas.camera.zoom;
                        let handle_tolerance = HANDLE_HIT_TOLERANCE / canvas.camera.zoom;

                        // Check for handle hit first (for scaling)
                        if let Some(handle_kind) =
                            hit_test_handles(shape, world_point, handle_tolerance)
                        {
                            // Start handle manipulation (scale)
                            self.manipulation = Some(ManipulationState::new(
                                id,
                                Some(handle_kind),
                                world_point,
                                shape.clone(),
                            ));
                            canvas.clear_selection();
                            canvas.select(id);
                            return;
                        }

                        // Check for boundary hit (for dragging)
                        if hit_test_boundary(shape, world_point, boundary_tolerance) {
                            // Start move operation
                            let mut original_shapes = std::collections::HashMap::new();
                            original_shapes.insert(id, shape.clone());
                            self.multi_move =
                                Some(MultiMoveState::new(world_point, original_shapes));
                            canvas.clear_selection();
                            canvas.select(id);
                            return;
                        }

                        // Click inside text - enter edit mode
                        self.enter_text_edit(canvas, id);
                        canvas.clear_selection();
                        canvas.select(id);
                    }
                }
                // If not clicking on text, will create new text on release
            }
            ToolKind::Select => {
                // Check for double-click on text shape to enter edit mode
                if input.is_double_click() {
                    let hits = canvas
                        .document
                        .shapes_at_point(world_point, 5.0 / canvas.camera.zoom);
                    if let Some(&id) = hits.first() {
                        if let Some(Shape::Text(_)) = canvas.document.get_shape(id) {
                            // Double-click on text - enter edit mode
                            self.enter_text_edit(canvas, id);
                            canvas.clear_selection();
                            canvas.select(id);
                            return;
                        }
                        if let Some(Shape::Math(_)) = canvas.document.get_shape(id) {
                            // Double-click on math - open editor
                            self.pending_math_edit = Some(id);
                            canvas.clear_selection();
                            canvas.select(id);
                            return;
                        }
                    }

                    // Check for double-click on rotation handle to reset rotation
                    let handle_tolerance = HANDLE_HIT_TOLERANCE / canvas.camera.zoom;
                    for &shape_id in &canvas.selection {
                        if let Some(shape) = canvas.document.get_shape(shape_id) {
                            if let Some(HandleKind::Rotate) =
                                hit_test_handles(shape, world_point, handle_tolerance)
                            {
                                // Double-click on rotation handle - reset to 0°
                                canvas.document.push_undo();
                                if let Some(shape) = canvas.document.get_shape_mut(shape_id) {
                                    shape.set_rotation(0.0);
                                }
                                return;
                            }
                        }
                    }
                }

                // First, check if we clicked on a handle of a selected shape
                let handle_tolerance = HANDLE_HIT_TOLERANCE / canvas.camera.zoom;

                for &shape_id in &canvas.selection {
                    if let Some(shape) = canvas.document.get_shape(shape_id) {
                        if let Some(handle_kind) =
                            hit_test_handles(shape, world_point, handle_tolerance)
                        {
                            // Start handle manipulation
                            self.manipulation = Some(ManipulationState::new(
                                shape_id,
                                Some(handle_kind),
                                world_point,
                                shape.clone(),
                            ));
                            return;
                        }
                    }
                }

                // Check for shape hit (for selection or move)
                let hits = canvas
                    .document
                    .shapes_at_point(world_point, 5.0 / canvas.camera.zoom);
                if let Some(&id) = hits.first() {
                    if input.shift() {
                        // Add to/toggle selection
                        if canvas.is_selected(id) {
                            canvas.selection.retain(|&s| s != id);
                        } else {
                            canvas.add_to_selection(id);
                        }
                    } else {
                        // If clicking on an already selected shape, start moving it
                        // Otherwise, replace selection
                        if !canvas.is_selected(id) {
                            canvas.clear_selection();
                            canvas.select(id);
                        }

                        // Start move - use MultiMoveState for all selected shapes
                        let mut original_shapes = std::collections::HashMap::new();
                        for &shape_id in &canvas.selection {
                            if let Some(shape) = canvas.document.get_shape(shape_id) {
                                original_shapes.insert(shape_id, shape.clone());
                            }
                        }

                        if !original_shapes.is_empty() {
                            // Alt/Option + drag = duplicate
                            if input.alt() {
                                let mut mm =
                                    MultiMoveState::new_duplicate(world_point, original_shapes);
                                // Create duplicates immediately with new IDs
                                for shape in mm.original_shapes.values() {
                                    let mut new_shape = shape.clone();
                                    new_shape.regenerate_id();
                                    let new_id = new_shape.id();
                                    mm.duplicated_ids.push(new_id);
                                    canvas.document.add_shape(new_shape);
                                }
                                // Update selection to the duplicates
                                canvas.clear_selection();
                                for &new_id in &mm.duplicated_ids {
                                    canvas.add_to_selection(new_id);
                                }
                                self.multi_move = Some(mm);
                            } else {
                                self.multi_move =
                                    Some(MultiMoveState::new(world_point, original_shapes));
                            }
                        }
                    }
                } else {
                    // Clicked on empty space - start selection rectangle
                    if !input.shift() {
                        canvas.clear_selection();
                    }
                    self.selection_rect = Some(SelectionRect {
                        start: world_point,
                        current: world_point,
                    });
                }
            }
            ToolKind::Pan => {
                // Pan is handled in mouse move
            }
            ToolKind::Freehand | ToolKind::Highlighter => {
                // Freehand/Highlighter doesn't snap - would be too jerky
                canvas.tool_manager.begin(world_point);
            }
            ToolKind::Eraser => {
                // Start eraser stroke
                self.eraser_points.clear();
                self.eraser_points.push(world_point);
                self.eraser_undo_started = false;
            }
            ToolKind::LaserPointer => {
                // Laser pointer just updates position
                self.laser_position = Some(world_point);
                self.laser_trail.push((world_point, 1.0));
            }
            ToolKind::Line | ToolKind::Arrow => {
                // Start line/arrow drawing - snap start point if enabled
                // Store start point for angle snapping visualization
                let start_point = if grid_snap_enabled {
                    let snap_result = snap_to_grid(world_point, GRID_SIZE);
                    self.last_snap = Some(snap_result);
                    snap_result.point
                } else {
                    world_point
                };
                self.line_start_point = Some(start_point);
                canvas.tool_manager.begin(start_point);
            }
            _ => {
                // Start shape drawing - snap start point if enabled
                let start_point = if grid_snap_enabled {
                    let snap_result = snap_to_grid(world_point, GRID_SIZE);
                    self.last_snap = Some(snap_result);
                    snap_result.point
                } else {
                    world_point
                };
                self.line_start_point = None; // Clear for non-line tools
                self.shape_start_point = if matches!(
                    canvas.tool_manager.current_tool,
                    ToolKind::Rectangle | ToolKind::Ellipse
                ) {
                    Some(start_point)
                } else {
                    None
                };
                canvas.tool_manager.begin(start_point);
            }
        }
    }

    /// Handle a release event (mouse up).
    /// `grid_snap_enabled` controls whether the end point should snap to grid.
    /// `angle_snap_enabled` enables 15° angle snapping for lines/arrows.
    pub fn handle_release(
        &mut self,
        canvas: &mut Canvas,
        world_point: Point,
        input: &InputState,
        current_style: &ShapeStyle,
        grid_snap_enabled: bool,
        angle_snap_enabled: bool,
    ) {
        // Clear rotation state
        self.rotation_state = None;

        // If we were manipulating a single shape (handle resize), finalize it
        if let Some(manip) = self.manipulation.take() {
            // Check if this was a rotation operation
            if matches!(manip.handle, Some(HandleKind::Rotate)) {
                // Rotation was already applied during drag, just need to push undo
                // Get the current rotation from the shape
                if let Some(shape) = canvas.document.get_shape(manip.shape_id) {
                    let current_rotation = shape.rotation();
                    let original_rotation = manip.original_shape.rotation();

                    // Only push undo if rotation actually changed
                    if (current_rotation - original_rotation).abs() > 0.001 {
                        // Restore original, push undo, then re-apply current rotation
                        if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                            *shape = manip.original_shape.clone();
                        }
                        canvas.document.push_undo();
                        if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                            shape.set_rotation(current_rotation);
                        }
                    }
                }
                return;
            }

            // Use manip.current_point which was already snapped during drag,
            // NOT the raw world_point which would cause a jump on release
            let delta = manip.delta();

            // Check if shape actually changed (delta is non-zero)
            if delta.x.abs() > 0.1 || delta.y.abs() > 0.1 {
                if let Some(final_shape) = canvas.document.get_shape(manip.shape_id).cloned() {
                    if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                        *shape = manip.original_shape;
                    }
                    canvas.document.push_undo();
                    if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                        *shape = final_shape;
                    }
                }
            }
            return;
        }

        // If we were moving multiple shapes, finalize them
        if let Some(mm) = self.multi_move.take() {
            let delta = mm.delta();

            if mm.is_duplicate {
                // Duplicate mode: shapes were already created, just need to finalize their positions
                // Check if actually moved
                if delta.x.abs() > 0.1 || delta.y.abs() > 0.1 {
                    // Push undo for the duplicate operation
                    canvas.document.push_undo();
                    let translation = kurbo::Affine::translate(delta);
                    for (idx, &dup_id) in mm.duplicated_ids.iter().enumerate() {
                        let original_shape = mm.original_shapes.values().nth(idx);
                        if let Some(orig) = original_shape {
                            let mut new_shape = orig.clone();
                            new_shape.transform(translation);
                            if let Some(shape) = canvas.document.get_shape_mut(dup_id) {
                                *shape = new_shape;
                            }
                        }
                    }
                } else {
                    // No movement - remove the duplicates (cancelled)
                    for &dup_id in &mm.duplicated_ids {
                        canvas.document.remove_shape(dup_id);
                    }
                    // Restore original selection
                    canvas.clear_selection();
                    for &id in mm.original_shapes.keys() {
                        canvas.add_to_selection(id);
                    }
                }
            } else {
                // Normal move mode
                // Check if shapes actually moved (delta is non-zero)
                if delta.x.abs() > 0.1 || delta.y.abs() > 0.1 {
                    // First restore all original shapes
                    for (shape_id, original_shape) in &mm.original_shapes {
                        if let Some(shape) = canvas.document.get_shape_mut(*shape_id) {
                            *shape = original_shape.clone();
                        }
                    }

                    // Now push undo and apply the final changes
                    canvas.document.push_undo();
                    let translation = kurbo::Affine::translate(delta);
                    for (shape_id, original_shape) in &mm.original_shapes {
                        let mut new_shape = original_shape.clone();
                        new_shape.transform(translation);
                        if let Some(shape) = canvas.document.get_shape_mut(*shape_id) {
                            *shape = new_shape;
                        }
                    }
                }
            }
            return;
        }

        // If we were doing marquee selection, finalize it
        if let Some(sel_rect) = self.selection_rect.take() {
            let rect = sel_rect.to_rect();
            // Only select if rectangle has meaningful size
            if rect.width() > 2.0 && rect.height() > 2.0 {
                let shapes_in_rect = canvas.document.shapes_in_rect(rect);
                if !input.shift() {
                    canvas.clear_selection();
                }
                for id in shapes_in_rect {
                    canvas.add_to_selection(id);
                }
            }
            return;
        }

        match canvas.tool_manager.current_tool {
            ToolKind::Select | ToolKind::Pan => {
                // Nothing to do
            }
            ToolKind::Freehand => {
                let points = canvas.tool_manager.freehand_points();
                let pressures = canvas.tool_manager.freehand_pressures();
                if points.len() >= 2 {
                    // Always use pressure data for consistent rendering
                    let mut freehand =
                        Freehand::from_points_with_pressure(points.to_vec(), pressures.to_vec());
                    freehand.simplify(2.0); // Simplify the path
                    freehand.style = current_style.clone(); // Apply current style
                    canvas.document.push_undo();
                    let id = freehand.id();
                    canvas.document.add_shape(Shape::Freehand(freehand));
                    canvas.select(id);
                }
                canvas.tool_manager.cancel();
            }
            ToolKind::Highlighter => {
                let points = canvas.tool_manager.freehand_points();
                let pressures = canvas.tool_manager.freehand_pressures();
                if points.len() >= 2 {
                    // Always use pressure data for consistent rendering
                    let mut freehand =
                        Freehand::from_points_with_pressure(points.to_vec(), pressures.to_vec());
                    freehand.simplify(2.0);
                    // Highlighter: wider stroke, semi-transparent
                    freehand.style = current_style.clone();
                    freehand.style.stroke_width = current_style.stroke_width.max(12.0);
                    freehand.style.stroke_color.a = 128; // 50% opacity
                    canvas.document.push_undo();
                    let id = freehand.id();
                    canvas.document.add_shape(Shape::Freehand(freehand));
                    canvas.select(id);
                }
                canvas.tool_manager.cancel();
            }
            ToolKind::Eraser => {
                // Erasing happens during drag; finish the current undo group.
                self.eraser_points.clear();
                self.eraser_undo_started = false;
            }
            ToolKind::LaserPointer => {
                // Laser pointer doesn't create anything, just clear position
                // Trail will fade out over time
            }
            ToolKind::Text => {
                // If we're already editing text, don't create new text
                if self.editing_text.is_some() {
                    return;
                }

                // Text tool: create text at click position with empty content
                let mut text = Text::new(world_point, String::new());
                text.style = current_style.clone();
                self.text_font.apply(&mut text);
                let shape = Shape::Text(text);
                let shape_id = shape.id();
                canvas.document.push_undo();
                canvas.document.add_shape(shape);
                // Enter edit mode for the new text
                canvas.clear_selection();
                canvas.add_to_selection(shape_id);
                self.enter_text_edit(canvas, shape_id);
                canvas.tool_manager.cancel();
            }
            ToolKind::Math => {
                // A consumed outside click never began a tool interaction.
                if matches!(
                    canvas.tool_manager.state,
                    drafftink_core::tools::ToolState::Idle
                ) {
                    return;
                }
                // Math tool: create an equation and immediately request the formula editor.
                let mut math = Math::new(world_point, String::new());
                math.style = current_style.clone();
                let shape = Shape::Math(math);
                let shape_id = shape.id();
                canvas.document.push_undo();
                canvas.document.add_shape(shape);
                canvas.clear_selection();
                canvas.add_to_selection(shape_id);
                self.pending_math_edit = Some(shape_id);
                canvas.tool_manager.cancel();
            }
            ToolKind::Line | ToolKind::Arrow => {
                // Complete line/arrow - use angle/grid snapping
                let end_point = if let Some(start) = self.line_start_point {
                    let angle_result = snap_line_endpoint_isometric(
                        start,
                        world_point,
                        angle_snap_enabled || input.shift(),
                        grid_snap_enabled,
                        false,
                        GRID_SIZE,
                    );
                    angle_result.point
                } else if grid_snap_enabled {
                    snap_to_grid(world_point, GRID_SIZE).point
                } else {
                    world_point
                };

                // Clear line start point
                self.line_start_point = None;

                if let Some(mut shape) = canvas.tool_manager.end(end_point) {
                    // Only add if shape has meaningful size
                    let bounds = shape.bounds();
                    if bounds.width() > 1.0 || bounds.height() > 1.0 {
                        // Geometric tools always start in Architect mode.
                        *shape.style_mut() = current_style.clone();
                        shape.style_mut().sloppiness = Sloppiness::Architect;
                        canvas.document.push_undo();
                        let id = shape.id();
                        canvas.document.add_shape(shape);
                        canvas.select(id);
                    }
                }
            }
            _ => {
                // Complete Rectangle/Ellipse with optional Shift constraint.
                let mut end_point = if grid_snap_enabled {
                    snap_to_grid(world_point, GRID_SIZE).point
                } else {
                    world_point
                };
                if input.shift()
                    && matches!(
                        canvas.tool_manager.current_tool,
                        ToolKind::Rectangle | ToolKind::Ellipse
                    )
                {
                    if let Some(start) = self.shape_start_point {
                        end_point = constrain_square_endpoint(start, end_point);
                    }
                }
                self.shape_start_point = None;

                if let Some(mut shape) = canvas.tool_manager.end(end_point) {
                    let bounds = shape.bounds();
                    if bounds.width() > 1.0 || bounds.height() > 1.0 {
                        *shape.style_mut() = current_style.clone();
                        if matches!(shape, Shape::Rectangle(_) | Shape::Ellipse(_)) {
                            shape.style_mut().sloppiness = Sloppiness::Architect;
                        }
                        canvas.document.push_undo();
                        let id = shape.id();
                        canvas.document.add_shape(shape);
                        canvas.select(id);
                    }
                }
            }
        }
    }

    /// Handle a move event while dragging.
    /// `grid_snap_enabled` controls whether points should snap to grid.
    /// `smart_snap_enabled` enables smart guide alignment.
    /// `angle_snap_enabled` enables 15° angle snapping for lines/arrows.
    pub fn handle_drag(
        &mut self,
        canvas: &mut Canvas,
        world_point: Point,
        input: &InputState,
        grid_snap_enabled: bool,
        smart_snap_enabled: bool,
        angle_snap_enabled: bool,
    ) {
        // Clear previous snap
        self.last_snap = None;
        self.last_angle_snap = None;
        self.smart_guides.clear();

        // If we're manipulating a shape, update it
        if let Some(manip) = &mut self.manipulation {
            // Check if this is a rotation handle
            if matches!(manip.handle, Some(HandleKind::Rotate)) {
                // Handle rotation - Shift key snaps to 15° increments
                let snap_to_15deg = input.shift();
                let center = manip.original_shape.bounds().center();

                // Apply rotation to the shape
                if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                    let angle = apply_rotation_from_drag(
                        shape,
                        &manip.original_shape,
                        manip.start_point,
                        world_point,
                        snap_to_15deg,
                    );

                    // Update rotation state for helper line rendering
                    self.rotation_state = Some(RotationState {
                        center,
                        angle,
                        snapped: snap_to_15deg,
                    });
                }

                // Update current_point for delta calculation on release
                manip.current_point = world_point;
                return;
            }

            // Check if we're manipulating a line or arrow endpoint
            let is_line_or_arrow = matches!(manip.original_shape, Shape::Line(_) | Shape::Arrow(_));

            // Calculate raw delta from cursor movement
            let raw_delta = kurbo::Vec2::new(
                world_point.x - manip.start_point.x,
                world_point.y - manip.start_point.y,
            );

            // Get the original handle/shape position that's being manipulated
            let original_position =
                get_manipulation_target_position(&manip.original_shape, manip.handle);

            // Calculate where the handle/shape would end up
            let target_position = Point::new(
                original_position.x + raw_delta.x,
                original_position.y + raw_delta.y,
            );

            // For line/arrow endpoint manipulation, use angle/grid/smart snapping
            let snap_result = if is_line_or_arrow && manip.handle.is_some() {
                // Get the other endpoint as the origin for polar snapping
                let other_endpoint = get_line_other_endpoint(&manip.original_shape, manip.handle);

                if angle_snap_enabled || input.shift() {
                    // First snap angle for direction
                    let angle_result = snap_line_endpoint_isometric(
                        other_endpoint,
                        target_position,
                        true,
                        false, // Don't grid snap yet
                        false,
                        GRID_SIZE,
                    );

                    let mut final_point = angle_result.point;

                    // Then use smart guides for magnitude along the snapped angle ray
                    if smart_snap_enabled {
                        // Create snap zone around the target point
                        let snap_zone = Rect::from_center_size(
                            angle_result.point,
                            Size::new(ENDPOINT_SNAP_RADIUS * 2.0, ENDPOINT_SNAP_RADIUS * 2.0),
                        );
                        let mut other_bounds =
                            collect_snap_candidates(canvas, &[manip.shape_id], Some(snap_zone));
                        // Include the line's own non-dragged endpoints
                        other_bounds.extend(self_snap_rects(&manip.original_shape, manip.handle));

                        let guide_result = snap_ray_to_smart_guides(
                            other_endpoint,
                            angle_result.angle_degrees,
                            angle_result.point,
                            &other_bounds,
                            SMART_GUIDE_THRESHOLD,
                        );

                        if guide_result.snapped_x || guide_result.snapped_y {
                            final_point = guide_result.point;
                            self.smart_guides = guide_result.guides;
                        }
                    }

                    // Apply grid snap last if enabled (snap to grid line intersections on ray)
                    if grid_snap_enabled && !smart_snap_enabled {
                        let grid_result = snap_line_endpoint_isometric(
                            other_endpoint,
                            target_position,
                            true,
                            true,
                            false,
                            GRID_SIZE,
                        );
                        final_point = grid_result.point;
                    }

                    if angle_result.snapped {
                        self.last_angle_snap = Some(AngleSnapResult {
                            point: final_point,
                            ..angle_result
                        });
                        self.line_start_point = Some(other_endpoint);
                    }

                    SnapResult {
                        point: final_point,
                        snapped_x: angle_result.snapped,
                        snapped_y: angle_result.snapped,
                    }
                } else if smart_snap_enabled {
                    // Smart guides for line/arrow endpoints (no angle snap)
                    let snap_zone = Rect::from_center_size(
                        target_position,
                        Size::new(ENDPOINT_SNAP_RADIUS * 2.0, ENDPOINT_SNAP_RADIUS * 2.0),
                    );
                    let mut other_bounds =
                        collect_snap_candidates(canvas, &[manip.shape_id], Some(snap_zone));
                    // Include the line's own non-dragged endpoints
                    other_bounds.extend(self_snap_rects(&manip.original_shape, manip.handle));

                    let guide_result = detect_smart_guides_for_point(
                        target_position,
                        &other_bounds,
                        SMART_GUIDE_THRESHOLD,
                    );

                    let mut result_point = guide_result.point;

                    if grid_snap_enabled {
                        let grid_result = snap_to_grid(result_point, GRID_SIZE);
                        result_point = grid_result.point;
                    }

                    if guide_result.snapped_x || guide_result.snapped_y {
                        self.smart_guides = guide_result.guides;
                    }

                    SnapResult {
                        point: result_point,
                        snapped_x: guide_result.snapped_x || grid_snap_enabled,
                        snapped_y: guide_result.snapped_y || grid_snap_enabled,
                    }
                } else if grid_snap_enabled {
                    snap_to_grid(target_position, GRID_SIZE)
                } else {
                    SnapResult::none(target_position)
                }
            } else {
                // Non-line shape: corner resize or move
                let is_corner_resize = matches!(
                    manip.handle,
                    Some(HandleKind::Corner(_) | HandleKind::Edge(_))
                );
                if is_corner_resize && smart_snap_enabled {
                    // Snap resize handle to other shapes' edges/centers
                    let snap_zone = Rect::from_center_size(
                        target_position,
                        Size::new(ENDPOINT_SNAP_RADIUS * 2.0, ENDPOINT_SNAP_RADIUS * 2.0),
                    );
                    let other_bounds =
                        collect_snap_candidates(canvas, &[manip.shape_id], Some(snap_zone));

                    let guide_result = detect_smart_guides_for_point(
                        target_position,
                        &other_bounds,
                        SMART_GUIDE_THRESHOLD,
                    );

                    let mut result_point = guide_result.point;

                    if grid_snap_enabled {
                        // Only grid-snap axes that weren't smart-snapped
                        let grid_result = snap_to_grid(result_point, GRID_SIZE);
                        if !guide_result.snapped_x {
                            result_point.x = grid_result.point.x;
                        }
                        if !guide_result.snapped_y {
                            result_point.y = grid_result.point.y;
                        }
                    }

                    if guide_result.snapped_x || guide_result.snapped_y {
                        self.smart_guides = guide_result.guides;
                    }

                    SnapResult {
                        point: result_point,
                        snapped_x: guide_result.snapped_x || grid_snap_enabled,
                        snapped_y: guide_result.snapped_y || grid_snap_enabled,
                    }
                } else if grid_snap_enabled {
                    snap_to_grid(target_position, GRID_SIZE)
                } else {
                    SnapResult::none(target_position)
                }
            };

            // Store snap result for visual guides (show where handle snaps to)
            if snap_result.is_snapped() && self.last_angle_snap.is_none() {
                self.last_snap = Some(snap_result);
            }

            // Calculate adjusted delta so that original + adjusted_delta = snapped_position
            let adjusted_delta = kurbo::Vec2::new(
                snap_result.point.x - original_position.x,
                snap_result.point.y - original_position.y,
            );

            // Update current_point to reflect the adjusted delta
            manip.current_point = Point::new(
                manip.start_point.x + adjusted_delta.x,
                manip.start_point.y + adjusted_delta.y,
            );

            // Apply manipulation preview to the shape
            let new_shape = if input.ctrl()
                && matches!(manip.original_shape, Shape::Image(_))
                && matches!(
                    manip.handle,
                    Some(HandleKind::Corner(_) | HandleKind::Edge(_))
                ) {
                apply_image_crop(&manip.original_shape, manip.handle, adjusted_delta)
            } else {
                apply_manipulation(
                    &manip.original_shape,
                    manip.handle,
                    adjusted_delta,
                    if matches!(manip.original_shape, Shape::Text(_) | Shape::Math(_)) {
                        !input.shift()
                    } else {
                        input.shift()
                    },
                )
            };
            if let Some(shape) = canvas.document.get_shape_mut(manip.shape_id) {
                *shape = new_shape;
            }
            return;
        }

        // If we're moving multiple shapes, update all of them
        if let Some(mm) = &mut self.multi_move {
            // Calculate raw delta from cursor movement
            let raw_delta = kurbo::Vec2::new(
                world_point.x - mm.start_point.x,
                world_point.y - mm.start_point.y,
            );

            // Only apply snapping if cursor moved more than 2px (prevents snap on click)
            let movement_threshold = 2.0;
            let has_meaningful_movement = raw_delta.hypot() > movement_threshold;

            // For multi-move, use the first shape's bounds as reference for snapping
            let reference_shape = mm.original_shapes.values().next();
            let snap_result = if let Some(ref_shape) = reference_shape {
                let original_bounds = ref_shape.bounds();
                let target_bounds = Rect::new(
                    original_bounds.x0 + raw_delta.x,
                    original_bounds.y0 + raw_delta.y,
                    original_bounds.x1 + raw_delta.x,
                    original_bounds.y1 + raw_delta.y,
                );

                // Collect other shape bounds for smart guide detection
                let exclude_ids: Vec<ShapeId> = mm.original_shapes.keys().copied().collect();
                let mut final_delta = raw_delta;

                // Only apply snapping if there's meaningful movement
                if has_meaningful_movement {
                    // Smart guides
                    if smart_snap_enabled {
                        // Expand target bounds for proximity filtering
                        let snap_zone =
                            target_bounds.inflate(MULTI_MOVE_SNAP_RADIUS, MULTI_MOVE_SNAP_RADIUS);
                        let other_bounds =
                            collect_snap_candidates(canvas, &exclude_ids, Some(snap_zone));

                        let guide_result = detect_smart_guides(
                            target_bounds,
                            &other_bounds,
                            SMART_GUIDE_THRESHOLD,
                        );
                        if guide_result.snapped_x || guide_result.snapped_y {
                            final_delta.x += guide_result.point.x - target_bounds.x0;
                            final_delta.y += guide_result.point.y - target_bounds.y0;
                            self.smart_guides = guide_result.guides;
                        }
                    }

                    // Grid snap (applied after smart guides)
                    if grid_snap_enabled {
                        let target_pos = Point::new(
                            original_bounds.x0 + final_delta.x,
                            original_bounds.y0 + final_delta.y,
                        );
                        let snap_result = snap_to_grid(target_pos, GRID_SIZE);
                        final_delta.x = snap_result.point.x - original_bounds.x0;
                        final_delta.y = snap_result.point.y - original_bounds.y0;
                        self.last_snap = Some(snap_result);
                    }
                }

                final_delta
            } else {
                raw_delta
            };

            // Update current_point
            mm.current_point = Point::new(
                mm.start_point.x + snap_result.x,
                mm.start_point.y + snap_result.y,
            );

            // Apply movement to shapes
            let translation = kurbo::Affine::translate(snap_result);
            if mm.is_duplicate {
                // For duplicate, move the duplicated shapes (originals stay in place)
                for (idx, &dup_id) in mm.duplicated_ids.iter().enumerate() {
                    // Get the corresponding original shape
                    let original_shape = mm.original_shapes.values().nth(idx);
                    if let (Some(orig), Some(shape)) =
                        (original_shape, canvas.document.get_shape_mut(dup_id))
                    {
                        let mut new_shape = orig.clone();
                        new_shape.transform(translation);
                        *shape = new_shape;
                    }
                }
            } else {
                // Normal move - move the original shapes
                for (shape_id, original_shape) in &mm.original_shapes {
                    let mut new_shape = original_shape.clone();
                    new_shape.transform(translation);
                    if let Some(shape) = canvas.document.get_shape_mut(*shape_id) {
                        *shape = new_shape;
                    }
                }
            }
            return;
        }

        // If we're doing marquee selection, update the rectangle (no snapping for selection)
        if let Some(sel_rect) = &mut self.selection_rect {
            sel_rect.current = world_point;
            return;
        }

        // Handle eraser tool - erase immediately while dragging
        if canvas.tool_manager.current_tool == ToolKind::Eraser && !self.eraser_points.is_empty() {
            self.eraser_points.push(world_point);
            self.apply_eraser(canvas);
            // Keep only the last point for the next segment
            if let Some(last) = self.eraser_points.last().copied() {
                self.eraser_points.clear();
                self.eraser_points.push(last);
            }
            return;
        }

        // Handle laser pointer tool
        if canvas.tool_manager.current_tool == ToolKind::LaserPointer {
            self.laser_position = Some(world_point);
            self.laser_trail.push((world_point, 1.0));
            // Keep trail limited
            if self.laser_trail.len() > 50 {
                self.laser_trail.remove(0);
            }
            return;
        }

        // Update tool manager (handles freehand point accumulation internally)
        if canvas.tool_manager.is_active() {
            let tool = canvas.tool_manager.current_tool;

            // Special handling for Line and Arrow tools with angle snapping
            if matches!(tool, ToolKind::Line | ToolKind::Arrow) {
                if let Some(start) = self.line_start_point {
                    // Use angle-aware snapping (grid + angle)
                    let angle_result = snap_line_endpoint_isometric(
                        start,
                        world_point,
                        angle_snap_enabled || input.shift(),
                        grid_snap_enabled,
                        false, // unused parameter kept for API compatibility
                        GRID_SIZE,
                    );

                    // Store for visualization
                    if angle_result.snapped {
                        self.last_angle_snap = Some(angle_result);
                    }

                    canvas.tool_manager.update(angle_result.point);
                    return;
                }
            }

            // Apply snapping for other shape creation tools (except freehand/highlighter)
            let mut point = if grid_snap_enabled
                && !matches!(tool, ToolKind::Freehand | ToolKind::Highlighter)
            {
                let snap_result = snap_to_grid(world_point, GRID_SIZE);
                self.last_snap = Some(snap_result);
                snap_result.point
            } else {
                world_point
            };

            // Shift constrains rectangles to squares and ellipses to circles.
            if input.shift() && matches!(tool, ToolKind::Rectangle | ToolKind::Ellipse) {
                if let Some(start) = self.shape_start_point {
                    point = constrain_square_endpoint(start, point);
                }
            }

            canvas.tool_manager.update(point);
        }
    }

    /// Clear the last snap result (call when dragging ends).
    pub fn clear_snap(&mut self) {
        self.last_snap = None;
        self.last_angle_snap = None;
        self.line_start_point = None;
        self.shape_start_point = None;
        self.smart_guides.clear();
    }

    /// Apply the current eraser mode to shapes intersecting the current eraser segment.
    fn apply_eraser(&mut self, canvas: &mut Canvas) {
        if self.eraser_points.is_empty() {
            return;
        }

        let radius = self.eraser_radius;

        match self.eraser_mode {
            EraserMode::Classic => {
                let mut shapes_to_remove: Vec<ShapeId> = Vec::new();
                for shape in canvas.document.shapes_ordered() {
                    if self
                        .eraser_points
                        .iter()
                        .any(|&point| shape.hit_test(point, radius))
                    {
                        shapes_to_remove.push(shape.id());
                    }
                }

                if !shapes_to_remove.is_empty() {
                    if !self.eraser_undo_started {
                        canvas.document.push_undo();
                        self.eraser_undo_started = true;
                    }
                    canvas.clear_selection();
                    for id in shapes_to_remove {
                        canvas.document.remove_shape(id);
                    }
                }
            }
            EraserMode::Manual => {
                if self.eraser_points.len() < 2 {
                    return;
                }
                let erase_start = self.eraser_points[0];
                let erase_end = *self.eraser_points.last().unwrap_or(&erase_start);

                let replacements: Vec<(ShapeId, Vec<Shape>)> = canvas
                    .document
                    .shapes_ordered()
                    .filter_map(|shape| match shape {
                        Shape::Freehand(freehand) if !freehand.closed => {
                            split_freehand_by_eraser(freehand, erase_start, erase_end, radius)
                                .map(|parts| (shape.id(), parts))
                        }
                        _ => split_contour_by_eraser(shape, erase_start, erase_end, radius)
                            .map(|parts| (shape.id(), parts)),
                    })
                    .collect();

                if !replacements.is_empty() {
                    if !self.eraser_undo_started {
                        canvas.document.push_undo();
                        self.eraser_undo_started = true;
                    }
                    canvas.clear_selection();
                    for (id, parts) in replacements {
                        canvas.document.replace_shape_with_many(id, parts);
                    }
                }
            }
        }
    }

    /// Update laser trail (fade out old points). Call each frame.
    pub fn update_laser_trail(&mut self, dt: f64) {
        // Fade out trail points
        for (_, alpha) in &mut self.laser_trail {
            *alpha -= dt * 3.0; // Fade over ~0.33 seconds
        }
        // Remove fully faded points
        self.laser_trail.retain(|(_, alpha)| *alpha > 0.0);
    }

    /// Get the current eraser path for rendering.
    pub fn eraser_path(&self) -> &[Point] {
        &self.eraser_points
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod contour_eraser_regressions {
    use super::*;
    use drafftink_core::shapes::{Line, Rectangle, SerializableColor};

    #[test]
    fn erasing_a_long_line_keeps_both_sides_and_style() {
        let mut line = Line::new(Point::ZERO, Point::new(200.0, 0.0));
        line.style.stroke_width = 3.0;
        let shape = Shape::Line(line);
        let parts = split_contour_by_eraser(
            &shape,
            Point::new(100.0, -10.0),
            Point::new(100.0, 10.0),
            5.0,
        )
        .unwrap();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].bounds().x1 < 100.0 && parts[1].bounds().x0 > 100.0);
        assert!(parts.iter().all(|s| s.style().stroke_width == 3.0));
        assert!(
            split_contour_by_eraser(&shape, Point::new(300.0, 0.0), Point::new(310.0, 0.0), 5.0)
                .is_none()
        );
    }

    #[test]
    fn filled_rotated_rectangle_keeps_fill_outside_cut_and_roundtrips() {
        let mut rectangle = Rectangle::new(Point::ZERO, 100.0, 100.0);
        rectangle.rotation = 0.4;
        rectangle.style.fill_color = Some(SerializableColor::black());
        let parts = split_contour_by_eraser(
            &Shape::Rectangle(rectangle),
            Point::new(50.0, 45.0),
            Point::new(50.0, 55.0),
            7.0,
        )
        .unwrap();
        assert!(parts.iter().any(|s| s.style().fill_color.is_some()));
        assert!(
            !parts
                .iter()
                .any(|s| s.hit_test(Point::new(50.0, 50.0), 0.0))
        );
        assert!(
            parts
                .iter()
                .any(|s| s.hit_test(Point::new(30.0, 30.0), 0.0))
        );
        let json = serde_json::to_string(&parts).unwrap();
        let restored: Vec<Shape> = serde_json::from_str(&json).unwrap();
        assert_eq!(parts.len(), restored.len());
    }

    #[test]
    fn newly_drawn_shape_is_selected_and_can_be_manipulated_with_drawing_tool() {
        let mut canvas = Canvas::new();
        let mut handler = EventHandler::new();
        let input = InputState::new();
        canvas.tool_manager.current_tool = ToolKind::Rectangle;
        handler.handle_press(&mut canvas, Point::ZERO, &input, false);
        handler.handle_release(
            &mut canvas,
            Point::new(100.0, 80.0),
            &input,
            &ShapeStyle::default(),
            false,
            false,
        );
        assert_eq!(canvas.selection.len(), 1);
        handler.handle_press(&mut canvas, Point::new(100.0, 80.0), &input, false);
        assert!(handler.manipulation.is_some());
        assert_eq!(canvas.tool_manager.current_tool, ToolKind::Rectangle);
    }

    #[test]
    fn manual_eraser_fragments_form_one_undo_group() {
        let mut canvas = Canvas::new();
        canvas
            .document
            .add_shape(Shape::Line(Line::new(Point::ZERO, Point::new(200.0, 0.0))));
        let mut handler = EventHandler::new();
        handler.eraser_mode = EraserMode::Manual;
        handler.eraser_radius = 5.0;
        handler.eraser_points = vec![Point::new(100.0, -10.0), Point::new(100.0, 10.0)];
        handler.apply_eraser(&mut canvas);
        assert_eq!(canvas.document.len(), 2);
        canvas.document.undo();
        assert_eq!(canvas.document.len(), 1);
        assert!(matches!(
            canvas.document.shapes_ordered().next(),
            Some(Shape::Line(_))
        ));
        canvas.document.redo();
        assert_eq!(canvas.document.len(), 2);
    }
}

#[cfg(test)]
mod future_font_regressions {
    use super::*;
    use drafftink_core::shapes::{FontWeight, TextFont};
    #[test]
    fn each_new_text_uses_the_last_explicit_font_choice() {
        let mut canvas = Canvas::new();
        let mut handler = EventHandler::new();
        let wanted = TextFont::from_name("Google Sans", "GoogleSans-Medium");
        handler.text_font = wanted.clone();
        canvas.tool_manager.current_tool = ToolKind::Text;
        let input = InputState::new();
        for x in [10.0, 120.0] {
            handler.handle_release(
                &mut canvas,
                Point::new(x, 20.0),
                &input,
                &ShapeStyle::default(),
                false,
                false,
            );
            let id = handler.editing_text.unwrap();
            if let Some(Shape::Text(text)) = canvas.document.get_shape_mut(id) {
                assert_eq!(text.custom_font.as_deref(), Some("Google Sans"));
                assert_eq!(text.font_weight, FontWeight::Medium);
                text.content = "kept".into();
            } else {
                panic!()
            }
            handler.exit_text_edit(&mut canvas);
        }
        assert_eq!(canvas.document.len(), 2);
        assert_eq!(handler.text_font, wanted);
    }
}

/// Screen velocity grows smoothly toward the edge and is bounded outside it.
pub(crate) fn edge_pan_velocity(point: Point, viewport: Size, margin: f64) -> kurbo::Vec2 {
    fn axis(p: f64, length: f64, margin: f64) -> f64 {
        let margin = margin.min(length * 0.25).max(1.0);
        if p < margin {
            -((margin - p) / margin).clamp(0.0, 1.0) * 600.0
        } else if p > length - margin {
            ((p - (length - margin)) / margin).clamp(0.0, 1.0) * 600.0
        } else {
            0.0
        }
    }
    kurbo::Vec2::new(
        axis(point.x, viewport.width, margin),
        axis(point.y, viewport.height, margin),
    )
}

#[cfg(test)]
mod edge_pan_tests {
    use super::*;
    #[test]
    fn velocity_only_near_edges_and_bounded() {
        let size = Size::new(800.0, 600.0);
        assert_eq!(
            edge_pan_velocity(Point::new(400.0, 300.0), size, 48.0),
            kurbo::Vec2::ZERO
        );
        assert!(edge_pan_velocity(Point::new(798.0, 300.0), size, 48.0).x > 500.0);
        assert!(edge_pan_velocity(Point::new(2.0, 300.0), size, 48.0).x < -500.0);
        let out = edge_pan_velocity(Point::new(900.0, 900.0), size, 48.0);
        assert_eq!(out, kurbo::Vec2::new(600.0, 600.0));
    }
}
