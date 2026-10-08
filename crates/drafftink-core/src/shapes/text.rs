//! Text shape.

use super::{ShapeId, ShapeStyle, ShapeTrait};
use kurbo::{Affine, BezPath, Point, Rect};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use uuid::Uuid;

/// Font family options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontFamily {
    /// GelPen hand-drawn style font (default).
    #[default]
    GelPen,
    /// Noto Sans clean sans-serif font.
    NotoSans,
    /// GelPen Serif handwritten font.
    GelPenSerif,
    /// Vanilla Extract handwritten font.
    VanillaExtract,
    /// XITS Math: broad mathematical/symbol coverage for plain text.
    XitsMath,
}

impl FontFamily {
    /// Get the font family name as used by the renderer.
    pub fn name(&self) -> &'static str {
        match self {
            FontFamily::GelPen => "GelPen",
            FontFamily::NotoSans => "Noto Sans",
            FontFamily::GelPenSerif => "GelPen Serif",
            FontFamily::VanillaExtract => "Vanilla Extract",
            FontFamily::XitsMath => "STIX Two Math",
        }
    }

    /// Get display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            FontFamily::GelPen => "GelPen",
            FontFamily::NotoSans => "Noto",
            FontFamily::GelPenSerif => "GelPen Serif",
            FontFamily::VanillaExtract => "Vanilla",
            FontFamily::XitsMath => "STIX Symbols",
        }
    }

    /// Get all available font families.
    pub fn all() -> &'static [FontFamily] {
        &[
            FontFamily::GelPen,
            FontFamily::NotoSans,
            FontFamily::GelPenSerif,
            FontFamily::VanillaExtract,
            FontFamily::XitsMath,
        ]
    }
}

/// Font weight options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontWeight {
    /// Light weight.
    Light,
    /// Regular weight (default).
    #[default]
    Regular,
    /// Heavy/Bold weight.
    Heavy,
    Medium,
}

impl FontWeight {
    pub fn value(self) -> f32 {
        match self {
            Self::Light => 300.0,
            Self::Regular => 400.0,
            Self::Medium => 500.0,
            Self::Heavy => 700.0,
        }
    }
    /// Get display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            FontWeight::Light => "Light",
            FontWeight::Regular => "Regular",
            FontWeight::Heavy => "Heavy",
            FontWeight::Medium => "Medium",
        }
    }

    /// Get all available font weights.
    pub fn all() -> &'static [FontWeight] {
        &[
            FontWeight::Light,
            FontWeight::Regular,
            FontWeight::Medium,
            FontWeight::Heavy,
        ]
    }
}

/// Font preference used for future text objects, separate from selected objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextFont {
    pub family: FontFamily,
    pub custom: Option<String>,
    pub weight: FontWeight,
    #[serde(default)]
    pub postscript: Option<String>,
}
impl Default for TextFont {
    fn default() -> Self {
        Self {
            family: FontFamily::NotoSans,
            custom: Some("Google Sans".into()),
            weight: FontWeight::Medium,
            postscript: Some("GoogleSans-Medium".into()),
        }
    }
}
impl TextFont {
    pub fn from_name(name: &str, postscript: &str) -> Self {
        if postscript.is_empty() {
            if let Some(family) = FontFamily::all().iter().copied().find(|f| f.name() == name) {
                return Self {
                    family,
                    custom: None,
                    weight: FontWeight::Regular,
                    postscript: None,
                };
            }
        }
        let face = format!("{} {}", name, postscript).to_lowercase();
        let weight = if face.contains("medium") {
            FontWeight::Medium
        } else if face.contains("bold") || face.contains("heavy") {
            FontWeight::Heavy
        } else if face.contains("light") {
            FontWeight::Light
        } else {
            FontWeight::Regular
        };
        let canonical = if name.eq_ignore_ascii_case("Google Sans Medium") {
            "Google Sans"
        } else {
            name
        };
        Self {
            family: FontFamily::NotoSans,
            custom: Some(canonical.to_string()),
            weight,
            postscript: (!postscript.is_empty()).then(|| postscript.to_string()),
        }
    }
    pub fn apply(&self, text: &mut Text) {
        text.font_family = self.family;
        text.font_weight = self.weight;
        text.custom_font = self.custom.clone();
        text.custom_font_postscript = self.postscript.clone();
        text.invalidate_cache();
    }
}

