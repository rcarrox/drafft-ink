//! Vello-based renderer implementation.

use crate::renderer::{RenderContext, Renderer, ShapeRenderer};
use crate::text_editor::TextEditState;
use drafftink_core::selection::{Handle, HandleKind, get_handles};
use drafftink_core::shapes::{FillPattern, Shape, ShapeStyle, ShapeTrait, StrokeStyle};
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape as KurboShape, Stroke};
use parley::layout::PositionedLayoutItem;
use parley::{FontContext, LayoutContext};
use peniko::{Brush, Color, Fill};
use roughr::core::{FillStyle, OptionsBuilder};
use vello::Scene;

/// Result of PNG rendering - contains the raw RGBA pixel data and dimensions.
#[derive(Debug)]
pub struct PngRenderResult {
    /// RGBA pixel data (4 bytes per pixel).
    pub rgba_data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Embedded GelPen fonts (Regular, Light, Heavy variants)
static GELPEN_REGULAR: &[u8] = include_bytes!("../assets/GelPen.ttf");
static GELPEN_LIGHT: &[u8] = include_bytes!("../assets/GelPenLight.ttf");
static GELPEN_HEAVY: &[u8] = include_bytes!("../assets/GelPenHeavy.ttf");
/// Embedded VanillaExtract font
static VANILLA_EXTRACT: &[u8] = include_bytes!("../assets/VanillaExtract.ttf");
/// Embedded GelPenSerif fonts for handwritten style
static GELPEN_SERIF_LIGHT: &[u8] = include_bytes!("../assets/GelPenSerifLight.ttf");
static GELPEN_SERIF_MEDIUM: &[u8] = include_bytes!("../assets/GelPenSerifMedium.ttf");
static GELPEN_SERIF_HEAVY: &[u8] = include_bytes!("../assets/GelPenSerifHeavy.ttf");
/// Embedded XITS Math font for LaTeX rendering
static XITS_MATH: &[u8] = include_bytes!("../assets/rex-xits.otf");
/// Embedded Noto Sans for UI elements
static NOTO_SANS: &[u8] = include_bytes!("../assets/NotoSans-Regular.ttf");
static NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/NotoSans-Bold.ttf");
static NOTO_SANS_ITALIC: &[u8] = include_bytes!("../assets/NotoSans-Italic.ttf");

fn text_font_stack(name: &str) -> parley::FontStack<'static> {
    parley::FontStack::List(
        vec![
            parley::FontFamily::Named(name.to_string().into()),
            parley::FontFamily::Named("Noto Sans".into()),
            parley::FontFamily::Named("STIX Two Math".into()),
        ]
        .into(),
    )
}

/// Cached text layout data for rendering.
#[derive(Clone)]
struct CachedTextLayout {
    layout: parley::Layout<Brush>,
    scene: Scene,
    width: f64,
    height: f64,
}

/// Vello-based renderer for GPU-accelerated 2D graphics.
pub struct VelloRenderer {
    /// The Vello scene being built.
    scene: Scene,
    /// Selection highlight color.
    selection_color: Color,
    /// Font context for text rendering (cached to avoid re-registering fonts).
    font_cx: FontContext,
    /// Layout context for text rendering.
    layout_cx: LayoutContext<Brush>,
    /// Current zoom level (for zoom-independent UI elements).
    zoom: f64,
    /// Image cache to avoid re-decoding images every frame.
    /// Key is the shape ID (as string), value is the decoded peniko ImageData.
    image_cache: std::collections::HashMap<u64, CachedImage>,
    image_cache_clock: u64,
    full_resolution_images: bool,
    /// Shape path cache for hand-drawn effects.
    /// Key: (shape_id, seed, stroke_index, roughness_bits, zoom_bucket, path_hash)
    shape_cache: std::collections::HashMap<(String, u32, u32, u64, i32, u64), CachedPath>,
    /// Text layout cache. Key: (shape_id, content_hash)
    text_cache: std::collections::HashMap<(drafftink_core::shapes::ShapeId, u64), CachedTextLayout>,
    cache_scope_prepared: bool,
    math_cache: std::collections::HashMap<drafftink_core::shapes::ShapeId, CachedMath>,
    math_primary_font: Option<std::sync::Arc<Vec<u8>>>,
    font_aliases: std::collections::HashMap<String, String>,
    registered_fonts: std::collections::HashMap<String, String>,
    registered_font_data: std::collections::HashMap<String, std::sync::Arc<Vec<u8>>>,
    #[cfg(test)]
    math_layout_builds: usize,
}

const IMAGE_CACHE_BUDGET: usize = 32 * 1024 * 1024;
const MAX_PREVIEW_SIDE: u32 = 2048;

fn transformed_rect(rect: Rect, transform: Affine) -> Rect {
    let corners = [
        transform * Point::new(rect.x0, rect.y0),
        transform * Point::new(rect.x1, rect.y0),
        transform * Point::new(rect.x1, rect.y1),
        transform * Point::new(rect.x0, rect.y1),
    ];
    let (x0, x1) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
        (min.min(p.x), max.max(p.x))
    });
    let (y0, y1) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
        (min.min(p.y), max.max(p.y))
    });
    Rect::new(x0, y0, x1, y1)
}
const PATH_CACHE_BUDGET: usize = 4 * 1024 * 1024;
struct CachedPath {
    path: BezPath,
    last_used: u64,
}
struct CachedImage {
    image: peniko::ImageData,
    bucket: u32,
    last_used: u64,
}
impl CachedImage {
    fn bytes(&self) -> usize {
        self.image.width as usize * self.image.height as usize * 4
    }
}
fn decode_image_preview(
    image: &drafftink_core::shapes::Image,
    max_side: u32,
) -> Option<peniko::ImageData> {
    let encoded = image.data()?;
    let decoded = ::image::load_from_memory(&encoded).ok()?;
    drop(encoded);
    // Consume RGBA data instead of cloning a second full-size pixel buffer.
    let mut rgba = decoded.into_rgba8();
    let side = rgba.width().max(rgba.height());
    if max_side != 0 && side > max_side {
        let width = ((rgba.width() as u64 * max_side as u64) / side as u64).max(1) as u32;
        let height = ((rgba.height() as u64 * max_side as u64) / side as u64).max(1) as u32;
        rgba = ::image::imageops::resize(
            &rgba,
            width,
            height,
            ::image::imageops::FilterType::Triangle,
        );
    }
    Some(peniko::ImageData {
        width: rgba.width(),
        height: rgba.height(),
        data: peniko::Blob::new(std::sync::Arc::new(rgba.into_vec())),
        format: peniko::ImageFormat::Rgba8,
        alpha_type: peniko::ImageAlphaType::Alpha,
    })
}

