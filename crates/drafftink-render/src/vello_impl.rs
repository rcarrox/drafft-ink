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
            .filter(|entry| entry.bucket == bucket)
        {
            cached.last_used = self.image_cache_clock;
            Some(cached.image.clone())
        } else {
            let decoded = decode_image_preview(image, bucket);
            if let Some(data) = &decoded {
                self.image_cache.insert(
                    source_key,
                    CachedImage {
                        image: data.clone(),
                        bucket,
                        last_used: self.image_cache_clock,
                    },
                );
                self.trim_image_cache();
            }
            decoded
        };
        let Some(image_data) = image_data else {
            self.render_image_placeholder(image, transform);
            return;
        };

        // Calculate the transform to scale and position the image
        let bounds = image.bounds();
        let scale_x = bounds.width() / (image.crop.width() * image_data.width as f64);
        let scale_y = bounds.height() / (image.crop.height() * image_data.height as f64);

        let mirror = Affine::translate((bounds.center().x, bounds.center().y))
            * Affine::scale_non_uniform(
                if image.flip_x { -1.0 } else { 1.0 },
                if image.flip_y { -1.0 } else { 1.0 },
            )
            * Affine::translate((-bounds.center().x, -bounds.center().y));
        let image_transform = transform
            * mirror
            * Affine::translate((
                bounds.x0 - image.crop.x0 * image_data.width as f64 * scale_x,
                bounds.y0 - image.crop.y0 * image_data.height as f64 * scale_y,
            ))
            * Affine::scale_non_uniform(scale_x, scale_y);

        self.scene.push_clip_layer(transform, &bounds);
        self.scene.draw_image(&image_data.into(), image_transform);
        self.scene.pop_layer();
    }

    /// Render a placeholder for images that couldn't be loaded.
    fn render_image_placeholder(
        &mut self,
        image: &drafftink_core::shapes::Image,
        transform: Affine,
    ) {
        let bounds = image.bounds();

        // Draw a gray rectangle with an X
        let rect_path = bounds.to_path(0.1);
        self.scene.fill(
            Fill::NonZero,
            transform,
            Color::from_rgba8(200, 200, 200, 255),
            None,
            &rect_path,
        );

        // Draw diagonal lines (X pattern)
        let stroke = Stroke::new(2.0);
        let mut x_path = BezPath::new();
        x_path.move_to(Point::new(bounds.x0, bounds.y0));
        x_path.line_to(Point::new(bounds.x1, bounds.y1));
        x_path.move_to(Point::new(bounds.x1, bounds.y0));
        x_path.line_to(Point::new(bounds.x0, bounds.y1));
        self.scene.stroke(
            &stroke,
            transform,
            Color::from_rgba8(150, 150, 150, 255),
            None,
            &x_path,
        );

        // Draw border
        self.scene.stroke(
            &stroke,
            transform,
            Color::from_rgba8(100, 100, 100, 255),
            None,
            &rect_path,
        );
    }

    pub fn formula_is_valid(&mut self, latex: &str) -> bool {
        self.prepare_math(&drafftink_core::shapes::Math::new(
            Point::ZERO,
            latex.to_string(),
        ))
    }

    fn prepare_math(&mut self, math: &drafftink_core::shapes::Math) -> bool {
        use drafftink_core::shapes::{FontFamily, FontWeight};
        let f = &math.font;
        let local = f
            .postscript
            .as_ref()
            .and_then(|k| self.registered_font_data.get(k))
            .or_else(|| {
                f.custom
                    .as_ref()
                    .and_then(|k| self.registered_font_data.get(k))
            })
            .cloned();
        let embedded = match (f.family, f.weight) {
            (FontFamily::GelPen, FontWeight::Light) => GELPEN_LIGHT,
            (FontFamily::GelPen, FontWeight::Heavy) => GELPEN_HEAVY,
            (FontFamily::GelPen, _) => GELPEN_REGULAR,
            (FontFamily::GelPenSerif, FontWeight::Light) => GELPEN_SERIF_LIGHT,
            (FontFamily::GelPenSerif, FontWeight::Heavy) => GELPEN_SERIF_HEAVY,
            (FontFamily::GelPenSerif, _) => GELPEN_SERIF_MEDIUM,
            (FontFamily::VanillaExtract, _) => VANILLA_EXTRACT,
            (FontFamily::XitsMath, _) => XITS_MATH,
            (FontFamily::NotoSans, FontWeight::Heavy) => NOTO_SANS_BOLD,
            _ => NOTO_SANS,
        };
        self.prepare_math_with_primary(
            math,
            local.as_deref().map(|v| v.as_slice()).unwrap_or(embedded),
        )
    }
    fn prepare_math_with_primary(
        &mut self,
        math: &drafftink_core::shapes::Math,
        primary_bytes: &[u8],
    ) -> bool {
        use crate::rex_backend::VelloBackend;
        use rex::font::backend::ttf_parser::TtfMathFont;
        use rex::layout::engine::LayoutBuilder;
        use rex::render::Renderer as RexRenderer;
        if math.latex.trim().is_empty() {
            return false;
        }
        let rgba = math.style.stroke_with_opacity().to_rgba8();
        let color = [rgba.r, rgba.g, rgba.b, rgba.a];
        if let Some(cached) = self.math_cache.get(&math.id()) {
            if cached.source == math.latex
                && cached.font_size == math.font_size.to_bits()
                && cached.color == color
                && cached.primary_font_id == primary_bytes.as_ptr() as usize
            {
                math.set_cached_size(cached.size.0, cached.size.1, cached.size.2);
                return true;
            }
        }
        let Ok(math_face) = ttf_parser::Face::parse(XITS_MATH, 0) else {
            return false;
        };
        let Ok(math_font) = TtfMathFont::new(math_face) else {
            return false;
        };
        // Ordinary letters/digits use their parent Text font; MATH supplies structural metrics.
        let primary_face = ttf_parser::Face::parse(primary_bytes, 0).ok();
        let Ok(nodes) = rex::parser::parse(&math.latex) else {
            return false;
        };
        let mixed_font = crate::rex_backend::MixedMathFont::new(math_font, primary_face);
        let engine = LayoutBuilder::new(&mixed_font)
            .font_size(math.font_size)
            .build();
        let Ok(layout) = engine.layout(&nodes) else {
            return false;
        };
        let size = layout.size();
        let mut scene = Scene::new();
        let mut backend = VelloBackend::new(
            &mut scene,
            &mixed_font.math,
            mixed_font.primary.as_ref(),
            Affine::IDENTITY,
            math.style.stroke_with_opacity(),
        );
        RexRenderer::new().render(&layout, &mut backend);
        math.set_cached_size(size.width, size.height, size.depth);
        self.math_cache.insert(
            math.id(),
            CachedMath {
                source: math.latex.clone(),
                font_size: math.font_size.to_bits(),
                primary_font_id: primary_bytes.as_ptr() as usize,
                color,
                scene,
                size: (size.width, size.height, size.depth),
            },
        );
        #[cfg(test)]
        {
            self.math_layout_builds += 1;
        }
        true
    }

    fn render_math(&mut self, math: &drafftink_core::shapes::Math, transform: Affine) {
        if math.latex.trim().is_empty() {
            return;
        }
        if !self.prepare_math(math) {
            self.render_math_error(math, transform, "Invalid formula");
            return;
        }
        if let Some(cached) = self.math_cache.get(&math.id()) {
            self.scene.append(
                &cached.scene,
                Some(
                    transform
                        * Affine::translate((math.position.x, math.position.y))
                        * Affine::scale_non_uniform(math.display_scale[0], math.display_scale[1]),
                ),
            );
        }
    }

    /// Render error placeholder for math that couldn't be rendered.
    fn render_math_error(
        &mut self,
        math: &drafftink_core::shapes::Math,
        transform: Affine,
        _msg: &str,
    ) {
        let bounds = math.bounds();
        let rect_path = bounds.to_path(0.1);
        self.scene.fill(
            Fill::NonZero,
            transform,
            Color::from_rgba8(255, 200, 200, 100),
            None,
            &rect_path,
        );
        let stroke = Stroke::new(1.0);
        self.scene.stroke(
            &stroke,
            transform,
            Color::from_rgba8(255, 100, 100, 255),
            None,
            &rect_path,
        );
    }

    /// Render a text shape in edit mode using PlainEditor state.
    /// This renders the text with cursor and selection highlights.
    pub fn render_text_editing(
        &mut self,
        text: &drafftink_core::shapes::Text,
        edit_state: &mut TextEditState,
        transform: Affine,
        anchor: Option<Point>,
    ) {
        use drafftink_core::shapes::{FontFamily as ShapeFontFamily, FontWeight};

        let style = &text.style;
        let brush = Brush::Solid(style.stroke_with_opacity());

        // Determine font name and parley weight based on family and weight
        let inline = self.prepare_inline_formulas(text);
        // Use same logic as render_text - all Roboto variants use "Roboto" family with weight
        let (font_name, parley_weight, is_italic) =
            if let Some(custom) = text.custom_font.as_deref() {
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
                    (ShapeFontFamily::GelPen, FontWeight::Light) => {
                        ("GelPenLight", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::GelPen, FontWeight::Regular | FontWeight::Medium) => {
                        ("GelPen", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::GelPen, FontWeight::Heavy) => {
                        ("GelPenHeavy", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::NotoSans, FontWeight::Light) => {
                        ("Noto Sans", parley::FontWeight::NORMAL, true)
                    }
                    (ShapeFontFamily::NotoSans, FontWeight::Medium) => {
                        ("Noto Sans", parley::FontWeight::new(500.0), false)
                    }
                    (ShapeFontFamily::NotoSans, FontWeight::Regular) => {
                        ("Noto Sans", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::NotoSans, FontWeight::Heavy) => {
                        ("Noto Sans", parley::FontWeight::BOLD, false)
                    }
                    (ShapeFontFamily::GelPenSerif, FontWeight::Light) => {
                        ("GelPenSerifLight", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::GelPenSerif, FontWeight::Regular | FontWeight::Medium) => {
                        ("GelPenSerif", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::GelPenSerif, FontWeight::Heavy) => {
                        ("GelPenSerifHeavy", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::VanillaExtract, _) => {
                        ("Vanilla Extract", parley::FontWeight::NORMAL, false)
                    }
                    (ShapeFontFamily::XitsMath, _) => {
                        ("STIX Two Math", parley::FontWeight::NORMAL, false)
                    }
                }
            };

        // Configure the editor styles
        edit_state.set_font_size(text.font_size as f32);
        edit_state.set_brush(brush.clone());

        // Set the font family and weight in the editor
        {
            use parley::StyleProperty;
            let styles = edit_state.editor_mut().edit_styles();
            styles.insert(StyleProperty::FontStack(text_font_stack(font_name)));
            styles.insert(StyleProperty::FontWeight(parley_weight));
            if is_italic {
                styles.insert(StyleProperty::FontStyle(parley::FontStyle::Italic));
            }
        }

        // Get the current text content from the editor
        let editor_text: String = edit_state.editor().text().to_string().replace('￼', "​");

        // Build a layout with per-character colors (PlainEditor doesn't support ranged styles)
        let mut builder =
            self.layout_cx
                .ranged_builder(&mut self.font_cx, &editor_text, 1.0, false);
        for &(id, index, width, height) in &inline {
            builder.push_inline_box(parley::InlineBox {
                id,
                index,
                width,
                height,
            });
            builder.push(parley::StyleProperty::FontSize(0.0), index..index + 3);
            builder.push(
                parley::StyleProperty::LineHeight(parley::LineHeight::Absolute(
                    height + text.font_size as f32 * 0.4,
                )),
                index..index + 3,
            );
        }
        builder.push_default(parley::StyleProperty::FontSize(text.font_size as f32));
        if text.char_styles.iter().any(|s| s.script != 0) {
            builder.push_default(parley::StyleProperty::LineHeight(
                parley::LineHeight::Absolute(text.font_size as f32 * 1.8),
            ));
        }
        builder.push_default(parley::StyleProperty::Brush(brush.clone()));
        builder.push_default(parley::StyleProperty::FontWeight(parley_weight));
        builder.push_default(parley::StyleProperty::FontStack(text_font_stack(font_name)));

        // Apply per-character colors
        let mut byte_offset = 0;
        for (char_idx, ch) in editor_text.chars().enumerate() {
            if let Some(Some(color)) = text.char_colors.get(char_idx) {
                let color: peniko::Color = (*color).into();
                let span_brush = Brush::Solid(color);
                let char_len = ch.len_utf8();
                builder.push(
                    parley::StyleProperty::Brush(span_brush),
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

        let mut styled_layout = builder.build(&editor_text);
        styled_layout.break_all_lines(None);
        styled_layout.align(
            None,
            parley::Alignment::Start,
            parley::AlignmentOptions::default(),
        );

        // Also update the editor's layout for cursor/selection geometry
        let layout = edit_state
            .editor_mut()
            .layout(&mut self.font_cx, &mut self.layout_cx);
        let _ = layout;
        let layout_width = styled_layout.width() as f64;
        let layout_height = styled_layout.height() as f64;
        edit_state.set_rich_layout(styled_layout.clone());
        edit_state.sync_cursor_script(text);

        // Update cached size so bounds() returns correct values
        text.set_cached_size(layout_width, layout_height);

        // Compute position that keeps anchor (top-left handle) fixed
        // anchor = position + (half_w, half_h) + rotate(-half_w, -half_h)
        // position = anchor - (half_w, half_h) - rotate(-half_w, -half_h)
        let rotation = text.rotation;
        let half_w = layout_width * text.display_scale[0] / 2.0;
        let half_h = layout_height * text.display_scale[1] / 2.0;

        let render_position = if let Some(anchor) = anchor {
            if rotation.abs() > 0.001 {
                let cos_r = rotation.cos();
                let sin_r = rotation.sin();
                let rot_x = -half_w * cos_r + half_h * sin_r;
                let rot_y = -half_w * sin_r - half_h * cos_r;
                Point::new(anchor.x - half_w - rot_x, anchor.y - half_h - rot_y)
            } else {
                anchor
            }
        } else {
            text.position
        };

        // Create transform using computed render position
        let text_transform = if rotation.abs() > 0.001 {
            let center = Point::new(render_position.x + half_w, render_position.y + half_h);
            let center_vec = kurbo::Vec2::new(center.x, center.y);
            transform
                * Affine::translate(center_vec)
                * Affine::rotate(rotation)
                * Affine::translate(-center_vec)
                * Affine::translate((render_position.x, render_position.y))
        } else {
            transform * Affine::translate((render_position.x, render_position.y))
        };

        let text_transform = text_transform
            * Affine::scale_non_uniform(text.display_scale[0], text.display_scale[1]);

        self.append_inline_formulas(text, &styled_layout, text_transform);
        self.draw_underlines(text, &styled_layout, text_transform, text.font_size as f32);
        // Render glyphs first (text content) - use styled_layout which has color spans
        for line in styled_layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let glyph_style = glyph_run.style();
                let mut x = glyph_run.offset();
                let y = glyph_run.baseline()
                    + text_script_offset(text, glyph_run.run().text_range().start)
                    - crate::text_editor::inline_baseline_shift(&line, text.font_size as f32);
                let run = glyph_run.run();
                if run.font_size() <= 0.0 {
                    continue;
                }
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                let glyphs: Vec<vello::Glyph> = glyph_run
                    .glyphs()
                    .map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
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
                        .brush(&glyph_style.brush)
                        .hint(true)
                        .transform(text_transform)
                        .glyph_transform(glyph_xform)
                        .font_size(font_size)
                        .normalized_coords(run.normalized_coords())
                        .draw(Fill::NonZero, glyphs.iter().cloned());
                    if synthesis.embolden() {
                        self.scene
                            .draw_glyphs(font)
                            .brush(&glyph_style.brush)
                            .hint(true)
                            .transform(text_transform)
                            .glyph_transform(glyph_xform)
                            .font_size(font_size)
                            .normalized_coords(run.normalized_coords())
                            .draw(
                                &Stroke::new(font_size as f64 / 24.0),
                                glyphs.iter().cloned(),
                            );
                    }
                }
            }
        }

        // Selection color (semi-transparent blue)
        let selection_color = self.selection_color.with_alpha(0.5);

        // Draw selection background (now layout is computed)
        edit_state.selection_geometry_with(|rect, _| {
            self.scene.fill(
                Fill::NonZero,
                text_transform,
                selection_color,
                None,
                &convert_rect(&rect),
            );
        });

        // Draw cursor if visible (now layout is computed)
        if edit_state.is_cursor_visible() {
            if let Some(cursor) = edit_state.cursor_geometry(1.5) {
                // Cursor color (contrasting with text)
                let cursor_color = Color::from_rgba8(0, 0, 0, 255);
                self.scene.fill(
                    Fill::NonZero,
                    text_transform,
                    cursor_color,
                    None,
                    &convert_rect(&cursor),
                );
            } else if edit_state.text().is_empty() {
                // If text is empty, show a placeholder cursor at origin
                let cursor_height = text.font_size * 1.2;
                let cursor_rect = Rect::new(0.0, 0.0, 1.5, cursor_height);
                self.scene.fill(
                    Fill::NonZero,
                    text_transform,
                    Color::from_rgba8(0, 0, 0, 255),
                    None,
                    &cursor_rect,
                );
            }
        }
    }

    /// DEBUG: Render anchor point visualization
    pub fn render_debug_anchor(&mut self, anchor: Point, transform: Affine) {
        let size = 10.0 / self.zoom;
        // Red cross at anchor position
        let mut path = BezPath::new();
        path.move_to(Point::new(anchor.x - size, anchor.y));
        path.line_to(Point::new(anchor.x + size, anchor.y));
        path.move_to(Point::new(anchor.x, anchor.y - size));
        path.line_to(Point::new(anchor.x, anchor.y + size));
        self.scene.stroke(
            &Stroke::new(2.0 / self.zoom),
            transform,
            Color::from_rgba8(255, 0, 0, 255),
            None,
            &path,
        );
        // Red circle
        let circle = kurbo::Circle::new(anchor, size / 2.0);
        self.scene.stroke(
            &Stroke::new(2.0 / self.zoom),
            transform,
            Color::from_rgba8(255, 0, 0, 255),
            None,
            &circle,
        );
    }

    /// Render shape-specific selection handles.
    /// Handles are scaled inversely with zoom to maintain constant screen size.
    fn render_shape_handles(&mut self, shape: &Shape, transform: Affine) {
        let handles = get_handles(shape);
        // Pinned shapes use identity transform and are already in viewport pixels.
        let screen_space = transform.as_coeffs() == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let pixel_scale = if screen_space { 1.0 } else { 1.0 / self.zoom };
        let handle_size = 6.0 * pixel_scale;
        let stroke_width = pixel_scale;
        let dash_len = 4.0 * pixel_scale;

        // For lines/arrows, draw a light dashed line connecting the endpoints
        // For rectangles/ellipses, draw the bounding box
        match shape {
            Shape::Line(_) | Shape::Arrow(_) => {
                // Just draw the endpoint handles, no bounding box
            }
            _ => {
                // Draw selection rectangle for non-line shapes
                let bounds = drafftink_core::selection::selection_bounds(shape);
                let rotation = shape.rotation();
                let stroke = Stroke::new(stroke_width).with_dashes(0.0, [dash_len, dash_len]);

                // Build path for the bounding box
                let mut path = BezPath::new();
                path.move_to(Point::new(bounds.x0, bounds.y0));
                path.line_to(Point::new(bounds.x1, bounds.y0));
                path.line_to(Point::new(bounds.x1, bounds.y1));
                path.line_to(Point::new(bounds.x0, bounds.y1));
                path.close_path();

                // Apply rotation around center if shape is rotated
                let box_transform = if rotation.abs() > 0.001 {
                    let center = bounds.center();
                    transform
                        * Affine::translate((center.x, center.y))
                        * Affine::rotate(rotation)
                        * Affine::translate((-center.x, -center.y))
                } else {
                    transform
                };

                self.scene
                    .stroke(&stroke, box_transform, self.selection_color, None, &path);
            }
        }

        // Draw handles
        for handle in handles {
            self.render_handle(&handle, transform, handle_size, pixel_scale);
        }
    }

    /// Render a single handle.
    /// Stroke widths are scaled inversely with zoom to maintain constant screen size.
    fn render_handle(&mut self, handle: &Handle, transform: Affine, size: f64, pixel_scale: f64) {
        let pos = handle.position;
        let stroke_width_thick = 2.0 * pixel_scale;
        let stroke_width_thin = 1.5 * pixel_scale;

        // Different handle shapes based on type
        match handle.kind {
            HandleKind::Endpoint(_) | HandleKind::IntermediatePoint(_) => {
                // Circle handle for endpoints and intermediate points (lines/arrows)
                let radius = size / 2.0;
                let ellipse = kurbo::Ellipse::new(pos, (radius, radius), 0.0);
                let path = ellipse.to_path(0.1);

                // White fill
                self.scene
                    .fill(Fill::NonZero, transform, Color::WHITE, None, &path);

                // Blue border
                self.scene.stroke(
                    &Stroke::new(stroke_width_thick),
                    transform,
                    self.selection_color,
                    None,
                    &path,
                );
            }
            HandleKind::SegmentMidpoint(_) => {
                // Smaller circle for segment midpoints (virtual handles)
                let radius = size / 3.0;
                let ellipse = kurbo::Ellipse::new(pos, (radius, radius), 0.0);
                let path = ellipse.to_path(0.1);

                // Light fill
                self.scene.fill(
                    Fill::NonZero,
                    transform,
                    Color::from_rgba8(200, 220, 255, 200),
                    None,
                    &path,
                );

                // Blue border
                self.scene.stroke(
                    &Stroke::new(stroke_width_thin),
                    transform,
                    self.selection_color,
                    None,
                    &path,
                );
            }
            HandleKind::Corner(_) | HandleKind::Edge(_) => {
                // Small circular controls; the hit area remains generous.
                let path = kurbo::Ellipse::new(pos, (size / 2.0, size / 2.0), 0.0).to_path(0.1);

                // White fill
                self.scene
                    .fill(Fill::NonZero, transform, Color::WHITE, None, &path);

                // Blue border
                self.scene.stroke(
                    &Stroke::new(stroke_width_thin),
                    transform,
                    self.selection_color,
                    None,
                    &path,
                );
            }
            HandleKind::Rotate => {
                // Rotation handle: circle with rotation icon
                let radius = size / 2.0;
                let ellipse = kurbo::Ellipse::new(pos, (radius, radius), 0.0);
                let path = ellipse.to_path(0.1);

                // White fill
                self.scene
                    .fill(Fill::NonZero, transform, Color::WHITE, None, &path);

                // Blue border
                self.scene.stroke(
                    &Stroke::new(stroke_width_thick),
                    transform,
                    self.selection_color,
                    None,
                    &path,
                );

                // Draw rotation arc inside the handle
                let arc_radius = radius * 0.5;
                let mut arc_path = BezPath::new();
                // Draw a 270° arc
                let start_angle = -std::f64::consts::FRAC_PI_4;
                let end_angle = start_angle + std::f64::consts::PI * 1.5;
                let steps = 12;
                for i in 0..=steps {
                    let t = i as f64 / steps as f64;
                    let angle = start_angle + t * (end_angle - start_angle);
                    let x = pos.x + arc_radius * angle.cos();
                    let y = pos.y + arc_radius * angle.sin();
                    if i == 0 {
                        arc_path.move_to(Point::new(x, y));
                    } else {
                        arc_path.line_to(Point::new(x, y));
                    }
                }
                self.scene.stroke(
                    &Stroke::new(stroke_width_thin),
                    transform,
                    self.selection_color,
                    None,
                    &arc_path,
                );
            }
        }
    }
}

impl Renderer for VelloRenderer {
    fn build_scene(&mut self, ctx: &RenderContext) {
        if !self.cache_scope_prepared {
            self.retain_open_document_caches(std::iter::once(&ctx.canvas.document));
        }
        self.cache_scope_prepared = false;
        // Clear the scene
        self.scene.reset();
        self.selection_color = ctx.selection_color;
        self.zoom = ctx.canvas.camera.zoom;

        let camera_transform = ctx.canvas.camera.transform();

        // Draw grid based on style
        use crate::renderer::GridStyle;
        let viewport = Rect::new(0.0, 0.0, ctx.viewport_size.width, ctx.viewport_size.height);
        match ctx.grid_style {
            GridStyle::None => {}
            GridStyle::Lines => self.render_grid_lines(viewport, camera_transform, 20.0),
            GridStyle::HorizontalLines => {
                self.render_horizontal_lines(viewport, camera_transform, 20.0)
            }
            GridStyle::CrossPlus => self.render_grid_crosses(viewport, camera_transform, 20.0),
            GridStyle::Dots => self.render_grid_dots(viewport, camera_transform, 20.0),
        }

        // Compute world-space viewport for culling
        let world_viewport = Rect::new(
            -ctx.canvas.camera.offset.x / ctx.canvas.camera.zoom,
            -ctx.canvas.camera.offset.y / ctx.canvas.camera.zoom,
            (-ctx.canvas.camera.offset.x + ctx.viewport_size.width) / ctx.canvas.camera.zoom,
            (-ctx.canvas.camera.offset.y + ctx.viewport_size.height) / ctx.canvas.camera.zoom,
        );

        // Draw all shapes in z-order (skip shape being edited or off-screen)
        for shape in ctx.canvas.document.shapes_ordered() {
            if ctx.canvas.document.is_pinned(shape.id()) {
                continue;
            }
            if ctx.editing_shape_id == Some(shape.id()) {
                continue;
            }
            // Viewport culling (inflate bounds to handle zero-area shapes like vertical/horizontal lines)
            let shape_bounds = shape.bounds().inflate(1.0, 1.0);
            if !shape_bounds.intersect(world_viewport).is_zero_area() {
                let is_selected = ctx.canvas.is_selected(shape.id());
                self.render_shape(shape, camera_transform, is_selected);
            }
        }

        // Pinned shapes are stored in screen pixels; draw them above the canvas using
        // the identity transform so their position and size do not follow the camera.
        for shape in ctx.canvas.document.shapes_ordered() {
            let Some(pin) = ctx.canvas.document.pinned_shapes.get(&shape.id()) else {
                continue;
            };
            let bounds = shape.bounds().inflate(4.0, 4.0);
            let bg = Color::from(pin.background);
            if bg.to_rgba8().a > 0 {
                let background_transform =
                    Affine::rotate_about(shape.rotation(), shape.bounds().center());
                let mut background_path = BezPath::new();
                background_path.move_to(Point::new(bounds.x0, bounds.y0));
                background_path.line_to(Point::new(bounds.x1, bounds.y0));
                background_path.line_to(Point::new(bounds.x1, bounds.y1));
                background_path.line_to(Point::new(bounds.x0, bounds.y1));
                background_path.close_path();
                self.scene.fill(
                    Fill::NonZero,
                    background_transform,
                    bg,
                    None,
                    &background_path,
                );
            }
            self.render_shape(shape, Affine::IDENTITY, ctx.canvas.is_selected(shape.id()));
            // Match assets/pin.svg; keep it centered at the top with a soft shadow.
            let mut pin_mark = BezPath::new();
            pin_mark.move_to(Point::new(4.0, 19.0));
            pin_mark.line_to(Point::new(8.455, 14.546));
            pin_mark.move_to(Point::new(12.273, 5.0));
            pin_mark.line_to(Point::new(18.0, 10.727));
            pin_mark.line_to(Point::new(13.546, 6.273));
            pin_mark.line_to(Point::new(9.727, 10.09));
            pin_mark.curve_to(Point::new(9.727, 10.09), Point::new(7.182, 9.454), Point::new(5.273, 11.363));
            pin_mark.line_to(Point::new(11.636, 17.726));
            pin_mark.curve_to(Point::new(13.545, 15.817), Point::new(12.909, 13.272), Point::new(12.909, 13.272));
            pin_mark.line_to(Point::new(16.727, 9.454));
            pin_mark.line_to(Point::new(13.546, 6.272));
            pin_mark.close_path();
            let marker = Affine::translate((bounds.center().x - 9.0, bounds.y0 - 14.0)) * Affine::scale(0.75);
            self.scene.stroke(&Stroke::new(3.2), marker * Affine::translate((0.7, 0.9)),
                Color::from_rgba8(0, 0, 0, 55), None, &pin_mark);
            self.scene.stroke(&Stroke::new(2.2), marker, Color::from_rgb8(65, 65, 65), None, &pin_mark);
        }

        // Draw preview shape if tool is active
        if let Some(preview) = ctx.canvas.tool_manager.preview_shape() {
            self.render_shape(&preview, camera_transform, false);
        }

        // Draw selection rectangle (marquee)
        if let Some(rect) = ctx.selection_rect {
            self.render_selection_rect(rect, camera_transform);
        }

        // Draw smart guides
        if !ctx.smart_guides.is_empty() {
            self.render_smart_guides(&ctx.smart_guides, camera_transform);
        }

        // Draw snap guides
        if let Some(snap_point) = ctx.snap_point {
            self.render_snap_guides(snap_point, camera_transform, ctx.viewport_size);
        }

        // Draw angle snap guides (polar rays and arc)
        if let Some(ref angle_info) = ctx.angle_snap_info {
            self.render_angle_snap_guides(angle_info, camera_transform, ctx.viewport_size);
        }

        // Draw rotation helper lines
        if let Some(ref rotation_info) = ctx.rotation_info {
            self.render_rotation_guides(rotation_info, camera_transform);
        }

        // Draw eraser cursor
        if let Some((pos, radius)) = ctx.eraser_cursor {
            self.render_eraser_cursor(pos, radius, camera_transform);
        }

        // Draw laser pointer
        if let Some((pos, ref trail)) = ctx.laser_pointer {
            self.render_laser_pointer(pos, trail, camera_transform, ctx.laser_color);
        }
    }
}

impl VelloRenderer {
    /// Render snap guides (crosshairs at the snap point).
    fn render_snap_guides(
        &mut self,
        snap_point: Point,
        transform: Affine,
        viewport_size: kurbo::Size,
    ) {
        // Snap guide color - subtle magenta/pink
        let guide_color = Color::from_rgba8(236, 72, 153, 180); // Pink-500 with alpha

        // Stroke width scaled inversely with zoom for constant screen appearance
        let stroke_width = 1.0 / self.zoom;
        let stroke = Stroke::new(stroke_width);

        // We need to draw lines in world coordinates that span the visible area
        // Get the inverse transform to convert screen bounds to world bounds
        let inv_transform = transform.inverse();
        let world_top_left = inv_transform * Point::new(0.0, 0.0);
        let world_bottom_right =
            inv_transform * Point::new(viewport_size.width, viewport_size.height);

        // Horizontal line through snap point (full width)
        let mut h_path = BezPath::new();
        h_path.move_to(Point::new(world_top_left.x, snap_point.y));
        h_path.line_to(Point::new(world_bottom_right.x, snap_point.y));
        self.scene
            .stroke(&stroke, transform, guide_color, None, &h_path);

        // Vertical line through snap point (full height)
        let mut v_path = BezPath::new();
        v_path.move_to(Point::new(snap_point.x, world_top_left.y));
        v_path.line_to(Point::new(snap_point.x, world_bottom_right.y));
        self.scene
            .stroke(&stroke, transform, guide_color, None, &v_path);

        // Small circle at the intersection
        let circle_radius = 4.0 / self.zoom;
        let circle = kurbo::Circle::new(snap_point, circle_radius);
        self.scene.stroke(
            &Stroke::new(stroke_width * 2.0),
            transform,
            guide_color,
            None,
            &circle,
        );
    }

    /// Render a small text badge (background rect + white text).
    fn render_badge(&mut self, text: &str, center: Point, bg_color: Color, transform: Affine) {
        use parley::{PositionedLayoutItem, StyleProperty};

        let font_size = 11.0_f32;
        let padding = 3.0 / self.zoom;

        // Build text layout using same font as text tool
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, false);
        builder.push_default(StyleProperty::FontSize(font_size));
        builder.push_default(StyleProperty::Brush(Brush::Solid(Color::WHITE)));
        builder.push_default(StyleProperty::FontStack(parley::FontStack::Single(
            parley::FontFamily::Named("Noto Sans".into()),
        )));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);

        // Text dimensions in screen pixels, convert to world coords
        let text_width = layout.width() as f64 / self.zoom;
        let text_height = layout.height() as f64 / self.zoom;

        // Background rect in world coords
        let rect = Rect::new(
            center.x - text_width / 2.0 - padding,
            center.y - text_height / 2.0 - padding,
            center.x + text_width / 2.0 + padding,
            center.y + text_height / 2.0 + padding,
        );
        self.scene
            .fill(Fill::NonZero, transform, bg_color, None, &rect);

        // Render text - scale down by zoom to match world coords
        let text_x = center.x - text_width / 2.0;
        let text_y = center.y - text_height / 2.0;
        let text_transform =
            transform * Affine::translate((text_x, text_y)) * Affine::scale(1.0 / self.zoom);

        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let mut x = glyph_run.offset();
                let y = glyph_run.baseline();
                let run = glyph_run.run();
                if run.font_size() <= 0.0 {
                    continue;
                }
                let font = run.font();
                let font_size = run.font_size();

                let glyphs: Vec<vello::Glyph> = glyph_run
                    .glyphs()
                    .map(|g| {
                        let gx = x + g.x;
                        let gy = y - g.y;
                        x += g.advance;
                        vello::Glyph {
                            id: g.id,
                            x: gx,
                            y: gy,
                        }
                    })
                    .collect();

                if !glyphs.is_empty() {
                    self.scene
                        .draw_glyphs(font)
                        .brush(&Brush::Solid(Color::WHITE))
                        .hint(true)
                        .transform(text_transform)
                        .font_size(font_size)
                        .normalized_coords(run.normalized_coords())
                        .draw(Fill::NonZero, glyphs.into_iter());
                }
            }
        }
    }

    /// Render smart guide lines (magenta alignment guides like Figma).
    fn render_smart_guides(
        &mut self,
        guides: &[drafftink_core::snap::SmartGuide],
        transform: Affine,
    ) {
        use drafftink_core::snap::SmartGuideKind;

        let guide_color = Color::from_rgba8(236, 72, 153, 255); // Pink/magenta
        let stroke_width = 1.0 / self.zoom;
        let stroke = Stroke::new(stroke_width);
        let cap_size = 4.0 / self.zoom;
        let x_size = 3.0 / self.zoom;
        let mut badges: Vec<(String, Point)> = Vec::new();

        for guide in guides {
            let mut path = BezPath::new();
            match guide.kind {
                SmartGuideKind::Vertical => {
                    path.move_to(Point::new(guide.position, guide.start));
                    path.line_to(Point::new(guide.position, guide.end));
                    // Draw X markers at snap points
                    for &y in &guide.snap_points {
                        path.move_to(Point::new(guide.position - x_size, y - x_size));
                        path.line_to(Point::new(guide.position + x_size, y + x_size));
                        path.move_to(Point::new(guide.position - x_size, y + x_size));
                        path.line_to(Point::new(guide.position + x_size, y - x_size));
                    }
                }
                SmartGuideKind::Horizontal => {
                    path.move_to(Point::new(guide.start, guide.position));
                    path.line_to(Point::new(guide.end, guide.position));
                    // Draw X markers at snap points
                    for &x in &guide.snap_points {
                        path.move_to(Point::new(x - x_size, guide.position - x_size));
                        path.line_to(Point::new(x + x_size, guide.position + x_size));
                        path.move_to(Point::new(x - x_size, guide.position + x_size));
                        path.line_to(Point::new(x + x_size, guide.position - x_size));
                    }
                }
                SmartGuideKind::EqualSpacingH => {
                    // Horizontal gap: line with vertical end caps
                    let y = guide.position;
                    path.move_to(Point::new(guide.start, y));
                    path.line_to(Point::new(guide.end, y));
                    path.move_to(Point::new(guide.start, y - cap_size));
                    path.line_to(Point::new(guide.start, y + cap_size));
                    path.move_to(Point::new(guide.end, y - cap_size));
                    path.line_to(Point::new(guide.end, y + cap_size));
                    // Queue badge
                    let dist = (guide.end - guide.start).abs();
                    let center = Point::new((guide.start + guide.end) / 2.0, y);
                    badges.push((format!("{:.0}", dist), center));
                }
                SmartGuideKind::EqualSpacingV => {
                    // Vertical gap: line with horizontal end caps
                    let x = guide.position;
                    path.move_to(Point::new(x, guide.start));
                    path.line_to(Point::new(x, guide.end));
                    path.move_to(Point::new(x - cap_size, guide.start));
                    path.line_to(Point::new(x + cap_size, guide.start));
                    path.move_to(Point::new(x - cap_size, guide.end));
                    path.line_to(Point::new(x + cap_size, guide.end));
                    // Queue badge
                    let dist = (guide.end - guide.start).abs();
                    let center = Point::new(x, (guide.start + guide.end) / 2.0);
                    badges.push((format!("{:.0}", dist), center));
                }
            }
            self.scene
                .stroke(&stroke, transform, guide_color, None, &path);
        }

        // Render badges after all strokes
        for (text, center) in badges {
            self.render_badge(&text, center, guide_color, transform);
        }
    }

    /// Render angle snap visualization (polar rays and angle arc).
    fn render_angle_snap_guides(
        &mut self,
        info: &crate::renderer::AngleSnapInfo,
        transform: Affine,
        viewport_size: kurbo::Size,
    ) {
        use std::f64::consts::PI;

        // Colors
        let ray_color = Color::from_rgba8(100, 100, 100, 60); // Very subtle gray
        let active_ray_color = Color::from_rgba8(236, 72, 153, 200); // Magenta for active snap angle
        let arc_color = Color::from_rgba8(236, 72, 153, 220); // Magenta for angle arc

        // Stroke widths scaled inversely with zoom
        let thin_stroke_width = 0.5 / self.zoom;
        let thick_stroke_width = 1.5 / self.zoom;

        let start = info.start_point;

        // Calculate ray length based on viewport
        let inv_transform = transform.inverse();
        let world_top_left = inv_transform * Point::new(0.0, 0.0);
        let world_bottom_right =
            inv_transform * Point::new(viewport_size.width, viewport_size.height);
        let viewport_diagonal = ((world_bottom_right.x - world_top_left.x).powi(2)
            + (world_bottom_right.y - world_top_left.y).powi(2))
        .sqrt();
        let ray_length = viewport_diagonal;

        // Draw polar rays at 15° intervals (0°, 15°, 30°, ..., 345°)
        let mut path = BezPath::new();
        for i in 0..24 {
            let angle_deg = i as f64 * 15.0;
            let angle_rad = angle_deg * PI / 180.0;
            let end_x = start.x + ray_length * angle_rad.cos();
            let end_y = start.y + ray_length * angle_rad.sin();
            path.move_to(start);
            path.line_to(Point::new(end_x, end_y));
        }
        self.scene.stroke(
            &Stroke::new(thin_stroke_width),
            transform,
            ray_color,
            None,
            &path,
        );

        // Highlight the active snap angle ray if snapped
        if info.is_snapped {
            let angle_rad = info.angle_degrees * PI / 180.0;
            let mut active_path = BezPath::new();
            active_path.move_to(start);
            active_path.line_to(Point::new(
                start.x + ray_length * angle_rad.cos(),
                start.y + ray_length * angle_rad.sin(),
            ));
            self.scene.stroke(
                &Stroke::new(thick_stroke_width),
                transform,
                active_ray_color,
                None,
                &active_path,
            );

            // Draw angle arc from 0° to the snapped angle
            let arc_radius = 30.0 / self.zoom;
            let segments = (info.angle_degrees.abs() / 5.0).ceil() as usize;
            let segments = segments.clamp(2, 72); // At least 2, at most 72 segments

            if segments > 1 {
                let mut arc_path = BezPath::new();
                let start_angle = 0.0_f64;
                let end_angle = info.angle_degrees * PI / 180.0;

                let first_x = start.x + arc_radius * start_angle.cos();
                let first_y = start.y + arc_radius * start_angle.sin();
                arc_path.move_to(Point::new(first_x, first_y));

                for i in 1..=segments {
                    let t = i as f64 / segments as f64;
                    let angle = start_angle + t * (end_angle - start_angle);
                    let x = start.x + arc_radius * angle.cos();
                    let y = start.y + arc_radius * angle.sin();
                    arc_path.line_to(Point::new(x, y));
                }

                self.scene.stroke(
                    &Stroke::new(thick_stroke_width),
                    transform,
                    arc_color,
                    None,
                    &arc_path,
                );
            }

            // Draw angle label (e.g., "45°")
            // Position the label at the midpoint of the arc
            let label_angle = info.angle_degrees * PI / 360.0; // Half the angle
            let label_radius = arc_radius + 15.0 / self.zoom;
            let _label_pos = Point::new(
                start.x + label_radius * label_angle.cos(),
                start.y + label_radius * label_angle.sin(),
            );

            // Note: Text rendering would require Parley/font setup which is complex
            // For now, we skip the text label - the arc itself shows the angle visually
        }
    }

    /// Render rotation helper lines (center crosshair and angle indicator).
    fn render_rotation_guides(&mut self, info: &crate::renderer::RotationInfo, transform: Affine) {
        use std::f64::consts::PI;

        // Colors
        let guide_color = Color::from_rgba8(236, 72, 153, 200); // Magenta
        let snap_color = Color::from_rgba8(34, 197, 94, 220); // Green when snapped

        let color = if info.snapped {
            snap_color
        } else {
            guide_color
        };

        // Stroke widths scaled inversely with zoom
        let stroke_width = 1.5 / self.zoom;

        let center = info.center;

        // Draw center crosshair
        let cross_size = 10.0 / self.zoom;
        let mut cross_path = BezPath::new();
        cross_path.move_to(Point::new(center.x - cross_size, center.y));
        cross_path.line_to(Point::new(center.x + cross_size, center.y));
        cross_path.move_to(Point::new(center.x, center.y - cross_size));
        cross_path.line_to(Point::new(center.x, center.y + cross_size));
        self.scene.stroke(
            &Stroke::new(stroke_width),
            transform,
            color,
            None,
            &cross_path,
        );

        // Draw rotation indicator line from center outward at current angle
        let indicator_length = 50.0 / self.zoom;
        // Angle is measured from top (negative Y), so we need to adjust
        let angle = info.angle - PI / 2.0;
        let end_x = center.x + indicator_length * angle.cos();
        let end_y = center.y + indicator_length * angle.sin();

        let mut indicator_path = BezPath::new();
        indicator_path.move_to(center);
        indicator_path.line_to(Point::new(end_x, end_y));
        self.scene.stroke(
            &Stroke::new(stroke_width * 1.5),
            transform,
            color,
            None,
            &indicator_path,
        );

        // Draw reference line at 0° (pointing up)
        let ref_end_y = center.y - indicator_length;
        let mut ref_path = BezPath::new();
        ref_path.move_to(center);
        ref_path.line_to(Point::new(center.x, ref_end_y));

        // Dashed stroke for reference line
        let dash_pattern = [4.0 / self.zoom, 4.0 / self.zoom];
        let dashed_stroke = Stroke::new(stroke_width).with_dashes(0.0, dash_pattern);
        self.scene.stroke(
            &dashed_stroke,
            transform,
            Color::from_rgba8(150, 150, 150, 150),
            None,
            &ref_path,
        );

        // Draw angle arc from 0° to current angle
        if info.angle.abs() > 0.01 {
            let arc_radius = 25.0 / self.zoom;
            let segments = ((info.angle.abs() * 180.0 / PI) / 5.0).ceil() as usize;
            let segments = segments.clamp(2, 72);

            let mut arc_path = BezPath::new();
            let start_angle = -PI / 2.0; // 0° is up
            let end_angle = info.angle - PI / 2.0;

            let first_x = center.x + arc_radius * start_angle.cos();
            let first_y = center.y + arc_radius * start_angle.sin();
            arc_path.move_to(Point::new(first_x, first_y));

            for i in 1..=segments {
                let t = i as f64 / segments as f64;
                let a = start_angle + t * (end_angle - start_angle);
                let x = center.x + arc_radius * a.cos();
                let y = center.y + arc_radius * a.sin();
                arc_path.line_to(Point::new(x, y));
            }

            self.scene.stroke(
                &Stroke::new(stroke_width),
                transform,
                color,
                None,
                &arc_path,
            );
        }
    }

    /// Render a selection rectangle (marquee).
    /// Stroke width and dash pattern are scaled inversely with zoom.
    fn render_selection_rect(&mut self, rect: Rect, transform: Affine) {
        // Fill with semi-transparent blue
        let fill_color = self.selection_color.with_alpha(0.1);
        let mut path = BezPath::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));
        path.line_to(Point::new(rect.x0, rect.y1));
        path.close_path();

        self.scene
            .fill(Fill::NonZero, transform, fill_color, None, &path);

        // Stroke with blue dashed line - scale inversely with zoom
        let stroke_width = 1.0 / self.zoom;
        let dash_len = 4.0 / self.zoom;
        let stroke = Stroke::new(stroke_width).with_dashes(0.0, [dash_len, dash_len]);
        self.scene
            .stroke(&stroke, transform, self.selection_color, None, &path);
    }

    /// Render eraser cursor (circle showing eraser radius).
    fn render_eraser_cursor(&mut self, pos: Point, radius: f64, transform: Affine) {
        let circle = kurbo::Circle::new(pos, radius);

        // Semi-transparent fill
        let fill_color = Color::from_rgba8(255, 100, 100, 50);
        self.scene
            .fill(Fill::NonZero, transform, fill_color, None, &circle);

        // Stroke
        let stroke_width = 2.0 / self.zoom;
        let stroke_color = Color::from_rgba8(200, 50, 50, 200);
        self.scene.stroke(
            &Stroke::new(stroke_width),
            transform,
            stroke_color,
            None,
            &circle,
        );
    }

    /// Render laser pointer with trail.
    fn render_laser_pointer(
        &mut self,
        pos: Point,
        trail: &[(Point, f64)],
        transform: Affine,
        laser_color: Color,
    ) {
        let rgb = laser_color.to_rgba8();
        // Draw trail with fading effect
        for (point, alpha) in trail {
            let a = (*alpha * 255.0) as u8;
            let color = Color::from_rgba8(rgb.r, rgb.g, rgb.b, a);
            let radius = 4.0 / self.zoom * *alpha;
            let circle = kurbo::Circle::new(*point, radius);
            self.scene
                .fill(Fill::NonZero, transform, color, None, &circle);
        }

        // Draw main pointer (bright red dot with glow)
        let glow_radius = 12.0 / self.zoom;
        let glow_color = Color::from_rgba8(rgb.r, rgb.g, rgb.b, 100);
        let glow = kurbo::Circle::new(pos, glow_radius);
        self.scene
            .fill(Fill::NonZero, transform, glow_color, None, &glow);

        let main_radius = 6.0 / self.zoom;
        let main_color = Color::from_rgba8(rgb.r, rgb.g, rgb.b, 255);
        let main = kurbo::Circle::new(pos, main_radius);
        self.scene
            .fill(Fill::NonZero, transform, main_color, None, &main);

        // White center
        let center_radius = 2.0 / self.zoom;
        let center = kurbo::Circle::new(pos, center_radius);
        self.scene
            .fill(Fill::NonZero, transform, Color::WHITE, None, &center);
    }
}

