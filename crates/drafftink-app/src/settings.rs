use drafftink_core::tools::ToolKind;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "drafftink.user_settings.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    #[serde(default)]
    pub schema_version: u8,
    pub shortcut_select: String,
    pub shortcut_pan: String,
    pub shortcut_draw: String,
    pub shortcut_highlighter: String,
    pub shortcut_eraser: String,
    pub shortcut_text: String,
    pub shortcut_math: String,
    pub shortcut_rectangle: String,
    pub shortcut_ellipse: String,
    pub shortcut_arrow: String,
    pub shortcut_line: String,
    pub shortcut_laser: String,
    pub intro_json: String,
    pub intro_name: String,
    pub export_folder_name: String,
    pub touchpad_zoom_speed: f64,
    pub autosave_enabled: bool,
    pub autosave_interval_secs: u64,
    #[serde(default = "default_optimize_images")]
    pub optimize_imported_images: bool,
    #[serde(default = "default_image_max_side")]
    pub image_max_side_px: u32,
    pub restore_last_document: bool,
    pub show_properties_for_tools: bool,
    pub hide_properties: bool,
    pub default_math_font: drafftink_core::shapes::TextFont,
    pub default_font: String,
    pub default_font_postscript: String,
    pub last_text_font: Option<drafftink_core::shapes::TextFont>,
    pub last_text_postscript: Option<String>,
    pub accent_color: [u8; 3],
    pub cursor_outline: [u8; 3],
    /// The six configurable quick colors used by the Stroke panel.
    #[serde(default = "default_stroke_colors")]
    pub stroke_colors: [[u8; 3]; 6],
    pub laser_color: [u8; 3],
    /// Keep the laser pointer active while the pointer moves without a mouse button.
    #[serde(default)]
    pub laser_permanent: bool,
    pub panel_positions: std::collections::BTreeMap<String, [f32; 2]>,
    /// Tool names hidden from the main vertical toolbar (shortcuts remain active).
    #[serde(default)]
    pub hidden_toolbar_tools: Vec<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            shortcut_select: "s".into(),
            shortcut_pan: "h".into(),
            shortcut_draw: "d".into(),
            shortcut_highlighter: "k".into(),
            shortcut_eraser: "e".into(),
            shortcut_text: "t".into(),
            shortcut_math: "m".into(),
            shortcut_rectangle: "r".into(),
            shortcut_ellipse: "o".into(),
            shortcut_arrow: "a".into(),
            shortcut_line: "l".into(),
            shortcut_laser: "z".into(),
            intro_json: String::new(),
            intro_name: String::new(),
            export_folder_name: String::new(),
            touchpad_zoom_speed: 2.0,
            autosave_enabled: true,
            autosave_interval_secs: 600,
            optimize_imported_images: true,
            image_max_side_px: 2048,
            restore_last_document: true,
            show_properties_for_tools: false,
            hide_properties: true,
            default_math_font: drafftink_core::shapes::default_math_font(),
            default_font: "Google Sans".into(),
            default_font_postscript: "GoogleSans-Medium".into(),
            last_text_font: None,
            last_text_postscript: None,
            accent_color: [59, 130, 246],
            cursor_outline: [0, 0, 0],
            stroke_colors: default_stroke_colors(),
            laser_color: [255, 0, 0],
            laser_permanent: false,
            panel_positions: Default::default(),
            hidden_toolbar_tools: Vec::new(),
        }
    }
}

fn default_optimize_images() -> bool { true }
fn default_image_max_side() -> u32 { 2048 }

fn default_stroke_colors() -> [[u8; 3]; 6] {
    [
        [59, 130, 246],  // blue
        [239, 68, 68],   // red
        [16, 185, 129],  // emerald
        [245, 158, 11],  // amber
        [168, 85, 247],  // purple
        [100, 116, 139], // slate
    ]
}

fn normalized_key(value: &str) -> String {
    value.trim().to_lowercase()
}

impl UserSettings {
    pub fn shortcut_for(&self, tool: ToolKind) -> &str {
        match tool {
            ToolKind::Select => &self.shortcut_select,
            ToolKind::Pan => &self.shortcut_pan,
            ToolKind::Freehand => &self.shortcut_draw,
            ToolKind::Highlighter => &self.shortcut_highlighter,
            ToolKind::Eraser => &self.shortcut_eraser,
            ToolKind::Text => &self.shortcut_text,
            ToolKind::Math => &self.shortcut_math,
            ToolKind::Rectangle => &self.shortcut_rectangle,
            ToolKind::Ellipse => &self.shortcut_ellipse,
            ToolKind::Arrow => &self.shortcut_arrow,
            ToolKind::Line => &self.shortcut_line,
            ToolKind::LaserPointer => &self.shortcut_laser,
        }
    }

