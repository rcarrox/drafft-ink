//! Vello backend for ReX math rendering with font fallback.

use kurbo::{Affine, BezPath, Point};
use peniko::Color;
use rex::font::MathFont;
use rex::font::backend::ttf_parser::TtfMathFont;
use rex::font::common::GlyphId;
use rex::render::{Backend, Cursor, FontBackend, GraphicsBackend, RGBA};
use std::collections::HashMap;
use vello::Scene;

/// Map Unicode math alphanumeric symbols to ASCII equivalents.
fn math_to_ascii(c: char) -> Option<char> {
    let cp = c as u32;
    match cp {
        // Math Italic Capital A-Z (U+1D434-1D44D)
        0x1D434..=0x1D44D => Some((b'A' + (cp - 0x1D434) as u8) as char),
        // Math Italic Small a-z (U+1D44E-1D467, hole at U+1D455 for 'h')
        0x1D44E..=0x1D454 => Some((b'a' + (cp - 0x1D44E) as u8) as char), // a-g
        0x1D456..=0x1D467 => Some((b'a' + (cp - 0x1D456 + 8) as u8) as char), // i-z
        0x210E => Some('h'), // Planck constant (math italic h)
        // Math Bold Capital A-Z (U+1D400-1D419)
        0x1D400..=0x1D419 => Some((b'A' + (cp - 0x1D400) as u8) as char),
        // Math Bold Small a-z (U+1D41A-1D433)
        0x1D41A..=0x1D433 => Some((b'a' + (cp - 0x1D41A) as u8) as char),
        // Math Bold Italic Capital A-Z (U+1D468-1D481)
        0x1D468..=0x1D481 => Some((b'A' + (cp - 0x1D468) as u8) as char),
        // Math Bold Italic Small a-z (U+1D482-1D49B)
        0x1D482..=0x1D49B => Some((b'a' + (cp - 0x1D482) as u8) as char),
        // Math Sans Capital A-Z (U+1D5A0-1D5B9)
        0x1D5A0..=0x1D5B9 => Some((b'A' + (cp - 0x1D5A0) as u8) as char),
        // Math Sans Small a-z (U+1D5BA-1D5D3)
        0x1D5BA..=0x1D5D3 => Some((b'a' + (cp - 0x1D5BA) as u8) as char),
        // Math Monospace Capital A-Z (U+1D670-1D689)
        0x1D670..=0x1D689 => Some((b'A' + (cp - 0x1D670) as u8) as char),
        // Math Monospace Small a-z (U+1D68A-1D6A3)
        0x1D68A..=0x1D6A3 => Some((b'a' + (cp - 0x1D68A) as u8) as char),
        // Digits (various styles)
        0x1D7CE..=0x1D7D7 => Some((b'0' + (cp - 0x1D7CE) as u8) as char),
        0x1D7D8..=0x1D7E1 => Some((b'0' + (cp - 0x1D7D8) as u8) as char),
        0x1D7E2..=0x1D7EB => Some((b'0' + (cp - 0x1D7E2) as u8) as char),
        0x1D7EC..=0x1D7F5 => Some((b'0' + (cp - 0x1D7EC) as u8) as char),
        0x1D7F6..=0x1D7FF => Some((b'0' + (cp - 0x1D7F6) as u8) as char),
        _ => None,
    }
}

/// Use the selected text face for ordinary inline operators when it contains
/// the glyph; keep stretchy and structural math symbols in the MATH face.
fn is_text_face_math_glyph(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '+' | '-' | '=' | '<' | '>' | '≤' | '≥' | '≠' | '±' | '×' | '÷' | '·' | '∞' | '∈' | '∉' | '∪' | '∩' | '∧' | '∨' | '¬' | '≈' | '≃' | '≅' | '≡' | '→' | '←' | '↔' | '⇒' | '⇔')
}