impl VelloRenderer {
    /// Calculate grid bounds from viewport and transform.
    fn grid_bounds(
        &self,
        viewport: Rect,
        transform: Affine,
        grid_size: f64,
    ) -> (f64, f64, f64, f64) {
        let inv = transform.inverse();
        let world_tl = inv * Point::new(viewport.x0, viewport.y0);
        let world_br = inv * Point::new(viewport.x1, viewport.y1);

        let start_x = (world_tl.x / grid_size).floor() * grid_size;
        let start_y = (world_tl.y / grid_size).floor() * grid_size;
        let end_x = (world_br.x / grid_size).ceil() * grid_size;
        let end_y = (world_br.y / grid_size).ceil() * grid_size;

        (start_x, start_y, end_x, end_y)
    }

    /// Render full grid lines.
    fn render_grid_lines(&mut self, viewport: Rect, transform: Affine, grid_size: f64) {
        let grid_color = Color::from_rgba8(200, 200, 200, 100);
        let stroke = Stroke::new(0.5);

        let (start_x, start_y, end_x, end_y) = self.grid_bounds(viewport, transform, grid_size);

        // Vertical lines
        let mut x = start_x;
        while x <= end_x {
            let mut path = BezPath::new();
            path.move_to(Point::new(x, start_y));
            path.line_to(Point::new(x, end_y));
            self.scene
                .stroke(&stroke, transform, grid_color, None, &path);
            x += grid_size;
        }

        // Horizontal lines
        let mut y = start_y;
        while y <= end_y {
            let mut path = BezPath::new();
            path.move_to(Point::new(start_x, y));
            path.line_to(Point::new(end_x, y));
            self.scene
                .stroke(&stroke, transform, grid_color, None, &path);
            y += grid_size;
        }
    }