/// Formatting stored per Unicode scalar, independently of font and color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharacterStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    #[serde(default)]
    pub script: i8,
}

/// A formula occupies one object-replacement character in the text flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineFormula {
    pub at: usize,
    pub math: super::Math,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub parts: [String; 4],
}

/// A text shape.
#[derive(Debug, Serialize, Deserialize)]
pub struct Text {
    pub(crate) id: ShapeId,
    /// Position (top-left corner of text bounding box).
    pub position: Point,
    /// The text content.
    pub content: String,
    /// Font size in pixels.
    pub font_size: f64,
    /// Font family.
    pub font_family: FontFamily,
    /// Font weight.
    pub font_weight: FontWeight,
    /// Optional local/system font family loaded from the user's computer.
    #[serde(default)]
    pub custom_font: Option<String>,
    #[serde(default)]
    pub custom_font_postscript: Option<String>,
    /// Rotation angle in radians (around center).
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "super::unit_display_scale")]
    pub display_scale: [f64; 2],
    /// Style properties.
    pub style: ShapeStyle,
    /// Per-character colors (one per char, None = use default style color).
    #[serde(default)]
    pub char_colors: Vec<Option<super::SerializableColor>>,
    #[serde(default)]
    pub char_styles: Vec<CharacterStyle>,
    #[serde(default)]
    pub formulas: Vec<InlineFormula>,
    /// Cached layout size (width, height) computed by the renderer.
    /// This is set after text layout and provides accurate bounds.
    /// Uses RwLock for thread-safe interior mutability.
    /// If None, approximate bounds are used.
    #[serde(skip)]
    cached_size: RwLock<Option<(f64, f64)>>,
}

impl Clone for Text {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            position: self.position,
            content: self.content.clone(),
            font_size: self.font_size,
            font_family: self.font_family,
            font_weight: self.font_weight,
            custom_font: self.custom_font.clone(),
            custom_font_postscript: self.custom_font_postscript.clone(),
            rotation: self.rotation,
            display_scale: self.display_scale,
            style: self.style.clone(),
            char_colors: self.char_colors.clone(),
            char_styles: self.char_styles.clone(),
            formulas: self.formulas.clone(),
            // Clone the cached size value, not the lock
            cached_size: RwLock::new(self.cached_size.read().ok().and_then(|guard| *guard)),
        }
    }
}

impl Text {
    pub fn editing_local_point(&self, world: Point) -> Point {
        let local = Affine::rotate_about(-self.rotation, self.bounds().center()) * world;
        Point::new(
            (local.x - self.position.x) / self.display_scale[0],
            (local.y - self.position.y) / self.display_scale[1],
        )
    }

    /// Default font size (M = Medium).
    pub const DEFAULT_FONT_SIZE: f64 = 20.0;

    /// Create a new text shape.
    pub fn new(position: Point, content: String) -> Self {
        let char_count = content.chars().count();
        Self {
            id: Uuid::new_v4(),
            position,
            content,
            font_size: Self::DEFAULT_FONT_SIZE,
            font_family: FontFamily::default(),
            font_weight: FontWeight::default(),
            custom_font: None,
            custom_font_postscript: None,
            rotation: 0.0,
            display_scale: [1.0, 1.0],
            style: ShapeStyle::default(),
            char_colors: vec![None; char_count],
            char_styles: Vec::new(),
            formulas: Vec::new(),
            cached_size: RwLock::new(None),
        }
    }

    /// Set the cached layout size (computed by the renderer).
    /// Uses interior mutability so this can be called during rendering.
    pub fn set_cached_size(&self, width: f64, height: f64) {
        if let Ok(mut cache) = self.cached_size.write() {
            *cache = Some((width, height));
        }
    }