    pub fn shortcut_for_mut(&mut self, tool: ToolKind) -> &mut String {
        match tool {
            ToolKind::Select => &mut self.shortcut_select,
            ToolKind::Pan => &mut self.shortcut_pan,
            ToolKind::Freehand => &mut self.shortcut_draw,
            ToolKind::Highlighter => &mut self.shortcut_highlighter,
            ToolKind::Eraser => &mut self.shortcut_eraser,
            ToolKind::Text => &mut self.shortcut_text,
            ToolKind::Math => &mut self.shortcut_math,
            ToolKind::Rectangle => &mut self.shortcut_rectangle,
            ToolKind::Ellipse => &mut self.shortcut_ellipse,
            ToolKind::Arrow => &mut self.shortcut_arrow,
            ToolKind::Line => &mut self.shortcut_line,
            ToolKind::LaserPointer => &mut self.shortcut_laser,
        }
    }

    pub fn tool_visible_in_toolbar(&self, tool: ToolKind) -> bool {
        !self
            .hidden_toolbar_tools
            .iter()
            .any(|hidden| hidden == &format!("{:?}", tool))
    }

    pub fn set_tool_visible_in_toolbar(&mut self, tool: ToolKind, visible: bool) {
        let name = format!("{:?}", tool);
        self.hidden_toolbar_tools.retain(|hidden| hidden != &name);
        if !visible {
            self.hidden_toolbar_tools.push(name);
        }
    }

    pub fn tool_for_key(&self, key: &str) -> Option<ToolKind> {
        let key = normalized_key(key);
        if key.is_empty() {
            return None;
        }
        [
            ToolKind::Select,
            ToolKind::Pan,
            ToolKind::Rectangle,
            ToolKind::Ellipse,
            ToolKind::Arrow,
            ToolKind::Line,
            ToolKind::Freehand,
            ToolKind::Highlighter,
            ToolKind::Eraser,
            ToolKind::Text,
            ToolKind::Math,
            ToolKind::LaserPointer,
        ]
        .into_iter()
        .find(|tool| normalized_key(self.shortcut_for(*tool)) == key)
    }

