//! Image shape for embedding raster images.

use super::{ShapeId, ShapeStyle, ShapeTrait};
use kurbo::{Affine, BezPath, Point, Rect, Shape as KurboShape};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Image format for stored image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    /// PNG format.
    Png,
    /// JPEG format.
    Jpeg,
    /// WebP format.
    WebP,
}

impl ImageFormat {
    /// Get MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
        }
    }

    /// Detect format from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(ImageFormat::Png),
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::WebP),
            _ => None,
        }
    }

    /// Detect format from magic bytes.
    pub fn from_magic_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }

        // PNG: 89 50 4E 47
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return Some(ImageFormat::Png);
        }

        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(ImageFormat::Jpeg);
        }

        // WebP: RIFF....WEBP
        if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
            return Some(ImageFormat::WebP);
        }

        None
    }
}

/// Immutable encoded source shared by duplicates and every Undo/Redo snapshot.
/// The JSON representation remains a plain base64 string for compatibility.
#[derive(Debug, Clone)]
pub struct SharedImageSource {
    encoded: std::sync::Arc<str>,
    key: std::sync::Arc<std::sync::OnceLock<u64>>,
}
impl From<String> for SharedImageSource {
    fn from(encoded: String) -> Self {
        Self {
            encoded: encoded.into(),
            key: Default::default(),
        }
    }
}
impl std::ops::Deref for SharedImageSource {
    type Target = str;
    fn deref(&self) -> &str {
        &self.encoded
    }
}
impl PartialEq for SharedImageSource {
    fn eq(&self, other: &Self) -> bool {
        self.encoded == other.encoded
    }
}
impl Eq for SharedImageSource {}
impl SharedImageSource {
    pub fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        *self.key.get_or_init(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.encoded.hash(&mut hasher);
            hasher.finish()
        })
    }
    pub fn shares_storage(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.encoded, &other.encoded)
    }
}
impl Serialize for SharedImageSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.encoded)
    }
}
impl<'de> Deserialize<'de> for SharedImageSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(String::deserialize(deserializer)?.into())
    }
}

/// An image shape that displays a raster image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub(crate) id: ShapeId,
    /// Top-left corner position.
    pub position: Point,
    /// Display width.
    pub width: f64,
    /// Display height.
    pub height: f64,
    /// Original image width in pixels.
    pub source_width: u32,
    /// Original image height in pixels.
    pub source_height: u32,
    /// Image format.
    pub format: ImageFormat,
    /// Image data as base64-encoded string (for CRDT compatibility).
    /// We use base64 string instead of Vec<u8> for easier JSON/CRDT serialization.
    pub data_base64: SharedImageSource,
    /// Rotation angle in radians (around center).
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    /// Visible source rectangle in normalized source coordinates.
    #[serde(default = "full_crop")]
    pub crop: Rect,
    /// Style properties (stroke used for optional border).
    pub style: ShapeStyle,
}

pub fn full_crop() -> Rect {
    Rect::new(0.0, 0.0, 1.0, 1.0)
}

impl Image {
    /// Create a new image shape from raw image data.
    ///
    /// # Arguments
    /// * `position` - Top-left corner position
    /// * `data` - Raw image bytes (PNG, JPEG, or WebP)
    /// * `source_width` - Original image width in pixels
    /// * `source_height` - Original image height in pixels
    /// * `format` - Image format
    pub fn new(
        position: Point,
        data: &[u8],
        source_width: u32,
        source_height: u32,
        format: ImageFormat,
    ) -> Self {
        use base64::{Engine, engine::general_purpose::STANDARD};

        Self {
            id: Uuid::new_v4(),
            position,
            width: source_width as f64,
            height: source_height as f64,
            source_width,
            source_height,
            format,
            data_base64: STANDARD.encode(data).into(),
            rotation: 0.0,
            crop: full_crop(),
            flip_x: false,
            flip_y: false,
            style: ShapeStyle::default(),
        }
    }