fn math_codepoints(math: &TtfMathFont<'_>) -> &'static HashMap<u16, char> {
    static CODEPOINTS: std::sync::OnceLock<HashMap<u16, char>> = std::sync::OnceLock::new();
    CODEPOINTS.get_or_init(|| {
        let mut map = HashMap::new();
        if let Some(cmap) = math.font().tables().cmap {
            for table in cmap.subtables.into_iter().filter(|t| t.is_unicode()) {
                table.codepoints(|cp| {
                    if let (Some(c), Some(gid)) = (char::from_u32(cp), table.glyph_index(cp)) {
                        map.insert(gid.0, c);
                    }
                });
            }
        }
        map
    })
}

// Keep the structural MATH font, but measure letters with the very same font
// and scale used to draw them. Substituting outlines only leaves incorrect gaps.
pub struct MixedMathFont<'a, 'p> {
    pub math: TtfMathFont<'a>,
    pub primary: Option<ttf_parser::Face<'p>>,
    codepoints: &'static HashMap<u16, char>,
}
impl<'a, 'p> MixedMathFont<'a, 'p> {
    pub fn new(math: TtfMathFont<'a>, primary: Option<ttf_parser::Face<'p>>) -> Self {
        let codepoints = math_codepoints(&math);
        Self {
            math,
            primary,
            codepoints,
        }
    }
    fn primary_glyph(&self, gid: GlyphId) -> Option<(ttf_parser::GlyphId, f64)> {
        let primary = self.primary.as_ref()?;
        let c = *self.codepoints.get(&Into::<u16>::into(gid))?;
        let c = math_to_ascii(c).unwrap_or(c);
        // Extensible roots, operators and delimiters must retain MATH outlines.
        if !is_text_face_math_glyph(c) || c == '\u{FFFC}' {
            return None;
        }
        let id = primary.glyph_index(c)?;
        primary.glyph_bounding_box(id)?;
        Some((
            id,
            0.75 * self.math.font().units_per_em() as f64 / primary.units_per_em() as f64,
        ))
    }
}
impl rex::font::MathFont for MixedMathFont<'_, '_> {
    fn glyph_index(&self, c: char) -> Option<GlyphId> {
        self.math.glyph_index(c)
    }
    fn glyph_from_gid(
        &self,
        gid: GlyphId,
    ) -> Result<rex::font::Glyph<'_, Self>, rex::error::FontError> {
        use rex::dimensions::{Unit, units::FUnit};
        let original = self.math.glyph_from_gid(gid)?;
        let mut glyph = rex::font::Glyph {
            font: self,
            gid,
            bbox: original.bbox,
            advance: original.advance,
            lsb: original.lsb,
            italics: original.italics,
            attachment: original.attachment,
        };
        if let Some((id, ratio)) = self.primary_glyph(gid) {
            let face = self.primary.as_ref().unwrap();
            let b = face.glyph_bounding_box(id).unwrap();
            let unit = |v: i16| Unit::<FUnit>::new(v as f64 * ratio);
            glyph.bbox = (unit(b.x_min), unit(b.y_min), unit(b.x_max), unit(b.y_max));
            glyph.advance =
                Unit::<FUnit>::new(face.glyph_hor_advance(id).unwrap_or(0) as f64 * ratio);
            glyph.lsb = unit(face.glyph_hor_side_bearing(id).unwrap_or(0));
            glyph.italics = Unit::new(0.0);
            glyph.attachment = Unit::new(glyph.advance.unitless(FUnit) * 0.5);
        }
        Ok(glyph)
    }
    fn kern_for(
        &self,
        gid: GlyphId,
        h: rex::dimensions::Unit<rex::dimensions::units::FUnit>,
        side: rex::font::kerning::Corner,
    ) -> Option<rex::dimensions::Unit<rex::dimensions::units::FUnit>> {
        if self.primary_glyph(gid).is_some() {
            None
        } else {
            self.math.kern_for(gid, h, side)
        }
    }
    fn italics(&self, gid: GlyphId) -> i16 {
        if self.primary_glyph(gid).is_some() {
            0
        } else {
            self.math.italics(gid)
        }
    }
    fn attachment(&self, gid: GlyphId) -> i16 {
        self.math.attachment(gid)
    }
    fn constants(
        &self,
        units: rex::dimensions::Unit<
            rex::dimensions::units::Ratio<
                rex::dimensions::units::Em,
                rex::dimensions::units::FUnit,
            >,
        >,
    ) -> rex::font::FontConstants {
        self.math.constants(units)
    }
    fn font_units_to_em(
        &self,
    ) -> rex::dimensions::Unit<
        rex::dimensions::units::Ratio<rex::dimensions::units::Em, rex::dimensions::units::FUnit>,
    > {
        self.math.font_units_to_em()
    }
    fn horz_variant(
        &self,
        gid: GlyphId,
        width: rex::dimensions::Unit<rex::dimensions::units::FUnit>,
    ) -> rex::font::VariantGlyph {
        self.math.horz_variant(gid, width)
    }
    fn vert_variant(
        &self,
        gid: GlyphId,
        height: rex::dimensions::Unit<rex::dimensions::units::FUnit>,
    ) -> rex::font::VariantGlyph {
        self.math.vert_variant(gid, height)
    }
    fn glyph_script_alternate(
        &self,
        gid: GlyphId,
        level: rex::font::common::ScriptLevel,
    ) -> Option<GlyphId> {
        if self.primary_glyph(gid).is_some() {
            None
        } else {
            self.math.glyph_script_alternate(gid, level)
        }
    }
}