    fn render_horizontal_lines(&mut self, viewport: Rect, transform: Affine, grid_size: f64) {
        let grid_color = Color::from_rgba8(200, 200, 200, 100);
        let stroke = Stroke::new(0.5);

        let (start_x, start_y, _, end_y) = self.grid_bounds(viewport, transform, grid_size);
        let end_x = start_x + viewport.width() / transform.as_coeffs()[0];

        let mut y = start_y;
        while y <= end_y {
            let mut path = BezPath::new();
            path.move_to(Point::new(start_x, y));
            path.line_to(Point::new(end_x, y));
            self.scene
                .stroke(&stroke, transform, grid_color, None, &path);
            y += grid_size;
        }
    }

    /// Render grid as small crosses (+) at intersections.
    /// Uses batched paths for performance.
    fn render_grid_crosses(&mut self, viewport: Rect, transform: Affine, grid_size: f64) {
        let grid_color = Color::from_rgba8(180, 180, 180, 60); // Reduced opacity
        let stroke = Stroke::new(1.0);
        let cross_size = 3.0; // Half-size of the cross arms

        let (start_x, start_y, end_x, end_y) = self.grid_bounds(viewport, transform, grid_size);

        // Batch all crosses into a single path for performance
        let mut path = BezPath::new();

        let mut x = start_x;
        while x <= end_x {
            let mut y = start_y;
            while y <= end_y {
                // Horizontal arm
                path.move_to(Point::new(x - cross_size, y));
                path.line_to(Point::new(x + cross_size, y));
                // Vertical arm
                path.move_to(Point::new(x, y - cross_size));
                path.line_to(Point::new(x, y + cross_size));
                y += grid_size;
            }
            x += grid_size;
        }

        // Single draw call for all crosses
        self.scene
            .stroke(&stroke, transform, grid_color, None, &path);
    }