struct CachedMath {
    source: String,
    font_size: u64,
    primary_font_id: usize,
    color: [u8; 4],
    scene: Scene,
    size: (f64, f64, f64),
}

impl Default for VelloRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a Parley BoundingBox to a Kurbo Rect.
fn convert_rect(rect: &parley::BoundingBox) -> Rect {
    Rect::new(rect.x0, rect.y0, rect.x1, rect.y1)
}

/// Simple seeded random number generator (xorshift32).
/// Used for deterministic hand-drawn effects.
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Random float in range [-1, 1]
    fn next_f64(&mut self) -> f64 {
        (self.next_u32() as f64 / u32::MAX as f64) * 2.0 - 1.0
    }

    /// Random offset scaled by amount
    fn offset(&mut self, amount: f64) -> f64 {
        self.next_f64() * amount
    }
}

/// Apply hand-drawn effect to a path based on roughness level.
/// This mimics the Excalidraw/rough.js algorithm:
/// - Endpoints are randomly offset (lines overshoot/undershoot at corners)
/// - Lines have a slight bow (curve in the middle)
/// - Each stroke_index produces completely different randomness
///
/// roughness: 0 = clean, 1 = slight wobble, 2 = very sketchy
/// seed: stable random seed from the shape's style (persisted, doesn't change on transform)
/// stroke_index: 0 or 1 for multi-stroke effect (different random offsets)
fn apply_hand_drawn_effect(
    path: &BezPath,
    roughness: f64,
    zoom: f64,
    seed: u32,
    stroke_index: u32,
) -> BezPath {
    if roughness <= 0.0 {
        return path.clone();
    }

    // Scale effect inversely with zoom so it looks consistent at all zoom levels
    let scale = 1.0 / zoom.sqrt();

    // Values tuned to match Excalidraw/rough.js feel
    // These create the "overshoot" effect at corners
    let max_randomness_offset = roughness * 2.0 * scale;
    let bowing = roughness * 1.0;

    // Use the shape's stable seed combined with stroke_index for deterministic randomness
    // The seed is stored in the shape's style, so it doesn't change when the shape is transformed
    let combined_seed = seed.wrapping_add(stroke_index.wrapping_mul(99991)); // Large prime for very different sequences
    let mut rng = SimpleRng::new(combined_seed);

    let mut result = BezPath::new();
    let mut last_point = Point::ZERO;

    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                // Offset the start point
                let wobbled = Point::new(
                    p.x + rng.offset(max_randomness_offset),
                    p.y + rng.offset(max_randomness_offset),
                );
                result.move_to(wobbled);
                last_point = *p;
            }
            PathEl::LineTo(p) => {
                // This is the key rough.js algorithm for lines:
                // 1. Calculate line length
                // 2. Add bowing (perpendicular offset at midpoint)
                // 3. Offset both endpoints randomly (creates overshoot)

                let dx = p.x - last_point.x;
                let dy = p.y - last_point.y;
                let len = (dx * dx + dy * dy).sqrt();

                // Calculate bowing amount - proportional to length
                let bow_offset = bowing * roughness * len / 200.0;
                let bow = rng.offset(bow_offset) * scale;

                // Perpendicular vector for bowing
                let (perp_x, perp_y) = if len > 0.001 {
                    (-dy / len, dx / len)
                } else {
                    (0.0, 0.0)
                };

                // Control point with bowing
                let mid_x = (last_point.x + p.x) / 2.0 + perp_x * bow;
                let mid_y = (last_point.y + p.y) / 2.0 + perp_y * bow;

                // End point with random offset (creates overshoot at corners)
                let end = Point::new(
                    p.x + rng.offset(max_randomness_offset),
                    p.y + rng.offset(max_randomness_offset),
                );

                // Use quadratic bezier for the bowed line
                result.quad_to(Point::new(mid_x, mid_y), end);
                last_point = *p;
            }
            PathEl::QuadTo(p1, p2) => {
                let wobbled_p1 = Point::new(
                    p1.x + rng.offset(max_randomness_offset * 0.7),
                    p1.y + rng.offset(max_randomness_offset * 0.7),
                );
                let wobbled_p2 = Point::new(
                    p2.x + rng.offset(max_randomness_offset),
                    p2.y + rng.offset(max_randomness_offset),
                );
                result.quad_to(wobbled_p1, wobbled_p2);
                last_point = *p2;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let wobbled_p1 = Point::new(
                    p1.x + rng.offset(max_randomness_offset * 0.5),
                    p1.y + rng.offset(max_randomness_offset * 0.5),
                );
                let wobbled_p2 = Point::new(
                    p2.x + rng.offset(max_randomness_offset * 0.5),
                    p2.y + rng.offset(max_randomness_offset * 0.5),
                );
                let wobbled_p3 = Point::new(
                    p3.x + rng.offset(max_randomness_offset),
                    p3.y + rng.offset(max_randomness_offset),
                );
                result.curve_to(wobbled_p1, wobbled_p2, wobbled_p3);
                last_point = *p3;
            }
            PathEl::ClosePath => {
                // Don't close - let the overshoot show at the closing corner too
                // The path visually closes but endpoints won't match perfectly
                result.close_path();
            }
        }
    }

    result
}

/// Generate fill pattern lines within the given bounds using roughr.
fn generate_fill_pattern(
    pattern: FillPattern,
    bounds: Rect,
    stroke_width: f64,
    seed: u32,
) -> BezPath {
    use roughr::core::{OpSetType, OpType};

    let fill_style = match pattern {
        FillPattern::Solid => return BezPath::new(),
        FillPattern::Hachure => FillStyle::Hachure,
        FillPattern::ZigZag => FillStyle::ZigZag,
        FillPattern::CrossHatch => FillStyle::CrossHatch,
        FillPattern::Dots => FillStyle::Dots,
        FillPattern::Dashed => FillStyle::Dashed,
        FillPattern::ZigZagLine => FillStyle::ZigZagLine,
    };

    let fill_color: roughr::Srgba = roughr::Srgba::new(0.0, 0.0, 0.0, 1.0);
    let options = OptionsBuilder::default()
        .seed(seed as u64)
        .fill_style(fill_style)
        .fill(fill_color)
        .stroke(fill_color)
        .fill_weight((stroke_width * 0.5) as f32)
        .hachure_gap((stroke_width * 4.0) as f32)
        .build()
        .unwrap();

    let generator = roughr::generator::Generator::default();
    let drawing = generator.rectangle::<f64>(
        bounds.x0,
        bounds.y0,
        bounds.width(),
        bounds.height(),
        &Some(options),
    );

    // Extract FillSketch ops directly from drawable.sets (like the official vello example)
    let mut path = BezPath::new();
    for set in drawing.sets.iter() {
        if set.op_set_type == OpSetType::FillSketch {
            for op in set.ops.iter() {
                match op.op {
                    OpType::Move => {
                        path.move_to(Point::new(op.data[0], op.data[1]));
                    }
                    OpType::LineTo => {
                        path.line_to(Point::new(op.data[0], op.data[1]));
                    }
                    OpType::BCurveTo => {
                        path.curve_to(
                            Point::new(op.data[0], op.data[1]),
                            Point::new(op.data[2], op.data[3]),
                            Point::new(op.data[4], op.data[5]),
                        );
                    }
                }
            }
        }
    }
    path
}