impl<'f, 'p> FontBackend<MixedMathFont<'f, 'p>> for VelloBackend<'_, 'f, 'p> {
    fn symbol(&mut self, pos: Cursor, gid: GlyphId, scale: f64, font: &MixedMathFont<'f, 'p>) {
        <Self as FontBackend<TtfMathFont<'_>>>::symbol(self, pos, gid, scale, &font.math);
    }
}

/// Vello backend for ReX rendering with primary font fallback.
pub struct VelloBackend<'a, 'f, 'p> {
    scene: &'a mut Scene,
    math_font: &'f TtfMathFont<'f>,
    primary_font: Option<&'p ttf_parser::Face<'p>>,
    /// Maps math font glyph IDs to codepoints for fallback lookup.
    glyph_to_codepoint: &'static HashMap<u16, char>,
    transform: Affine,
    color_stack: Vec<Color>,
    current_color: Color,
}

impl<'a, 'f, 'p> VelloBackend<'a, 'f, 'p> {
    pub fn new(
        scene: &'a mut Scene,
        math_font: &'f TtfMathFont<'f>,
        primary_font: Option<&'p ttf_parser::Face<'p>>,
        transform: Affine,
        color: Color,
    ) -> Self {
        let glyph_to_codepoint = math_codepoints(math_font);

        Self {
            scene,
            math_font,
            primary_font,
            glyph_to_codepoint,
            transform,
            color_stack: Vec::new(),
            current_color: color,
        }
    }
}

struct PathBuilder(BezPath);