    /// Reconstruct an image with a specific ID (for CRDT/storage).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reconstruct(
        id: ShapeId,
        position: Point,
        width: f64,
        height: f64,
        source_width: u32,
        source_height: u32,
        format: ImageFormat,
        data_base64: String,
        rotation: f64,
        style: ShapeStyle,
    ) -> Self {
        Self {
            id,
            position,
            width,
            height,
            source_width,
            source_height,
            format,
            data_base64: data_base64.into(),
            rotation,
            crop: full_crop(),
            flip_x: false,
            flip_y: false,
            style,
        }
    }

    /// Create an image shape with specific display dimensions.
    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Scale the image to fit within max dimensions while preserving aspect ratio.
    pub fn fit_within(mut self, max_width: f64, max_height: f64) -> Self {
        let aspect = self.source_width as f64 / self.source_height as f64;
        let target_aspect = max_width / max_height;

        if aspect > target_aspect {
            // Image is wider than target - fit to width
            self.width = max_width;
            self.height = max_width / aspect;
        } else {
            // Image is taller than target - fit to height
            self.height = max_height;
            self.width = max_height * aspect;
        }

        self
    }

    /// Get the raw image data (decoded from base64).
    pub fn data(&self) -> Option<Vec<u8>> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.decode(self.data_base64.as_bytes()).ok()
    }

    /// Get the bounding rectangle.
    pub fn as_rect(&self) -> Rect {
        Rect::new(
            self.position.x,
            self.position.y,
            self.position.x + self.width,
            self.position.y + self.height,
        )
    }

    /// Get the size of the image data in bytes.
    pub fn data_size(&self) -> usize {
        // Base64 is ~4/3 the size of raw data
        self.data_base64.len() * 3 / 4
    }
}

impl ShapeTrait for Image {
    fn id(&self) -> ShapeId {
        self.id
    }

    fn bounds(&self) -> Rect {
        self.as_rect()
    }

    fn hit_test(&self, point: Point, tolerance: f64) -> bool {
        let rect = self.as_rect().inflate(tolerance, tolerance);
        rect.contains(point)
    }

    fn to_path(&self) -> BezPath {
        // Return bounding box as path (for selection rendering)
        self.as_rect().to_path(0.1)
    }

    fn style(&self) -> &ShapeStyle {
        &self.style
    }

    fn style_mut(&mut self) -> &mut ShapeStyle {
        &mut self.style
    }

    fn transform(&mut self, affine: Affine) {
        let center = affine * self.bounds().center();
        let scale = affine.as_coeffs();
        self.width *= scale[0].hypot(scale[1]);
        self.height *= scale[2].hypot(scale[3]);
        self.position = Point::new(center.x - self.width / 2.0, center.y - self.height / 2.0);
        if scale[1].abs() < 1e-10 && scale[2].abs() < 1e-10 {
            self.flip_x ^= scale[0] < 0.0;
            self.flip_y ^= scale[3] < 0.0;
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
    fn test_image_creation() {
        // 1x1 red PNG (minimal valid PNG)
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A,
            0x0A,
            // ... minimal PNG would be larger, just testing structure
        ];

        let format = ImageFormat::from_magic_bytes(&png_data);
        assert_eq!(format, Some(ImageFormat::Png));
    }

    #[test]
    fn test_format_detection() {
        assert_eq!(ImageFormat::from_extension("png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_extension("PNG"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_extension("jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("jpeg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("webp"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("gif"), None);
    }

    #[test]
    fn test_fit_within() {
        let data = vec![0u8; 10];
        let img = Image::new(Point::ZERO, &data, 1000, 500, ImageFormat::Png);

        // Fit 1000x500 (2:1 aspect) into 400x400 box
        let fitted = img.fit_within(400.0, 400.0);
        assert!((fitted.width - 400.0).abs() < 0.01);
        assert!((fitted.height - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_bounds() {
        let data = vec![0u8; 10];
        let img = Image::new(Point::new(10.0, 20.0), &data, 100, 50, ImageFormat::Png);
        let bounds = img.bounds();
        assert!((bounds.x0 - 10.0).abs() < f64::EPSILON);
        assert!((bounds.y0 - 20.0).abs() < f64::EPSILON);
        assert!((bounds.x1 - 110.0).abs() < f64::EPSILON);
        assert!((bounds.y1 - 70.0).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod shared_source_tests {
    use super::*;
    #[test]
    fn snapshots_share_source_bytes_and_json_is_unchanged() {
        let image = Image::new(
            Point::ZERO,
            &vec![42; 1024 * 1024],
            600,
            400,
            ImageFormat::Png,
        );
        let clones: Vec<_> = (0..50).map(|_| image.clone()).collect();
        for clone in &clones {
            assert!(image.data_base64.shares_storage(&clone.data_base64));
            assert_eq!(image.data_base64.key(), clone.data_base64.key());
        }
        let json = serde_json::to_value(&image).unwrap();
        assert!(json["data_base64"].is_string());
        let restored: Image = serde_json::from_value(json).unwrap();
        assert_eq!(restored.data(), image.data());
        assert_eq!(restored.data_base64.key(), image.data_base64.key());
    }
}