    /// Render grid as dots at intersections.
    /// Uses batched rectangles for performance.
    fn render_grid_dots(&mut self, viewport: Rect, transform: Affine, grid_size: f64) {
        let grid_color = Color::from_rgba8(160, 160, 160, 70); // Reduced opacity
        let dot_size = 1.5; // Half-size of the dot

        let (start_x, start_y, end_x, end_y) = self.grid_bounds(viewport, transform, grid_size);

        // Batch all dots into a single path for performance
        let mut path = BezPath::new();

        let mut x = start_x;
        while x <= end_x {
            let mut y = start_y;
            while y <= end_y {
                // Add a small square for each dot (cheaper than ellipse)
                let rect = Rect::new(x - dot_size, y - dot_size, x + dot_size, y + dot_size);
                path.move_to(Point::new(rect.x0, rect.y0));
                path.line_to(Point::new(rect.x1, rect.y0));
                path.line_to(Point::new(rect.x1, rect.y1));
                path.line_to(Point::new(rect.x0, rect.y1));
                path.close_path();
                y += grid_size;
            }
            x += grid_size;
        }

        // Single draw call for all dots
        self.scene
            .fill(Fill::NonZero, transform, grid_color, None, &path);
    }

    #[allow(dead_code)]
    fn render_selection_handles(&mut self, bounds: Rect, transform: Affine) {
        let handle_size = 16.0;
        let stroke = Stroke::new(2.0);

        // Selection rectangle
        let mut path = BezPath::new();
        path.move_to(Point::new(bounds.x0, bounds.y0));
        path.line_to(Point::new(bounds.x1, bounds.y0));
        path.line_to(Point::new(bounds.x1, bounds.y1));
        path.line_to(Point::new(bounds.x0, bounds.y1));
        path.close_path();

        self.scene
            .stroke(&stroke, transform, self.selection_color, None, &path);

        // Corner handles
        let corners = [
            Point::new(bounds.x0, bounds.y0),
            Point::new(bounds.x1, bounds.y0),
            Point::new(bounds.x1, bounds.y1),
            Point::new(bounds.x0, bounds.y1),
        ];

        for corner in corners {
            let handle_rect = Rect::new(
                corner.x - handle_size / 2.0,
                corner.y - handle_size / 2.0,
                corner.x + handle_size / 2.0,
                corner.y + handle_size / 2.0,
            );

            // White fill
            self.scene.fill(
                Fill::NonZero,
                transform,
                Color::WHITE,
                None,
                &handle_rect.to_path(0.1),
            );

            // Blue border
            self.scene.stroke(
                &Stroke::new(1.5),
                transform,
                self.selection_color,
                None,
                &handle_rect.to_path(0.1),
            );
        }
    }
}