impl ttf_parser::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(Point::new(x as f64, y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(Point::new(x as f64, y as f64));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(
            Point::new(x1 as f64, y1 as f64),
            Point::new(x as f64, y as f64),
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(
            Point::new(x1 as f64, y1 as f64),
            Point::new(x2 as f64, y2 as f64),
            Point::new(x as f64, y as f64),
        );
    }
    fn close(&mut self) {
        self.0.close_path();
    }
}

impl<'f, 'p> FontBackend<TtfMathFont<'f>> for VelloBackend<'_, 'f, 'p> {
    fn symbol(&mut self, pos: Cursor, gid: GlyphId, scale: f64, _ctx: &TtfMathFont<'f>) {
        // Try primary font first if available
        if let Some(primary) = self.primary_font {
            if let Some(&codepoint) = self.glyph_to_codepoint.get(&gid.into()) {
                // Map math italic/bold Unicode to ASCII for primary font lookup
                let lookup_char = math_to_ascii(codepoint).unwrap_or(codepoint);
                if let Some(primary_gid) = primary
                    .glyph_index(lookup_char)
                    .filter(|_| is_text_face_math_glyph(lookup_char) && lookup_char != '\u{FFFC}')
                {
                    // Use primary font (slightly smaller to match text tool rendering)
                    let units_per_em = primary.units_per_em() as f64;
                    let adjusted_scale = scale * 0.75;
                    let glyph_transform = self.transform
                        * Affine::translate(kurbo::Vec2::new(pos.x, pos.y))
                        * Affine::scale_non_uniform(
                            adjusted_scale / units_per_em,
                            -adjusted_scale / units_per_em,
                        );

                    let mut builder = PathBuilder(BezPath::new());
                    if primary.outline_glyph(primary_gid, &mut builder).is_some()
                        && !builder.0.elements().is_empty()
                    {
                        self.scene.fill(
                            vello::peniko::Fill::NonZero,
                            glyph_transform,
                            self.current_color,
                            None,
                            &builder.0,
                        );
                        return;
                    }
                }
            }
        }

        // Fallback to math font
        let ttf_parser::cff::Matrix {
            sx,
            ky,
            kx,
            sy,
            tx,
            ty,
        } = self.math_font.font_matrix();
        let font_matrix = Affine::new([
            sx as f64, ky as f64, kx as f64, sy as f64, tx as f64, ty as f64,
        ]);

        let glyph_transform = self.transform
            * Affine::translate(kurbo::Vec2::new(pos.x, pos.y))
            * Affine::scale_non_uniform(scale, -scale)
            * font_matrix;

        let mut builder = PathBuilder(BezPath::new());
        self.math_font
            .font()
            .outline_glyph(gid.into(), &mut builder);

        self.scene.fill(
            vello::peniko::Fill::NonZero,
            glyph_transform,
            self.current_color,
            None,
            &builder.0,
        );
    }
}

impl GraphicsBackend for VelloBackend<'_, '_, '_> {
    fn rule(&mut self, pos: Cursor, width: f64, height: f64) {
        let rect = kurbo::Rect::new(pos.x, pos.y, pos.x + width, pos.y + height);
        self.scene.fill(
            vello::peniko::Fill::NonZero,
            self.transform,
            self.current_color,
            None,
            &rect,
        );
    }

    fn begin_color(&mut self, RGBA(r, g, b, a): RGBA) {
        self.color_stack.push(self.current_color);
        self.current_color = Color::from_rgba8(r, g, b, a);
    }

    fn end_color(&mut self) {
        if let Some(color) = self.color_stack.pop() {
            self.current_color = color;
        }
    }
}

impl<'f, 'p> Backend<TtfMathFont<'f>> for VelloBackend<'_, 'f, 'p> {}
impl<'f, 'p> Backend<MixedMathFont<'f, 'p>> for VelloBackend<'_, 'f, 'p> {}

#[cfg(test)]
mod mixed_font_tests {
    use super::*;
    #[test]
    fn primary_advance_matches_rendered_outline_scale() {
        use rex::dimensions::units::FUnit;
        let math = TtfMathFont::new(
            ttf_parser::Face::parse(include_bytes!("../assets/rex-xits.otf"), 0).unwrap(),
        )
        .unwrap();
        let primary = ttf_parser::Face::parse(include_bytes!("../assets/GelPen.ttf"), 0).unwrap();
        let font = MixedMathFont::new(math, Some(primary));
        for c in ['x', '1', '5', 'W', 'i'] {
            let glyph = font.glyph(c).unwrap();
            let p = font.primary.as_ref().unwrap();
            let expected = p.glyph_hor_advance(p.glyph_index(c).unwrap()).unwrap() as f64
                * 0.75
                * font.math.font().units_per_em() as f64
                / p.units_per_em() as f64;
            assert!((glyph.advance.unitless(FUnit) - expected).abs() < 1e-8);
        }
        for c in ['+', '='] {
            let gid = font.math.glyph_index(c).expect("math font contains operator");
            assert!(font.primary_glyph(gid).is_some(), "{c} should use text font when available");
        }
        let less_equal = font.math.glyph_index('≤').expect("math font contains relation");
        let primary = font.primary.as_ref().unwrap();
        let available_in_primary = primary
            .glyph_index('≤')
            .is_some_and(|id| primary.glyph_bounding_box(id).is_some());
        assert_eq!(font.primary_glyph(less_equal).is_some(), available_in_primary);
        let root = font.math.glyph_index('√').unwrap();
        assert!(font.primary_glyph(root).is_none());
    }
}