impl VelloRenderer {
    /// Create a new Vello renderer.
    pub fn new() -> Self {
        let mut font_cx = FontContext::new();
        // Register all font variants
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(GELPEN_REGULAR)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(GELPEN_LIGHT)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(GELPEN_HEAVY)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(VANILLA_EXTRACT)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(GELPEN_SERIF_LIGHT)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(GELPEN_SERIF_MEDIUM)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(GELPEN_SERIF_HEAVY)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(NOTO_SANS)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(NOTO_SANS_BOLD)),
            None,
        );
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(NOTO_SANS_ITALIC)),
            None,
        );
        // Register XITS Math as a plain-text option for broad symbol coverage.
        font_cx.collection.register_fonts(
            vello::peniko::Blob::new(std::sync::Arc::new(XITS_MATH)),
            None,
        );

        Self {
            scene: Scene::new(),
            selection_color: Color::from_rgba8(59, 130, 246, 255),
            font_cx,
            layout_cx: LayoutContext::new(),
            zoom: 1.0,
            image_cache: std::collections::HashMap::new(),
            image_cache_clock: 0,
            full_resolution_images: false,
            shape_cache: std::collections::HashMap::new(),
            text_cache: std::collections::HashMap::new(),
            cache_scope_prepared: false,
            math_cache: std::collections::HashMap::new(),
            math_primary_font: None,
            registered_fonts: Default::default(),
            registered_font_data: Default::default(),
            font_aliases: Default::default(),
            #[cfg(test)]
            math_layout_builds: 0,
        }
    }

    pub fn retain_open_document_caches<'a>(
        &mut self,
        documents: impl IntoIterator<Item = &'a drafftink_core::canvas::CanvasDocument>,
    ) {
        // Keep decoded pixels only for sources still referenced by this canvas.
        // Duplicated images and Undo snapshots share encoded bytes and cache keys.
        fn image_keys(
            shape: &Shape,
            keys: &mut std::collections::HashSet<u64>,
            ids: &mut std::collections::HashSet<drafftink_core::shapes::ShapeId>,
        ) {
            ids.insert(shape.id());
            match shape {
                Shape::Image(image) => {
                    keys.insert(image.data_base64.key());
                }
                Shape::Text(text) => {
                    for formula in &text.formulas {
                        ids.insert(formula.math.id());
                    }
                }
                Shape::Group(group) => {
                    for child in group.children() {
                        image_keys(child, keys, ids);
                    }
                }
                _ => {}
            }
        }
        let mut live_images = std::collections::HashSet::new();
        let mut live_ids = std::collections::HashSet::new();
        for document in documents {
            for shape in document.shapes_ordered() {
                image_keys(shape, &mut live_images, &mut live_ids);
            }
        }
        self.image_cache.retain(|key, _| live_images.contains(key));
        self.math_cache.retain(|id, _| live_ids.contains(id));
        self.text_cache.retain(|(id, _), _| live_ids.contains(id));
        self.shape_cache.retain(|key, _| {
            key.0
                .parse::<drafftink_core::shapes::ShapeId>()
                .ok()
                .is_some_and(|id| live_ids.contains(&id))
        });
        self.cache_scope_prepared = true;
    }

    pub fn image_cache_bytes(&self) -> usize {
        self.image_cache.values().map(CachedImage::bytes).sum()
    }
    pub fn path_cache_bytes(&self) -> usize {
        self.shape_cache
            .iter()
            .map(|(key, entry)| {
                key.0.capacity() + entry.path.elements().len() * std::mem::size_of::<PathEl>()
            })
            .sum()
    }
    fn trim_image_cache(&mut self) {
        while self.image_cache_bytes() > IMAGE_CACHE_BUDGET {
            let Some(key) = self
                .image_cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| *key)
            else {
                break;
            };
            self.image_cache.remove(&key);
        }
    }

    /// Get the built scene for rendering.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Take ownership of the scene (resets internal scene).
    pub fn take_scene(&mut self) -> Scene {
        std::mem::take(&mut self.scene)
    }

    pub fn recycle_scene(&mut self, mut scene: Scene) {
        scene.reset();
        self.scene = scene;
    }

    /// Get mutable references to both font and layout contexts for text editing.
    pub fn contexts_mut(&mut self) -> (&mut FontContext, &mut LayoutContext<Brush>) {
        (&mut self.font_cx, &mut self.layout_cx)
    }

    /// Register a font selected from the user's computer (Chrome/Edge Local Font Access).
    /// Parley reads the family name from the font tables, so the text shape only needs
    /// to remember that family name.
    pub fn register_custom_font(
        &mut self,
        family: &str,
        postscript: &str,
        bytes: Vec<u8>,
    ) -> String {
        if let Some(canonical) = self.registered_fonts.get(postscript) {
            return canonical.clone();
        }
        if bytes.is_empty() {
            return family.to_string();
        }
        let canonical = ttf_parser::Face::parse(&bytes, 0)
            .ok()
            .and_then(|face| {
                [16, 1].into_iter().find_map(|id| {
                    face.names()
                        .into_iter()
                        .filter(|name| name.name_id == id)
                        .find_map(|name| name.to_string())
                })
            })
            .unwrap_or_else(|| family.to_string());
        self.font_aliases
            .insert(family.to_string(), canonical.clone());
        if let Ok(face) = ttf_parser::Face::parse(&bytes, 0) {
            for name in face
                .names()
                .into_iter()
                .filter(|name| name.name_id == 1 || name.name_id == 16)
            {
                if let Some(name) = name.to_string() {
                    self.font_aliases.insert(name, canonical.clone());
                }
            }
        }
        let bytes = std::sync::Arc::new(bytes);
        if postscript.to_lowercase().replace('-', "") == "googlesansmedium" {
            self.math_primary_font = Some(bytes.clone());
        }
        self.registered_font_data
            .insert(postscript.to_string(), bytes.clone());
        self.registered_font_data
            .insert(family.to_string(), bytes.clone());
        self.registered_font_data
            .insert(canonical.clone(), bytes.clone());
        self.font_cx
            .collection
            .register_fonts(vello::peniko::Blob::new(bytes), None);
        self.registered_fonts
            .insert(postscript.to_string(), canonical.clone());
        self.text_cache.clear();
        self.math_cache.clear();
        canonical
    }

    /// Build a scene for export (shapes only, no grid/selection/guides).
    /// Returns the scene and the scaled bounds (for texture dimensions).
    ///
    /// `scale` is the export resolution multiplier (1 = 1x, 2 = 2x, 3 = 3x).
    pub fn build_export_scene(
        &mut self,
        document: &drafftink_core::canvas::CanvasDocument,
        scale: f64,
    ) -> (Scene, Option<Rect>) {
        self.build_export_scene_with_pins(document, scale, None)
    }

    /// Build an export scene with viewport-pinned objects mapped to the world
    /// position they occupy in the current view. `screen_to_world` is the
    /// inverse camera transform captured at the moment of export.
    pub fn build_export_scene_with_pins(
        &mut self,
        document: &drafftink_core::canvas::CanvasDocument,
        scale: f64,
        screen_to_world: Option<Affine>,
    ) -> (Scene, Option<Rect>) {
        self.scene.reset();
        self.full_resolution_images = true;
        self.zoom = scale;

        let mut bounds = document.bounds();
        if let Some(screen_to_world) = screen_to_world {
            for shape in document.shapes_ordered() {
                if !document.is_pinned(shape.id()) {
                    continue;
                }
                let rotation = Affine::rotate_about(shape.rotation(), shape.bounds().center());
                let screen_bounds = shape.bounds().inflate(4.0, 4.0);
                let world_bounds = transformed_rect(screen_bounds, screen_to_world * rotation);
                bounds = Some(bounds.map_or(world_bounds, |current| current.union(world_bounds)));
            }
        }

        // If no shapes, return empty scene
        if bounds.is_none() {
            self.full_resolution_images = false;
            return (std::mem::take(&mut self.scene), None);
        }

        let bounds = bounds.unwrap();

        // Add padding around the content (in logical pixels)
        let padding = 20.0;
        let padded_bounds = bounds.inflate(padding, padding);

        // Transform: translate to origin, then scale up
        let transform =
            Affine::scale(scale) * Affine::translate((-padded_bounds.x0, -padded_bounds.y0));

        // Scaled output dimensions
        let scaled_width = padded_bounds.width() * scale;
        let scaled_height = padded_bounds.height() * scale;

        // Fill background with white (at scaled size)
        let bg_rect = Rect::new(0.0, 0.0, scaled_width, scaled_height);
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::WHITE,
            None,
            &bg_rect,
        );

        // Render all shapes with scaled transform
        for shape in document.shapes_ordered() {
            if document.is_pinned(shape.id()) {
                let Some(screen_to_world) = screen_to_world else { continue };
                let rotation = Affine::rotate_about(shape.rotation(), shape.bounds().center());
                let background = shape.bounds().inflate(4.0, 4.0).to_path(0.1);
                let background_transform = transform * screen_to_world * rotation;
                if let Some(pin) = document.pinned_shapes.get(&shape.id()) {
                    let bg = Color::from(pin.background);
                    if bg.to_rgba8().a > 0 {
                        self.scene.fill(Fill::NonZero, background_transform, bg, None, &background);
                    }
                }
                self.render_shape(shape, transform * screen_to_world, false);
            } else {
                self.render_shape(shape, transform, false);
            }
        }

        // Return scaled bounds for texture dimensions
        let scaled_bounds = Rect::new(0.0, 0.0, scaled_width, scaled_height);
        {
            self.full_resolution_images = false;
            (std::mem::take(&mut self.scene), Some(scaled_bounds))
        }
    }

    /// Build a scene for exporting selected shapes only.
    ///
    /// `scale` is the export resolution multiplier (1 = 1x, 2 = 2x, 3 = 3x).
    pub fn build_export_scene_selection(
        &mut self,
        document: &drafftink_core::canvas::CanvasDocument,
        selection: &[drafftink_core::shapes::ShapeId],
        scale: f64,
    ) -> (Scene, Option<Rect>) {
        self.build_export_scene_selection_with_pins(document, selection, scale, None)
    }

    /// Selection export equivalent that includes pinned objects at their
    /// current viewport position and preserves their rotated backgrounds.
    pub fn build_export_scene_selection_with_pins(
        &mut self,
        document: &drafftink_core::canvas::CanvasDocument,
        selection: &[drafftink_core::shapes::ShapeId],
        scale: f64,
        screen_to_world: Option<Affine>,
    ) -> (Scene, Option<Rect>) {
        self.scene.reset();
        self.full_resolution_images = true;
        self.zoom = scale;

        if selection.is_empty() {
            self.full_resolution_images = false;
            return (std::mem::take(&mut self.scene), None);
        }

        // Calculate bounds of selected shapes
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        let mut shapes_to_render = Vec::new();
        for &shape_id in selection {
            if let Some(shape) = document.get_shape(shape_id) {
                let pinned = document.is_pinned(shape_id);
                if pinned && screen_to_world.is_none() {
                    continue;
                }
                let placement = if pinned { screen_to_world.unwrap() } else { Affine::IDENTITY };
                let rotation = Affine::rotate_about(shape.rotation(), shape.bounds().center());
                let mut b = transformed_rect(shape.bounds(), placement * rotation);
                if pinned {
                    b = transformed_rect(shape.bounds().inflate(4.0, 4.0), placement * rotation);
                }
                min_x = min_x.min(b.x0);
                min_y = min_y.min(b.y0);
                max_x = max_x.max(b.x1);
                max_y = max_y.max(b.y1);
                shapes_to_render.push((shape, pinned, placement, rotation));
            }
        }

        if shapes_to_render.is_empty() {
            self.full_resolution_images = false;
            return (std::mem::take(&mut self.scene), None);
        }

        let bounds = Rect::new(min_x, min_y, max_x, max_y);

        // Add padding around the content (in logical pixels)
        let padding = 20.0;
        let padded_bounds = bounds.inflate(padding, padding);

        // Transform: translate to origin, then scale up
        let transform =
            Affine::scale(scale) * Affine::translate((-padded_bounds.x0, -padded_bounds.y0));

        // Scaled output dimensions
        let scaled_width = padded_bounds.width() * scale;
        let scaled_height = padded_bounds.height() * scale;

        // Fill background with white (at scaled size)
        let bg_rect = Rect::new(0.0, 0.0, scaled_width, scaled_height);
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::WHITE,
            None,
            &bg_rect,
        );

        // Render selected shapes with scaled transform
        for (shape, pinned, placement, rotation) in shapes_to_render {
            if pinned {
                if let Some(pin) = document.pinned_shapes.get(&shape.id()) {
                    let bg = Color::from(pin.background);
                    if bg.to_rgba8().a > 0 {
                        let background = shape.bounds().inflate(4.0, 4.0).to_path(0.1);
                        self.scene.fill(Fill::NonZero, transform * placement * rotation, bg, None, &background);
                    }
                }
                self.render_shape(shape, transform * placement, false);
            } else {
                self.render_shape(shape, transform, false);
            }
        }

        // Return scaled bounds for texture dimensions
        let scaled_bounds = Rect::new(0.0, 0.0, scaled_width, scaled_height);
        {
            self.full_resolution_images = false;
            (std::mem::take(&mut self.scene), Some(scaled_bounds))
        }
    }

    /// Get or compute a cached hand-drawn path.
    fn get_cached_hand_drawn(
        &mut self,
        shape_id: &str,
        path: &BezPath,
        roughness: f64,
        seed: u32,
        stroke_index: u32,
    ) -> BezPath {
        use std::hash::{Hash, Hasher};
        // Quantize zoom to reduce cache misses (bucket by 0.25 increments)
        let zoom_bucket = (self.zoom * 4.0) as i32;
        let roughness_bits = roughness.to_bits();
        // Hash path geometry so cache invalidates when shape moves/resizes
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for el in path.elements() {
            match el {
                kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => {
                    p.x.to_bits().hash(&mut hasher);
                    p.y.to_bits().hash(&mut hasher);
                }
                kurbo::PathEl::QuadTo(p1, p2) => {
                    p1.x.to_bits().hash(&mut hasher);
                    p1.y.to_bits().hash(&mut hasher);
                    p2.x.to_bits().hash(&mut hasher);
                    p2.y.to_bits().hash(&mut hasher);
                }
                kurbo::PathEl::CurveTo(p1, p2, p3) => {
                    p1.x.to_bits().hash(&mut hasher);
                    p1.y.to_bits().hash(&mut hasher);
                    p2.x.to_bits().hash(&mut hasher);
                    p2.y.to_bits().hash(&mut hasher);
                    p3.x.to_bits().hash(&mut hasher);
                    p3.y.to_bits().hash(&mut hasher);
                }
                kurbo::PathEl::ClosePath => {
                    0u8.hash(&mut hasher);
                }
            }
        }
        let path_hash = hasher.finish();
        let key = (
            shape_id.to_string(),
            seed,
            stroke_index,
            roughness_bits,
            zoom_bucket,
            path_hash,
        );

        self.image_cache_clock = self.image_cache_clock.wrapping_add(1);
        if let Some(cached) = self.shape_cache.get_mut(&key) {
            cached.last_used = self.image_cache_clock;
            return cached.path.clone();
        }

        let result = apply_hand_drawn_effect(path, roughness, self.zoom, seed, stroke_index);
        // A stroke needs its current geometry/zoom only, not hundreds of old drag versions.
        self.shape_cache
            .retain(|(id, _, stroke, _, _, _), _| id != shape_id || *stroke != stroke_index);
        self.shape_cache.insert(
            key,
            CachedPath {
                path: result.clone(),
                last_used: self.image_cache_clock,
            },
        );
        while self.path_cache_bytes() > PATH_CACHE_BUDGET {
            let Some(oldest) = self
                .shape_cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.shape_cache.remove(&oldest);
        }
        result
    }

    /// Render a shape path with the given style (with caching).
    fn render_path_cached(
        &mut self,
        shape_id: &str,
        path: &BezPath,
        style: &ShapeStyle,
        transform: Affine,
    ) {
        let roughness = style.sloppiness.roughness();
        let seed = style.seed;

        // Fill if present
        if let Some(fill_color) = style.fill_with_opacity() {
            let fill_path = if roughness > 0.0 {
                self.get_cached_hand_drawn(shape_id, path, roughness * 0.3, seed, 0)
            } else {
                path.clone()
            };

            match style.fill_pattern {
                FillPattern::Solid => {
                    self.scene
                        .fill(Fill::NonZero, transform, fill_color, None, &fill_path);
                }
                _ => {
                    // For patterns, first fill with a lighter version of the color as background
                    let bg_color = Color::from_rgba8(
                        fill_color.to_rgba8().r,
                        fill_color.to_rgba8().g,
                        fill_color.to_rgba8().b,
                        (fill_color.to_rgba8().a as f32 * 0.15) as u8,
                    );
                    self.scene
                        .fill(Fill::NonZero, transform, bg_color, None, &fill_path);

                    // Then draw the pattern lines clipped to shape
                    let bounds = path.bounding_box();
                    let pattern_path =
                        generate_fill_pattern(style.fill_pattern, bounds, style.stroke_width, seed);

                    // Clip pattern to shape boundary
                    self.scene.push_clip_layer(transform, &fill_path);
                    let pattern_stroke = Stroke::new(style.stroke_width * 0.5);
                    self.scene
                        .stroke(&pattern_stroke, transform, fill_color, None, &pattern_path);
                    self.scene.pop_layer();
                }
            }
        }

        // For hand-drawn style, draw multiple strokes like rough.js
        if roughness > 0.0 {
            let stroke = outline_stroke(style.stroke_width, style.stroke_style);

            // First stroke
            let path1 = self.get_cached_hand_drawn(shape_id, path, roughness, seed, 0);
            self.scene.stroke(
                &stroke,
                transform,
                style.stroke_with_opacity(),
                None,
                &path1,
            );

            // Second stroke with different seed
            let path2 = self.get_cached_hand_drawn(shape_id, path, roughness, seed, 1);
            self.scene.stroke(
                &stroke,
                transform,
                style.stroke_with_opacity(),
                None,
                &path2,
            );
        } else {
            // Clean stroke for Architect mode
            let stroke = outline_stroke(style.stroke_width, style.stroke_style);
            self.scene
                .stroke(&stroke, transform, style.stroke_with_opacity(), None, path);
        }
    }

    /// Render a path with stroke only (no fill) - used for lines and arrows.
    fn render_stroke_only(
        &mut self,
        path: &BezPath,
        style: &ShapeStyle,
        stroke_style: StrokeStyle,
        transform: Affine,
    ) {
        let roughness = style.sloppiness.roughness();
        let seed = style.seed;

        let pattern = if style.stroke_style != StrokeStyle::Solid {
            style.stroke_style
        } else {
            stroke_style
        };
        let stroke = outline_stroke(style.stroke_width, pattern);

        if roughness > 0.0 {
            let path1 = apply_hand_drawn_effect(path, roughness, self.zoom, seed, 0);
            self.scene.stroke(
                &stroke,
                transform,
                style.stroke_with_opacity(),
                None,
                &path1,
            );
            let path2 = apply_hand_drawn_effect(path, roughness, self.zoom, seed, 1);
            self.scene.stroke(
                &stroke,
                transform,
                style.stroke_with_opacity(),
                None,
                &path2,
            );
        } else {
            self.scene
                .stroke(&stroke, transform, style.stroke_with_opacity(), None, path);
        }
    }

    /// Render a freehand shape with variable width based on pressure.
    fn render_freehand_with_pressure(
        &mut self,
        freehand: &drafftink_core::shapes::Freehand,
        transform: Affine,
    ) {
        use kurbo::Vec2;

        if freehand.points.len() < 2 {
            return;
        }

        let style = &freehand.style;
        let base_size = style.stroke_width * 2.3;
        let thinning = 0.6;
        let color = style.stroke_with_opacity();

        // Build a filled polygon that represents the variable-width stroke
        let mut left_points: Vec<Point> = Vec::new();
        let mut right_points: Vec<Point> = Vec::new();

        for i in 0..freehand.points.len() {
            let point = freehand.points[i];
            let pressure = freehand.pressure_at(i);
            // Apply easing: sin((pressure * π) / 2)
            let eased_pressure = (pressure * std::f64::consts::PI / 2.0).sin();
            // Apply thinning: size * (1 - thinning * (1 - pressure))
            let width = base_size * (1.0 - thinning * (1.0 - eased_pressure));

            // Calculate perpendicular direction
            let dir = if i == 0 {
                // First point: use direction to next point
                let next = freehand.points[i + 1];
                Vec2::new(next.x - point.x, next.y - point.y)
            } else if i == freehand.points.len() - 1 {
                // Last point: use direction from previous point
                let prev = freehand.points[i - 1];
                Vec2::new(point.x - prev.x, point.y - prev.y)
            } else {
                // Middle points: average of incoming and outgoing directions
                let prev = freehand.points[i - 1];
                let next = freehand.points[i + 1];
                Vec2::new(next.x - prev.x, next.y - prev.y)
            };

            let len = dir.hypot();
            if len < f64::EPSILON {
                continue;
            }

            // Perpendicular unit vector
            let perp = Vec2::new(-dir.y / len, dir.x / len);
            let half_width = width / 2.0;

            left_points.push(Point::new(
                point.x + perp.x * half_width,
                point.y + perp.y * half_width,
            ));
            right_points.push(Point::new(
                point.x - perp.x * half_width,
                point.y - perp.y * half_width,
            ));
        }

        if left_points.is_empty() {
            return;
        }

        // Build the path: left side forward, right side backward
        let mut path = BezPath::new();
        path.move_to(left_points[0]);
        for point in left_points.iter().skip(1) {
            path.line_to(*point);
        }
        // Connect to right side (reversed)
        for point in right_points.iter().rev() {
            path.line_to(*point);
        }
        path.close_path();

        // Fill the path (no sloppiness - freehand is already hand-drawn)
        self.scene
            .fill(Fill::NonZero, transform, color, None, &path);
    }

    fn draw_underlines(
        &mut self,
        text: &drafftink_core::shapes::Text,
        layout: &parley::Layout<Brush>,
        transform: Affine,
        font_size: f32,
    ) {
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(run) = item {
                    if let Some(decoration) = &run.style().underline {
                        let thickness = decoration
                            .size
                            .unwrap_or((run.run().font_size() / 16.0).max(1.0));
                        let y = run.baseline()
                            + text_script_offset(text, run.run().text_range().start)
                            - crate::text_editor::inline_baseline_shift(&line, font_size)
                            + decoration.offset.unwrap_or(run.run().font_size() / 10.0);
                        let width: f32 = run.glyphs().map(|g| g.advance).sum();
                        self.scene.fill(
                            Fill::NonZero,
                            transform,
                            &decoration.brush,
                            None,
                            &Rect::new(
                                run.offset() as f64,
                                y as f64,
                                (run.offset() + width) as f64,
                                (y + thickness) as f64,
                            ),
                        );
                    }
                }
            }
        }
    }

    pub fn text_formula_geometry(
        &self,
        text: &drafftink_core::shapes::Text,
        byte: usize,
    ) -> Option<Rect> {
        let cached = self
            .text_cache
            .iter()
            .find(|((id, _), _)| *id == text.id())?
            .1;
        let id = cached
            .layout
            .inline_boxes()
            .iter()
            .find(|b| b.index == byte)?
            .id;
        for line in cached.layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::InlineBox(b) = item {
                    if b.id == id {
                        return Some(Rect::new(
                            b.x as f64,
                            b.y as f64,
                            (b.x + b.width) as f64,
                            (b.y + b.height) as f64,
                        ));
                    }
                }
            }
        }
        None
    }
    pub fn text_formula_at(
        &self,
        text: &drafftink_core::shapes::Text,
        local: Point,
    ) -> Option<usize> {
        text.formulas.iter().find_map(|formula| {
            let byte = text.content.char_indices().nth(formula.at)?.0;
            self.text_formula_geometry(text, byte)?
                .contains(local)
                .then_some(byte)
        })
    }

    fn prepare_inline_formulas(
        &mut self,
        text: &drafftink_core::shapes::Text,
    ) -> Vec<(u64, usize, f32, f32)> {
        let local = text
            .custom_font_postscript
            .as_ref()
            .and_then(|key| self.registered_font_data.get(key))
            .or_else(|| {
                text.custom_font
                    .as_ref()
                    .and_then(|key| self.registered_font_data.get(key))
            })
            .cloned();
        use drafftink_core::shapes::{FontFamily, FontWeight};
        let embedded: &[u8] = match (text.font_family, text.font_weight) {
            (FontFamily::NotoSans, FontWeight::Heavy) => NOTO_SANS_BOLD,
            (FontFamily::NotoSans, FontWeight::Light) => NOTO_SANS_ITALIC,
            (FontFamily::GelPen, FontWeight::Light) => GELPEN_LIGHT,
            (FontFamily::GelPen, FontWeight::Heavy) => GELPEN_HEAVY,
            (FontFamily::GelPen, _) => GELPEN_REGULAR,
            (FontFamily::GelPenSerif, FontWeight::Light) => GELPEN_SERIF_LIGHT,
            (FontFamily::GelPenSerif, FontWeight::Heavy) => GELPEN_SERIF_HEAVY,
            (FontFamily::GelPenSerif, _) => GELPEN_SERIF_MEDIUM,
            (FontFamily::VanillaExtract, _) => VANILLA_EXTRACT,
            (FontFamily::XitsMath, _) => XITS_MATH,
            _ => NOTO_SANS,
        };
        let primary = local.as_deref().map(|v| v.as_slice()).unwrap_or(embedded);
        let mut boxes = Vec::new();
        for (index, formula) in text.formulas.iter().enumerate() {
            let mut math = formula.math.clone();
            math.font_size = text.font_size;
            math.style = text.style.clone();
            if self.prepare_math_with_primary(&math, primary) {
                if let Some((width, height, depth)) = math.cached_size() {
                    if let Some((byte, '\u{fffc}')) = text.content.char_indices().nth(formula.at) {
                        boxes.push((
                            index as u64,
                            byte,
                            width as f32 + 4.0,
                            2.0 * ((height as f32
                                - crate::text_editor::math_layout_axis(
                                    XITS_MATH,
                                    0,
                                    text.font_size as f32,
                                ))
                            .max(
                                crate::text_editor::math_layout_axis(
                                    XITS_MATH,
                                    0,
                                    text.font_size as f32,
                                ) - depth as f32,
                            ))
                            .max(text.font_size as f32 * 0.8)
                                + 4.0,
                        ));
                    }
                }
            }
        }
        let height = boxes.iter().map(|b| b.3).fold(0.0f32, f32::max);
        for b in &mut boxes {
            b.3 = height;
        }
        boxes
    }

    fn append_inline_formulas(
        &mut self,
        text: &drafftink_core::shapes::Text,
        layout: &parley::Layout<Brush>,
        transform: Affine,
    ) {
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::InlineBox(inline) = item {
                    if let Some(formula) = text.formulas.get(inline.id as usize) {
                        if let Some(cached) = self.math_cache.get(&formula.math.id()) {
                            self.scene.append(
                                &cached.scene,
                                Some(
                                    transform
                                        * Affine::translate((
                                            inline.x as f64 + 2.0,
                                            inline.y as f64
                                                + inline.height as f64 * 0.5
                                                + crate::text_editor::math_layout_axis(
                                                    XITS_MATH,
                                                    0,
                                                    text.font_size as f32,
                                                )
                                                    as f64,
                                        )),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Render a text shape using Parley for proper text layout.
    fn render_text(&mut self, text: &drafftink_core::shapes::Text, transform: Affine) {
        use parley::StyleProperty;
        use parley::layout::PositionedLayoutItem;
        use std::hash::{Hash, Hasher};

        // Skip empty text
        if text.content.is_empty() {
            let cursor_height = text.font_size * 1.2;
            let cursor = kurbo::Line::new(
                Point::new(text.position.x, text.position.y),
                Point::new(text.position.x, text.position.y + cursor_height),
            );
            let stroke = Stroke::new(2.0);
            self.scene.stroke(
                &stroke,
                transform,
                Color::from_rgba8(100, 100, 100, 200),
                None,
                &cursor,
            );
            return;
        }

        use drafftink_core::shapes::{FontFamily, FontWeight};

        let inline = self.prepare_inline_formulas(text);
        // Build cache key from content hash
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.content.hash(&mut hasher);
        (text.font_family as u8).hash(&mut hasher);
        text.custom_font.hash(&mut hasher);
        text.custom_font_postscript.hash(&mut hasher);
        (text.font_weight as u8).hash(&mut hasher);
        text.font_size.to_bits().hash(&mut hasher);
        for formula in &text.formulas {
            formula.at.hash(&mut hasher);
            formula.math.latex.hash(&mut hasher);
        }
        text.char_styles.hash(&mut hasher);
        text.char_colors.len().hash(&mut hasher);
        for c in &text.char_colors {
            c.is_some().hash(&mut hasher);
            if let Some(color) = c {
                color.r.hash(&mut hasher);
                color.g.hash(&mut hasher);
                color.b.hash(&mut hasher);
            }
        }
        text.style.stroke_color.r.hash(&mut hasher);
        text.style.stroke_color.g.hash(&mut hasher);
        text.style.stroke_color.b.hash(&mut hasher);
        text.style.stroke_color.a.hash(&mut hasher);
        text.style.opacity.to_bits().hash(&mut hasher);
        let cache_key = (text.id(), hasher.finish());

        // Check cache
        if let Some(cached) = self.text_cache.get(&cache_key) {
            text.set_cached_size(cached.width, cached.height);
            let text_transform = transform
                * Affine::translate((text.position.x, text.position.y))
                * Affine::scale_non_uniform(text.display_scale[0], text.display_scale[1]);

            self.scene.append(&cached.scene, Some(text_transform));
            return;
        }

        let style = &text.style;
        let brush = Brush::Solid(style.stroke_with_opacity());
        let font_size = text.font_size as f32;

        let (font_name, parley_weight, is_italic) = if let Some(custom) =
            text.custom_font.as_deref()
        {
            (
                self.font_aliases
                    .get(custom)
                    .map(String::as_str)
                    .unwrap_or("Noto Sans"),
                parley::FontWeight::new(text.font_weight.value()),
                false,
            )
        } else {
            match (&text.font_family, &text.font_weight) {
                (FontFamily::GelPen, FontWeight::Light) => {
                    ("GelPenLight", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::GelPen, FontWeight::Regular | FontWeight::Medium) => {
                    ("GelPen", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::GelPen, FontWeight::Heavy) => {
                    ("GelPenHeavy", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::NotoSans, FontWeight::Light) => {
                    ("Noto Sans", parley::FontWeight::NORMAL, true)
                }
                (FontFamily::NotoSans, FontWeight::Medium) => {
                    ("Noto Sans", parley::FontWeight::new(500.0), false)
                }
                (FontFamily::NotoSans, FontWeight::Regular) => {
                    ("Noto Sans", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::NotoSans, FontWeight::Heavy) => {
                    ("Noto Sans", parley::FontWeight::BOLD, false)
                }
                (FontFamily::GelPenSerif, FontWeight::Light) => {
                    ("GelPenSerifLight", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::GelPenSerif, FontWeight::Regular | FontWeight::Medium) => {
                    ("GelPenSerif", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::GelPenSerif, FontWeight::Heavy) => {
                    ("GelPenSerifHeavy", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::VanillaExtract, _) => {
                    ("Vanilla Extract", parley::FontWeight::NORMAL, false)
                }
                (FontFamily::XitsMath, _) => ("STIX Two Math", parley::FontWeight::NORMAL, false),
            }
        };

        let layout_text = text.content.replace('￼', "​");
        let mut builder =
            self.layout_cx
                .ranged_builder(&mut self.font_cx, &layout_text, 1.0, false);
        for &(id, index, width, height) in &inline {
            builder.push_inline_box(parley::InlineBox {
                id,
                index,
                width,
                height,
            });
            builder.push(StyleProperty::FontSize(0.0), index..index + 3);
            builder.push(
                StyleProperty::LineHeight(parley::LineHeight::Absolute(height + font_size * 0.4)),
                index..index + 3,
            );
        }
        builder.push_default(StyleProperty::FontSize(font_size));
        if text.char_styles.iter().any(|s| s.script != 0) {
            builder.push_default(StyleProperty::LineHeight(parley::LineHeight::Absolute(
                font_size * 1.8,
            )));
        }
        builder.push_default(StyleProperty::Brush(brush.clone()));
        builder.push_default(StyleProperty::FontWeight(parley_weight));
        if is_italic {
            builder.push_default(StyleProperty::FontStyle(parley::FontStyle::Italic));
        }
        builder.push_default(StyleProperty::FontStack(text_font_stack(font_name)));

        let mut byte_offset = 0;
        for (char_idx, ch) in text.content.chars().enumerate() {
            if let Some(Some(color)) = text.char_colors.get(char_idx) {
                let color: peniko::Color = (*color).into();
                let span_brush = Brush::Solid(color);
                let char_len = ch.len_utf8();
                builder.push(
                    StyleProperty::Brush(span_brush),
                    byte_offset..byte_offset + char_len,
                );
            }
            if let Some(style) = text.char_styles.get(char_idx) {
                let range = byte_offset..byte_offset + ch.len_utf8();
                if style.script != 0 {
                    builder.push(
                        parley::StyleProperty::FontSize(
                            text.font_size as f32 * if style.script > 0 { 0.65 } else { 0.64 },
                        ),
                        range.clone(),
                    );
                }
                if style.bold {
                    builder.push(
                        parley::StyleProperty::FontWeight(parley::FontWeight::BOLD),
                        range.clone(),
                    );
                }
                if style.italic {
                    builder.push(
                        parley::StyleProperty::FontStyle(parley::FontStyle::Italic),
                        range.clone(),
                    );
                }
                if style.underline {
                    builder.push(parley::StyleProperty::Underline(true), range);
                }
            }
            byte_offset += ch.len_utf8();
        }

        let mut layout = builder.build(&layout_text);
        layout.break_all_lines(None);
        layout.align(
            None,
            parley::Alignment::Start,
            parley::AlignmentOptions::default(),
        );

        let layout_width = layout.width() as f64;
        let layout_height = layout.height() as f64;
        text.set_cached_size(layout_width, layout_height);
        let previous_scene = std::mem::take(&mut self.scene);
        let text_transform = Affine::IDENTITY;
        self.draw_underlines(text, &layout, text_transform, text.font_size as f32);
        self.append_inline_formulas(text, &layout, text_transform);
        let mut glyph_count = 0;

        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let mut x = glyph_run.offset();
                let y = glyph_run.baseline()
                    + text_script_offset(text, glyph_run.run().text_range().start)
                    - crate::text_editor::inline_baseline_shift(&line, text.font_size as f32);
                let run = glyph_run.run();
                if run.font_size() <= 0.0 {
                    continue;
                }
                let font = run.font();
                let run_font_size = run.font_size();
                let synthesis = run.synthesis();
                let skew_angle = synthesis
                    .skew()
                    .map(|angle| angle.to_radians().tan() as f64);
                let glyph_xform = skew_angle.map(|angle| Affine::skew(angle, 0.0));
                let run_brush = glyph_run.style().brush.clone();

                let glyphs: Vec<vello::Glyph> = glyph_run
                    .glyphs()
                    .map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        glyph_count += 1;
                        vello::Glyph {
                            id: glyph.id,
                            x: gx,
                            y: gy,
                        }
                    })
                    .collect();

                if !glyphs.is_empty() {
                    self.scene
                        .draw_glyphs(font)
                        .brush(&run_brush)
                        .hint(true)
                        .transform(text_transform)
                        .glyph_transform(glyph_xform)
                        .font_size(run_font_size)
                        .normalized_coords(run.normalized_coords())
                        .draw(Fill::NonZero, glyphs.iter().cloned());

                    if synthesis.embolden() {
                        self.scene
                            .draw_glyphs(font)
                            .brush(&run_brush)
                            .hint(true)
                            .transform(text_transform)
                            .glyph_transform(glyph_xform)
                            .font_size(run_font_size)
                            .normalized_coords(run.normalized_coords())
                            .draw(
                                &Stroke::new(run_font_size as f64 / 24.0),
                                glyphs.iter().cloned(),
                            );
                    }
                }
            }
        }

        let scene = std::mem::replace(&mut self.scene, previous_scene);
        self.scene.append(
            &scene,
            Some(
                transform
                    * Affine::translate((text.position.x, text.position.y))
                    * Affine::scale_non_uniform(text.display_scale[0], text.display_scale[1]),
            ),
        );
        // One layout per text object; old edit versions must not accumulate.
        self.text_cache.retain(|(id, _), _| *id != text.id());
        self.text_cache.insert(
            cache_key,
            CachedTextLayout {
                layout: layout.clone(),
                scene,
                width: layout_width,
                height: layout_height,
            },
        );

        if glyph_count == 0 {
            let width = text.content.len() as f64 * text.font_size * 0.6;
            let height = text.font_size * 1.2;
            let rect = Rect::new(
                text.position.x,
                text.position.y,
                text.position.x + width.max(20.0),
                text.position.y + height,
            );
            self.scene.fill(
                Fill::NonZero,
                transform,
                Color::from_rgba8(255, 100, 100, 100),
                None,
                &rect,
            );
        }
    }

    /// Render an image shape.
    fn render_image(&mut self, image: &drafftink_core::shapes::Image, transform: Affine) {
        let source_key = image.data_base64.key();
        self.image_cache_clock = self.image_cache_clock.wrapping_add(1);
        let needed = ((image.width * self.zoom / image.crop.width().max(0.001))
            .max(image.height * self.zoom / image.crop.height().max(0.001)))
        .ceil();
        let bucket = (needed.clamp(1.0, MAX_PREVIEW_SIDE as f64) as u32).next_power_of_two();
        let image_data = if self.full_resolution_images {
            // Full-quality exports don't replace or retain the display preview.
            decode_image_preview(image, 0)
        } else if let Some(cached) = self
            .image_cache
            .get_mut(&source_key)
            .filter(|entry| entry.bucket == buck