fn outline_stroke(width: f64, pattern: StrokeStyle) -> Stroke {
    let stroke = Stroke::new(width);
    match pattern {
        StrokeStyle::Solid => stroke,
        StrokeStyle::Dashed => stroke.with_dashes(0.0, [width * 6.0, width * 3.0]),
        StrokeStyle::DashedShort => stroke.with_dashes(0.0, [width * 2.0, width * 2.0]),
        StrokeStyle::Dotted => stroke
            .with_caps(kurbo::Cap::Round)
            .with_dashes(0.0, [0.01, width * 2.5]),
    }
}

impl ShapeRenderer for VelloRenderer {
    fn render_shape(&mut self, shape: &Shape, transform: Affine, selected: bool) {
        if let Shape::Math(math) = shape {
            self.prepare_math(math);
        }
        // Get rotation and apply rotation transform around shape center
        let rotation = shape.rotation();
        let shape_transform = if rotation.abs() > 0.001 {
            let center = shape.bounds().center();
            let center_vec = kurbo::Vec2::new(center.x, center.y);
            transform
                * Affine::translate(center_vec)
                * Affine::rotate(rotation)
                * Affine::translate(-center_vec)
        } else {
            transform
        };

        // Special handling for different shape types
        match shape {
            Shape::Text(text) => {
                self.render_text(text, shape_transform);
            }
            Shape::Group(group) => {
                // Render each child in the group
                for child in group.children() {
                    // Children are not individually selected when the group is selected
                    self.render_shape(child, shape_transform, false);
                }
            }
            Shape::Image(image) => {
                self.render_image(image, shape_transform);
            }
            Shape::Line(line) => {
                let path = shape.to_path();
                if line.closed {
                    self.render_path_cached(
                        &shape.id().to_string(),
                        &path,
                        shape.style(),
                        shape_transform,
                    );
                } else {
                    self.render_stroke_only(
                        &path,
                        shape.style(),
                        line.stroke_style,
                        shape_transform,
                    );
                }
            }
            Shape::Arrow(arrow) => {
                let path = shape.to_path();
                self.render_stroke_only(&path, shape.style(), arrow.stroke_style, shape_transform);
            }
            Shape::Freehand(freehand) => {
                if freehand.closed {
                    let path = shape.to_path();
                    self.render_path_cached(
                        &shape.id().to_string(),
                        &path,
                        shape.style(),
                        shape_transform,
                    );
                } else if freehand.has_pressure() {
                    self.render_freehand_with_pressure(freehand, shape_transform);
                } else {
                    let path = shape.to_path();
                    self.render_stroke_only(
                        &path,
                        shape.style(),
                        StrokeStyle::Solid,
                        shape_transform,
                    );
                }
            }
            Shape::Math(math) => {
                self.render_math(math, shape_transform);
            }
            _ => {
                let path = shape.to_path();
                self.render_path_cached(
                    &shape.id().to_string(),
                    &path,
                    shape.style(),
                    shape_transform,
                );
            }
        }

        // Draw selection highlight with shape-specific handles
        // Use original transform for handles (they're already rotated in get_handles)
        if selected {
            self.render_shape_handles(shape, transform);
        }
    }

