use drafftink_core::tools::ToolKind;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "drafftink.user_settings.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
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
    pub autosave_enabled: bool,
    pub autosave_interval_secs: u64,
    pub restore_last_document: bool,
    pub show_properties_for_tools: bool,
    pub default_font: String,
    pub default_font_postscript: String,
    pub last_text_font: Option<drafftink_core::shapes::TextFont>,
    pub last_text_postscript: Option<String>,
    pub cursor_outline: [u8; 3],
    pub laser_color: [u8; 3],
    pub panel_positions: std::collections::BTreeMap<String, [f32; 2]>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
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
            autosave_enabled: true,
            autosave_interval_secs: 5,
            restore_last_document: true,
            show_properties_for_tools: false,
            default_font: "Google Sans".into(),
            default_font_postscript: "GoogleSans-Medium".into(),
            last_text_font: None,
            last_text_postscript: None,
            cursor_outline: [0, 0, 0],
            laser_color: [255, 0, 0],
            panel_positions: Default::default(),
        }
    }
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
        self.autosave_interval_secs = self.autosave_interval_secs.clamp(1, 3600);

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
    settings.sanitize();
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
}

#[cfg(test)]
mod font_settings_regressions {
    use super::*;
    #[test]
    fn old_settings_receive_google_medium_defaults() {
        let settings: UserSettings =
            serde_json::from_str(r#"{"shortcut_pan":"h","autosave_interval_secs":15}"#).unwrap();
        assert_eq!(settings.default_font, "Google Sans");
        assert_eq!(settings.default_font_postscript, "GoogleSans-Medium");
        assert_eq!(settings.cursor_outline, [0, 0, 0]);
        assert_eq!(settings.autosave_interval_secs, 15);
    }
    #[test]
    fn font_color_and_panel_preferences_survive_reload() {
        let mut settings = UserSettings::default();
        settings.last_text_font =
            Some(drafftink_core::shapes::TextFont::from_name("Noto Sans", ""));
        settings.last_text_postscript = Some(String::new());
        settings.laser_color = [4, 100, 220];
        settings
            .panel_positions
            .insert("properties".into(), [150.0, 210.0]);
        let restored: UserSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.last_text_font, settings.last_text_font);
        assert_eq!(restored.laser_color, settings.laser_color);
        assert_eq!(restored.panel_positions, settings.panel_positions);
    }
}