    /// Clear the cached size (call when text properties change).
    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cached_size.write() {
            *cache = None;
        }
    }

    /// Apply a color to a character index range.
    pub fn apply_color_to_range(
        &mut self,
        start_char: usize,
        end_char: usize,
        color: super::SerializableColor,
    ) {
        // Ensure char_colors is the right size
        let char_count = self.content.chars().count();
        self.char_colors.resize(char_count, None);

        for i in start_char..end_char.min(char_count) {
            self.char_colors[i] = Some(color);
        }
    }

    /// Synchronize spans using the actual replacement, including deletion before
    /// the caret and equal-length replacement (external text expanders).
    pub fn sync_spans_after_edit(&mut self, old: &str) {
        let before: Vec<char> = old.chars().collect();
        let after: Vec<char> = self.content.chars().collect();
        let prefix = before
            .iter()
            .zip(&after)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before[prefix..]
            .iter()
            .rev()
            .zip(after[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        if prefix == before.len() && prefix == after.len() {
            return;
        }
        self.char_colors.resize(before.len(), None);
        self.char_styles
            .resize(before.len(), CharacterStyle::default());
        let inherited = self
            .char_styles
            .get(prefix.saturating_sub(1))
            .copied()
            .unwrap_or_default();
        let inserted = after.len() - prefix - suffix;
        self.char_colors.splice(
            prefix..before.len() - suffix,
            std::iter::repeat_n(None, inserted),
        );
        self.char_styles.splice(
            prefix..before.len() - suffix,
            std::iter::repeat_n(inherited, inserted),
        );
        self.formulas.retain_mut(|formula| {
            if formula.at < prefix {
                return true;
            }
            if formula.at < before.len() - suffix {
                return false;
            }
            formula.at = formula.at - (before.len() - prefix - suffix) + inserted;
            true
        });
        self.invalidate_cache();
    }

    pub fn toggle_script(&mut self, range: std::ops::Range<usize>, script: i8) {
        self.char_styles
            .resize(self.content.chars().count(), CharacterStyle::default());
        let start = self.content[..range.start].chars().count();
        let end = self.content[..range.end].chars().count();
        let value = if self.char_styles[start..end]
            .iter()
            .all(|s| s.script == script)
        {
            0
        } else {
            script
        };
        for style in &mut self.char_styles[start..end] {
            style.script = value;
        }
        self.invalidate_cache();
    }
    pub fn set_inserted_script(&mut self, old: &str, script: i8) {
        let before: Vec<_> = old.chars().collect();
        let after: Vec<_> = self.content.chars().collect();
        let prefix = before
            .iter()
            .zip(&after)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before[prefix..]
            .iter()
            .rev()
            .zip(after[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        self.char_styles
            .resize(after.len(), CharacterStyle::default());
        for style in &mut self.char_styles[prefix..after.len() - suffix] {
            style.script = script;
        }
        self.invalidate_cache();
    }
    pub fn toggle_format(&mut self, range: std::ops::Range<usize>, kind: char) {
        self.char_styles
            .resize(self.content.chars().count(), CharacterStyle::default());
        let start = self.content[..range.start].chars().count();
        let end = self.content[..range.end].chars().count();
        let selected = &mut self.char_styles[start..end];
        let active = !selected.iter().all(|s| match kind {
            'b' => s.bold,
            'i' => s.italic,
            _ => s.underline,
        });
        for style in selected {
            match kind {
                'b' => style.bold = active,
                'i' => style.italic = active,
                _ => style.underline = active,
            }
        }
        self.invalidate_cache();
    }

    /// Sync char_colors with content after text edit.
    /// `edit_char_pos` is the character index where the edit occurred.
    /// `old_char_count` is the character count before the edit.
    pub fn sync_char_colors_after_edit(&mut self, edit_char_pos: usize, old_char_count: usize) {
        let new_char_count = self.content.chars().count();
        if new_char_count == old_char_count {
            return; // No change in length
        }

        if new_char_count > old_char_count {
            // Characters inserted - insert None values at edit position
            let inserted = new_char_count - old_char_count;
            let insert_pos = edit_char_pos.min(self.char_colors.len());
            for _ in 0..inserted {
                if insert_pos <= self.char_colors.len() {
                    self.char_colors.insert(insert_pos, None);
                } else {
                    self.char_colors.push(None);
                }
            }
        } else {
            // Characters deleted - remove values at edit position
            let deleted = old_char_count - new_char_count;
            let delete_pos = edit_char_pos.min(self.char_colors.len());
            for _ in 0..deleted {
                if delete_pos < self.char_colors.len() {
                    self.char_colors.remove(delete_pos);
                }
            }
        }
        // Ensure correct final size
        self.char_colors.resize(new_char_count, None);
    }

    /// Simple resize sync (for cases where edit position is unknown).
    pub fn sync_char_colors_with_content(&mut self) {
        let char_count = self.content.chars().count();
        self.char_colors.resize(char_count, None);
    }

    /// Reconstruct a text with a specific ID (for CRDT/storage).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reconstruct(
        id: ShapeId,
        position: Point,
        content: String,
        font_size: f64,
        font_family: FontFamily,
        font_weight: FontWeight,
        rotation: f64,
        style: ShapeStyle,
        char_colors: Vec<Option<super::SerializableColor>>,
    ) -> Self {
        Self {
            id,
            position,
            content,
            font_size,
            font_family,
            font_weight,
            custom_font: None,
            custom_font_postscript: None,
            rotation,
            display_scale: [1.0, 1.0],
            style,
            char_colors,
            char_styles: Vec::new(),
            formulas: Vec::new(),
            cached_size: RwLock::new(None),
        }
    }

    /// Create a new text shape with font size.
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Set the font family.
    pub fn with_font_family(mut self, family: FontFamily) -> Self {
        self.font_family = family;
        self
    }

    /// Set the font weight.
    pub fn with_font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }

    /// Set the text content.
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.invalidate_cache();
    }

    /// Get the text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Approximate width based on character count and font size.
    /// This is a rough estimate; actual width depends on the font.
    fn approximate_width(&self) -> f64 {
        // For multi-line text, find the widest line
        let max_line_len = self
            .content
            .lines()
            .map(|line| line.len())
            .max()
            .unwrap_or(0);

        // Average character width varies by font family and weight
        // These values are empirically determined approximations
        let char_width_factor = match (&self.font_family, &self.font_weight) {
            // GelPen is a handwritten-style font with medium width
            (FontFamily::GelPen, FontWeight::Light) => 0.50,
            (FontFamily::GelPen, FontWeight::Regular | FontWeight::Medium) => 0.55,
            (FontFamily::GelPen, FontWeight::Heavy) => 0.60,
            // Noto Sans is a clean sans-serif font
            (FontFamily::NotoSans, FontWeight::Light) => 0.50,
            (FontFamily::NotoSans, FontWeight::Regular | FontWeight::Medium) => 0.52,
            (FontFamily::NotoSans, FontWeight::Heavy) => 0.55,
            // GelPen Serif is a handwritten font with wider characters
            (FontFamily::GelPenSerif, FontWeight::Light) => 0.55,
            (FontFamily::GelPenSerif, FontWeight::Regular | FontWeight::Medium) => 0.58,
            (FontFamily::GelPenSerif, FontWeight::Heavy) => 0.60,
            // Vanilla Extract is a handwritten font
            (FontFamily::VanillaExtract, _) => 0.50,
            (FontFamily::XitsMath, _) => 0.55,
        };

        max_line_len as f64 * self.font_size * char_width_factor
    }

    /// Approximate height based on font size and number of lines.
    fn approximate_height(&self) -> f64 {
        // Count lines (empty content = 1 line)
        let line_count = self.content.lines().count().max(1);
        // Add 1 if content ends with newline (lines() doesn't count trailing empty line)
        let line_count = if self.content.ends_with('\n') {
            line_count + 1
        } else {
            line_count
        };
        // Line height is typically 1.2 * font_size
        line_count as f64 * self.font_size * 1.2
    }
}