    fn render_grid(&mut self, viewport: Rect, transform: Affine, grid_size: f64) {
        // Default implementation - full lines
        self.render_grid_lines(viewport, transform, grid_size);
    }

    fn render_selection_handles(&mut self, _bounds: Rect, _transform: Affine) {
        // Not used - we use shape-specific handles via render_shape_handles
    }
}

impl VelloRenderer {
    /// Draw a remote user's cursor at a screen position.
    ///
    /// The cursor is rendered as a small pointer arrow with the user's color.
    pub fn draw_cursor(&mut self, screen_pos: Point, color: Color) {
        // Draw cursor pointer (simple triangle pointing up-right)
        let mut path = BezPath::new();
        // Triangle: tip at screen_pos, pointing up-right
        path.move_to(screen_pos); // tip
        path.line_to(Point::new(screen_pos.x, screen_pos.y + 18.0)); // bottom-left
        path.line_to(Point::new(screen_pos.x + 14.0, screen_pos.y + 14.0)); // bottom-right
        path.close_path();

        // Fill the cursor
        self.scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            color,
            None,
            &path,
        );

        // White stroke for visibility against any background
        let stroke = Stroke::new(1.5);
        self.scene
            .stroke(&stroke, Affine::IDENTITY, Color::WHITE, None, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drafftink_core::canvas::Canvas;
    use drafftink_core::shapes::Rectangle;

    #[test]
    fn test_renderer_creation() {
        let renderer = VelloRenderer::new();
        assert!(renderer.scene().encoding().is_empty());
    }

    #[test]
    fn test_build_empty_scene() {
        let mut renderer = VelloRenderer::new();
        let canvas = Canvas::new();
        let ctx = RenderContext::new(&canvas, kurbo::Size::new(800.0, 600.0));

        renderer.build_scene(&ctx);
        // Scene should have grid lines at minimum
    }

    #[test]
    fn test_build_scene_with_shapes() {
        let mut renderer = VelloRenderer::new();
        let mut canvas = Canvas::new();

        let rect = Rectangle::new(Point::new(100.0, 100.0), 200.0, 150.0);
        canvas.document.add_shape(Shape::Rectangle(rect));

        let ctx = RenderContext::new(&canvas, kurbo::Size::new(800.0, 600.0));
        renderer.build_scene(&ctx);
    }

    #[test]
    fn export_includes_pinned_shapes_at_the_camera_position_at_capture_time() {
        use drafftink_core::canvas::{Canvas, PinnedShape};
        use drafftink_core::shapes::SerializableColor;

        let mut canvas = Canvas::new();
        let mut rectangle = Rectangle::new(Point::new(200.0, 120.0), 50.0, 30.0);
        rectangle.rotation = std::f64::consts::FRAC_PI_4;
        let id = rectangle.id();
        canvas.document.add_shape(Shape::Rectangle(rectangle));
        canvas.document.pinned_shapes.insert(
            id,
            PinnedShape {
                background: SerializableColor::new(190, 220, 250, 255),
            },
        );

        // Camera screen->world transform for a 0.5 zoom and non-zero pan.
        let screen_to_world = Affine::translate((-120.0, -60.0)) * Affine::scale(2.0);
        let mut renderer = VelloRenderer::new();
        let (_, bounds) =
            renderer.build_export_scene_with_pins(&canvas.document, 1.0, Some(screen_to_world));
        let bounds = bounds.expect("a pinned-only canvas should export");
        assert!(bounds.width() > 150.0, "pinned screen position/scale must affect export bounds");
        assert!(bounds.height() > 130.0, "rotated pinned background must affect export bounds");

        let (_, without_capture_transform) = renderer.build_export_scene(&canvas.document, 1.0);
        assert!(without_capture_transform.is_none(), "the default exporter must not guess a viewport position");
    }
}

#[cfg(test)]
mod cache_regressions {
    use super::*;
    use drafftink_core::shapes::{Image, ImageFormat, Math};
    #[test]
    fn stable_formula_reuses_layout_until_content_changes() {
        let mut renderer = VelloRenderer::new();
        let mut math = Math::new(Point::ZERO, "x^{3}".into());
        for _ in 0..10 {
            renderer.render_shape(&Shape::Math(math.clone()), Affine::IDENTITY, false);
        }
        assert_eq!(renderer.math_layout_builds, 1);
        assert_eq!(renderer.math_cache.len(), 1);
        math.position = Point::new(50.0, 70.0);
        math.rotation = 0.7;
        math.display_scale = [1.5, 1.0];
        renderer.render_shape(&Shape::Math(math.clone()), Affine::IDENTITY, false);
        assert_eq!(renderer.math_layout_builds, 1);
        math.set_latex("x^{4}".into());
        renderer.render_shape(&Shape::Math(math), Affine::IDENTITY, false);
        assert_eq!(renderer.math_layout_builds, 2);
        assert_eq!(renderer.math_cache.len(), 1);
    }
    #[test]
    fn duplicates_decode_once_and_deleted_sources_leave_cache() {
        let image = ::image::DynamicImage::ImageRgba8(::image::RgbaImage::from_pixel(
            8,
            8,
            ::image::Rgba([255, 0, 0, 255]),
        ));
        let mut png = std::io::Cursor::new(Vec::new());
        image.write_to(&mut png, ::image::ImageFormat::Png).unwrap();
        let mut renderer = VelloRenderer::new();
        for x in [0.0, 100.0] {
            let image = Image::new(Point::new(x, 0.0), png.get_ref(), 8, 8, ImageFormat::Png);
            renderer.render_shape(&Shape::Image(image), Affine::IDENTITY, false);
        }
        assert_eq!(renderer.image_cache.len(), 1);
        let canvas = drafftink_core::Canvas::new();
        renderer.build_scene(&RenderContext::new(&canvas, kurbo::Size::new(800.0, 600.0)));
        assert!(renderer.image_cache.is_empty());
    }
}

#[cfg(test)]
mod inline_formula_render_tests {
    use super::*;
    use drafftink_core::shapes::{CharacterStyle, InlineFormula, Math, Text};
    #[test]
    fn structured_text_formulas_have_real_layout_and_keep_following_text() {
        let mut renderer = VelloRenderer::new();
        let mut text = Text::new(Point::ZERO, "Avant \u{fffc} après".into());
        text.font_family = drafftink_core::shapes::FontFamily::NotoSans;
        text.char_styles = vec![
            CharacterStyle {
                bold: true,
                italic: true,
                underline: true,
                script: 0,
            };
            text.content.chars().count()
        ];
        for latex in [
            r"\frac{\frac{1}{2}}{\frac{3}{4}}",
            r"\sqrt{x+1}",
            r"\sqrt[3]{x+1}",
            r"\sum_{i=1}^{n} i",
            r"\prod_{i=1}^{n} i",
            r"\int_{0}^{1} x\,dx",
            r"\lim_{x\to 0} x",
        ] {
            text.formulas = vec![InlineFormula {
                at: 6,
                math: Math::new(Point::ZERO, latex.into()),
                kind: "test".into(),
                parts: Default::default(),
            }];
            assert!(
                renderer.formula_is_valid(latex),
                "unsupported formula {latex}"
            );
            renderer.render_text(&text, Affine::IDENTITY);
            let inline = renderer.prepare_inline_formulas(&text);
            assert_eq!(inline.len(), 1);
            assert!(inline[0].2 > 0.0 && inline[0].3 > 0.0);
            let mut editor = crate::TextEditState::new(&text.content, text.font_size as f32);
            renderer.render_text_editing(&text, &mut editor, Affine::IDENTITY, None);
            assert!(editor.cursor_geometry(1.5).is_some());
            let mut only = Text::new(Point::ZERO, "\u{fffc}".into());
            only.formulas = text.formulas.clone();
            only.formulas[0].at = 0;
            renderer.render_text(&only, Affine::IDENTITY);
            assert!(
                only.bounds().height() >= {
                    let size = renderer.math_cache[&only.formulas[0].math.id()].size;
                    size.1 - size.2
                },
                "formula escaped bounds: {latex}"
            );
        }
    }
    #[test]
    fn unavailable_custom_font_still_renders_and_edits() {
        let mut renderer = VelloRenderer::new();
        let mut text = Text::new(Point::ZERO, "123^4".into());
        text.custom_font = Some("Unavailable Private Font".into());
        renderer.render_text(&text, Affine::IDENTITY);
        let mut editor = crate::TextEditState::new(&text.content, text.font_size as f32);
        renderer.render_text_editing(&text, &mut editor, Affine::IDENTITY, None);
        let (fonts, layouts) = renderer.contexts_mut();
        editor.handle_key(
            crate::TextKey::End,
            crate::TextModifiers::default(),
            fonts,
            layouts,
        );
        editor.handle_key(
            crate::TextKey::Backspace,
            crate::TextModifiers::default(),
            fonts,
            layouts,
        );
        assert_eq!(editor.text(), "123^");
    }
}

#[cfg(test)]
mod symbol_font_tests {
    use super::*;
    #[test]
    fn text_symbol_fallback_provides_real_glyphs() {
        let mut renderer = VelloRenderer::new();
        let content = "≥≤Σ∏∫∞";
        let mut builder =
            renderer
                .layout_cx
                .ranged_builder(&mut renderer.font_cx, content, 1.0, false);
        builder.push_default(parley::StyleProperty::FontSize(20.0));
        builder.push_default(parley::StyleProperty::FontStack(text_font_stack(
            "Noto Sans",
        )));
        let mut layout = builder.build(content);
        layout.break_all_lines(None);
        let glyphs: Vec<_> = layout
            .lines()
            .flat_map(|line| {
                line.items().filter_map(|item| {
                    if let PositionedLayoutItem::GlyphRun(run) = item {
                        Some(run.glyphs().map(|g| g.id).collect::<Vec<_>>())
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .collect();
        assert_eq!(glyphs.len(), 6);
        assert!(
            glyphs.iter().all(|id| *id != 0),
            "missing mathematical glyphs: {glyphs:?}"
        );
    }
}

#[cfg(test)]
mod inline_axis_tests {
    use super::*;
    use drafftink_core::shapes::{InlineFormula, Math, Text};
    #[test]
    fn formula_axes_match_equals_and_stay_in_bounds() {
        let mut renderer = VelloRenderer::new();
        let mut text = Text::new(Point::ZERO, "a = \u{fffc} = \u{fffc} z".into());
        text.formulas = vec![
            InlineFormula {
                at: 4,
                math: Math::new(Point::ZERO, r"\frac{1}{8}".into()),
                kind: "Code".into(),
                parts: Default::default(),
            },
            InlineFormula {
                at: 8,
                math: Math::new(Point::ZERO, r"\int_{1}^{5} x\,dx".into()),
                kind: "Code".into(),
                parts: Default::default(),
            },
        ];
        renderer.render_text(&text, Affine::IDENTITY);
        let layout = &renderer.text_cache.values().next().unwrap().layout;
        let line = layout.lines().next().unwrap();
        let shift = crate::text_editor::inline_baseline_shift(&line, text.font_size as f32);
        let axis = line
            .items()
            .filter_map(|item| match item {
                parley::PositionedLayoutItem::InlineBox(b) => Some(b.y + b.height / 2.0),
                _ => None,
            })
            .next()
            .unwrap();
        for item in line.items() {
            match item {
                parley::PositionedLayoutItem::GlyphRun(g) if g.run().font_size() > 0.0 => {
                    let font = g.run().font();
                    let a = crate::text_editor::font_math_axis(
                        font.data.data(),
                        font.index,
                        g.run().font_size(),
                    );
                    assert!((g.baseline() - shift - a - axis).abs() < 0.001);
                }
                parley::PositionedLayoutItem::InlineBox(b) => {
                    assert!((b.y + b.height / 2.0 - axis).abs() < 0.001)
                }
                _ => {}
            }
        }
        assert!(text.bounds().height() >= line.metrics().line_height as f64);
    }
}

#[cfg(test)]
mod memory_budget_tests {
    use super::*;
    use drafftink_core::shapes::{Image, ImageFormat};
    fn fixture_image() -> Image {
        let raw = ::image::DynamicImage::ImageRgba8(::image::RgbaImage::from_pixel(
            1200,
            2000,
            ::image::Rgba([20, 150, 220, 255]),
        ));
        let mut png = std::io::Cursor::new(Vec::new());
        raw.write_to(&mut png, ::image::ImageFormat::Png).unwrap();
        Image::new(Point::ZERO, png.get_ref(), 1200, 2000, ImageFormat::Png).with_size(160.0, 120.0)
    }
    #[test]
    fn small_display_uses_small_preview_but_export_retains_original() {
        let image = fixture_image();
        let source = image.data_base64.clone();
        let mut renderer = VelloRenderer::new();
        renderer.render_image(&image, Affine::IDENTITY);
        let cached = &renderer.image_cache[&image.data_base64.key()];
        assert_eq!((cached.image.width, cached.image.height), (153, 256));
        assert_eq!(renderer.image_cache_bytes(), 156672);
        let full = decode_image_preview(&image, 0).unwrap();
        assert_eq!((full.width, full.height), (1200, 2000));
        let mut document = drafftink_core::canvas::CanvasDocument::new();
        document.add_shape(Shape::Image(image.clone()));
        let (scene, bounds) = renderer.build_export_scene(&document, 1.0);
        assert!(bounds.is_some() && !scene.encoding().is_empty());
        assert!(!renderer.full_resolution_images);
        assert_eq!(renderer.image_cache_bytes(), 156672);
        assert!(source.shares_storage(&image.data_base64));
        // A zoom increase refreshes the preview without changing source/crop/geometry.
        renderer.zoom = 10.0;
        renderer.render_image(&image, Affine::IDENTITY);
        assert!(renderer.image_cache_bytes() > 156672);
        assert!(renderer.image_cache_bytes() <= IMAGE_CACHE_BUDGET);
        assert_eq!(source, image.data_base64);
    }
    #[test]
    fn lru_budget_evicts_oldest_and_switching_canvas_releases_decoded_images() {
        let mut renderer = VelloRenderer::new();
        for key in 0..33u64 {
            renderer.image_cache.insert(
                key,
                CachedImage {
                    image: peniko::ImageData {
                        data: peniko::Blob::new(std::sync::Arc::new(vec![0u8; 512 * 512 * 4])),
                        width: 512,
                        height: 512,
                        format: peniko::ImageFormat::Rgba8,
                        alpha_type: peniko::ImageAlphaType::Alpha,
                    },
                    bucket: 512,
                    last_used: key,
                },
            );
        }
        renderer.trim_image_cache();
        assert_eq!(renderer.image_cache_bytes(), IMAGE_CACHE_BUDGET);
        assert!(!renderer.image_cache.contains_key(&0) && renderer.image_cache.contains_key(&32));
        let empty = drafftink_core::canvas::CanvasDocument::new();
        renderer.retain_open_document_caches(std::iter::once(&empty));
        assert_eq!(renderer.image_cache_bytes(), 0);
    }
}

#[cfg(test)]
mod path_cache_memory_tests {
    use super::*;
    #[test]
    fn drags_keep_only_current_stroke_and_deleted_objects_release_paths() {
        let mut renderer = VelloRenderer::new();
        let id = drafftink_core::shapes::Rectangle::new(Point::ZERO, 40.0, 20.0)
            .id()
            .to_string();
        for n in 0..600 {
            let mut path = BezPath::new();
            path.move_to((n as f64, 0.0));
            path.line_to((n as f64 + 20.0, 10.0));
            renderer.get_cached_hand_drawn(&id, &path, 1.0, 42, 0);
        }
        assert_eq!(renderer.shape_cache.len(), 1);
        assert!(renderer.path_cache_bytes() <= PATH_CACHE_BUDGET);
        let empty = drafftink_core::canvas::CanvasDocument::new();
        renderer.retain_open_document_caches(std::iter::once(&empty));
        assert_eq!(renderer.path_cache_bytes(), 0);
    }
}

fn text_script_offset(text: &drafftink_core::shapes::Text, byte: usize) -> f32 {
    let index = text
        .content
        .get(..byte)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    match text.char_styles.get(index).map(|s| s.script).unwrap_or(0) {
        1 => -text.font_size as f32 * 0.4,
        -1 => text.font_size as f32 * 0.2,
        _ => 0.0,
    }
}

#[cfg(test)]
mod parent_font_and_script_tests {
    use super::*;
    use drafftink_core::shapes::{FontFamily, InlineFormula, Math, Text};
    #[test]
    fn inline_math_changes_font_with_parent_and_invalidates_cached_scene() {
        let mut renderer = VelloRenderer::new();
        let mut text = Text::new(Point::ZERO, "= \u{fffc}".into());
        text.font_family = FontFamily::NotoSans;
        text.formulas.push(InlineFormula {
            at: 2,
            math: Math::new(Point::ZERO, "x+123".into()),
            kind: "Code".into(),
            parts: Default::default(),
        });
        renderer.render_text(&text, Affine::IDENTITY);
        let id = text.formulas[0].math.id();
        let before = renderer.math_cache[&id].primary_font_id;
        text.font_family = FontFamily::GelPen;
        renderer.render_text(&text, Affine::IDENTITY);
        assert_ne!(before, renderer.math_cache[&id].primary_font_id);
        assert_eq!(
            renderer.math_cache[&id].primary_font_id,
            GELPEN_REGULAR.as_ptr() as usize
        );
        text.content = "AbXYα≤@".into();
        text.formulas.clear();
        text.toggle_script(0..text.content.len(), 1);
        renderer.render_text(&text, Affine::IDENTITY);
        assert_eq!(text.content, "AbXYα≤@");
        assert!(text.bounds().height() > 0.0);
    }
}

#[cfg(test)]
mod math_symbols_tests {
    use super::*;
    #[test]
    fn binomial_and_infinite_bounds_render_with_gelpen() {
        let mut renderer = VelloRenderer::new();
        for latex in [
            r"\binom{n}{k}",
            r"\int_{0}^{\infty} x",
            r"\lim_{n\to\infty} x",
            r"[0,\infty]",
        ] {
            let math = drafftink_core::shapes::Math::new(Point::ZERO, latex.into());
            assert!(renderer.prepare_math(&math), "{latex}");
            assert_eq!(
                renderer.math_cache[&math.id()].primary_font_id,
                GELPEN_REGULAR.as_ptr() as usize
            );
            assert!(math.bounds().height() > 0.0);
        }
    }
}

#[cfg(test)]
mod cursor_and_math_font_tests {
    use super::*;
    use crate::{TextEditState, TextKey, TextModifiers};
    use drafftink_core::shapes::{FontFamily, Math, Text, TextFont};
    #[test]
    fn empty_next_line_and_navigation_follow_script_style() {
        let mut r = VelloRenderer::new();
        let mut t = Text::new(Point::ZERO, "x45".into());
        t.font_family = FontFamily::NotoSans;
        t.toggle_script(1..3, 1);
        let mut e = TextEditState::new(&t.content, t.font_size as f32);
        {
            let (f, l) = r.contexts_mut();
            e.driver(f, l).select_byte_range(1, 1);
        }
        r.render_text_editing(&t, &mut e, Affine::IDENTITY, None);
        assert_eq!(e.script_value(), 1);
        {
            let (f, l) = r.contexts_mut();
            e.driver(f, l).select_byte_range(0, 0);
        }
        r.render_text_editing(&t, &mut e, Affine::IDENTITY, None);
        assert_eq!(e.script_value(), 0);
        {
            let (f, l) = r.contexts_mut();
            e.driver(f, l).move_to_text_end();
            e.handle_key(TextKey::ToggleSuperscript, TextModifiers::default(), f, l);
            e.handle_key(
                TextKey::Character(" ".into()),
                TextModifiers::default(),
                f,
                l,
            );
        }
        assert_eq!(e.script_value(), 1);
        {
            let (f, l) = r.contexts_mut();
            e.handle_key(TextKey::Right, TextModifiers::default(), f, l);
            assert_eq!(e.script_value(), 0);
            e.handle_key(TextKey::Enter, TextModifiers::default(), f, l);
        }
        t.content = e.text();
        r.render_text_editing(&t, &mut e, Affine::IDENTITY, None);
        let c = e.cursor_geometry(1.5).unwrap();
        assert!(c.x0 < 1.0);
        assert!(c.y0 > t.font_size);
    }
    #[test]
    fn math_font_change_invalidates_layout() {
        let mut r = VelloRenderer::new();
        let mut m = Math::new(Point::ZERO, "123x".into());
        assert!(r.prepare_math(&m));
        let id = r.math_cache[&m.id()].primary_font_id;
        m.font = TextFont::from_name("Noto Sans", "");
        assert!(r.prepare_math(&m));
        assert_ne!(r.math_cache[&m.id()].primary_font_id, id);
    }
}

#[cfg(test)]
mod raw_latex_tests {
    use super::*;
    #[test]
    fn relations_and_successive_indices_render() {
        let mut renderer = VelloRenderer::new();
        for text in [
            r"x \neq y \in A",
            r"x_{i} y_{j}",
            r"x_{i_j}",
            r"\frac{a}{b}",
            r"\sqrt[n]{x}",
        ] {
            assert!(renderer.formula_is_valid(text), "{text}");
        }
    }
}