    pub fn sanitize(&mut self) {
        if self.schema_version == 0 {
            // 5 s was the original shipped default; migrate existing users to
            // the safer ten-minute default while preserving custom intervals.
            if self.autosave_interval_secs == 5 {
                self.autosave_interval_secs = 600;
            }
            self.schema_version = 1;
        }
        self.touchpad_zoom_speed = if self.touchpad_zoom_speed.is_finite() {
            self.touchpad_zoom_speed.clamp(0.25, 8.0)
        } else {
            2.0
        };
        self.autosave_interval_secs = self.autosave_interval_secs.clamp(1, 3600);
        self.image_max_side_px = self.image_max_side_px.clamp(1024, 4096);
        self.hidden_toolbar_tools.retain(|hidden| {
            [
                ToolKind::Select,
                ToolKind::Pan,
                ToolKind::Rectangle,
                ToolKind::Ellipse,
                ToolKind::Arrow,
                ToolKind::Line,
                ToolKind::Freehand,
                ToolKind::Highlighter,
                ToolKind::Eraser,
                ToolKind::Text,
                ToolKind::Math,
                ToolKind::LaserPointer,
            ]
            .into_iter()
            .any(|tool| format!("{:?}", tool) == *hidden)
        });

        // Migrate the 0.4.x defaults to the new 0.5.x layout without
        // overwriting users who already customized any of these keys.
        if self.shortcut_pan.eq_ignore_ascii_case("m")
            && self.shortcut_draw.eq_ignore_ascii_case("b")
            && self.shortcut_math == "9"
        {
            self.shortcut_pan = "h".into();
            self.shortcut_draw = "d".into();
            self.shortcut_math = "m".into();
        }
        for tool in [
            ToolKind::Select,
            ToolKind::Pan,
            ToolKind::Rectangle,
            ToolKind::Ellipse,
            ToolKind::Arrow,
            ToolKind::Line,
            ToolKind::Freehand,
            ToolKind::Highlighter,
            ToolKind::Eraser,
            ToolKind::Text,
            ToolKind::Math,
            ToolKind::LaserPointer,
        ] {
            let value = self.shortcut_for_mut(tool);
            *value = normalized_key(value);
            if value.chars().count() > 1 {
                *value = value
                    .chars()
                    .next()
                    .map(|c| c.to_string())
                    .unwrap_or_default();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_settings() -> UserSettings {
    let Some(window) = web_sys::window() else {
        return UserSettings::default();
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return UserSettings::default();
    };
    let Ok(Some(json)) = storage.get_item(STORAGE_KEY) else {
        return UserSettings::default();
    };
    let mut settings = serde_json::from_str::<UserSettings>(&json).unwrap_or_default();
    let needs_persist = settings.schema_version == 0;
    settings.sanitize();
    if needs_persist {
        save_settings(&settings);
    }
    settings
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_settings() -> UserSettings {
    UserSettings::default()
}

#[cfg(target_arch = "wasm32")]
pub fn save_settings(settings: &UserSettings) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    if let Ok(json) = serde_json::to_string(settings) {
        let _ = storage.set_item(STORAGE_KEY, &json);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_settings(_settings: &UserSettings) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_configurable() {
        let mut settings = UserSettings::default();
        settings.shortcut_draw = "q".into();
        assert_eq!(settings.tool_for_key("Q"), Some(ToolKind::Freehand));
    }

    #[test]
    fn toolbar_visibility_preferences_round_trip_and_keep_shortcuts_available() {
        let mut settings = UserSettings::default();
        settings.set_tool_visible_in_toolbar(ToolKind::Highlighter, false);
        let restored: UserSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(!restored.tool_visible_in_toolbar(ToolKind::Highlighter));
        assert_eq!(restored.tool_for_key("k"), Some(ToolKind::Highlighter));
        assert!(restored.tool_visible_in_toolbar(ToolKind::Freehand));
    }

    #[test]
    fn custom_stroke_palette_survives_reload_and_old_settings_get_defaults() {
        let mut settings = UserSettings::default();
        settings.stroke_colors = [
            [12, 34, 56],
            [65, 43, 21],
            [91, 82, 73],
            [14, 25, 36],
            [47, 58, 69],
            [70, 81, 92],
        ];
        let expected = settings.stroke_colors;
        let restored: UserSettings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.stroke_colors, expected);
        let older: UserSettings = serde_json::from_str(r#"{"shortcut_pan":"h"}"#).unwrap();
        assert_eq!(older.stroke_colors, default_stroke_colors());
    }
}

#[cfg(test)]
mod font_settings_regressions {
    use super::*;
    #[test]
    fn default_png_autosave_interval_is_ten_minutes() {
        assert_eq!(UserSettings::default().autosave_interval_secs, 600);
    }

    #[test]
    fn old_default_autosave_interval_migrates_without_overwriting_custom_values() {
        let mut old_default: UserSettings = serde_json::from_str(r#"{"autosave_interval_secs":5}"#).unwrap();
        old_default.sanitize();
        assert_eq!(old_default.autosave_interval_secs, 600);
        let mut customized: UserSettings = serde_json::from_str(r#"{"autosave_interval_secs":45}"#).unwrap();
        customized.sanitize();
        assert_eq!(customized.autosave_interval_secs, 45);
    }

    #[test]
    fn old_settings_receive_google_medium_defaults() {
        let settings: UserSettings =
            serde_json::from_str(r#"{"shortcut_pan":"h","autosave_interval_secs":15}"#).unwrap();
        assert_eq!(settings.default_font, "Google Sans");
        assert_eq!(settings.default_font_postscript, "GoogleSans-Medium");
        assert_eq!(settings.accent_color, [59, 130, 246]);
        assert_eq!(settings.cursor_outline, [0, 0, 0]);
        assert_eq!(settings.autosave_interval_secs, 15);
    }
    #[test]
    fn font_color_and_panel_preferences_survive_reload() {
        let mut settings = UserSettings::default();
        settings.last_text_font =
            Some(drafftink_core::shapes::TextFont::from_name("Noto Sans", ""));
        settings.last_text_postscript = Some(String::new());
        settings.accent_color = [180, 35, 100];
        settings.laser_color = [4, 100, 220];
        settings
            .panel_positions
            .insert("properties".into(), [150.0, 210.0]);
        let restored: UserSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.last_text_font, settings.last_text_font);
        assert_eq!(restored.accent_color, settings.accent_color);
        assert_eq!(restored.laser_color, settings.laser_color);
        assert_eq!(restored.panel_positions, settings.panel_positions);
    }
}

pub fn settings_backup_json(settings: &UserSettings) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({"format":"qurso-settings", "version":1, "settings":settings}))
}

pub fn parse_settings_backup(json: &str) -> Result<UserSettings, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if value["format"] != "qurso-settings" || value["version"] != 1 {
        return Err("Ce fichier n'est pas une configuration Qurso compatible.".into());
    }
    let mut settings: UserSettings = serde_json::from_value(value["settings"].clone()).map_err(|error| error.to_string())?;
    settings.sanitize();
    Ok(settings)
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    #[test]
    fn settings_backup_preserves_customizations_and_rejects_documents() {
        let mut settings = UserSettings::default();
        settings.stroke_colors[0] = [12,34,56];
        settings.shortcut_draw = "q".into();
        settings.intro_json = "{\"shapes\":[]}".into();
        settings.panel_positions.insert("toolbar".into(), [50.0,80.0]);
        let restored = parse_settings_backup(&settings_backup_json(&settings).unwrap()).unwrap();
        assert_eq!(restored.stroke_colors,settings.stroke_colors);
        assert_eq!(restored.shortcut_draw,"q");
        assert_eq!(restored.intro_json,settings.intro_json);
        assert_eq!(restored.panel_positions,settings.panel_positions);
        assert!(parse_settings_backup("{\"shapes\":[]}").is_err());
    }
}