impl ShapeTrait for Text {
    fn id(&self) -> ShapeId {
        self.id
    }

    fn bounds(&self) -> Rect {
        // Use cached size if available, otherwise approximate
        let (width, height) = self
            .cached_size
            .read()
            .ok()
            .and_then(|guard| *guard)
            .map(|(w, h)| (w.max(20.0), h))
            .unwrap_or_else(|| {
                (
                    self.approximate_width().max(20.0),
                    self.approximate_height(),
                )
            });
        // Position is top-left corner
        Rect::from_points(
            self.position,
            Point::new(
                self.position.x + width * self.display_scale[0],
                self.position.y + height * self.display_scale[1],
            ),
        )
    }

    fn hit_test(&self, point: Point, tolerance: f64) -> bool {
        let bounds = self.bounds().inflate(tolerance, tolerance);
        bounds.contains(point)
    }

    fn to_path(&self) -> BezPath {
        // Text doesn't have a simple path representation
        // Return the bounding box as a path for selection purposes
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
        let c = affine.as_coeffs();
        self.display_scale[0] *= c[0].hypot(c[1]);
        self.display_scale[1] *= c[2].hypot(c[3]);
        if affine.as_coeffs()[1].abs() < 1e-10 && affine.as_coeffs()[2].abs() < 1e-10 {
            if affine.as_coeffs()[0] * affine.as_coeffs()[3] < 0.0 {
                self.rotation = -self.rotation;
            }
            self.display_scale[0] *= affine.as_coeffs()[0].signum();
            self.display_scale[1] *= affine.as_coeffs()[3].signum();
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
    fn test_text_creation() {
        let text = Text::new(Point::new(100.0, 100.0), "Hello".to_string());
        assert_eq!(text.content(), "Hello");
        assert!((text.font_size - Text::DEFAULT_FONT_SIZE).abs() < f64::EPSILON); // Default is 20.0 (M size)
    }

    #[test]
    fn test_text_with_font_size() {
        let text = Text::new(Point::new(0.0, 0.0), "Test".to_string()).with_font_size(32.0);
        assert!((text.font_size - 32.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hit_test() {
        let text = Text::new(Point::new(100.0, 100.0), "Hello World".to_string());
        let bounds = text.bounds();
        let center = Point::new((bounds.x0 + bounds.x1) / 2.0, (bounds.y0 + bounds.y1) / 2.0);
        assert!(text.hit_test(center, 0.0));
        assert!(!text.hit_test(Point::new(0.0, 0.0), 0.0));
    }

    #[test]
    fn test_bounds() {
        let text = Text::new(Point::new(100.0, 100.0), "Hi".to_string());
        let bounds = text.bounds();
        assert!(bounds.width() > 0.0);
        assert!(bounds.height() > 0.0);
    }
}

#[cfg(test)]
mod formatting_tests {
    use super::*;
    #[test]
    fn formatting_tracks_unicode_edits_and_formula_positions() {
        let mut text = Text::new(Point::ZERO, "éabc\u{fffc}".into());
        text.toggle_format(2..4, 'b');
        text.toggle_format(3..4, 'i');
        text.toggle_format(2..4, 'u');
        text.formulas.push(InlineFormula {
            at: 4,
            math: super::super::Math::new(Point::ZERO, r"\frac{1}{2}".into()),
            kind: "Fraction".into(),
            parts: ["1".into(), "2".into(), String::new(), String::new()],
        });
        let old = text.content.clone();
        text.content = "éaxc\u{fffc}".into();
        text.sync_spans_after_edit(&old);
        assert!(text.char_styles[1].bold && text.char_styles[2].bold);
        assert_eq!(text.formulas[0].at, 4);
        let old = text.content.clone();
        text.content = "éc\u{fffc}".into();
        text.sync_spans_after_edit(&old);
        assert_eq!(text.formulas[0].at, 2);
        let restored: Text = serde_json::from_str(&serde_json::to_string(&text).unwrap()).unwrap();
        assert_eq!(restored.char_styles, text.char_styles);
        assert_eq!(restored.formulas[0].math.latex, r"\frac{1}{2}");
    }
}

#[cfg(test)]
mod script_style_tests {
    use super::*;
    #[test]
    fn any_character_scripts_preserve_original_text_and_json() {
        let original = "AbXYZ09αβ≤≥@!?";
        let mut text = Text::new(Point::ZERO, original.into());
        text.toggle_script(0..original.len(), 1);
        assert_eq!(text.content, original);
        assert!(text.char_styles.iter().all(|s| s.script == 1));
        let restored: Text = serde_json::from_str(&serde_json::to_string(&text).unwrap()).unwrap();
        assert_eq!(restored.char_styles, text.char_styles);
        text.toggle_script(0..original.len(), 1);
        assert!(text.char_styles.iter().all(|s| s.script == 0));
        text.toggle_script(0..original.len(), -1);
        assert!(text.char_styles.iter().all(|s| s.script == -1));
    }
}
