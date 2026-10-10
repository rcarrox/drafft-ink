//! UI components using egui.

use drafftink_core::shapes::{
    FillPattern, FontFamily, FontWeight, Shape, ShapeId, ShapeStyle, StrokeStyle, TextFont,
};
use drafftink_core::sync::ConnectionState;
use drafftink_core::tools::{EraserMode, ToolKind};
use drafftink_render::GridStyle;
use egui::{
    Align2, Color32, Context, CornerRadius, Frame, ImageSource, Margin, Pos2, Rect, Stroke, Vec2,
    include_image,
};

#[cfg(target_arch = "wasm32")]
use web_time::Instant as StatusInstant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as StatusInstant;

use crate::math_input::friendly_math_to_latex;
use crate::settings::UserSettings;

// Re-export from widgets crate for consistent styling
use drafftink_widgets::{
    ColorGrid, ColorSwatch, ColorSwatchWithWheel, FontSizeButton, IconButton, IconButtonStyle, NoColorSwatch,
    StrokeWidthButton, TAILWIND_COLORS, ToggleButton, default_btn, input_text,
    menu_item as widgets_menu_item, menu_item_enabled as widgets_menu_item_enabled,
    menu_separator as widgets_menu_separator, panel_frame as widgets_panel_frame, primary_btn,
    secondary_btn, section_label as widgets_section_label,
};

/// Properties of the currently selected shape(s) for the right panel.
#[derive(Debug, Clone, Default)]
pub struct SelectedShapeProps {
    /// Is a single shape selected?
    pub has_selection: bool,
    /// Number of selected shapes.
    pub selection_count: usize,
    /// Is the selected shape a text shape?
    pub is_text: bool,
    /// Is the selected shape a math shape?
    pub is_math: bool,
    /// Is the selected shape a rectangle?
    pub is_rectangle: bool,
    /// Is the selected shape a line?
    pub is_line: bool,
    /// Is the selected shape an arrow?
    pub is_arrow: bool,
    /// Is the selected shape a freehand?
    pub is_freehand: bool,
    /// Font size (for text shapes).
    pub font_size: f32,
    /// Font family (for text shapes).
    pub font_family: FontFamily,
    /// Font weight (for text shapes).
    pub font_weight: FontWeight,
    /// Local/system font family, if one is active.
    pub custom_font: Option<String>,
    pub custom_font_postscript: Option<String>,
    /// Corner radius (for rectangle shapes).
    pub corner_radius: f32,
    /// Path style for lines/arrows (0 = Direct, 1 = Flowing, 2 = Angular).
    pub path_style: u8,
    /// Stroke style for lines/arrows (0 = Solid, 1 = Dashed, 2 = Dotted).
    pub stroke_style: u8,
    /// Arrowhead style at the beginning/end (0=None, 1=Open, 2=Filled).
    pub arrow_start_head: u8,
    pub arrow_end_head: u8,
    /// Sloppiness level (0 = Architect, 1 = Artist, 2 = Cartoonist).
    pub sloppiness: u8,
    /// Fill pattern (0 = Solid, 1 = Hachure, etc).
    pub fill_pattern: u8,
    /// Has fill color set.
    pub has_fill: bool,
    /// Is a drawing tool active (show panel for new shapes)?
    pub is_drawing_tool: bool,
    /// Is the active tool for rectangles?
    pub tool_is_rectangle: bool,
    /// Calligraphy mode for freehand.
    pub calligraphy_mode: bool,
    /// Pressure simulation mode for freehand.
    pub pressure_simulation: bool,
    /// Shape opacity (0.0-1.0).
    pub opacity: f32,
}

impl SelectedShapeProps {
    /// Create from a selected shape.
    pub fn from_shape(shape: &Shape) -> Self {
        Self::from_shape_with_count(shape, 1)
    }

    /// Create from a selected shape with a specific count.
    pub fn from_shape_with_count(shape: &Shape, count: usize) -> Self {
        let sloppiness = shape.style().sloppiness as u8;
        let fill_pattern = shape.style().fill_pattern as u8;
        let has_fill = shape.style().fill_color.is_some();
        let opacity = shape.style().opacity as f32;

        match shape {
            Shape::Text(text) => Self {
                has_selection: true,
                selection_count: count,
                is_text: true,
                font_size: text.font_size as f32,
                font_family: text.font_family,
                font_weight: text.font_weight,
                custom_font: text.custom_font.clone(),
                custom_font_postscript: text.custom_font_postscript.clone(),
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Rectangle(rect) => Self {
                has_selection: true,
                selection_count: count,
                is_rectangle: true,
                corner_radius: rect.corner_radius as f32,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Line(line) => Self {
                has_selection: true,
                selection_count: count,
                is_line: true,
                path_style: line.path_style as u8,
                stroke_style: if line.style.stroke_style == StrokeStyle::Solid {
                    line.stroke_style
                } else {
                    line.style.stroke_style
                } as u8,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Arrow(arrow) => Self {
                has_selection: true,
                selection_count: count,
                is_arrow: true,
                path_style: arrow.path_style as u8,
                stroke_style: if arrow.style.stroke_style == StrokeStyle::Solid {
                    arrow.stroke_style
                } else {
                    arrow.style.stroke_style
                } as u8,
                arrow_start_head: arrow.start_head as u8,
                arrow_end_head: arrow.end_head as u8,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Freehand(_) => Self {
                has_selection: true,
                selection_count: count,
                is_freehand: true,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Math(math) => Self {
                has_selection: true,
                selection_count: count,
                is_math: true,
                font_size: math.font_size as f32,
                font_family: math.font.family,
                font_weight: math.font.weight,
                custom_font: math.font.custom.clone(),
                custom_font_postscript: math.font.postscript.clone(),
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            _ => Self {
                has_selection: true,
                selection_count: count,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
        }
    }

    /// Create props for drawing tool mode (no selection, configuring new shapes).
    pub fn for_tool(
        tool: drafftink_core::tools::ToolKind,
        ui_state: &UiState,
        calligraphy: bool,
        pressure_sim: bool,
    ) -> Self {
        use drafftink_core::tools::ToolKind;
        Self {
            is_drawing_tool: true,
            tool_is_rectangle: tool == ToolKind::Rectangle,
            is_text: tool == ToolKind::Text,
            is_line: tool == ToolKind::Line,
            is_arrow: tool == ToolKind::Arrow,
            is_freehand: tool == ToolKind::Freehand || tool == ToolKind::Highlighter,
            calligraphy_mode: calligraphy,
            pressure_simulation: pressure_sim,
            sloppiness: ui_state.sloppiness as u8,
            fill_pattern: ui_state.fill_pattern as u8,
            has_fill: ui_state.fill_color.is_some(),
            path_style: ui_state.path_style,
            corner_radius: ui_state.corner_radius,
            ..Default::default()
        }
    }
}

// Tailwind colors are now imported from drafftink_widgets

const STROKE_WIDTHS: &[(f32, &str)] = &[
    (1.0, "Thin"),
    (2.0, "Normal"),
    (4.0, "Bold"),
    (8.0, "Extra Bold"),
];

/// Which color popover is open
#[derive(Clone, Copy, PartialEq)]
pub enum ColorPopover {
    None,
    StrokeFull, // Full color grid for stroke
    FillFull,   // Full color grid for fill
    BgFull,     // Full color grid for background
    PinnedFull, // Full color grid for pinned-object background
}

/// Peer info for UI display
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: String,
    /// User name (if set)
    pub name: Option<String>,
    /// User color (CSS color string)
    pub color: String,
    /// Whether cursor is currently visible
    pub has_cursor: bool,
}

#[derive(Debug, Clone)]
pub struct InlineFormulaDraft {
    pub text_id: drafftink_core::shapes::ShapeId,
    pub range: std::ops::Range<usize>,
    pub kind: String,
    pub parts: [String; 4],
    pub active_field: usize,
    pub request_focus: bool,
}

#[derive(Debug, Clone)]
pub struct TextCommandEditor {
    pub text_id: ShapeId,
    pub formula_id: ShapeId,
    pub source: String,
    pub request_focus: bool,
}

#[derive(Debug, Clone)]
pub struct MathEditorState {
    pub shape_id: ShapeId,
    pub input: String,
    pub original_source: String,
    pub original_latex: String,
    pub is_new: bool,
}

/// One viewport-fixed Qurso Time widget, with its own clock and settings.
pub struct TimeWidgetState {
    pub id: u64,
    pub open: bool,
    pub selected: bool,
    pub position: Pos2,
    pub size: Vec2,
    pub resize_origin_size: Option<Vec2>,
    pub resize_origin_position: Option<Pos2>,
    pub resize_origin_pointer: Option<Pos2>,
    pub active_resize_index: Option<usize>,
    pub move_origin_position: Option<Pos2>,
    pub move_origin_pointer: Option<Pos2>,
    pub mode: u8,
    pub stopwatch_running: bool,
    pub stopwatch_elapsed: f64,
    pub stopwatch_started_at: Option<StatusInstant>,
    pub countdown_seconds: u32,
    pub countdown_remaining: f64,
    pub countdown_started_at: Option<StatusInstant>,
    pub alarm_enabled: bool,
    pub alarm_hour: u8,
    pub alarm_minute: u8,
    pub alarm_last_fired: String,
    pub options_open: bool,
    pub stopwatch_centiseconds: bool,
    pub clock_seconds: bool,
    pub timezone: String,
    pub text_color: Color32,
    pub background_color: Color32,
    pub text_size: u8,
}

impl TimeWidgetState {
    pub fn new(id: u64, position: Pos2) -> Self {
        Self {
            id, open: true, selected: false, position, size: Vec2::new(260.0, 112.0),
            resize_origin_size: None, resize_origin_position: None, resize_origin_pointer: None, active_resize_index: None,
            move_origin_position: None, move_origin_pointer: None, mode: 0,
            stopwatch_running: false, stopwatch_elapsed: 0.0, stopwatch_started_at: None,
            countdown_seconds: 300, countdown_remaining: 300.0, countdown_started_at: None,
            alarm_enabled: false, alarm_hour: 8, alarm_minute: 0, alarm_last_fired: String::new(),
            options_open: false, stopwatch_centiseconds: true, clock_seconds: false,
            timezone: "Local".into(), text_color: Color32::WHITE,
            background_color: Color32::from_rgb(42, 42, 42), text_size: 2,
        }
    }
}

/// UI state and actions.
pub struct UiState {
    pub presentation_mode: bool,
    pub test_controls: std::collections::BTreeMap<String, [f32; 4]>,
    pub inline_formula_draft: Option<InlineFormulaDraft>,
    pub text_command_editor: Option<TextCommandEditor>,
    pub text_command_pos: Pos2,
    pub text_command_rect: Option<Rect>,
    pub save_status: String,
    pub save_status_seen: String,
    pub save_status_since: Option<StatusInstant>,
    /// Visibility and monotonic stopwatch state; independent from canvas camera.
    pub stopwatch_open: bool,
    pub time_widgets: Vec<TimeWidgetState>,
    pub next_time_widget_id: u64,
    pub stopwatch_running: bool,
    pub stopwatch_elapsed: f64,
    pub stopwatch_started_at: Option<StatusInstant>,
    pub time_mode: u8,
    pub countdown_seconds: u32,
    pub countdown_remaining: f64,
    pub countdown_started_at: Option<StatusInstant>,
    pub alarm_enabled: bool,
    pub alarm_hour: u8,
    pub alarm_minute: u8,
    pub time_options_open: bool,
    pub alarm_last_fired: String,
    pub insert_menu_open: bool,
    pub preset_menu_open: bool,
    pub preset_manager_open: bool,
    pub presets: Vec<PresetInfo>,
    pub toolbar_rect: Option<Rect>,
    pub insert_button_rect: Option<Rect>,
    pub insert_panel_rect: Option<Rect>,
    pub inline_formula_error: String,
    /// Currently selected tool (mirrored from canvas).
    pub current_tool: ToolKind,
    /// Eraser behavior (whole object vs manual stroke trimming).
    pub eraser_mode: EraserMode,
    /// Current stroke color for new shapes.
    pub stroke_color: Color32,
    /// Current fill color for new shapes (None = no fill).
    pub fill_color: Option<Color32>,
    /// Current stroke width for new shapes.
    pub stroke_width: f32,
    pub stroke_style: StrokeStyle,
    /// Number of selected shapes.
    pub selection_count: usize,
    /// Whether all currently selected shapes are pinned to the viewport.
    pub selection_pinned: bool,
    /// Background color used for viewport-pinned shapes.
    pub pin_background: Color32,
    /// Whether the hamburger menu is open.
    pub menu_open: bool,
    /// Which color popover is currently open.
    pub color_popover: ColorPopover,
    /// Current grid style.
    pub grid_style: GridStyle,
    /// Current zoom level (1.0 = 100%).
    pub zoom_level: f64,
    /// Whether grid snapping is enabled.
    pub grid_snap_enabled: bool,
    /// Whether smart guides are enabled.
    pub smart_snap_enabled: bool,
    /// Whether angle snapping is enabled (for lines/arrows only).
    pub angle_snap_enabled: bool,
    /// Export scale factor (1 = 1x, 2 = 2x, 3 = 3x).
    pub export_scale: u8,
    /// Current sloppiness level for new shapes.
    pub sloppiness: drafftink_core::shapes::Sloppiness,
    /// Current fill pattern for new shapes.
    pub fill_pattern: FillPattern,
    /// Current corner radius for new rectangles.
    pub corner_radius: f32,
    /// Current path style for new lines/arrows (0=Direct, 1=Flowing, 2=Angular).
    pub path_style: u8,
    /// Arrow marker defaults for subsequently created arrows.
    pub arrow_start_head: u8,
    pub arrow_end_head: u8,
    // Collaboration state
    /// WebSocket connection state.
    pub connection_state: ConnectionState,
    /// Current room ID (if connected).
    pub current_room: Option<String>,
    /// Number of peers in the room.
    pub peer_count: usize,
    /// Server URL for collaboration.
    pub server_url: String,
    /// Room ID input for joining.
    pub room_input: String,
    /// Connected peers for presence display.
    pub peers: Vec<PeerInfo>,
    /// Local user's display name.
    pub user_name: String,
    /// Local user's color (hex string like "#6366f1").
    pub user_color: String,
    /// Canvas background color.
    pub bg_color: Color32,
    /// Whether the collaboration modal is open.
    pub collab_modal_open: bool,
    /// Whether the keyboard shortcuts modal is open.
    pub shortcuts_modal_open: bool,
    /// Whether the settings dialog is open.
    pub settings_open: bool,
    settings_before_edit: Option<UserSettings>,
    /// Persistent user settings.
    pub settings: UserSettings,
    /// Whether the save dialog is open.
    pub save_dialog_open: bool,
    /// Whether the open dialog is open.
    pub open_dialog_open: bool,
    /// Whether the open recent dialog is open.
    pub open_recent_dialog_open: bool,
    /// Input for save document name.
    pub save_name_input: String,
    /// List of recent document names.
    pub recent_documents: Vec<String>,
    /// Selected document from recent list.
    pub selected_recent_document: Option<String>,
    /// Clipboard for copied/cut shapes (JSON serialized).
    pub clipboard_shapes: Option<String>,
    /// Last picked stroke color from color grid.
    pub last_picked_stroke: Option<Color32>,
    /// Last picked fill color from color grid.
    pub last_picked_fill: Option<Color32>,
    /// Inline math editor state.
    pub math_editor: Option<MathEditorState>,
    /// A French dead caret was already inserted; strip the composition echo.
    pub math_dead_caret: bool,
    /// Screen position of the math object currently being edited.
    pub math_editor_screen_pos: Option<Pos2>,
    pub math_editor_rect: Option<Rect>,
    pub math_font: TextFont,
    /// Local fonts exposed by Chrome/Edge: (family, PostScript name).
    pub local_fonts: Vec<(String, String)>,
    /// Whether a system-font scan is currently in progress.
    pub local_fonts_loading: bool,
    pub current_text_font: TextFont,
    pub current_text_postscript: String,
    pub font_search: String,
    pub font_error: String,
    pub math_input_font_ready: bool,
    pub laser_color_open: bool,
    pub laser_color_rect: Option<Rect>,
    pub geometry: drafftink_core::shapes::GeometryKind,
    pub context_properties: bool,
    /// F1 can temporarily hide properties even when Settings normally shows them.
    pub properties_hotkey_hidden: bool,
    pub context_rects: Vec<Rect>,
    pub laser_color_pos: Pos2,
    /// Names of the open tabs, in order (synced from the app each frame).
    pub tab_names: Vec<String>,
    /// Index of the active tab within `tab_names`.
    pub active_tab: usize,
    /// Tab currently being renamed inline.
    pub renaming_tab: Option<usize>,
    /// Temporary rename buffer.
    pub tab_rename_buffer: String,
}

impl Default for UiState {
    fn default() -> Self {
        let settings = crate::settings::load_settings();
        let current_text_font = settings.last_text_font.clone().unwrap_or_else(|| {
            TextFont::from_name(&settings.default_font, &settings.default_font_postscript)
        });
        let current_text_postscript = settings
            .last_text_postscript
            .clone()
            .unwrap_or_else(|| settings.default_font_postscript.clone());
        Self {
            current_tool: ToolKind::Select,
            presentation_mode: false,
            test_controls: Default::default(),
            inline_formula_draft: None,
            text_command_editor: None,
            text_command_rect: None,
            text_command_pos: Pos2::new(400.0, 300.0),
            save_status: String::new(),
            save_status_seen: String::new(),
            save_status_since: None,
            stopwatch_open: false,
            time_widgets: Vec::new(),
            next_time_widget_id: 1,
            stopwatch_running: false,
            stopwatch_elapsed: 0.0,
            stopwatch_started_at: None,
            time_mode: 0,
            countdown_seconds: 300,
            countdown_remaining: 300.0,
            countdown_started_at: None,
            alarm_enabled: false,
            alarm_hour: 8,
            alarm_minute: 0,
            time_options_open: false,
            alarm_last_fired: String::new(),
            insert_menu_open: false,
            preset_menu_open: false,
            preset_manager_open: false,
            presets: Vec::new(),
            toolbar_rect: None,
            insert_button_rect: None,
            insert_panel_rect: None,
            inline_formula_error: String::new(),
            eraser_mode: EraserMode::Classic,
            stroke_color: TAILWIND_COLORS[11].shades[6], // Indigo 500
            fill_color: None,
            stroke_width: 2.0,
            stroke_style: StrokeStyle::Solid,
            selection_count: 0,
            selection_pinned: false,
            pin_background: Color32::WHITE,
            menu_open: false,
            color_popover: ColorPopover::None,
            grid_style: GridStyle::default(),
            zoom_level: drafftink_core::camera::BASE_ZOOM,
            grid_snap_enabled: false,
            smart_snap_enabled: false,
            angle_snap_enabled: false,
            export_scale: 2, // Default to 2x for good quality
            sloppiness: drafftink_core::shapes::Sloppiness::Architect,
            fill_pattern: FillPattern::Solid,
            corner_radius: 0.0, // Sharp corners by default
            path_style: 1,      // Flowing by default
            arrow_start_head: 0,
            arrow_end_head: 1,
            // Collaboration defaults
            connection_state: ConnectionState::Disconnected,
            current_room: None,
            peer_count: 0,
            server_url: "/ws".to_string(),
            room_input: String::new(),
            peers: Vec::new(),
            user_name: String::new(),
            user_color: "#6366f1".to_string(), // Indigo
            bg_color: Color32::WHITE,
            collab_modal_open: false,
            shortcuts_modal_open: false,
            settings_open: false,
            settings_before_edit: None,
            settings,
            save_dialog_open: false,
            open_dialog_open: false,
            open_recent_dialog_open: false,
            save_name_input: String::new(),
            recent_documents: Vec::new(),
            selected_recent_document: None,
            clipboard_shapes: None,
            last_picked_stroke: None,
            last_picked_fill: None,
            math_editor: None,
            math_dead_caret: false,
            math_editor_screen_pos: None,
            math_editor_rect: None,
            math_font: drafftink_core::shapes::default_math_font(),
            local_fonts: Vec::new(),
            local_fonts_loading: false,
            current_text_font,
            current_text_postscript,
            font_search: String::new(),
            font_error: String::new(),
            math_input_font_ready: false,
            laser_color_open: false,
            laser_color_rect: None,
            geometry: Default::default(),
            context_properties: false,
            properties_hotkey_hidden: false,
            context_rects: Vec::new(),
            laser_color_pos: Pos2::new(72.0, 200.0),
            tab_names: Vec::new(),
            active_tab: 0,
            renaming_tab: None,
            tab_rename_buffer: String::new(),
        }
    }
}

impl UiState {
    /// Update UI state from a shape's style.
    pub fn update_from_style(&mut self, style: &ShapeStyle) {
        let sc = style.stroke_color;
        self.stroke_color = Color32::from_rgba_unmultiplied(sc.r, sc.g, sc.b, sc.a);
        self.stroke_width = style.stroke_width as f32;
        self.stroke_style = style.stroke_style;
        self.fill_color = style
            .fill_color
            .map(|fc| Color32::from_rgba_unmultiplied(fc.r, fc.g, fc.b, fc.a));
        self.fill_pattern = style.fill_pattern;
        self.sloppiness = style.sloppiness;
    }

    /// Convert current UI style to ShapeStyle.
    /// Generates a new random seed for each shape created.
    pub fn to_shape_style(&self) -> ShapeStyle {
        use drafftink_core::shapes::SerializableColor;
        ShapeStyle {
            stroke_color: SerializableColor::new(
                self.stroke_color.r(),
                self.stroke_color.g(),
                self.stroke_color.b(),
                self.stroke_color.a(),
            ),
            stroke_width: self.stroke_width as f64,
            stroke_style: self.stroke_style,
            fill_color: self
                .fill_color
                .map(|c| SerializableColor::new(c.r(), c.g(), c.b(), c.a())),
            fill_pattern: self.fill_pattern,
            sloppiness: self.sloppiness,
            ..ShapeStyle::default() // Generates a new random seed
        }
    }
}

/// Actions that can be triggered by the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    /// Change the current tool.
    SetTool(ToolKind),
    SetGeometry(drafftink_core::shapes::GeometryKind),
    /// Change eraser behavior.
    SetEraserMode(EraserMode),
    SetDefaultFont(String, String),
    SetLaserColor(Color32),
    SetLaserPermanent(bool),
    ResetFloatingPanels,
    ToggleStopwatch,
    ImportMedia,
    ListPresets,
    AddPreset,
    RenamePreset(String, String),
    DeletePreset(String),
    InsertUserPreset(String),
    InsertTextSymbol(String),
    OpenInlineFormula(String),
    EditTextCommand(String, bool, bool),
    CommitInlineFormula(String, String, [String; 4], bool),
    /// Change stroke color.
    SetStrokeColor(Color32),
    /// Change fill color.
    SetFillColor(Option<Color32>),
    TogglePinned,
    SetPinnedBackground(Color32),
    /// Change stroke width.
    SetStrokeWidth(f32),
    SetOutlinePattern(StrokeStyle),
    /// Save document with current name to local storage.
    SaveLocal,
    /// Show save dialog to rename and save document locally.
    SaveLocalAs,
    /// Show open dialog with dropdown list.
    ShowOpenDialog,
    /// Show open recent dialog with all recent documents.
    ShowOpenRecentDialog,
    /// Save document with given name to local storage.
    SaveLocalWithName(String),
    /// Load document by name from local storage.
    LoadLocal(String),
    /// Save document (to local storage on WASM, file dialog on native).
    SaveDocument,
    /// Load document (from local storage on WASM, file dialog on native).
    LoadDocument,
    /// Download document as JSON file (WASM only, same as SaveDocument on native).
    DownloadDocument,
    /// Upload document from JSON file (WASM only, same as LoadDocument on native).
    UploadDocument,
    /// Import a Mermaid diagram from the clipboard as native, editable shapes.
    ImportMermaidFromClipboard,
    /// Switch to the tab at the given index.
    SwitchTab(usize),
    /// Create a new blank canvas tab.
    NewCanvas,
    /// Rename a canvas tab.
    RenameTab(usize, String),
    /// Close the tab at the given index.
    CloseTab(usize),
    /// Load an Excalidraw library (`.excalidrawlib`) into a new tab.
    LoadLibrary,
    /// Export document as PNG file.
    ExportPng,
    /// Copy selection to clipboard as PNG.
    CopyPng,
    /// Toggle grid style (cycles through styles).
    ToggleGrid,
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
    /// Reset zoom to 100%.
    ZoomReset,
    /// Center canvas at origin (0,0).
    CenterCanvas,
    /// Toggle grid snap.
    ToggleGridSnap,
    /// Toggle smart guides.
    ToggleSmartSnap,
    /// Toggle angle snap (for lines/arrows).
    ToggleAngleSnap,
    /// Set font size for text shapes.
    SetFontSize(f32),
    /// Set font size for math shapes.
    SetMathFontSize(f32),
    SetMathFont(TextFont),
    SetDefaultMathFont(TextFont),
    /// Set built-in font family for text shapes.
    SetFontFamily(u8), // 0=GelPen, 1=NotoSans, 2=GelPenSerif, 3=VanillaExtract, 4=XITS
    /// Ask Chrome/Edge for the list of installed local fonts.
    ScanLocalFonts,
    /// Load and apply a local font: (family, PostScript name).
    SetLocalFont(String, String),
    /// Set font weight for text shapes.
    SetFontWeight(u8), // 0 = Light, 1 = Regular, 2 = Heavy
    /// Set corner radius for rectangle shapes.
    SetCornerRadius(f32),
    /// Set export scale (1, 2, or 3).
    SetExportScale(u8),
    /// Clear document (remove all shapes).
    ClearDocument,
    /// Show intro/welcome screen.
    ShowIntro,
    /// Set sloppiness level for selected shapes.
    SetSloppiness(u8), // 0 = Architect, 1 = Artist, 2 = Cartoonist
    /// Set fill pattern for selected shapes.
    SetFillPattern(u8), // 0 = Solid, 1 = Hachure, 2 = ZigZag, 3 = CrossHatch, 4 = Dots, 5 = Dashed, 6 = ZigZagLine
    /// Set path style for selected lines/arrows.
    SetPathStyle(u8), // 0 = Direct, 1 = Flowing, 2 = Angular
    /// Set stroke style for selected lines/arrows.
    SetStrokeStyle(u8), // 0 = Solid, 1 = Dashed, 2 = Dotted
    /// Configure the start/end marker for a selected arrow.
    SetArrowHead(bool, u8), // bool selects start (true) or end (false); 0=None, 1=Open, 2=Filled
    /// Undo the last action.
    Undo,
    /// Redo the last undone action.
    Redo,
    /// Connect to collaboration server.
    Connect(String), // server URL
    /// Disconnect from collaboration server.
    Disconnect,
    /// Join a collaboration room.
    JoinRoom(String), // room ID
    /// Leave current collaboration room.
    LeaveRoom,
    /// Set user display name.
    SetUserName(String),
    /// Set user color (hex string).
    SetUserColor(String),
    /// Set canvas background color.
    SetBgColor(Color32),
    /// Bring selected shapes to front (topmost).
    BringToFront,
    /// Send selected shapes to back (bottommost).
    SendToBack,
    /// Move selected shapes one layer forward.
    BringForward,
    /// Move selected shapes one layer backward.
    SendBackward,
    /// Zoom to fit all elements (or selection if any).
    ZoomToFit,
    /// Duplicate selected shapes.
    Duplicate,
    /// Copy selected shapes to clipboard.
    CopyShapes,
    /// Cut selected shapes to clipboard.
    CutShapes,
    /// Paste shapes from clipboard.
    PasteShapes,
    /// Align selected shapes to top.
    AlignTop,
    /// Align selected shapes to bottom.
    AlignBottom,
    /// Align selected shapes to left.
    AlignLeft,
    /// Align selected shapes to right.
    AlignRight,
    /// Align selected shapes to horizontal center.
    AlignCenterH,
    /// Align selected shapes to vertical center.
    AlignCenterV,
    /// Show keyboard shortcuts help.
    ShowShortcuts,
    /// Persist the current settings.
    SaveSettings,
    /// Import a JSON document as the startup intro.
    ImportIntroJson,
    /// Clear the startup intro document.
    ClearIntro,
    /// Choose a persistent export folder (Chrome/Edge).
    ChooseExportFolder,
    /// Toggle calligraphy mode for freehand tool.
    ToggleCalligraphy,
    /// Toggle pressure simulation for freehand tool.
    TogglePressureSimulation,
    /// Flip selected shapes horizontally.
    FlipHorizontal,
    /// Flip selected shapes vertically.
    FlipVertical,
    /// Set opacity for selected shapes.
    SetOpacity(f32),
    /// Live preview of a friendly math expression (source, translated LaTeX).
    PreviewMath(ShapeId, String, String),
    /// Commit/close the current inline math editor.
    /// Carries the pre-edit state so undo can be created only when the edit is accepted.
    FinishMath(ShapeId, String, String, bool, String, String),
    /// Cancel math editing and restore the previous content, deleting a new formula.
    CancelMath(ShapeId, String, String, bool),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PresetInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub rename: String,
}

/// Tool definitions with SVG icons
struct Tool {
    kind: ToolKind,
    label: &'static str,
    icon: ImageSource<'static>,
}

fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            kind: ToolKind::Select,
            label: "Select",
            icon: include_image!("../assets/select.svg"),
        },
        Tool {
            kind: ToolKind::Pan,
            label: "Pan",
            icon: include_image!("../assets/pan.svg"),
        },
        Tool {
            kind: ToolKind::Rectangle,
            label: "Rectangle",
            icon: include_image!("../assets/rectangle.svg"),
        },
        Tool {
            kind: ToolKind::Ellipse,
            label: "Ellipse",
            icon: include_image!("../assets/ellipse.svg"),
        },
        Tool {
            kind: ToolKind::Arrow,
            label: "Arrow",
            icon: include_image!("../assets/arrow.svg"),
        },
        Tool {
            kind: ToolKind::Line,
            label: "Line",
            icon: include_image!("../assets/line.svg"),
        },
        Tool {
            kind: ToolKind::Freehand,
            label: "Draw",
            icon: include_image!("../assets/freehand.svg"),
        },
        Tool {
            kind: ToolKind::Highlighter,
            label: "Highlighter",
            icon: include_image!("../assets/highlighter.svg"),
        },
        Tool {
            kind: ToolKind::Eraser,
            label: "Eraser",
            icon: include_image!("../assets/eraser.svg"),
        },
        Tool {
            kind: ToolKind::Text,
            label: "Text",
            icon: include_image!("../assets/text.svg"),
        },
        Tool {
            kind: ToolKind::Math,
            label: "Math",
            icon: include_image!("../assets/math.svg"),
        },
        Tool {
            kind: ToolKind::LaserPointer,
            label: "Laser",
            icon: include_image!("../assets/laser.svg"),
        },
    ]
}

/// Render all UI and return any triggered action.
pub fn render_ui(
    ctx: &Context,
    ui_state: &mut UiState,
    selected_props: &SelectedShapeProps,
) -> Option<UiAction> {
    let [r, g, b] = ui_state.settings.accent_color;
    let accent = Color32::from_rgb(r, g, b);
    ctx.style_mut(|style| {
        style.visuals.selection.bg_fill = accent;
        style.visuals.widgets.active.bg_fill = accent;
        style.visuals.widgets.active.bg_stroke.color = accent;
        style.visuals.widgets.hovered.bg_stroke.color = accent;
        style.visuals.hyperlink_color = accent;
    });
    ui_state.test_controls.clear();
    if ctx.input(|i| i.pointer.primary_pressed())
        && !egui::Popup::is_any_open(ctx)
        && ctx
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|p| !ui_state.context_rects.iter().any(|r| r.contains(p)))
    {
        ui_state.context_properties = false;
    }
    ui_state.context_rects.clear();
    render_stopwatch(ctx, ui_state);
    if ui_state.presentation_mode {
        return None;
    }
    if ui_state.save_status != ui_state.save_status_seen {
        ui_state.save_status_seen.clone_from(&ui_state.save_status);
        ui_state.save_status_since = Some(StatusInstant::now());
    }
    let save_notice_elapsed = ui_state.save_status_since.map(|at| at.elapsed().as_secs_f32());
    if !ui_state.save_status.is_empty() && save_notice_elapsed.is_some_and(|elapsed| elapsed < 6.0) {
        let elapsed = save_notice_elapsed.unwrap_or_default();
        let opacity = if elapsed <= 5.0 { 1.0 } else { 1.0 - (elapsed - 5.0) };
        ctx.request_repaint_after(std::time::Duration::from_millis(80));
        egui::Area::new(egui::Id::new("save_notice"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -65.0])
            .show(ctx, |ui| {
                let mut frame = egui::Frame::popup(ui.style());
                frame.fill = frame.fill.gamma_multiply(opacity);
                frame.stroke.color = frame.stroke.color.gamma_multiply(opacity);
                frame.show(ui, |ui| {
                    ui.label(egui::RichText::new(&ui_state.save_status).color(Color32::WHITE.gamma_multiply(opacity)));
                });
            });
    }
    egui_extras::install_image_loaders(ctx);

    let toolbar_action = render_toolbar(ctx, ui_state);
    let properties_action = render_properties_panel(ctx, ui_state);
    let file_action = render_file_menu(ctx, ui_state);
    let bottom_action = render_bottom_toolbar(ctx, ui_state);
    let right_panel_action = render_right_panel(ctx, ui_state, selected_props);
    let math_action = render_math_editor(ctx, ui_state);
    let inline_action = render_inline_formula_dialog(ctx, ui_state);
    let command_action = render_text_command_editor(ctx, ui_state);
    let settings_action = render_settings_dialog(ctx, ui_state);
    let tab_action = render_tab_bar(ctx, ui_state);
    let insert_action = render_insert_panel(ctx, ui_state);

    // Render presence panel (no actions returned)
    render_presence_panel(ctx, ui_state);

    // Return the first action (toolbar takes precedence)
    toolbar_action
        .or(properties_action)
        .or(file_action)
        .or(bottom_action)
        .or(right_panel_action)
        .or(math_action)
        .or(inline_action)
        .or(command_action)
        .or(settings_action)
        .or(insert_action)
        .or(tab_action)
}

fn render_insert_panel(ctx: &Context, state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    if state.insert_menu_open {
        let pos = state.insert_button_rect.map_or(Pos2::new(64.0, 250.0), |r| Pos2::new(r.right() + 6.0, r.top()));
        let panel = egui::Area::new(egui::Id::new("insert_menu"))
            .fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
                panel_frame().show(ui, |ui| {
                    ui.set_min_width(170.0);
                    ui.visuals_mut().override_text_color = Some(Color32::BLACK);
                    ui.visuals_mut().widgets.inactive.fg_stroke.color = Color32::BLACK;
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_gray(235);
                    ui.visuals_mut().widgets.active.bg_fill = Color32::from_gray(225);
                    let image_pdf = ui.add(egui::Button::new("Image / PDF").fill(Color32::TRANSPARENT).stroke(Stroke::NONE));
                    state.test_controls.insert("Insert Image / PDF".into(), [image_pdf.rect.min.x, image_pdf.rect.min.y, image_pdf.rect.max.x, image_pdf.rect.max.y]);
                    if image_pdf.clicked() { action = Some(UiAction::ImportMedia); state.insert_menu_open = false; }
                    let time = ui.add(egui::Button::new("Time").fill(Color32::TRANSPARENT).stroke(Stroke::NONE));
                    state.test_controls.insert("Insert Time".into(), [time.rect.min.x, time.rect.min.y, time.rect.max.x, time.rect.max.y]);
                    if time.clicked() { action = Some(UiAction::ToggleStopwatch); state.insert_menu_open = false; }
                    let label = if state.preset_menu_open { "Preset  ‹" } else { "Preset  ›" };
                    let preset = ui.add(egui::Button::new(label).fill(Color32::TRANSPARENT).stroke(Stroke::NONE));
                    state.test_controls.insert("Insert Preset".into(), [preset.rect.min.x, preset.rect.min.y, preset.rect.max.x, preset.rect.max.y]);
                    if preset.clicked() {
                        state.preset_menu_open = !state.preset_menu_open;
                        if state.preset_menu_open { action = Some(UiAction::ListPresets); }
                    }
                    if state.preset_menu_open {
                        ui.separator();
                        for preset in &state.presets {
                            if ui.button(&preset.name).clicked() { action = Some(UiAction::InsertUserPreset(preset.id.clone())); state.insert_menu_open = false; }
                        }
                        if state.presets.is_empty() { ui.label("Aucun preset"); }
                    }
                });
            });
        state.insert_panel_rect = Some(panel.response.rect);
        if action.is_none() && ctx.input(|input| input.pointer.any_click()) {
            if let Some(pointer) = ctx.input(|input| input.pointer.interact_pos()) {
                let toolbar = state.toolbar_rect.unwrap_or(Rect::NOTHING);
                if !panel.response.rect.contains(pointer) && !toolbar.contains(pointer) {
                    state.insert_menu_open = false;
                    state.preset_menu_open = false;
                }
            }
        }
    }
    if state.preset_manager_open {
        egui::Window::new("PNG Preset").id(egui::Id::new("preset_manager"))
            .open(&mut state.preset_manager_open).collapsible(false).resizable(false)
            .frame(Frame::window(&ctx.style()).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_gray(220))).shadow(egui::epaint::Shadow::NONE))
            .default_pos(Pos2::new(80.0, 160.0)).show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(Color32::WHITE);
                ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(38);
                ui.visuals_mut().widgets.inactive.fg_stroke.color = Color32::WHITE;
                ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_gray(65);
                ui.visuals_mut().widgets.hovered.fg_stroke.color = Color32::WHITE;
                ui.visuals_mut().widgets.active.bg_fill = Color32::from_gray(85);
                ui.visuals_mut().widgets.active.fg_stroke.color = Color32::WHITE;
                ui.label("Images PNG enregistrées dans ce navigateur.");
                if ui.button("＋ Ajouter un PNG…").clicked() { action = Some(UiAction::AddPreset); }
                ui.separator();
                let mut rename_action = None;
                let mut delete_action = None;
                for preset in &mut state.presets {
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut preset.rename).desired_width(140.0));
                        if ui.button("Renommer").clicked() && !preset.rename.trim().is_empty() { rename_action = Some((preset.id.clone(), preset.rename.trim().to_string())); }
                        if ui.button("Supprimer").clicked() { delete_action = Some(preset.id.clone()); }
                    });
                }
                if let Some((id, name)) = rename_action { action = Some(UiAction::RenamePreset(id, name)); }
                if let Some(id) = delete_action { action = Some(UiAction::DeletePreset(id)); }
            });
    }
    action
}

fn render_stopwatch(ctx: &Context, state: &mut UiState) {
    if state.time_widgets.is_empty() { return; }
    let text_sizes = [16.0, 21.0, 27.0, 34.0];
    let mut remove = Vec::new();
    for widget in &mut state.time_widgets {
        if !widget.open { remove.push(widget.id); continue; }
        let now = StatusInstant::now();
        let elapsed = widget.stopwatch_elapsed + widget.stopwatch_started_at.map_or(0.0, |at| at.elapsed().as_secs_f64());
        let remaining = (widget.countdown_remaining - widget.countdown_started_at.map_or(0.0, |at| at.elapsed().as_secs_f64())).max(0.0);
        if widget.countdown_started_at.is_some() && remaining <= 0.0 {
            widget.countdown_remaining = 0.0;
            widget.countdown_started_at = None;
            play_time_alarm();
        }
        if widget.stopwatch_running || widget.countdown_started_at.is_some() || widget.mode == 2 || widget.alarm_enabled {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let (clock_date, clock) = qurso_clock_now(&widget.timezone);
        let mut segments = clock.split(':');
        let clock_hour = segments.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
        let clock_minute = segments.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
        let clock_second = segments.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
        let alarm_key = format!("{clock_date} {clock_hour:02}:{clock_minute:02}");
        if widget.alarm_enabled && clock_hour == widget.alarm_hour && clock_minute == widget.alarm_minute && widget.alarm_last_fired != alarm_key {
            widget.alarm_last_fired = alarm_key;
            play_time_alarm();
        }
        let shown = if widget.mode == 1 { remaining } else { elapsed };
        let total = shown.max(0.0) as u64;
        let hours = total / 3600;
        let minutes = (total / 60) % 60;
        let seconds = total % 60;
        let centiseconds = ((shown.fract() * 100.0) as u64).min(99);
        let display = if widget.mode == 2 {
            if widget.clock_seconds { format!("{clock_hour:02}:{clock_minute:02}:{clock_second:02}") } else { format!("{clock_hour:02}:{clock_minute:02}") }
        } else if hours > 0 {
            if widget.stopwatch_centiseconds { format!("{hours:02}:{minutes:02}:{seconds:02}:{centiseconds:02}") } else { format!("{hours:02}:{minutes:02}:{seconds:02}") }
        } else if widget.stopwatch_centiseconds { format!("{minutes:02}:{seconds:02}:{centiseconds:02}") } else { format!("{minutes:02}:{seconds:02}") };
        let pointer_pressed = ctx.input(|input| input.pointer.primary_pressed());
        let pointer_down = ctx.input(|input| input.pointer.button_down(egui::PointerButton::Primary));
        let pointer_released = ctx.input(|input| input.pointer.primary_released());
        if pointer_pressed {
            if let Some(pointer) = ctx.input(|input| input.pointer.interact_pos()) {
                let widget_rect = Rect::from_min_size(widget.position, widget.size);
                let was_selected = widget.selected;
                widget.selected = widget_rect.contains(pointer);
                if widget.selected && was_selected {
                    let corners = [widget_rect.left_top(), widget_rect.right_top(), widget_rect.left_bottom(), widget_rect.right_bottom()];
                    if let Some(index) = corners.iter().position(|point| point.distance(pointer) <= 14.0) {
                        widget.active_resize_index = Some(index);
                        widget.resize_origin_size = Some(widget.size);
                        widget.resize_origin_position = Some(widget.position);
                        widget.resize_origin_pointer = Some(pointer);
                        widget.move_origin_position = None;
                        widget.move_origin_pointer = None;
                    } else {
                        widget.move_origin_position = Some(widget.position);
                        widget.move_origin_pointer = Some(pointer);
                        widget.active_resize_index = None;
                    }
                } else {
                    widget.active_resize_index = None;
                    widget.move_origin_position = None;
                    widget.move_origin_pointer = None;
                }
            }
        }
        if pointer_down {
            if let Some(pointer) = ctx.input(|input| input.pointer.interact_pos()) {
                if let (Some(index), Some(origin_size), Some(origin_position), Some(pointer_origin)) = (
                    widget.active_resize_index, widget.resize_origin_size, widget.resize_origin_position, widget.resize_origin_pointer,
                ) {
                    let delta = pointer - pointer_origin;
                    let mut size = origin_size;
                    let mut position_delta = Vec2::ZERO;
                    if index % 2 == 0 { size.x -= delta.x; position_delta.x = delta.x; } else { size.x += delta.x; }
                    if index < 2 { size.y -= delta.y; position_delta.y = delta.y; } else { size.y += delta.y; }
                    let clamped = size.max(Vec2::new(180.0, 75.0));
                    if index % 2 == 0 { position_delta.x = origin_size.x - clamped.x; }
                    if index < 2 { position_delta.y = origin_size.y - clamped.y; }
                    widget.size = clamped;
                    widget.position = origin_position + position_delta;
                } else if let (Some(origin), Some(pointer_origin)) = (widget.move_origin_position, widget.move_origin_pointer) {
                    widget.position = origin + (pointer - pointer_origin);
                }
            }
        }
        if pointer_released {
            widget.active_resize_index = None;
            widget.resize_origin_size = None;
            widget.resize_origin_position = None;
            widget.resize_origin_pointer = None;
            widget.move_origin_position = None;
            widget.move_origin_pointer = None;
        }
        let mut settings_rect = None;
        let mut close_rect = None;
        let mut resize_rects = Vec::new();
        let area = egui::Area::new(egui::Id::new(("qurso_time_widget", widget.id)))
            .current_pos(widget.position).movable(false).order(egui::Order::Foreground).show(ctx, |ui| {
                let frame = egui::Frame::new().fill(widget.background_color)
                    .stroke(Stroke::new(2.0, if widget.selected { Color32::from_rgb(80, 145, 255) } else { Color32::from_rgba_unmultiplied(255,255,255,28) }))
                    .corner_radius(CornerRadius::same(12)).inner_margin(Margin::same(12)).show(ui, |ui| {
                        ui.set_min_size(widget.size);
                        if widget.selected {
                            ui.horizontal(|ui| {
                                ui.add_space((widget.size.x - 48.0).max(0.0));
                                let settings = ui.small_button("⚙").on_hover_text("Options du chronomètre");
                                settings_rect = Some(settings.rect);
                                if settings.clicked() { widget.options_open = true; }
                                let close = ui.small_button("×").on_hover_text("Supprimer le widget");
                                close_rect = Some(close.rect);
                                if close.clicked() { widget.open = false; }
                            });
                        } else {
                            ui.add_space(18.0);
                        }
                        let display_height = (widget.size.y - 48.0).max(24.0);
                        ui.allocate_ui(Vec2::new(widget.size.x, display_height), |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(egui::RichText::new(display).size(text_sizes[widget.text_size.min(3) as usize]).monospace().color(widget.text_color));
                            });
                        });
                        ui.horizontal(|ui| {
                            if widget.mode != 2 {
                                let running = if widget.mode == 1 { widget.countdown_started_at.is_some() } else { widget.stopwatch_running };
                                let icon = if running { include_image!("../assets/time-pause.svg") } else { include_image!("../assets/time-play.svg") };
                                let response = ui.add(egui::Image::new(icon).fit_to_exact_size(Vec2::splat(25.0)).sense(egui::Sense::click()));
                                if response.clicked() {
                                    if widget.mode == 1 {
                                        if running { widget.countdown_remaining = remaining; widget.countdown_started_at = None; }
                                        else { if widget.countdown_remaining <= 0.0 { widget.countdown_remaining = widget.countdown_seconds as f64; } widget.countdown_started_at = Some(now); }
                                    } else if running { widget.stopwatch_elapsed = elapsed; widget.stopwatch_started_at = None; widget.stopwatch_running = false; }
                                    else { widget.stopwatch_started_at = Some(now); widget.stopwatch_running = true; }
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widget.selected { ui.add_space(8.0); }
                            });
                        });
                    });
                if widget.selected {
                    let rect = frame.response.rect;
                    let painter = ui.painter();
                    let radius = 4.0;
                    for (index, point) in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()].into_iter().enumerate() {
                        let handle_rect = Rect::from_center_size(point, Vec2::splat(14.0));
                        resize_rects.push(handle_rect);
                        ui.interact(handle_rect, egui::Id::new(("qurso_time_resize", widget.id, index)), egui::Sense::hover());
                        painter.circle_filled(point, radius, Color32::WHITE);
                        painter.circle_stroke(point, radius, Stroke::new(1.2, Color32::from_rgb(65, 135, 255)));
                    }
                }
            });
        widget.selected |= area.response.hovered() || area.response.dragged() || area.response.clicked();
        widget.position = area.response.rect.min;
        let rect_values = |rect: Rect| [rect.min.x, rect.min.y, rect.max.x, rect.max.y];
        state.test_controls.insert(format!("Time widget {}", widget.id), rect_values(area.response.rect));
        if let Some(rect) = settings_rect { state.test_controls.insert(format!("Time options {}", widget.id), rect_values(rect)); }
        if let Some(rect) = close_rect { state.test_controls.insert(format!("Time close {}", widget.id), rect_values(rect)); }
        if widget.selected {
            for (index, rect) in resize_rects.into_iter().enumerate() {
                state.test_controls.insert(format!("Time resize {} {index}", widget.id), rect_values(rect));
            }
        }
        area.response.context_menu(|ui| {
            if ui.button("Options…").clicked() { widget.options_open = true; ui.close(); }
            if ui.button("Fermer").clicked() { widget.open = false; ui.close(); }
        });
        if widget.options_open {
            egui::Window::new("").id(egui::Id::new(("qurso_time_options", widget.id)))
                .open(&mut widget.options_open).collapsible(false).resizable(false).title_bar(false)
                .frame(Frame::window(&ctx.style()).fill(Color32::from_gray(228)).stroke(Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24))).shadow(egui::epaint::Shadow::NONE))
                .default_pos(widget.position + Vec2::new(widget.size.x + 8.0, 0.0)).show(ctx, |ui| {
                    {
                        let visuals = ui.visuals_mut();
                        visuals.override_text_color = Some(Color32::from_gray(30));
                        visuals.widgets.inactive.bg_fill = Color32::from_rgb(14, 14, 14);
                        visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
                        visuals.widgets.hovered.bg_fill = Color32::from_gray(35);
                        visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
                        visuals.widgets.active.bg_fill = Color32::from_gray(45);
                        visuals.widgets.active.fg_stroke.color = Color32::WHITE;
                    }
                    ui.label("Mode");
                    egui::ComboBox::from_id_salt(("qurso_time_mode", widget.id)).selected_text(["Chronomètre", "Compte à rebours", "Heure / alarme"][widget.mode.min(2) as usize]).show_ui(ui, |ui| {
                        ui.selectable_value(&mut widget.mode, 0, "Chronomètre");
                        ui.selectable_value(&mut widget.mode, 1, "Compte à rebours");
                        ui.selectable_value(&mut widget.mode, 2, "Heure / alarme");
                    });
                    if widget.mode == 1 {
                        ui.horizontal(|ui| { ui.label("Durée"); ui.add(egui::DragValue::new(&mut widget.countdown_seconds).range(1..=86400).suffix(" s")); });
                        ui.checkbox(&mut widget.stopwatch_centiseconds, "Afficher les centièmes");
                        if widget.countdown_started_at.is_none() { widget.countdown_remaining = widget.countdown_seconds as f64; }
                        if ui.button("Réinitialiser").clicked() {
                            widget.countdown_started_at = None;
                            widget.countdown_remaining = widget.countdown_seconds as f64;
                        }
                    } else if widget.mode == 0 {
                        ui.checkbox(&mut widget.stopwatch_centiseconds, "Afficher les centièmes");
                        if ui.button("Réinitialiser").clicked() { widget.stopwatch_running = false; widget.stopwatch_started_at = None; widget.stopwatch_elapsed = 0.0; }
                    }
                    if widget.mode == 2 {
                        ui.checkbox(&mut widget.clock_seconds, "Afficher les secondes");
                        ui.checkbox(&mut widget.alarm_enabled, "Alarme");
                        ui.horizontal(|ui| { ui.label("Heure"); ui.add(egui::DragValue::new(&mut widget.alarm_hour).range(0..=23)); ui.label(":"); ui.add(egui::DragValue::new(&mut widget.alarm_minute).range(0..=59)); });
                        egui::ComboBox::from_id_salt(("qurso_timezone", widget.id)).selected_text(&widget.timezone).show_ui(ui, |ui| {
                            for zone in ["Local", "UTC", "Europe/Paris", "Europe/London", "America/New_York", "Asia/Tokyo"] { ui.selectable_value(&mut widget.timezone, zone.to_string(), zone); }
                        });
                        if ui.button("Tester l’alarme").clicked() { play_time_alarm(); }
                    }
                    ui.separator();
                    ui.horizontal(|ui| { ui.label("Taille"); egui::ComboBox::from_id_salt(("qurso_time_size", widget.id)).selected_text(["S", "M", "L", "XL"][widget.text_size.min(3) as usize]).show_ui(ui, |ui| { for (index, size) in ["S", "M", "L", "XL"].iter().enumerate() { ui.selectable_value(&mut widget.text_size, index as u8, *size); } }); });
                    ui.horizontal(|ui| { ui.label("Texte"); ui.color_edit_button_srgba(&mut widget.text_color); ui.label("Fond"); ui.color_edit_button_srgba(&mut widget.background_color); });
                });
        }
    }
    state.time_widgets.retain(|widget| !remove.contains(&widget.id) && widget.open);
    state.stopwatch_open = !state.time_widgets.is_empty();
}

#[cfg(target_arch = "wasm32")]
fn qurso_clock_now(timezone: &str) -> (String, String) {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else { return (String::new(), "00:00:00".into()); };
    let Ok(function) = js_sys::Reflect::get(window.as_ref(), &wasm_bindgen::JsValue::from_str("qursoClockNow")).and_then(|value| value.dyn_into::<js_sys::Function>().map_err(|_| wasm_bindgen::JsValue::NULL)) else { return (String::new(), "00:00:00".into()); };
    let Ok(value) = function.call1(window.as_ref(), &wasm_bindgen::JsValue::from_str(timezone)) else { return (String::new(), "00:00:00".into()); };
    let value = value.as_string().unwrap_or_default();
    let (date, time) = value.split_once('|').unwrap_or(("", "00:00:00"));
    (date.to_string(), time.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn qurso_clock_now(_timezone: &str) -> (String, String) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let days = now.as_secs() / 86_400;
    let seconds = now.as_secs() % 86_400;
    (format!("day-{days}"), format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60))
}

fn play_time_alarm() {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        use wasm_bindgen::JsCast;
        if let Ok(value) = js_sys::Reflect::get(window.as_ref(), &wasm_bindgen::JsValue::from_str("qursoPlayAlarm")) {
            if let Ok(function) = value.dyn_into::<js_sys::Function>() { let _ = function.call0(window.as_ref()); }
        }
    }
}

/// Render the tab strip (top-center) with one button per open canvas plus a
/// control to load an Excalidraw library as a new tab. Icons in a library tab
/// are browsed on their own canvas and copied into a working tab, so no
/// separate thumbnail rendering is needed.
fn render_tab_bar(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let tabs = ui_state.tab_names.clone();
    let active = ui_state.active_tab;
    let closable = tabs.len() > 1;

    egui::Area::new(egui::Id::new("tab_bar"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                    for (i, name) in tabs.iter().enumerate() {
                        if ui_state.renaming_tab == Some(i) {
                            let response = Frame::new()
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_gray(190)))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(4, 1))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut ui_state.tab_rename_buffer)
                                            .desired_width(90.0)
                                            .text_color(Color32::BLACK)
                                            .background_color(Color32::WHITE)
                                            .frame(false),
                                    )
                                })
                                .inner;

                            response.request_focus();
                            let commit = response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter));
                            let cancel = ui.input(|input| input.key_pressed(egui::Key::Escape));
                            if commit {
                                let name = ui_state.tab_rename_buffer.trim().to_string();
                                if !name.is_empty() {
                                    action = Some(UiAction::RenameTab(i, name));
                                }
                                ui_state.renaming_tab = None;
                            } else if cancel {
                                ui_state.renaming_tab = None;
                            }
                        } else {
                            let selected = i == active;
                            let label = egui::RichText::new(name).size(12.0).color(if selected {
                                Color32::WHITE
                            } else {
                                Color32::from_gray(90)
                            });
                            let btn = egui::Button::new(label)
                                .fill(if selected {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(egui::CornerRadius::same(4));
                            let response = ui.add(btn);
                            ui_state.test_controls.insert(
                                format!("Canvas {i}"),
                                [
                                    response.rect.min.x,
                                    response.rect.min.y,
                                    response.rect.max.x,
                                    response.rect.max.y,
                                ],
                            );
                            if response.clicked() {
                                action = Some(UiAction::SwitchTab(i));
                            }
                            if response.double_clicked() {
                                ui_state.renaming_tab = Some(i);
                                ui_state.tab_rename_buffer = name.clone();
                            }
                        }

                        if closable {
                            let close = egui::Button::new(
                                egui::RichText::new("×")
                                    .size(13.0)
                                    .color(Color32::from_gray(120)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(egui::CornerRadius::same(4));
                            if ui.add(close).on_hover_text("Fermer le canvas").clicked() {
                                action = Some(UiAction::CloseTab(i));
                            }
                        }
                    }

                    ui.add_space(6.0);
                    let add_canvas = egui::Button::new(
                        egui::RichText::new("+ Canvas")
                            .size(12.0)
                            .color(Color32::from_gray(90)),
                    )
                    .fill(Color32::TRANSPARENT)
                    .corner_radius(egui::CornerRadius::same(4));
                    let response = ui
                        .add(add_canvas)
                        .on_hover_text("Ajouter un nouveau canvas vide");
                    ui_state.test_controls.insert(
                        "New canvas".into(),
                        [
                            response.rect.min.x,
                            response.rect.min.y,
                            response.rect.max.x,
                            response.rect.max.y,
                        ],
                    );
                    if response.clicked() {
                        action = Some(UiAction::NewCanvas);
                    }

                });
            });
        });

    action
}

fn floating_area(_ctx: &Context, state: &UiState, id: &str, default: Pos2) -> egui::Area {
    let pos = state
        .settings
        .panel_positions
        .get(id)
        .map(|p| Pos2::new(p[0], p[1]))
        .unwrap_or(default);
    // Egui constrains an area's initial size before its contents are measured.
    // Without these hints, a small tool panel starts as 600x400 and both moves
    // other panels and expands its drag grip to the full available width.
    let size = match id {
        "toolbar" => Vec2::new(58.0, 590.0),
        "right_panel" => Vec2::new(260.0, 400.0),
        "bottom_toolbar" => Vec2::new(440.0, 38.0),
        "properties" => Vec2::new(340.0, 112.0),
        "laser_palette" => Vec2::new(210.0, 140.0),
        _ => Vec2::new(300.0, 200.0),
    };
    let area = egui::Area::new(egui::Id::new(id)).default_size(size);
    if id == "laser_palette" {
        area.fixed_pos(default)
            .movable(false)
            .constrain(true)
            .order(egui::Order::Foreground)
    } else {
        area.default_pos(pos)
            .movable(true)
            .constrain(true)
            .order(egui::Order::Foreground)
    }
}
fn remember_panel(state: &mut UiState, id: &str, response: &egui::Response) {
    if response.drag_stopped() {
        state
            .settings
            .panel_positions
            .insert(id.into(), [response.rect.min.x, response.rect.min.y]);
        crate::settings::save_settings(&state.settings);
    }
}
fn panel_grip(ui: &mut egui::Ui) {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().clamp(24.0, 400.0), 7.0),
        // Let the movable floating Area receive the drag; a child drag response
        // captures the pointer and prevents toolbar/stroke panels from moving.
        egui::Sense::hover(),
    );
    for offset in [-6.0, 0.0, 6.0] {
        ui.painter().circle_filled(
            rect.center() + Vec2::new(offset, 0.0),
            1.0,
            Color32::from_gray(175),
        );
    }
    response
        .on_hover_text("Glisser pour déplacer le panneau")
        .on_hover_cursor(egui::CursorIcon::Grab);
}
fn stroke_pattern_button(ui: &mut egui::Ui, pattern: StrokeStyle, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(28.0, 24.0), egui::Sense::click());
    let bg = if selected {
        ui.visuals().selection.bg_fill
    } else {
        Color32::WHITE
    };
    let fg = if selected {
        Color32::WHITE
    } else {
        Color32::from_gray(45)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_gray(218)),
        egui::StrokeKind::Inside,
    );
    let y = rect.center().y;
    let mut x = rect.left() + 4.0;
    let end = rect.right() - 4.0;
    match pattern {
        StrokeStyle::Solid => {
            ui.painter()
                .line_segment([Pos2::new(x, y), Pos2::new(end, y)], Stroke::new(2.0, fg));
        }
        StrokeStyle::Dashed | StrokeStyle::DashedShort => {
            let dash = if pattern == StrokeStyle::Dashed {
                8.0
            } else {
                3.0
            };
            while x < end {
                ui.painter().line_segment(
                    [Pos2::new(x, y), Pos2::new((x + dash).min(end), y)],
                    Stroke::new(2.0, fg),
                );
                x += dash + 3.0;
            }
        }
        StrokeStyle::Dotted => {
            while x <= end {
                ui.painter().circle_filled(Pos2::new(x, y), 1.4, fg);
                x += 5.0;
            }
        }
    }
    let label = match pattern {
        StrokeStyle::Solid => "Solid",
        StrokeStyle::Dashed => "Dashed 1 — longs",
        StrokeStyle::DashedShort => "Dashed 2 — courts",
        StrokeStyle::Dotted => "Dotted",
    };
    response.on_hover_text(label).clicked()
}

/// Render the toolbar and return any triggered action.
fn render_toolbar(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let tools = get_tools();
    let mut laser_anchor = None;

    let screen = ctx.input(|i| i.content_rect());
    let output = floating_area(
        ctx,
        ui_state,
        "toolbar",
        Pos2::new(12.0, (screen.height() - 590.0).max(24.0) / 2.0),
    )
    .show(ctx, |ui| {
        panel_frame().show(ui, |ui| {
            panel_grip(ui);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                for tool in &tools {
                    if !ui_state.settings.tool_visible_in_toolbar(tool.kind) {
                        continue;
                    }
                    let is_selected = ui_state.current_tool == tool.kind;
                    let label = if tool.kind == ToolKind::Ellipse {
                        ui_state.geometry.label()
                    } else {
                        tool.label
                    };
                    let special_pointer_icon = matches!(tool.kind, ToolKind::Select | ToolKind::Pan);
                    let icon = if tool.kind == ToolKind::Ellipse {
                        match ui_state.geometry {
                            drafftink_core::shapes::GeometryKind::Triangle => include_image!("../assets/triangle.svg"),
                            drafftink_core::shapes::GeometryKind::Parallelogram => include_image!("../assets/parallelogram.svg"),
                            drafftink_core::shapes::GeometryKind::Trapezoid => include_image!("../assets/trapezoid.svg"),
                            drafftink_core::shapes::GeometryKind::Diamond => include_image!("../assets/diamond.svg"),
                            _ => tool.icon.clone(),
                        }
                    } else {
                        tool.icon.clone()
                    };
                    let mut button = IconButton::new(icon, label)
                        .shortcut(ui_state.settings.shortcut_for(tool.kind));
                    let mut style = IconButtonStyle::tool();
                    style.size = Vec2::splat(40.0);
                    style.icon_size = Vec2::splat(24.0);
                    if special_pointer_icon {
                        // Keep the supplied white interior/black outline SVG intact,
                        // while using the same hover and selected backgrounds as all tools.
                        style.icon_tint = None;
                        style.selected_icon_tint = None;
                        style.icon_offset = Vec2::new(4.0, 0.0);
                    }
                    if tool.kind == ToolKind::LaserPointer {
                        let [r, g, b] = ui_state.settings.laser_color;
                        let laser = Color32::from_rgb(r, g, b);
                        style.icon_tint = Some(laser);
                        style.selected_icon_tint = Some(laser);
                        style.hover_icon_tint = Some(laser);
                    }
                    let icon_offset = style.icon_offset;
                    let icon_size = style.icon_size;
                    button = button.style(style).selected(is_selected);
                    let response = button.show_response(ui);
                    if special_pointer_icon {
                        let icon_rect = Rect::from_center_size(
                            response.rect.center() + icon_offset,
                            icon_size,
                        );
                        ui_state.test_controls.insert(
                            format!("tool_{:?}_icon", tool.kind),
                            [icon_rect.min.x, icon_rect.min.y, icon_rect.max.x, icon_rect.max.y],
                        );
                    }
                    if tool.kind == ToolKind::LaserPointer {
                        laser_anchor = Some(response.rect.right_center() + Vec2::new(12.0, 0.0));
                    }
                    ui_state.test_controls.insert(
                        format!("tool_{:?}", tool.kind),
                        [
                            response.rect.min.x,
                            response.rect.min.y,
                            response.rect.max.x,
                            response.rect.max.y,
                        ],
                    );
                    if response.clicked() {
                        action = Some(UiAction::SetTool(tool.kind));
                    }
                    if tool.kind == ToolKind::Ellipse {
                        response.context_menu(|ui| {
                            for kind in drafftink_core::shapes::GeometryKind::ALL {
                                let option =
                                    ui.selectable_label(ui_state.geometry == kind, kind.label());
                                ui_state.test_controls.insert(
                                    format!("geometry_{:?}", kind),
                                    [
                                        option.rect.min.x,
                                        option.rect.min.y,
                                        option.rect.max.x,
                                        option.rect.max.y,
                                    ],
                                );
                                if option.clicked() {
                                    action = Some(UiAction::SetGeometry(kind));
                                    ui.close();
                                }
                            }
                        });
                    }
                    if tool.kind == ToolKind::LaserPointer && response.secondary_clicked() {
                        ui_state.laser_color_open = !ui_state.laser_color_open;
                    }
                }
                ui.separator();
                let insert_response = IconButton::new(include_image!("../assets/add.svg"), "Insérer")
                    .style(IconButtonStyle::tool()).show_response(ui);
                ui_state.insert_button_rect = Some(insert_response.rect);
                ui_state.test_controls.insert(
                    "tool_Insert".into(),
                    [insert_response.rect.min.x, insert_response.rect.min.y, insert_response.rect.max.x, insert_response.rect.max.y],
                );
                if insert_response.clicked() {
                    ui_state.insert_menu_open = !ui_state.insert_menu_open;
                }
            });
        });
    });

    remember_panel(ui_state, "toolbar", &output.response);
    ui_state.toolbar_rect = Some(output.response.rect);
    ui_state.test_controls.insert(
        "toolbar".into(),
        [
            output.response.rect.min.x,
            output.response.rect.min.y,
            output.response.rect.max.x,
            output.response.rect.max.y,
        ],
    );
    if ui_state.laser_color_open {
        if let Some(anchor) = laser_anchor {
            ui_state.laser_color_pos = anchor;
        } else {
            ui_state.laser_color_open = false;
            ui_state.laser_color_rect = None;
        }
    }
    if ui_state.laser_color_open
        && ctx.input(|i| i.pointer.primary_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|pos| {
            let in_palette = ui_state.laser_color_rect.is_some_and(|rect| rect.contains(pos));
            let toolbar_grip = Rect::from_min_size(
                output.response.rect.min,
                Vec2::new(output.response.rect.width(), 12.0),
            );
            !in_palette && !toolbar_grip.contains(pos)
        })
    {
        ui_state.laser_color_open = false;
        ui_state.laser_color_rect = None;
    }
    if ui_state.laser_color_open {
        let output = floating_area(ctx, ui_state, "laser_palette", ui_state.laser_color_pos).show(
            ctx,
            |ui| {
                panel_frame().show(ui, |ui| {
                    ui.set_width(190.0);
                    panel_grip(ui);
                    ui.label(egui::RichText::new("Couleur du laser").color(Color32::BLACK));
                    ui.horizontal(|ui| {
                        for &index in QUICK_COLORS {
                            let color = TAILWIND_COLORS[index].shades[6];
                            if color_swatch_selectable(
                                ui,
                                color,
                                TAILWIND_COLORS[index].name,
                                ui_state.settings.laser_color == [color.r(), color.g(), color.b()],
                            ) {
                                action = Some(UiAction::SetLaserColor(color));
                            }
                        }
                    });
                    let mut color = ui_state.settings.laser_color;
                    if ui.color_edit_button_srgb(&mut color).changed() {
                        action = Some(UiAction::SetLaserColor(Color32::from_rgb(
                            color[0], color[1], color[2],
                        )));
                    }
                    let mut permanent = ui_state.settings.laser_permanent;
                    let permanent_response = ui.checkbox(&mut permanent, "Permanent");
                    ui_state.test_controls.insert(
                        "laser_permanent".into(),
                        [
                            permanent_response.rect.min.x,
                            permanent_response.rect.min.y,
                            permanent_response.rect.max.x,
                            permanent_response.rect.max.y,
                        ],
                    );
                    if permanent_response.changed() {
                        action = Some(UiAction::SetLaserPermanent(permanent));
                    }
                });
            },
        );
        ui_state.laser_color_rect = Some(output.response.rect);
        ui_state.test_controls.insert(
            "laser_palette".into(),
            [
                output.response.rect.min.x,
                output.response.rect.min.y,
                output.response.rect.max.x,
                output.response.rect.max.y,
            ],
        );
    }
    action
}

/// Render the bottom-left toolbar with grid toggle and zoom controls.
fn render_bottom_toolbar(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let mut bg_color_rect = Rect::NOTHING;

    // Get screen rect to position at bottom
    #[allow(deprecated)]
    let screen_rect = ctx.input(|i| i.content_rect());
    let toolbar_height = 36.0;
    let margin = 12.0;
    let bottom_y = screen_rect.max.y
        - margin
        - toolbar_height
        - if screen_rect.width() < 900.0 {
            48.0
        } else {
            0.0
        };

    let output = floating_area(
        ctx,
        ui_state,
        "bottom_toolbar",
        Pos2::new(margin, bottom_y.max(margin)),
    )
    .interactable(true)
    .order(egui::Order::Foreground)
    .show(ctx, |ui| {
        // Light panel frame
        Frame::new()
            .fill(Color32::from_rgba_unmultiplied(250, 250, 252, 250))
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, Color32::from_gray(220)))
            .shadow(egui::epaint::Shadow {
                spread: 0,
                blur: 6,
                offset: [0, 2],
                color: Color32::from_black_alpha(10),
            })
            .inner_margin(Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                    let text_color = Color32::from_gray(80);

                    // Undo button
                    if IconButton::new(include_image!("../assets/undo.svg"), "Undo (Ctrl+Z)")
                        .small()
                        .show(ui)
                    {
                        action = Some(UiAction::Undo);
                    }

                    ui.add_space(2.0);

                    // Redo button
                    if IconButton::new(include_image!("../assets/redo.svg"), "Redo (Ctrl+Shift+Z)")
                        .small()
                        .show(ui)
                    {
                        action = Some(UiAction::Redo);
                    }

                    // Separator after undo/redo
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("|")
                            .size(14.0)
                            .color(Color32::from_gray(200)),
                    );
                    ui.add_space(8.0);

                    // Grid toggle button - draw grid pattern icon
                    let grid_tooltip = match ui_state.grid_style {
                        GridStyle::None => "No grid (click to show grid)",
                        GridStyle::Lines => "Grid (click for lines)",
                        GridStyle::HorizontalLines => "Lines (click for crosses)",
                        GridStyle::CrossPlus => "Crosses (click for dots)",
                        GridStyle::Dots => "Dots (click to hide)",
                    };

                    if grid_style_button(ui, ui_state.grid_style, grid_tooltip) {
                        action = Some(UiAction::ToggleGrid);
                    }

                    ui.add_space(4.0);

                    // Background color button - opens Tailwind color picker
                    let (clicked, rect) =
                        color_swatch_current(ui, ui_state.bg_color, "Background color");
                    bg_color_rect = rect;
                    if clicked {
                        ui_state.color_popover = if ui_state.color_popover == ColorPopover::BgFull {
                            ColorPopover::None
                        } else {
                            ColorPopover::BgFull
                        };
                    }

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("|")
                            .size(14.0)
                            .color(Color32::from_gray(200)),
                    );
                    ui.add_space(8.0);

                    // Zoom out button
                    let minus_response = ui.add(
                        egui::Label::new(
                            egui::RichText::new("\u{2212}") // − minus sign
                                .size(16.0)
                                .color(text_color),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if minus_response.clicked() {
                        action = Some(UiAction::ZoomOut);
                    }
                    minus_response.clone().on_hover_text("Zoom out");
                    minus_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                    ui.add_space(12.0);

                    // Current zoom level (clickable to reset)
                    // Display zoom relative to BASE_ZOOM (so BASE_ZOOM = 100%)
                    let zoom_pct = (ui_state.zoom_level / drafftink_core::camera::BASE_ZOOM * 100.0)
                        .round() as i32;
                    let zoom_response = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{}%", zoom_pct))
                                .size(13.0)
                                .color(text_color),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if zoom_response.clicked() {
                        action = Some(UiAction::ZoomReset);
                    }
                    zoom_response.clone().on_hover_text("Reset to 100%");
                    zoom_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                    ui.add_space(12.0);

                    // Zoom in button
                    let plus_response = ui.add(
                        egui::Label::new(egui::RichText::new("+").size(16.0).color(text_color))
                            .sense(egui::Sense::click()),
                    );
                    if plus_response.clicked() {
                        action = Some(UiAction::ZoomIn);
                    }
                    plus_response.clone().on_hover_text("Zoom in");
                    plus_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                    ui.add_space(8.0);

                    // Center button - circle with dot icon
                    if IconButton::new(
                        include_image!("../assets/center.svg"),
                        "Center canvas at origin",
                    )
                    .small()
                    .show(ui)
                    {
                        action = Some(UiAction::CenterCanvas);
                    }

                    ui.add_space(4.0);

                    // Zoom to fit button
                    let fit_tooltip = if ui_state.selection_count > 0 {
                        "Zoom to fit selection"
                    } else {
                        "Zoom to fit all elements"
                    };
                    if IconButton::new(include_image!("../assets/zoom-fit.svg"), fit_tooltip)
                        .small()
                        .show(ui)
                    {
                        action = Some(UiAction::ZoomToFit);
                    }

                    // Separator before snap buttons
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("|")
                            .size(14.0)
                            .color(Color32::from_gray(200)),
                    );
                    ui.add_space(8.0);

                    // Grid snap button
                    let grid_snap_tooltip = if ui_state.grid_snap_enabled {
                        "Grid Snap: On"
                    } else {
                        "Grid Snap: Off"
                    };
                    if IconButton::new(include_image!("../assets/snap-grid.svg"), grid_snap_tooltip)
                        .small()
                        .selected(ui_state.grid_snap_enabled)
                        .show(ui)
                    {
                        action = Some(UiAction::ToggleGridSnap);
                    }

                    ui.add_space(4.0);

                    // Smart guides button
                    let smart_snap_tooltip = if ui_state.smart_snap_enabled {
                        "Smart Guides: On"
                    } else {
                        "Smart Guides: Off"
                    };
                    if IconButton::new(
                        include_image!("../assets/snap-shapes.svg"),
                        smart_snap_tooltip,
                    )
                    .small()
                    .selected(ui_state.smart_snap_enabled)
                    .show(ui)
                    {
                        action = Some(UiAction::ToggleSmartSnap);
                    }

                    ui.add_space(4.0);

                    // Angle snap button
                    let angle_snap_tooltip = if ui_state.angle_snap_enabled {
                        "Angle Snap: On (15°)"
                    } else {
                        "Angle Snap: Off"
                    };
                    if IconButton::new(include_image!("../assets/angle.svg"), angle_snap_tooltip)
                        .small()
                        .selected(ui_state.angle_snap_enabled)
                        .show(ui)
                    {
                        action = Some(UiAction::ToggleAngleSnap);
                    }
                });
            });
    });

    remember_panel(ui_state, "bottom_toolbar", &output.response);
    // Render background color popover if open
    if ui_state.color_popover == ColorPopover::BgFull {
        // Position popover above the button (since we're at the bottom of the screen)
        if let Some(selected_color) = ColorGrid::new(ui_state.bg_color, "Background Color")
            .above()
            .show(ctx, bg_color_rect)
        {
            action = Some(UiAction::SetBgColor(selected_color));
            ui_state.color_popover = ColorPopover::None;
        }
    }

    action
}

// Quick color indices into TAILWIND_COLORS: Blue, Red, Emerald, Amber, Purple, Slate
// Using 500-level for strokes (index 5), 100-level for fills (index 1)
const QUICK_COLORS: &[usize] = &[10, 0, 6, 2, 13, 17]; // Blue, Red, Emerald, Amber, Purple, Slate

/// Render the properties panel at the top.
fn render_properties_panel(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let mut stroke_rect = Rect::NOTHING;
    let mut fill_rect = Rect::NOTHING;
    let screen = ctx.input(|i| i.content_rect());
    let output = floating_area(
        ctx,
        ui_state,
        "properties",
        Pos2::new((screen.width() - 340.0).max(24.0) / 2.0, 12.0),
    )
    .show(ctx, |ui| {
        panel_frame().show(ui, |ui| {
            panel_grip(ui);
            if ui_state.current_tool == ToolKind::Eraser {
                ui.horizontal(|ui| {
                    widgets_section_label(ui, "Eraser");
                    for mode in [EraserMode::Classic, EraserMode::Manual] {
                        if ui
                            .selectable_label(ui_state.eraser_mode == mode, format!("{mode:?}"))
                            .clicked()
                        {
                            action = Some(UiAction::SetEraserMode(mode));
                        }
                    }
                });
            }
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 4.0);
                    widgets_section_label(ui, "Stroke");
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        for &idx in QUICK_COLORS {
                            let color = TAILWIND_COLORS[idx].shades[6];
                            if color_swatch_selectable(
                                ui,
                                color,
                                TAILWIND_COLORS[idx].name,
                                ui_state.stroke_color == color,
                            ) {
                                action = Some(UiAction::SetStrokeColor(color));
                                ui_state.color_popover = ColorPopover::None;
                            }
                        }
                        let (clicked, rect) =
                            color_swatch_current(ui, ui_state.stroke_color, "Pick stroke color");
                        stroke_rect = rect;
                        if clicked {
                            ui_state.color_popover =
                                if ui_state.color_popover == ColorPopover::StrokeFull {
                                    ColorPopover::None
                                } else {
                                    ColorPopover::StrokeFull
                                };
                        }
                    });
                    widgets_section_label(ui, "Fill");
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        if fill_swatch(ui, None, "None", ui_state.fill_color.is_none()) {
                            action = Some(UiAction::SetFillColor(None));
                        }
                        if color_swatch_selectable(
                            ui,
                            Color32::WHITE,
                            "White",
                            ui_state.fill_color == Some(Color32::WHITE),
                        ) {
                            action = Some(UiAction::SetFillColor(Some(Color32::WHITE)));
                        }
                        for &idx in &QUICK_COLORS[..4] {
                            let color = TAILWIND_COLORS[idx].shades[1];
                            if color_swatch_selectable(
                                ui,
                                color,
                                TAILWIND_COLORS[idx].name,
                                ui_state.fill_color == Some(color),
                            ) {
                                action = Some(UiAction::SetFillColor(Some(color)));
                            }
                        }
                        let (clicked, rect) = color_swatch_current(
                            ui,
                            ui_state.fill_color.unwrap_or(Color32::TRANSPARENT),
                            "Pick fill color",
                        );
                        fill_rect = rect;
                        if clicked {
                            ui_state.color_popover =
                                if ui_state.color_popover == ColorPopover::FillFull {
                                    ColorPopover::None
                                } else {
                                    ColorPopover::FillFull
                                };
                        }
                    });
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 4.0);
                    widgets_section_label(ui, "Stroke width");
                    ui.horizontal(|ui| {
                        for &(width, name) in STROKE_WIDTHS {
                            if StrokeWidthButton::new(
                                width,
                                name,
                                (ui_state.stroke_width - width).abs() < 0.1,
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::SetStrokeWidth(width));
                            }
                        }
                    });
                    widgets_section_label(ui, "Stroke style");
                    ui.horizontal(|ui| {
                        for pattern in [
                            StrokeStyle::Solid,
                            StrokeStyle::Dashed,
                            StrokeStyle::DashedShort,
                            StrokeStyle::Dotted,
                        ] {
                            if stroke_pattern_button(ui, pattern, ui_state.stroke_style == pattern)
                            {
                                action = Some(UiAction::SetOutlinePattern(pattern));
                            }
                        }
                    });
                });
            });
        });
    });
    remember_panel(ui_state, "properties", &output.response);
    let (color, rect, title) = match ui_state.color_popover {
        ColorPopover::StrokeFull => (ui_state.stroke_color, stroke_rect, "Stroke Color"),
        ColorPopover::FillFull => (
            ui_state.fill_color.unwrap_or(Color32::TRANSPARENT),
            fill_rect,
            "Fill Color",
        ),
        _ => return action,
    };

    if let Some(color) = ColorGrid::new(color, title).below().show(ctx, rect) {
        if ui_state.color_popover == ColorPopover::StrokeFull {
            ui_state.last_picked_stroke = Some(color);
            action = Some(UiAction::SetStrokeColor(color));
        } else {
            ui_state.last_picked_fill = Some(color);
            action = Some(UiAction::SetFillColor(Some(color)));
        }
        ui_state.color_popover = ColorPopover::None;
    }
    action
}

// colors_match is now imported from drafftink_widgets

/// Color swatch circle that can show selection state with black inner offset - uses drafftink_widgets.
fn color_swatch_selectable(ui: &mut egui::Ui, color: Color32, name: &str, selected: bool) -> bool {
    let (clicked, _) = ColorSwatch::new(color, name).selected(selected).show(ui);
    clicked
}

/// Current color swatch with hue wheel background (color picker button) - uses drafftink_widgets.
fn color_swatch_current(ui: &mut egui::Ui, color: Color32, tooltip: &str) -> (bool, Rect) {
    ColorSwatchWithWheel::new(color, tooltip).show(ui)
}

// hue_to_rgb is now imported from drafftink_widgets

/// Grid style button - draws icon representing current grid style.
/// This is kept local because it draws a custom procedural icon.
fn grid_style_button(ui: &mut egui::Ui, style: GridStyle, tooltip: &str) -> bool {
    let size = Vec2::new(24.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let icon_color = if response.hovered() {
            Color32::from_gray(40)
        } else {
            Color32::from_gray(80)
        };

        let center = rect.center();
        let grid_size = 5.0; // Size of grid cell in icon

        match style {
            GridStyle::None => {
                // Draw empty square with slash
                let sq_size = 12.0;
                let sq = Rect::from_center_size(center, Vec2::splat(sq_size));
                ui.painter().rect_stroke(
                    sq,
                    CornerRadius::same(2),
                    Stroke::new(1.5, icon_color),
                    egui::StrokeKind::Inside,
                );
                ui.painter().line_segment(
                    [sq.left_bottom(), sq.right_top()],
                    Stroke::new(1.0, Color32::from_gray(160)),
                );
            }
            GridStyle::Lines => {
                // Draw 3x3 grid lines
                for i in 0..3 {
                    let offset = (i as f32 - 1.0) * grid_size;
                    // Vertical line
                    ui.painter().line_segment(
                        [
                            Pos2::new(center.x + offset, center.y - grid_size * 1.5),
                            Pos2::new(center.x + offset, center.y + grid_size * 1.5),
                        ],
                        Stroke::new(1.0, icon_color),
                    );
                    // Horizontal line
                    ui.painter().line_segment(
                        [
                            Pos2::new(center.x - grid_size * 1.5, center.y + offset),
                            Pos2::new(center.x + grid_size * 1.5, center.y + offset),
                        ],
                        Stroke::new(1.0, icon_color),
                    );
                }
            }
            GridStyle::HorizontalLines => {
                // Draw 3 horizontal lines
                for i in 0..3 {
                    let offset = (i as f32 - 1.0) * grid_size;
                    ui.painter().line_segment(
                        [
                            Pos2::new(center.x - grid_size * 1.5, center.y + offset),
                            Pos2::new(center.x + grid_size * 1.5, center.y + offset),
                        ],
                        Stroke::new(1.0, icon_color),
                    );
                }
            }
            GridStyle::CrossPlus => {
                // Draw small crosses at grid intersections
                let cross_size = 2.0;
                for i in -1..=1 {
                    for j in -1..=1 {
                        let cx = center.x + (i as f32) * grid_size;
                        let cy = center.y + (j as f32) * grid_size;
                        // Horizontal arm
                        ui.painter().line_segment(
                            [
                                Pos2::new(cx - cross_size, cy),
                                Pos2::new(cx + cross_size, cy),
                            ],
                            Stroke::new(1.0, icon_color),
                        );
                        // Vertical arm
                        ui.painter().line_segment(
                            [
                                Pos2::new(cx, cy - cross_size),
                                Pos2::new(cx, cy + cross_size),
                            ],
                            Stroke::new(1.0, icon_color),
                        );
                    }
                }
            }
            GridStyle::Dots => {
                // Draw dots at grid intersections
                let dot_radius = 1.5;
                for i in -1..=1 {
                    for j in -1..=1 {
                        let cx = center.x + (i as f32) * grid_size;
                        let cy = center.y + (j as f32) * grid_size;
                        ui.painter()
                            .circle_filled(Pos2::new(cx, cy), dot_radius, icon_color);
                    }
                }
            }
        }
    }

    let clicked = response.clicked();
    response.on_hover_text(tooltip);
    clicked
}

/// Render the right-side properties panel for selected shapes.
fn render_right_panel(
    ctx: &Context,
    ui_state: &mut UiState,
    props: &SelectedShapeProps,
) -> Option<UiAction> {
    if !ui_state.selection_pinned && ui_state.color_popover == ColorPopover::PinnedFull {
        ui_state.color_popover = ColorPopover::None;
    }
    if ui_state.properties_hotkey_hidden {
        return None;
    }
    if ui_state.settings.hide_properties && !ui_state.context_properties {
        return None;
    }
    let mut props = props.clone();
    if ui_state.current_tool == ToolKind::Text && !props.has_selection {
        props.font_family = ui_state.current_text_font.family;
        props.font_weight = ui_state.current_text_font.weight;
        props.custom_font = ui_state.current_text_font.custom.clone();
        props.custom_font_postscript = ui_state.current_text_font.postscript.clone();
    }
    // Text properties follow the Text tool itself: they stay visible while Text
    // is active even if generic tool-properties are disabled, and disappear as
    // soon as another tool is selected. Other tool panels respect Settings.
    let text_tool_active = ui_state.current_tool == ToolKind::Text;
    if props.has_selection && props.is_text && !text_tool_active && !ui_state.context_properties {
        return None;
    }
    if !props.has_selection
        && (!props.is_drawing_tool
            || (!ui_state.settings.show_properties_for_tools && !text_tool_active))
    {
        return None;
    }

    let mut action = None;
    let mut pin_background_rect = Rect::NOTHING;
    let panel_width = 260.0;
    let margin = 12.0;

    let screen = ctx.input(|i| i.content_rect());
    let output = floating_area(
        ctx,
        ui_state,
        "right_panel",
        Pos2::new(
            screen.right() - panel_width - margin,
            (screen.height() - 360.0).max(24.0) / 2.0,
        ),
    )
    .interactable(true)
    .order(egui::Order::Foreground)
    .show(ctx, |ui| {
        Frame::new()
            .fill(Color32::from_rgba_unmultiplied(250, 250, 252, 250))
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, Color32::from_gray(220)))
            .shadow(egui::epaint::Shadow {
                spread: 0,
                blur: 6,
                offset: [0, 2],
                color: Color32::from_black_alpha(10),
            })
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(panel_width - 24.0);

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 8.0);

                    panel_grip(ui);
                    if props.has_selection {
                        ui.horizontal(|ui| {
                            let title = if ui_state.selection_pinned { "Détacher de l’écran" } else { "Épingler à l’écran" };
                            if IconButton::new(include_image!("../assets/pin.svg"), title)
                                .small()
                                .selected(ui_state.selection_pinned)
                                .show(ui)
                            {
                                action = Some(UiAction::TogglePinned);
                            }
                            if ui_state.selection_pinned {
                                ui.label("Fond épinglé");
                                ui.add_space(5.0);
                                let (clicked, rect) = color_swatch_current(ui, ui_state.pin_background, "Couleur de fond épinglé");
                                pin_background_rect = rect;
                                if clicked {
                                    ui_state.color_popover = if ui_state.color_popover == ColorPopover::PinnedFull { ColorPopover::None } else { ColorPopover::PinnedFull };
                                }
                            }
                        });
                    }
                    // Text-specific properties
                    if props.is_text {
                        let visuals = ui.visuals_mut();
                        visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(245);
                        visuals.widgets.inactive.bg_fill = Color32::from_gray(245);
                        visuals.widgets.inactive.fg_stroke =
                            Stroke::new(1.0, Color32::from_gray(65));
                        visuals.widgets.hovered = visuals.widgets.inactive;
                        visuals.widgets.active = visuals.widgets.inactive;
                        // Font Family
                        ui.label(
                            egui::RichText::new("Font Family")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );

                        let current_font = if props.custom_font_postscript.as_deref()
                            == Some("GoogleSans-Medium")
                        {
                            "Google Sans Medium".to_string()
                        } else {
                            props
                                .custom_font
                                .clone()
                                .unwrap_or_else(|| props.font_family.display_name().to_string())
                        };

                        egui::ComboBox::from_id_salt("text_builtin_font")
                            .selected_text(current_font)
                            .width(165.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        props.custom_font.is_none()
                                            && props.font_family == FontFamily::GelPen,
                                        "GelPen",
                                    )
                                    .clicked()
                                {
                                    action = Some(UiAction::SetFontFamily(0));
                                }
                                if ui
                                    .selectable_label(
                                        props.custom_font.is_none()
                                            && props.font_family == FontFamily::NotoSans,
                                        "Noto Sans",
                                    )
                                    .clicked()
                                {
                                    action = Some(UiAction::SetFontFamily(1));
                                }
                                if ui
                                    .selectable_label(
                                        props.custom_font.is_none()
                                            && props.font_family == FontFamily::GelPenSerif,
                                        "GelPen Serif",
                                    )
                                    .clicked()
                                {
                                    action = Some(UiAction::SetFontFamily(2));
                                }
                                if ui
                                    .selectable_label(
                                        props.custom_font.is_none()
                                            && props.font_family == FontFamily::VanillaExtract,
                                        "Vanilla",
                                    )
                                    .clicked()
                                {
                                    action = Some(UiAction::SetFontFamily(3));
                                }
                                if ui
                                    .selectable_label(
                                        props.custom_font.is_none()
                                            && props.font_family == FontFamily::XitsMath,
                                        "STIX Symbols",
                                    )
                                    .clicked()
                                {
                                    action = Some(UiAction::SetFontFamily(4));
                                }

                                if !ui_state.local_fonts.is_empty() {
                                    ui.separator();
                                    for (family, postscript) in &ui_state.local_fonts {
                                        let selected = props.custom_font_postscript.as_deref()
                                            == Some(postscript.as_str())
                                            || (props.custom_font_postscript.is_none()
                                                && props.custom_font.as_deref()
                                                    == Some(family.as_str()));
                                        if ui
                                            .selectable_label(
                                                selected,
                                                format!("{} — {}", family, postscript),
                                            )
                                            .clicked()
                                        {
                                            action = Some(UiAction::SetLocalFont(
                                                family.clone(),
                                                postscript.clone(),
                                            ));
                                        }
                                    }
                                }
                            });

                        ui.add_space(4.0);

                        // Font Weight
                        ui.label(
                            egui::RichText::new("Font Weight")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            let is_light = props.font_weight == FontWeight::Light;
                            if ToggleButton::new("Light", is_light).show(ui) && !is_light {
                                action = Some(UiAction::SetFontWeight(0));
                            }

                            let is_regular = props.font_weight == FontWeight::Regular;
                            if ToggleButton::new("Regular", is_regular).show(ui) && !is_regular {
                                action = Some(UiAction::SetFontWeight(1));
                            }

                            let is_medium = props.font_weight == FontWeight::Medium;
                            if ToggleButton::new("Medium", is_medium).show(ui) && !is_medium {
                                action = Some(UiAction::SetFontWeight(3));
                            }
                            let is_heavy = props.font_weight == FontWeight::Heavy;
                            if ToggleButton::new("Heavy", is_heavy).show(ui) && !is_heavy {
                                action = Some(UiAction::SetFontWeight(2));
                            }
                        });

                        ui.add_space(4.0);

                        // Font Size - S/M/L/XL buttons
                        ui.label(
                            egui::RichText::new("Font Size")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            // S = 16px
                            let is_small = (props.font_size - 16.0).abs() < 1.0;
                            if FontSizeButton::new("S", 16.0, is_small).show(ui) {
                                action = Some(UiAction::SetFontSize(16.0));
                            }

                            // M = 20px (default)
                            let is_medium = (props.font_size - 20.0).abs() < 1.0;
                            if FontSizeButton::new("M", 20.0, is_medium).show(ui) {
                                action = Some(UiAction::SetFontSize(20.0));
                            }

                            // L = 28px
                            let is_large = (props.font_size - 28.0).abs() < 1.0;
                            if FontSizeButton::new("L", 28.0, is_large).show(ui) {
                                action = Some(UiAction::SetFontSize(28.0));
                            }

                            // XL = 36px
                            let is_xlarge = (props.font_size - 36.0).abs() < 1.0;
                            if FontSizeButton::new("XL", 36.0, is_xlarge).show(ui) {
                                action = Some(UiAction::SetFontSize(36.0));
                            }
                        });
                    }

                    // Math font applies to the selected object, independently of Text.
                    if props.is_math {
                        let font = TextFont {
                            family: props.font_family,
                            weight: props.font_weight,
                            custom: props.custom_font.clone(),
                            postscript: props.custom_font_postscript.clone(),
                        };
                        if let Some(font) = math_font_picker(
                            ui,
                            "math_object_font",
                            &font,
                            &ui_state.local_fonts,
                            &mut ui_state.test_controls,
                        ) {
                            action = Some(UiAction::SetMathFont(font));
                        }
                        ui.label(
                            egui::RichText::new("Font Size")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            let is_small = (props.font_size - 16.0).abs() < 1.0;
                            if FontSizeButton::new("S", 16.0, is_small).show(ui) {
                                action = Some(UiAction::SetMathFontSize(16.0));
                            }

                            let is_medium = (props.font_size - 20.0).abs() < 1.0;
                            if FontSizeButton::new("M", 20.0, is_medium).show(ui) {
                                action = Some(UiAction::SetMathFontSize(20.0));
                            }

                            let is_large = (props.font_size - 28.0).abs() < 1.0;
                            if FontSizeButton::new("L", 28.0, is_large).show(ui) {
                                action = Some(UiAction::SetMathFontSize(28.0));
                            }

                            let is_xlarge = (props.font_size - 36.0).abs() < 1.0;
                            if FontSizeButton::new("XL", 36.0, is_xlarge).show(ui) {
                                action = Some(UiAction::SetMathFontSize(36.0));
                            }
                        });
                    }

                    // Rectangle-specific properties (for selected rect OR rectangle tool)
                    if props.is_rectangle || props.tool_is_rectangle {
                        // Corner Radius - On/Off toggle
                        ui.label(
                            egui::RichText::new("Rounded Corners")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            // Off = 0px (sharp corners)
                            let is_off = props.corner_radius < 1.0;
                            if ToggleButton::new("Off", is_off).show(ui) && !is_off {
                                action = Some(UiAction::SetCornerRadius(0.0));
                            }

                            // On = 32px (adaptive radius)
                            let is_on = props.corner_radius >= 1.0;
                            if ToggleButton::new("On", is_on).show(ui) && !is_on {
                                action = Some(UiAction::SetCornerRadius(32.0));
                            }
                        });
                    }

                    // Sloppiness (for all shapes except text and freehand/highlighter)
                    if !props.is_text && !props.is_freehand {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Sloppiness")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            // Architect = 0 (clean lines)
                            let is_architect = props.sloppiness == 0;
                            if ToggleButton::new("Architect", is_architect).show(ui)
                                && !is_architect
                            {
                                action = Some(UiAction::SetSloppiness(0));
                            }

                            // Artist = 1 (slight wobble)
                            let is_artist = props.sloppiness == 1;
                            if ToggleButton::new("Artist", is_artist).show(ui) && !is_artist {
                                action = Some(UiAction::SetSloppiness(1));
                            }

                            // Cartoonist = 2 (very sketchy)
                            let is_cartoonist = props.sloppiness == 2;
                            if ToggleButton::new("Cartoonist", is_cartoonist).show(ui)
                                && !is_cartoonist
                            {
                                action = Some(UiAction::SetSloppiness(2));
                            }

                            // Drunk = 3 (chaotic)
                            let is_drunk = props.sloppiness == 3;
                            if ToggleButton::new("Drunk", is_drunk).show(ui) && !is_drunk {
                                action = Some(UiAction::SetSloppiness(3));
                            }
                        });
                    }

                    // Fill pattern (only for shapes with fill, not lines/arrows/freehand)
                    if props.has_fill && !props.is_line && !props.is_arrow && !props.is_freehand {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Fill Pattern")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            let patterns =
                                [(0u8, "Solid"), (1, "Hatch"), (3, "Cross"), (4, "Dots")];
                            for (idx, name) in patterns {
                                let is_selected = props.fill_pattern == idx;
                                if ToggleButton::new(name, is_selected).show(ui) && !is_selected {
                                    action = Some(UiAction::SetFillPattern(idx));
                                }
                            }
                        });
                    }

                    // Path style (for lines and arrows only)
                    if props.is_line || props.is_arrow {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Path")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            let is_direct = props.path_style == 0;
                            if ToggleButton::new("Direct", is_direct).show(ui) && !is_direct {
                                action = Some(UiAction::SetPathStyle(0));
                            }

                            let is_flowing = props.path_style == 1;
                            if ToggleButton::new("Flowing", is_flowing).show(ui) && !is_flowing {
                                action = Some(UiAction::SetPathStyle(1));
                            }

                            let is_angular = props.path_style == 2;
                            if ToggleButton::new("Angular", is_angular).show(ui) && !is_angular {
                                action = Some(UiAction::SetPathStyle(2));
                            }
                        });

                        // Stroke style (solid/dashed/dotted)
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Stroke")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            let is_solid = props.stroke_style == 0;
                            if ToggleButton::new("Solid", is_solid).show(ui) && !is_solid {
                                action = Some(UiAction::SetStrokeStyle(0));
                            }

                            let is_dashed = props.stroke_style == 1;
                            if ToggleButton::new("Dashed", is_dashed).show(ui) && !is_dashed {
                                action = Some(UiAction::SetStrokeStyle(1));
                            }

                            let is_dotted = props.stroke_style == 2;
                            if ToggleButton::new("Dotted", is_dotted).show(ui) && !is_dotted {
                                action = Some(UiAction::SetStrokeStyle(2));
                            }
                        });
                        if props.is_arrow && props.has_selection {
                            for (is_start, label, selected) in [
                                (true, "Départ", props.arrow_start_head),
                                (false, "Arrivée", props.arrow_end_head),
                            ] {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(label)
                                            .size(11.0)
                                            .color(Color32::from_gray(100)),
                                    );
                                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                                    for (value, name) in
                                        [(0u8, "Aucune"), (1, "Ouverte"), (2, "Pleine")]
                                    {
                                        if ToggleButton::new(name, selected == value).show(ui)
                                            && selected != value
                                        {
                                            action = Some(UiAction::SetArrowHead(is_start, value));
                                        }
                                    }
                                });
                            }
                        }
                    }

                    // Calligraphy mode (for freehand tool only)
                    if props.is_freehand {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Style")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            if ToggleButton::new("Normal", !props.calligraphy_mode).show(ui)
                                && props.calligraphy_mode
                            {
                                action = Some(UiAction::ToggleCalligraphy);
                            }
                            if ToggleButton::new("Calligraphy", props.calligraphy_mode).show(ui)
                                && !props.calligraphy_mode
                            {
                                action = Some(UiAction::ToggleCalligraphy);
                            }
                        });

                        // Pressure simulation toggle
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            if ToggleButton::new("Uniform", !props.pressure_simulation).show(ui)
                                && props.pressure_simulation
                            {
                                action = Some(UiAction::TogglePressureSimulation);
                            }
                            if ToggleButton::new("Pressure", props.pressure_simulation).show(ui)
                                && !props.pressure_simulation
                            {
                                action = Some(UiAction::TogglePressureSimulation);
                            }
                        });
                    }

                    // Z-Order controls (only when shapes are selected, not for drawing tools)
                    if props.has_selection && !props.is_drawing_tool {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Layer")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            // Back (send to bottommost)
                            if IconButton::new(
                                include_image!("../assets/layer-back.svg"),
                                "Send to Back",
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::SendToBack);
                            }

                            // Backward (one layer down)
                            if IconButton::new(
                                include_image!("../assets/layer-backward.svg"),
                                "Send Backward",
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::SendBackward);
                            }

                            // Forward (one layer up)
                            if IconButton::new(
                                include_image!("../assets/layer-forward.svg"),
                                "Bring Forward",
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::BringForward);
                            }

                            // Front (bring to topmost)
                            if IconButton::new(
                                include_image!("../assets/layer-front.svg"),
                                "Bring to Front",
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::BringToFront);
                            }
                        });

                        // Flip/Mirror controls
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Transform")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                            if IconButton::new(
                                include_image!("../assets/flip-h.svg"),
                                "Flip Horizontal",
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::FlipHorizontal);
                            }
                            if IconButton::new(
                                include_image!("../assets/flip-v.svg"),
                                "Flip Vertical",
                            )
                            .show(ui)
                            {
                                action = Some(UiAction::FlipVertical);
                            }
                        });

                        // Opacity control
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Opacity")
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.horizontal(|ui| {
                            let mut opacity = props.opacity;
                            let slider = egui::Slider::new(&mut opacity, 0.0..=1.0)
                                .show_value(false)
                                .custom_formatter(|v, _| format!("{}%", (v * 100.0) as i32));
                            if ui.add(slider).changed() {
                                action = Some(UiAction::SetOpacity(opacity));
                            }
                            ui.label(
                                egui::RichText::new(format!("{}%", (props.opacity * 100.0) as i32))
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                        });

                        // Alignment controls (only when 2+ shapes are selected)
                        if props.selection_count >= 2 {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Align")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                if IconButton::new(
                                    include_image!("../assets/align-left.svg"),
                                    "Align Left",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::AlignLeft);
                                }
                                if IconButton::new(
                                    include_image!("../assets/align-center-v.svg"),
                                    "Align Center (Vertical)",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::AlignCenterV);
                                }
                                if IconButton::new(
                                    include_image!("../assets/align-right.svg"),
                                    "Align Right",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::AlignRight);
                                }
                                if IconButton::new(
                                    include_image!("../assets/align-top.svg"),
                                    "Align Top",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::AlignTop);
                                }
                                if IconButton::new(
                                    include_image!("../assets/align-center-h.svg"),
                                    "Align Center (Horizontal)",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::AlignCenterH);
                                }
                                if IconButton::new(
                                    include_image!("../assets/align-bottom.svg"),
                                    "Align Bottom",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::AlignBottom);
                                }
                            });
                        }
                    }
                });
            });
    });

    ui_state.context_rects.push(output.response.rect);
    remember_panel(ui_state, "right_panel", &output.response);
    if ui_state.color_popover == ColorPopover::PinnedFull {
        let palette_pos = Pos2::new(
            (pin_background_rect.left() - 100.0).clamp(4.0, (ctx.screen_rect().right() - 520.0).max(4.0)),
            (pin_background_rect.bottom() + 8.0).clamp(4.0, (ctx.screen_rect().bottom() - 270.0).max(4.0)),
        );
        if ctx.input(|input| input.pointer.any_click()) {
            if let Some(pointer) = ctx.input(|input| input.pointer.interact_pos()) {
                let palette_rect = Rect::from_min_size(palette_pos, Vec2::new(520.0, 270.0));
                if !palette_rect.contains(pointer) && !pin_background_rect.contains(pointer) {
                    ui_state.color_popover = ColorPopover::None;
                }
            }
        }
        if let Some(color) = ColorGrid::new(ui_state.pin_background, "Pinned Object Background")
            .below()
            .show(ctx, pin_background_rect)
        {
            action = Some(UiAction::SetPinnedBackground(color));
            ui_state.color_popover = ColorPopover::None;
        }
    }
    action
}

/// Render the hamburger menu at top-left.
///
/// The personal/local build intentionally exposes no collaboration controls.
fn render_file_menu(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let has_selection = ui_state.selection_count > 0;

    // Single hamburger button in the personal/local build.
    egui::Area::new(egui::Id::new("hamburger_button"))
        .anchor(Align2::LEFT_TOP, Vec2::new(12.0, 12.0))
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                if hamburger_button(ui, ui_state.menu_open) {
                    ui_state.menu_open = !ui_state.menu_open;
                }
            });
        });

    // Dropdown menu (only shown when open)
    if ui_state.menu_open {
        egui::Area::new(egui::Id::new("file_menu_dropdown"))
            .anchor(Align2::LEFT_TOP, Vec2::new(12.0, 56.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                panel_frame().show(ui, |ui| {
                    ui.set_width(180.0); // Fixed menu width
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                        if menu_item(ui, "Save Local", "Ctrl+S") {
                            action = Some(UiAction::SaveLocal);
                            ui_state.menu_open = false;
                        }
                        if menu_item(ui, "Save Local As...", "") {
                            action = Some(UiAction::SaveLocalAs);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        if menu_item(ui, "Open", "Ctrl+O") {
                            action = Some(UiAction::ShowOpenDialog);
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Open Recent...", "") {
                            action = Some(UiAction::ShowOpenRecentDialog);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        if menu_item(ui, "Clear", "") {
                            action = Some(UiAction::ClearDocument);
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Show Intro", "") {
                            action = Some(UiAction::ShowIntro);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        // Download/Upload for file export/import (WASM shows both, native just uses Save/Open)
                        #[cfg(target_arch = "wasm32")]
                        {
                            if menu_item(ui, "Export JSON", "") {
                                action = Some(UiAction::DownloadDocument);
                                ui_state.menu_open = false;
                            }
                            if menu_item(ui, "Import JSON/Excalidraw", "") {
                                action = Some(UiAction::UploadDocument);
                                ui_state.menu_open = false;
                            }
                            widgets_menu_separator(ui);
                        }

                        if menu_item(ui, "Import Mermaid (Clipboard)", "") {
                            action = Some(UiAction::ImportMermaidFromClipboard);
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "PNG Preset…", "") {
                            action = Some(UiAction::ListPresets);
                            ui_state.preset_manager_open = true;
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Import Library...", "") {
                            action = Some(UiAction::LoadLibrary);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        if menu_item(ui, "Export PNG", "Ctrl+E") {
                            action = Some(UiAction::ExportPng);
                            ui_state.menu_open = false;
                        }
                        // Copy as PNG (show disabled state if no selection)
                        if menu_item_enabled(ui, "Copy as PNG", "Ctrl+Shift+C", has_selection) {
                            action = Some(UiAction::CopyPng);
                            ui_state.menu_open = false;
                        }

                        // Export scale selector
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0); // Align with menu item text
                            ui.label(
                                egui::RichText::new("Scale:")
                                    .size(11.0)
                                    .color(Color32::from_rgb(100, 116, 139)),
                            );
                            ui.add_space(4.0);
                            for scale in [1u8, 2, 3] {
                                let label = format!("{}x", scale);
                                let selected = ui_state.export_scale == scale;
                                let btn = egui::Button::new(
                                    egui::RichText::new(&label).size(11.0).color(if selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(80)
                                    }),
                                )
                                .fill(if selected {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(egui::Stroke::NONE)
                                .corner_radius(egui::CornerRadius::same(4))
                                .min_size(Vec2::new(24.0, 20.0));
                                if ui.add(btn).clicked() {
                                    action = Some(UiAction::SetExportScale(scale));
                                }
                            }
                        });

                        widgets_menu_separator(ui);

                        if menu_item(ui, "Keyboard Shortcuts", "?") {
                            action = Some(UiAction::ShowShortcuts);
                            ui_state.menu_open = false;
                        }
                        if menu_item(ui, "Settings", "") {
                            ui_state.settings_open = true;
                            ui_state.menu_open = false;
                        }
                    });
                });
            });

        // Close menu when clicking outside
        if ctx.input(|i| i.pointer.any_click()) {
            let buttons_rect = Rect::from_min_size(Pos2::new(12.0, 12.0), Vec2::new(48.0, 48.0));
            let menu_rect = Rect::from_min_size(
                Pos2::new(12.0, 56.0),
                Vec2::new(180.0, 280.0), // Includes the library import item.
            );
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !buttons_rect.contains(pos) && !menu_rect.contains(pos) {
                    ui_state.menu_open = false;
                }
            }
        }
    }

    // Render shortcuts modal if open
    if ui_state.shortcuts_modal_open {
        render_shortcuts_modal(ctx, ui_state);
    }

    // Render save dialog if open
    if ui_state.save_dialog_open {
        if let Some(save_action) = render_save_dialog(ctx, ui_state) {
            action = Some(save_action);
        }
    }

    // Render open dialog if open
    if ui_state.open_dialog_open {
        if let Some(open_action) = render_open_dialog(ctx, ui_state) {
            action = Some(open_action);
        }
    }

    // Render open recent dialog if open
    if ui_state.open_recent_dialog_open {
        if let Some(open_action) = render_open_recent_dialog(ctx, ui_state) {
            action = Some(open_action);
        }
    }

    action
}

/// Hamburger menu button (three horizontal lines).
fn hamburger_button(ui: &mut egui::Ui, is_open: bool) -> bool {
    let size = Vec2::new(32.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg_color = if is_open {
            ui.visuals().selection.bg_fill
        } else if response.hovered() {
            Color32::from_gray(235)
        } else {
            Color32::TRANSPARENT
        };

        let line_color = if is_open {
            Color32::WHITE
        } else {
            Color32::from_gray(80)
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), bg_color);

        // Draw three horizontal lines
        let line_width = 14.0;
        let line_height = 2.0;
        let spacing = 4.0;
        let start_x = rect.center().x - line_width / 2.0;
        let center_y = rect.center().y;

        for i in -1..=1 {
            let y = center_y + (i as f32) * spacing;
            let line_rect = Rect::from_min_size(
                Pos2::new(start_x, y - line_height / 2.0),
                Vec2::new(line_width, line_height),
            );
            ui.painter()
                .rect_filled(line_rect, CornerRadius::same(1), line_color);
        }
    }

    response.on_hover_text("Menu").clicked()
}

/// Collaboration button with connection status indicator.
fn collab_button(ui: &mut egui::Ui, connection_state: ConnectionState, is_open: bool) -> bool {
    let size = Vec2::new(32.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        // Background
        let bg_color = if is_open {
            ui.visuals().selection.bg_fill
        } else if response.hovered() {
            Color32::from_gray(235)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), bg_color);

        // Status indicator color
        let status_color = match connection_state {
            ConnectionState::Connected => Color32::from_rgb(34, 197, 94), // Green
            ConnectionState::Connecting => Color32::from_rgb(250, 204, 21), // Yellow
            ConnectionState::Disconnected => Color32::from_gray(160),     // Gray
            ConnectionState::Error => Color32::from_rgb(239, 68, 68),     // Red
        };

        let icon_color = if is_open {
            Color32::WHITE
        } else {
            Color32::from_gray(80)
        };

        // Draw two person silhouettes (simplified, smaller)
        let center = rect.center();

        // Left person (head + body arc)
        let left_x = center.x - 4.0;
        ui.painter()
            .circle_filled(Pos2::new(left_x, center.y - 3.0), 2.5, icon_color);
        ui.painter()
            .circle_filled(Pos2::new(left_x, center.y + 4.0), 3.5, icon_color);

        // Right person (head + body arc)
        let right_x = center.x + 4.0;
        ui.painter()
            .circle_filled(Pos2::new(right_x, center.y - 3.0), 2.5, icon_color);
        ui.painter()
            .circle_filled(Pos2::new(right_x, center.y + 4.0), 3.5, icon_color);

        // Status dot in top-right corner
        let dot_pos = Pos2::new(rect.right() - 5.0, rect.top() + 5.0);
        ui.painter().circle_filled(dot_pos, 3.5, status_color);
        ui.painter()
            .circle_stroke(dot_pos, 3.5, Stroke::new(1.5, Color32::WHITE));
    }

    let tooltip = match connection_state {
        ConnectionState::Connected => "Connected - Click to manage collaboration",
        ConnectionState::Connecting => "Connecting...",
        ConnectionState::Disconnected => "Not connected - Click to collaborate",
        ConnectionState::Error => "Connection error - Click to retry",
    };
    response.on_hover_text(tooltip).clicked()
}

/// Render the collaboration modal dialog.
fn render_collaboration_modal(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Semi-transparent backdrop
    #[allow(deprecated)]
    let screen_rect = ctx.input(|i| i.content_rect());
    egui::Area::new(egui::Id::new("collab_modal_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(80));
            // Click on backdrop closes modal
            if response.clicked() {
                ui_state.collab_modal_open = false;
            }
        });

    // Modal dialog
    let modal_width = 320.0;

    egui::Area::new(egui::Id::new("collab_modal"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .shadow(egui::epaint::Shadow {
                    spread: 2,
                    blur: 20,
                    offset: [0, 4],
                    color: Color32::from_black_alpha(40),
                })
                .inner_margin(Margin::same(24))
                .show(ui, |ui| {
                    // Apply light theme visuals for this modal
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(245);
                    ui.visuals_mut().widgets.inactive.bg_stroke =
                        Stroke::new(1.0, Color32::from_gray(200));
                    ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_gray(235);
                    ui.visuals_mut().widgets.hovered.bg_stroke =
                        Stroke::new(1.0, Color32::from_gray(180));
                    ui.visuals_mut().widgets.active.bg_fill = Color32::from_gray(225);
                    ui.visuals_mut().widgets.active.bg_stroke =
                        Stroke::new(1.0, ui.visuals().selection.bg_fill);
                    ui.visuals_mut().extreme_bg_color = Color32::WHITE;
                    ui.visuals_mut().override_text_color = Some(Color32::from_gray(30));
                    ui.visuals_mut().selection.bg_fill =
                        ui.visuals().selection.bg_fill.gamma_multiply(0.3);
                    ui.visuals_mut().selection.stroke =
                        Stroke::new(1.0, ui.visuals().selection.bg_fill);

                    ui.set_width(modal_width);

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 12.0);

                        // Header with close button
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Collaborate")
                                    .size(18.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.collab_modal_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);

                        // Connection status
                        let status_color = match ui_state.connection_state {
                            ConnectionState::Connected => Color32::from_rgb(34, 197, 94),
                            ConnectionState::Connecting => Color32::from_rgb(250, 204, 21),
                            ConnectionState::Disconnected => Color32::from_gray(148),
                            ConnectionState::Error => Color32::from_rgb(239, 68, 68),
                        };
                        let status_text = match ui_state.connection_state {
                            ConnectionState::Connected => {
                                if let Some(ref room) = ui_state.current_room {
                                    format!("Connected to room: {}", room)
                                } else {
                                    "Connected (no room)".to_string()
                                }
                            }
                            ConnectionState::Connecting => "Connecting...".to_string(),
                            ConnectionState::Disconnected => "Not connected".to_string(),
                            ConnectionState::Error => "Connection error".to_string(),
                        };

                        ui.horizontal(|ui| {
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(dot_rect.center(), 5.0, status_color);
                            ui.label(
                                egui::RichText::new(&status_text)
                                    .size(13.0)
                                    .color(Color32::from_gray(60)),
                            );
                        });

                        if ui_state.current_room.is_some() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} peer(s) online",
                                    ui_state.peer_count
                                ))
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                            );
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Server URL
                        ui.label(
                            egui::RichText::new("Server URL")
                                .size(12.0)
                                .strong()
                                .color(Color32::from_gray(60)),
                        );
                        input_text(ui, &mut ui_state.server_url, modal_width, "/ws");

                        ui.add_space(4.0);

                        // Connect/Disconnect button (styled)
                        ui.horizontal(|ui| {
                            let button_text = match ui_state.connection_state {
                                ConnectionState::Disconnected | ConnectionState::Error => "Connect",
                                ConnectionState::Connected | ConnectionState::Connecting => {
                                    "Disconnect"
                                }
                            };
                            if primary_btn(ui, button_text) {
                                match ui_state.connection_state {
                                    ConnectionState::Disconnected | ConnectionState::Error => {
                                        action =
                                            Some(UiAction::Connect(ui_state.server_url.clone()));
                                    }
                                    ConnectionState::Connected | ConnectionState::Connecting => {
                                        action = Some(UiAction::Disconnect);
                                    }
                                }
                            }
                        });

                        // Room controls (only when connected)
                        if ui_state.connection_state == ConnectionState::Connected {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            ui.label(
                                egui::RichText::new("Room")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_gray(60)),
                            );
                            input_text(
                                ui,
                                &mut ui_state.room_input,
                                modal_width,
                                "Enter room name",
                            );

                            ui.add_space(4.0);

                            if ui_state.current_room.is_some() {
                                let leave_btn = egui::Button::new(
                                    egui::RichText::new("Leave Room").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(239, 68, 68))
                                .min_size(Vec2::new(modal_width, 36.0))
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(leave_btn).clicked() {
                                    action = Some(UiAction::LeaveRoom);
                                }
                            } else if !ui_state.room_input.is_empty() {
                                let join_btn = egui::Button::new(
                                    egui::RichText::new("Join Room").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(34, 197, 94))
                                .min_size(Vec2::new(modal_width, 36.0))
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(join_btn).clicked() {
                                    action = Some(UiAction::JoinRoom(ui_state.room_input.clone()));
                                }
                            }

                            ui.separator();
                            ui.add_space(4.0);

                            // Your Profile section
                            ui.label(
                                egui::RichText::new("Your Profile")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_gray(60)),
                            );
                            ui.add_space(4.0);

                            // Name input
                            ui.label(
                                egui::RichText::new("Display Name")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            let name_response =
                                input_text(ui, &mut ui_state.user_name, modal_width, "Anonymous");
                            if name_response.lost_focus() {
                                action = Some(UiAction::SetUserName(ui_state.user_name.clone()));
                            }

                            ui.add_space(8.0);

                            // Color picker
                            ui.label(
                                egui::RichText::new("Cursor Color")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                                let colors = [
                                    ("#ef4444", "Red"),
                                    ("#f97316", "Orange"),
                                    ("#eab308", "Yellow"),
                                    ("#22c55e", "Green"),
                                    ("#06b6d4", "Cyan"),
                                    ("#3b82f6", "Blue"),
                                    ("#6366f1", "Indigo"),
                                    ("#a855f7", "Purple"),
                                    ("#ec4899", "Pink"),
                                ];
                                for (hex, name) in colors {
                                    let color = parse_color(hex);
                                    let is_selected = ui_state.user_color == hex;
                                    let size = 24.0;
                                    let (rect, response) = ui.allocate_exact_size(
                                        Vec2::new(size, size),
                                        egui::Sense::click(),
                                    );

                                    // Draw color circle
                                    ui.painter().circle_filled(rect.center(), size / 2.0, color);

                                    // Selection indicator (checkmark or ring)
                                    if is_selected {
                                        ui.painter().circle_stroke(
                                            rect.center(),
                                            size / 2.0 + 2.0,
                                            Stroke::new(2.0, Color32::from_gray(60)),
                                        );
                                    }

                                    // Hover effect
                                    if response.hovered() && !is_selected {
                                        ui.painter().circle_stroke(
                                            rect.center(),
                                            size / 2.0 + 1.0,
                                            Stroke::new(1.0, Color32::from_gray(150)),
                                        );
                                    }

                                    if response.clicked() {
                                        ui_state.user_color = hex.to_string();
                                        action = Some(UiAction::SetUserColor(hex.to_string()));
                                    }
                                    response.on_hover_text(name);
                                }
                            });
                        }
                    });
                });
        });

    action
}

/// Menu item with label and shortcut - uses drafftink_widgets.
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    widgets_menu_item(ui, label, shortcut)
}

/// Menu item with label, shortcut, and enabled state - uses drafftink_widgets.
fn menu_item_enabled(ui: &mut egui::Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    widgets_menu_item_enabled(ui, label, shortcut, enabled)
}

/// Common panel frame style - uses drafftink_widgets.
fn panel_frame() -> Frame {
    widgets_panel_frame()
}

/// Vertical separator for panels.
fn panel_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let line_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + 4.0),
        Vec2::new(1.0, rect.height() - 8.0),
    );
    ui.painter()
        .rect_filled(line_rect, 0.0, Color32::from_gray(220));
    ui.add_space(1.0);
}

/// Color swatch circle for fill colors (supports "none") - uses drafftink_widgets.
fn fill_swatch(ui: &mut egui::Ui, color: Option<Color32>, name: &str, selected: bool) -> bool {
    match color {
        Some(c) => {
            let (clicked, _) = ColorSwatch::new(c, name).selected(selected).show(ui);
            clicked
        }
        None => NoColorSwatch::new(name).selected(selected).show(ui),
    }
}

/// Parse a CSS color string to Color32.
fn parse_color(color: &str) -> Color32 {
    // Handle hex colors
    if let Some(hex) = color.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            return Color32::from_rgb(r, g, b);
        }
    }
    // Default color for unparseable strings
    Color32::from_rgb(100, 116, 139) // slate-500
}

/// Render the presence panel showing connected users.
fn render_presence_panel(ctx: &Context, ui_state: &UiState) {
    // Only show if in a room with peers
    if ui_state.current_room.is_none() || ui_state.peers.is_empty() {
        return;
    }

    let margin = 12.0;

    egui::Area::new(egui::Id::new("presence_panel"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-margin, -margin))
        .interactable(false)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

                // List of peers
                for peer in ui_state.peers.iter().take(8) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                        // Color dot
                        let color = parse_color(&peer.color);
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 4.0, color);

                        // Name
                        let display_name = peer.name.as_deref().unwrap_or("Anonymous");
                        ui.label(
                            egui::RichText::new(display_name)
                                .size(10.0)
                                .color(Color32::from_gray(100)),
                        );
                    });
                }
            });
        });
}

/// Render the keyboard shortcuts modal.
fn render_shortcuts_modal(ctx: &Context, ui_state: &mut UiState) {
    use crate::shortcuts::ShortcutRegistry;

    // Backdrop
    egui::Area::new(egui::Id::new("shortcuts_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.shortcuts_modal_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("shortcuts_modal"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.set_width(500.0);
                ui.vertical(|ui| {
                    // Header
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Keyboard Shortcuts")
                                .size(16.0)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if default_btn(ui, "X") {
                                ui_state.shortcuts_modal_open = false;
                            }
                        });
                    });

                    ui.add_space(12.0);

                    // Shortcuts list
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for shortcut in ShortcutRegistry::all() {
                                let configured_key = match shortcut.description {
                                    "Selection tool" => {
                                        Some(ui_state.settings.shortcut_select.as_str())
                                    }
                                    "Pan tool" => Some(ui_state.settings.shortcut_pan.as_str()),
                                    "Draw tool" => Some(ui_state.settings.shortcut_draw.as_str()),
                                    "Highlighter tool" => {
                                        Some(ui_state.settings.shortcut_highlighter.as_str())
                                    }
                                    "Eraser tool (Classic / Manual)" => {
                                        Some(ui_state.settings.shortcut_eraser.as_str())
                                    }
                                    "Text tool" => Some(ui_state.settings.shortcut_text.as_str()),
                                    "Math formula tool" => {
                                        Some(ui_state.settings.shortcut_math.as_str())
                                    }
                                    "Rectangle tool" => {
                                        Some(ui_state.settings.shortcut_rectangle.as_str())
                                    }
                                    "Ellipse tool" => {
                                        Some(ui_state.settings.shortcut_ellipse.as_str())
                                    }
                                    "Arrow tool" => Some(ui_state.settings.shortcut_arrow.as_str()),
                                    "Line tool" => Some(ui_state.settings.shortcut_line.as_str()),
                                    "Laser pointer" => {
                                        Some(ui_state.settings.shortcut_laser.as_str())
                                    }
                                    _ => None,
                                };
                                let shortcut_text = configured_key
                                    .map(|key| key.to_uppercase())
                                    .unwrap_or_else(|| shortcut.format());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(shortcut_text)
                                            .size(12.0)
                                            .family(egui::FontFamily::Monospace)
                                            .color(Color32::from_rgb(100, 116, 139)),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(shortcut.description)
                                                    .size(12.0)
                                                    .color(Color32::from_gray(200)),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(4.0);
                            }
                        });
                });
            });
        });
}

/// Render the save dialog modal.
fn render_save_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Backdrop
    egui::Area::new(egui::Id::new("save_dialog_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.save_dialog_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("save_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Save Document")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.save_dialog_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("Document name:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );
                        let response = input_text(ui, &mut ui_state.save_name_input, 300.0, "");

                        if response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !ui_state.save_name_input.trim().is_empty()
                        {
                            action = Some(UiAction::SaveLocalWithName(
                                ui_state.save_name_input.trim().to_string(),
                            ));
                            ui_state.save_dialog_open = false;
                        }

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            if secondary_btn(ui, "Cancel") {
                                ui_state.save_dialog_open = false;
                            }
                            if primary_btn(ui, "Save")
                                && !ui_state.save_name_input.trim().is_empty()
                            {
                                action = Some(UiAction::SaveLocalWithName(
                                    ui_state.save_name_input.trim().to_string(),
                                ));
                                ui_state.save_dialog_open = false;
                            }
                        });
                    });
                });
        });

    action
}

/// Render the open dialog modal with dropdown.
fn render_open_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Backdrop
    egui::Area::new(egui::Id::new("open_dialog_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.open_dialog_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("open_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Open Document")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.open_dialog_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("Select document:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );

                        if ui_state.recent_documents.is_empty() {
                            ui.label(
                                egui::RichText::new("No saved documents")
                                    .color(Color32::from_gray(150)),
                            );
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    for doc_name in &ui_state.recent_documents.clone() {
                                        if ui.button(doc_name).clicked() {
                                            action = Some(UiAction::LoadLocal(doc_name.clone()));
                                            ui_state.open_dialog_open = false;
                                        }
                                    }
                                });
                        }
                    });
                });
        });

    action
}

/// Render the open recent dialog modal.
fn render_open_recent_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Backdrop
    egui::Area::new(egui::Id::new("open_recent_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.open_recent_dialog_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("open_recent_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Open Recent")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.open_recent_dialog_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("Select document:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );

                        if ui_state.recent_documents.is_empty() {
                            ui.label(
                                egui::RichText::new("No recent documents")
                                    .color(Color32::from_gray(150)),
                            );
                        } else {
                            ui.add_space(4.0);

                            let selected_text = ui_state
                                .selected_recent_document
                                .as_deref()
                                .unwrap_or("Select a document")
                                .to_string();
                            ui.scope(|ui| {
                                ui.visuals_mut().widgets.inactive.bg_stroke =
                                    Stroke::new(1.0, Color32::from_gray(220));
                                ui.visuals_mut().widgets.hovered.bg_stroke =
                                    Stroke::new(1.0, Color32::from_gray(180));
                                ui.visuals_mut().widgets.active.bg_stroke =
                                    Stroke::new(1.0, ui.visuals().selection.bg_fill);
                                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::WHITE;
                                ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::WHITE;

                                egui::ComboBox::from_id_salt("recent_docs_dropdown")
                                    .selected_text(selected_text)
                                    .width(260.0)
                                    .show_ui(ui, |ui| {
                                        for doc_name in &ui_state.recent_documents.clone() {
                                            ui.selectable_value(
                                                &mut ui_state.selected_recent_document,
                                                Some(doc_name.clone()),
                                                doc_name,
                                            );
                                        }
                                    });
                            });

                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                if primary_btn(ui, "Open") {
                                    if let Some(doc_name) = &ui_state.selected_recent_document {
                                        action = Some(UiAction::LoadLocal(doc_name.clone()));
                                        ui_state.open_recent_dialog_open = false;
                                    }
                                }
                            });
                        }
                    });
                });
        });

    action
}

/// Render persistent application settings.
fn render_settings_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    if !ui_state.settings_open {
        if let Some(previous) = ui_state.settings_before_edit.take() {
            ui_state.settings = previous;
        }
        return None;
    }

    if ui_state.settings_before_edit.is_none() {
        ui_state.settings_before_edit = Some(ui_state.settings.clone());
    }
    let cancel_key = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    let save_key = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let mut action = None;
    let mut close = false;

    egui::Area::new(egui::Id::new("settings_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let rect = ctx.input(|i| i.content_rect());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(65));
        });

    egui::Area::new(egui::Id::new("settings_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgb(252, 252, 253))
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(205)))
                .inner_margin(Margin::same(18))
                .show(ui, |ui| {
                    ui.visuals_mut().override_text_color = Some(Color32::from_gray(30));
                    // The app can use egui's dark widget palette even on this white dialog.
                    // Keep every field/checkbox legible in both normal and hover states.
                    let visuals = ui.visuals_mut();
                    visuals.widgets.inactive.bg_fill = Color32::from_gray(245);
                    visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(245);
                    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_gray(30));
                    visuals.widgets.hovered = visuals.widgets.inactive;
                    visuals.widgets.active = visuals.widgets.inactive;
                    visuals.widgets.hovered.bg_fill = Color32::from_gray(235);
                    visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(235);
                    ui.set_width(520.0);
                    ui.set_max_height(650.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Settings")
                                .size(17.0)
                                .strong()
                                .color(Color32::from_gray(25)),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if default_btn(ui, "X") {
                                    close = true;
                                }
                            },
                        );
                    });
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
                        widgets_section_label(ui, "Police Math par défaut");
                        if let Some(font)=math_font_picker(ui,"default_math_font",&ui_state.settings.default_math_font,&ui_state.local_fonts,&mut ui_state.test_controls) { action=Some(UiAction::SetDefaultMathFont(font)); }
                        ui.add_space(12.0);
                        widgets_section_label(ui, "Police Text par défaut");
                        let label = if ui_state.settings.default_font_postscript == "GoogleSans-Medium" {
                            "Google Sans Medium".to_string()
                        } else { ui_state.settings.default_font.clone() };
                        input_text(ui, &mut ui_state.font_search, 280.0, "Rechercher une police");
                        egui::ComboBox::from_id_salt("default_font_picker")
                            .selected_text(label).width(280.0).height(250.0).show_ui(ui, |ui| {
                                if ui.selectable_label(ui_state.settings.default_font_postscript == "GoogleSans-Medium", "Google Sans Medium").clicked() {
                                    action = Some(UiAction::SetDefaultFont("Google Sans".into(), "GoogleSans-Medium".into()));
                                }
                                for family in FontFamily::all() {
                                    if ui.selectable_label(ui_state.settings.default_font_postscript.is_empty() && ui_state.settings.default_font == family.name(), family.display_name()).clicked() {
                                        action = Some(UiAction::SetDefaultFont(family.name().into(), String::new()));
                                    }
                                }
                                let search = ui_state.font_search.to_lowercase();
                                for (family, postscript) in &ui_state.local_fonts {
                                    if !search.is_empty() && !format!("{} {}", family, postscript).to_lowercase().contains(&search) { continue; }
                                    if ui.selectable_label(ui_state.settings.default_font_postscript == *postscript, format!("{} — {}", family, postscript)).clicked() {
                                        action = Some(UiAction::SetDefaultFont(family.clone(), postscript.clone()));
                                    }
                                }
                            });
                        if ui_state.local_fonts_loading { ui.spinner(); }
                        else if default_btn(ui, "Actualiser mes polices") {
                            ui_state.local_fonts_loading = true;
                            action = Some(UiAction::ScanLocalFonts);
                        }
                        ui.label(egui::RichText::new("La version Windows lit vos polices localement. Aucun fichier de police n’est envoyé à un serveur distant.")
                            .size(11.0).color(Color32::from_gray(100)));
                        if !ui_state.font_error.is_empty() {
                            ui.label(egui::RichText::new(&ui_state.font_error).size(11.0).color(Color32::from_rgb(160,65,25)));
                        }
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Couleur des boutons et sélections").color(Color32::BLACK));
                            let response = ui.color_edit_button_srgb(&mut ui_state.settings.accent_color);
                            ui_state.test_controls.insert("accent_color".into(), [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Contour du curseur souris").color(Color32::BLACK));
                            ui.color_edit_button_srgb(&mut ui_state.settings.cursor_outline);
                        });
                        if default_btn(ui, "Réinitialiser la disposition des panneaux") {
                            action = Some(UiAction::ResetFloatingPanels);
                        }
                        ui.add_space(18.0);
                        widgets_section_label(ui, "Raccourcis des outils");
                        ui.add_space(5.0);
                        let tools = [
                            ("Sélection", ToolKind::Select),
                            ("Pan", ToolKind::Pan),
                            ("Draw", ToolKind::Freehand),
                            ("Highlighter", ToolKind::Highlighter),
                            ("Eraser", ToolKind::Eraser),
                            ("Text", ToolKind::Text),
                            ("Math", ToolKind::Math),
                            ("Rectangle", ToolKind::Rectangle),
                            ("Ellipse", ToolKind::Ellipse),
                            ("Arrow", ToolKind::Arrow),
                            ("Line", ToolKind::Line),
                            ("Laser", ToolKind::LaserPointer),
                        ];
                        egui::Grid::new("settings_shortcuts_grid")
                            .num_columns(2)
                            .spacing(Vec2::new(18.0, 5.0))
                            .show(ui, |ui| {
                                for (label, tool) in tools {
                                    ui.label(
                                        egui::RichText::new(label)
                                            .color(Color32::from_gray(45)),
                                    );
                                    let value = ui_state.settings.shortcut_for_mut(tool);
                                    Frame::new()
                                        .fill(Color32::WHITE)
                                        .stroke(Stroke::new(1.0, Color32::from_gray(190)))
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(5, 2))
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::singleline(value)
                                                    .desired_width(42.0)
                                                    .char_limit(1)
                                                    .text_color(Color32::BLACK)
                                                    .background_color(Color32::WHITE)
                                                    .frame(false),
                                            );
                                        });
                                    ui.end_row();
                                }
                            });

                        ui.add_space(14.0);
                        widgets_section_label(ui, "Outils visibles — barre verticale");
                        ui.horizontal_wrapped(|ui| {
                            for (label, tool) in tools {
                                let mut visible = ui_state.settings.tool_visible_in_toolbar(tool);
                                if ui.checkbox(&mut visible, label).changed() {
                                    ui_state.settings.set_tool_visible_in_toolbar(tool, visible);
                                }
                            }
                        });

                        ui.add_space(14.0);
                        ui.checkbox(&mut ui_state.settings.hide_properties, "Masquer Properties (clic droit sur un objet ; Stroke reste visible)");
                        ui.checkbox(
                            &mut ui_state.settings.show_properties_for_tools,
                            "Afficher Properties pour les outils actifs",
                        );

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(10.0);
                        widgets_section_label(ui, "Document d'introduction");
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            let label = if ui_state.settings.intro_json.trim().is_empty() {
                                "Aucun document".to_string()
                            } else if ui_state.settings.intro_name.is_empty() {
                                "Document configuré".to_string()
                            } else {
                                ui_state.settings.intro_name.clone()
                            };
                            ui.label(
                                egui::RichText::new(label)
                                    .color(Color32::from_gray(55)),
                            );
                            if secondary_btn(ui, "Importer JSON") {
                                action = Some(UiAction::ImportIntroJson);
                            }
                            if !ui_state.settings.intro_json.trim().is_empty()
                                && default_btn(ui, "Effacer")
                            {
                                action = Some(UiAction::ClearIntro);
                            }
                        });
                        ui.label(
                            egui::RichText::new("Vide = aucun document d'introduction.")
                                .size(11.0)
                                .color(Color32::from_gray(120)),
                        );

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(10.0);
                        widgets_section_label(ui, "Export");
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            let folder = if ui_state.settings.export_folder_name.is_empty() {
                                "Téléchargements du navigateur"
                            } else {
                                ui_state.settings.export_folder_name.as_str()
                            };
                            ui.label(
                                egui::RichText::new(folder)
                                    .color(Color32::from_gray(55)),
                            );
                            if secondary_btn(ui, "Choisir le dossier") {
                                action = Some(UiAction::ChooseExportFolder);
                            }
                        });
                        ui.label(
                            egui::RichText::new(
                                "Chrome/Edge : le dossier choisi est mémorisé. Le navigateur peut redemander l'autorisation après un redémarrage.",
                            )
                            .size(11.0)
                            .color(Color32::from_gray(120)),
                        );

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(10.0);
                        widgets_section_label(ui, "Images importées");
                        ui.checkbox(&mut ui_state.settings.optimize_imported_images, "Réduire les grandes images à l’import");
                        ui.add_enabled_ui(ui_state.settings.optimize_imported_images, |ui| {
                            ui.add(egui::Slider::new(&mut ui_state.settings.image_max_side_px, 1024..=4096).step_by(256.0).text("Côté maximal (px)"));
                        });
                        ui.label("Les PNG gardent leur transparence ; JPEG/WebP sont recompressés pour réduire la mémoire.");

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(10.0);
                        widgets_section_label(ui,"Zoom au pavé tactile");
                        ui.add(egui::Slider::new(&mut ui_state.settings.touchpad_zoom_speed,0.25..=8.0).text("Vitesse ×"));
                        ui.label("2× par défaut ; 1× retrouve la vitesse précédente.");
                        ui.separator();
                        widgets_section_label(ui, "Sauvegarde automatique");
                        ui.add_space(5.0);
                        ui.checkbox(
                            &mut ui_state.settings.autosave_enabled,
                            "PNG automatique dans le dossier choisi",
                        );
                        ui.horizontal(|ui| {
                            ui.label("Intervalle");
                            ui.add(
                                egui::DragValue::new(
                                    &mut ui_state.settings.autosave_interval_secs,
                                )
                                .range(1..=3600)
                                .suffix(" s"),
                            );
                        });
                        ui.label("Choisir un dossier d’export autorisé pour l’enregistrement PNG régulier.");
                        ui.label("Ctrl+S : PNG complet ; Ctrl+Shift+S : document JSON.");
                        if !ui_state.save_status.is_empty()
                            && ui_state.save_status_since.is_some_and(|at| at.elapsed().as_secs_f32() < 6.0)
                        {
                            let elapsed = ui_state.save_status_since.unwrap().elapsed().as_secs_f32();
                            let opacity = if elapsed <= 5.0 { 1.0 } else { 1.0 - (elapsed - 5.0) };
                            ui.label(egui::RichText::new(&ui_state.save_status).color(Color32::WHITE.gamma_multiply(opacity)));
                        }
                        ui.checkbox(
                            &mut ui_state.settings.restore_last_document,
                            "Restaurer la dernière feuille au démarrage",
                        );

                        ui.add_space(18.0);
                        ui.horizontal(|ui| {
                            if primary_btn(ui, "Enregistrer") {
                                ui_state.settings.sanitize();
                                action = Some(UiAction::SaveSettings);
                                close = true;
                            }
                            if default_btn(ui, "Fermer") {
                                close = true;
                            }
                        });
                    });
                });
        });

    if save_key && !cancel_key {
        ui_state.settings.sanitize();
        action = Some(UiAction::SaveSettings);
        close = true;
    }
    close |= cancel_key;
    if close {
        if !matches!(action, Some(UiAction::SaveSettings)) {
            if let Some(previous) = ui_state.settings_before_edit.take() {
                ui_state.settings = previous;
            }
        }
        ui_state.settings_before_edit = None;
        ui_state.settings_open = false;
    }

    action
}

/// Render a compact inline formula editor next to the formula on the canvas.
/// The formula itself is updated live, so there is no blocking modal/panel.
fn render_math_editor(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let input_family = if ui_state.math_font.custom.is_some() && ui_state.math_input_font_ready {
        egui::FontFamily::Name("math_medium".into())
    } else if ui_state.math_font.family == FontFamily::GelPen {
        egui::FontFamily::Name("math_gelpen".into())
    } else {
        egui::FontFamily::Proportional
    };
    let screen_rect = ctx.input(|i| i.content_rect());
    let pos = ui_state
        .math_editor_screen_pos
        .unwrap_or(screen_rect.center());
    let x = pos.x.clamp(12.0, (screen_rect.right() - 390.0).max(12.0));
    let y = (pos.y + 28.0).clamp(12.0, (screen_rect.bottom() - 72.0).max(12.0));

    // Read completion keys before TextEdit or egui focus navigation consumes them.
    let finish_requested =
        ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter));
    let mut action = None;
    let mut close = false;

    {
        let editor = ui_state.math_editor.as_mut()?;
        let shape_id = editor.shape_id;

        let output = egui::Area::new(egui::Id::new("math_inline_editor"))
            .fixed_pos(Pos2::new(x, y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgb(250, 250, 252))
                    .corner_radius(CornerRadius::same(8))
                    .stroke(Stroke::new(1.0, Color32::from_gray(205)))
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        let edit_id = egui::Id::new(("math_source", shape_id));
                        // Egui may surrender focus at frame start on Escape. Restore
                        // this active editor before processing any preceding text events.
                        ui.memory_mut(|m| m.request_focus(edit_id));
                        let mut output = Frame::new()
                            .fill(Color32::WHITE)
                            .corner_radius(CornerRadius::same(6))
                            .stroke(Stroke::new(1.0, Color32::from_gray(210)))
                            .inner_margin(Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.visuals_mut().text_cursor.stroke =
                                    Stroke::new(2.0, Color32::BLACK);
                                ui.visuals_mut().text_cursor.blink = true;
                                ui.visuals_mut().text_cursor.on_duration = 0.75;
                                ui.visuals_mut().text_cursor.off_duration = 0.25;
                                egui::TextEdit::singleline(&mut editor.input)
                                    .id(edit_id)
                                    .desired_width(350.0)
                                    .font(egui::FontId::new(14.0, input_family.clone()))
                                    .text_color(Color32::BLACK)
                                    .frame(false)
                                    .hint_text(r"\frac{x+1}{8}")
                                    .show(ui)
                            })
                            .inner;
                        let response = &output.response;
                        response.request_focus();

                        if response.changed() {
                            action = Some(UiAction::PreviewMath(
                                shape_id,
                                editor.input.clone(),
                                editor.input.clone(),
                            ));
                        }

                        if finish_requested {
                            action = Some(UiAction::FinishMath(
                                shape_id,
                                editor.original_source.clone(),
                                editor.original_latex.clone(),
                                editor.is_new,
                                editor.input.clone(),
                                editor.input.clone(),
                            ));
                            close = true;
                        }
                    });
            });
        ui_state.math_editor_rect = Some(output.response.rect);
    }

    if close {
        ui_state.math_editor_rect = None;
        ui_state.math_editor = None;
        ui_state.math_dead_caret = false;
        ui_state.math_editor_screen_pos = None;
    }

    action
}

#[cfg(test)]
mod settings_keyboard_regressions {
    use super::*;
    fn frame(ctx: &Context, state: &mut UiState, key: Option<egui::Key>) -> Option<UiAction> {
        let events = key
            .into_iter()
            .map(|key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect();
        let mut action = None;
        let _ = ctx.run(
            egui::RawInput {
                events,
                focused: true,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 900.0))),
                ..Default::default()
            },
            |ctx| {
                action = render_settings_dialog(ctx, state);
            },
        );
        action
    }
    #[test]
    fn escape_restores_and_enter_saves_settings() {
        let ctx = Context::default();
        let mut state = UiState::default();
        let original = state.settings.accent_color;
        state.settings_open = true;
        frame(&ctx, &mut state, None);
        state.settings.accent_color = [14, 80, 130];
        frame(&ctx, &mut state, Some(egui::Key::Escape));
        assert!(!state.settings_open);
        assert_eq!(state.settings.accent_color, original);
        state.settings_open = true;
        frame(&ctx, &mut state, None);
        state.settings.accent_color = [14, 80, 130];
        assert!(matches!(
            frame(&ctx, &mut state, Some(egui::Key::Enter)),
            Some(UiAction::SaveSettings)
        ));
        assert!(!state.settings_open);
        assert_eq!(state.settings.accent_color, [14, 80, 130]);
    }
}

#[cfg(test)]
mod math_editor_regressions {
    use super::*;
    fn frame(ctx: &Context, state: &mut UiState, events: Vec<egui::Event>) -> Option<UiAction> {
        let input = egui::RawInput {
            events,
            focused: true,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
            ..Default::default()
        };
        let mut action = None;
        let _ = ctx.run(input, |ctx| {
            if let Some(result) = render_math_editor(ctx, state) {
                action = Some(result);
            }
        });
        action
    }
    fn setup(text: &str) -> (Context, UiState) {
        let ctx = Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "math_gelpen".into(),
            egui::FontData::from_static(include_bytes!("../../drafftink-render/assets/GelPen.ttf"))
                .into(),
        );
        fonts.font_data.insert(
            "math_symbols".into(),
            egui::FontData::from_static(include_bytes!(
                "../../drafftink-render/assets/rex-xits.otf"
            ))
            .into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name("math_gelpen".into()),
            vec!["math_gelpen".into(), "math_symbols".into()],
        );
        ctx.set_fonts(fonts);
        let mut state = UiState::default();
        state.math_editor = Some(MathEditorState {
            shape_id: ShapeId::new_v4(),
            input: text.into(),
            original_source: text.into(),
            original_latex: friendly_math_to_latex(text),
            is_new: false,
        });
        frame(&ctx, &mut state, vec![]);
        frame(&ctx, &mut state, vec![]);
        (ctx, state)
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn exponent_then_digit_never_jumps_to_front() {
        let (ctx, mut state) = setup("x");
        frame(&ctx, &mut state, vec![egui::Event::Text("^".into())]);
        frame(&ctx, &mut state, vec![egui::Event::Text("3".into())]);
        assert_eq!(state.math_editor.unwrap().input, "x^3");
    }
    #[test]
    fn markers_replace_selection_at_cursor() {
        let (ctx, mut state) = setup("xyz");
        let id = egui::Id::new(("math_source", state.math_editor.as_ref().unwrap().shape_id));
        let mut edit = egui::TextEdit::load_state(&ctx, id).unwrap();
        edit.cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(1),
                egui::text::CCursor::new(2),
            )));
        edit.store(&ctx, id);
        frame(
            &ctx,
            &mut state,
            vec![egui::Event::Text("_".into()), egui::Event::Text("1".into())],
        );
        assert_eq!(state.math_editor.unwrap().input, "x_1z");
    }
    #[test]
    fn escape_commits_even_with_text_in_same_frame() {
        let (ctx, mut state) = setup("x^");
        let action = frame(
            &ctx,
            &mut state,
            vec![egui::Event::Text("3".into()), key(egui::Key::Escape)],
        );
        assert!(
            matches!(&action, Some(UiAction::FinishMath(_, _, _, _, source, _)) if source == "x^3"),
            "action={action:?}, editor={:?}",
            state.math_editor
        );
        assert!(state.math_editor.is_none());
    }
    #[test]
    fn right_arrow_keeps_raw_latex_and_cursor() {
        let (ctx, mut state) = setup("1/2^3");
        frame(&ctx, &mut state, vec![key(egui::Key::ArrowRight)]);
        frame(&ctx, &mut state, vec![egui::Event::Text("+1".into())]);
        assert_eq!(state.math_editor.unwrap().input, "1/2^3+1");
    }
    #[test]
    fn french_dead_caret_composition_is_not_duplicated() {
        assert_eq!(crate::math_input::dead_caret_text("^3"), "3");
        assert_eq!(crate::math_input::dead_caret_text("3"), "3");
        assert_eq!(crate::math_input::dead_caret_text("â"), "a");
    }
}

#[cfg(test)]
mod floating_panel_regressions {
    use super::*;
    #[test]
    fn initial_toolbar_is_compact_on_narrow_viewport() {
        let ctx = Context::default();
        let state = UiState::default();
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(714.0, 668.0))),
            ..Default::default()
        };
        let mut rect = Rect::NOTHING;
        let _ = ctx.run(raw, |ctx| {
            let output =
                floating_area(ctx, &state, "toolbar", Pos2::new(12.0, 114.0)).show(ctx, |ui| {
                    panel_frame().show(ui, |ui| {
                        panel_grip(ui);
                        ui.allocate_exact_size(Vec2::new(32.0, 420.0), egui::Sense::hover());
                    });
                });
            rect = output.response.rect;
        });
        assert!(rect.width() < 80.0, "toolbar width {}", rect.width());
    }
}

fn render_inline_formula_dialog(ctx: &Context, state: &mut UiState) -> Option<UiAction> {
    let finish_requested =
        ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
    let exit_text = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let draft = state.inline_formula_draft.as_mut()?;
    let mut action = None;
    let mut close = false;
    egui::Window::new("Insérer dans Text")
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .default_pos(ctx.input(|i| i.content_rect()).center() - Vec2::new(180.0, 150.0))
        .frame(
            Frame::new()
                .fill(Color32::from_gray(250))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(12)),
        )
        .show(ctx, |ui| {
            *ui.visuals_mut() = egui::Visuals::light();
            ui.visuals_mut().text_cursor.stroke = Stroke::new(2.0, Color32::BLACK);
            egui::ComboBox::from_id_salt("inline_formula_kind")
                .selected_text(&draft.kind)
                .show_ui(ui, |ui| {
                    for kind in [
                        "Fraction",
                        "Racine",
                        "Racine n-ième",
                        "Somme",
                        "Produit",
                        "Intégrale",
                        "Limite",
                    ] {
                        ui.selectable_value(&mut draft.kind, kind.to_string(), kind);
                    }
                });
            let labels: &[&str] = match draft.kind.as_str() {
                "Fraction" => &["Numérateur", "Dénominateur"],
                "Racine" => &["Expression"],
                "Racine n-ième" => &["Expression", "Indice de la racine"],
                "Limite" => &["Expression", "Variable", "Vers"],
                _ => &[
                    "Expression",
                    "Variable",
                    "Borne inférieure",
                    "Borne supérieure",
                ],
            };
            for (i, label) in labels.iter().enumerate() {
                ui.label(*label);
                let id = egui::Id::new(("inline_formula_part", draft.text_id, i));
                if (finish_requested || draft.request_focus) && draft.active_field == i {
                    ui.memory_mut(|m| m.request_focus(id));
                }
                let response = ui.add(
                    egui::TextEdit::singleline(&mut draft.parts[i])
                        .id(id)
                        .desired_width(320.0),
                );
                if response.has_focus() {
                    draft.active_field = i;
                }
                state.test_controls.insert(
                    label.to_string(),
                    [
                        response.rect.min.x,
                        response.rect.min.y,
                        response.rect.max.x,
                        response.rect.max.y,
                    ],
                );
            }
            draft.request_focus = false;
            ui.label("Fractions imbriquées : (a/b)/(c/d) · sqrt(x) · x^2");
            if !state.inline_formula_error.is_empty() {
                ui.colored_label(Color32::RED, &state.inline_formula_error);
            }
            ui.horizontal(|ui| {
                let response = ui.button("Insérer / mettre à jour");
                state.test_controls.insert(
                    "Insérer".into(),
                    [
                        response.rect.min.x,
                        response.rect.min.y,
                        response.rect.max.x,
                        response.rect.max.y,
                    ],
                );
                if response.clicked() || finish_requested {
                    let parts = draft
                        .parts
                        .each_ref()
                        .map(|p| crate::math_input::friendly_math_to_latex(p));
                    let latex = match draft.kind.as_str() {
                        "Fraction" => format!(r"\frac{{{}}}{{{}}}", parts[0], parts[1]),
                        "Racine" => format!(r"\sqrt{{{}}}", parts[0]),
                        "Racine n-ième" => format!(r"\sqrt[{}]{{{}}}", parts[1], parts[0]),
                        "Somme" => format!(
                            r"\sum_{{{}={}}}^{{{}}} {}",
                            parts[1], parts[2], parts[3], parts[0]
                        ),
                        "Produit" => format!(
                            r"\prod_{{{}={}}}^{{{}}} {}",
                            parts[1], parts[2], parts[3], parts[0]
                        ),
                        "Intégrale" => format!(
                            r"\int_{{{}}}^{{{}}} {}\,d{}",
                            parts[2], parts[3], parts[0], parts[1]
                        ),
                        _ => format!(r"\lim_{{{}\to {}}} {}", parts[1], parts[2], parts[0]),
                    };
                    action = Some(UiAction::CommitInlineFormula(
                        latex,
                        draft.kind.clone(),
                        draft.parts.clone(),
                        exit_text,
                    ));
                }
                if ui.button("Fermer").clicked() {
                    close = true;
                }
            });
        });
    if close {
        state.inline_formula_draft = None;
    }
    action
}

#[cfg(test)]
mod inline_escape_tests {
    use super::*;
    #[test]
    fn escape_keeps_latest_field_input_and_requests_text_commit() {
        let ctx = egui::Context::default();
        let mut state = UiState::default();
        state.inline_formula_draft = Some(InlineFormulaDraft {
            text_id: ShapeId::new_v4(),
            range: 0..0,
            kind: "Fraction".into(),
            parts: [String::new(), "2".into(), String::new(), String::new()],
            active_field: 0,
            request_focus: true,
        });
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            render_inline_formula_dialog(ctx, &mut state);
        });
        let input = egui::RawInput {
            events: vec![
                egui::Event::Text("3".into()),
                egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut action = None;
        let _ = ctx.run(input, |ctx| {
            action = render_inline_formula_dialog(ctx, &mut state);
        });
        assert!(
            matches!(action,Some(UiAction::CommitInlineFormula(ref latex,_,_,true)) if latex==r"\frac{3}{2}")
        );
    }
}

fn render_text_command_editor(ctx: &Context, state: &mut UiState) -> Option<UiAction> {
    let finish = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
    let exit = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let editor = state.text_command_editor.as_mut()?;
    let mut action = None;
    let window = egui::Window::new("Formule dans Text")
        .title_bar(false)
        .id(egui::Id::new(("text_command_window", editor.formula_id)))
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .current_pos(state.text_command_pos)
        .constrain(false)
        .movable(false)
        .frame(
            Frame::new()
                .fill(Color32::from_gray(250))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10)),
        )
        .show(ctx, |ui| {
            *ui.visuals_mut() = egui::Visuals::light();
            ui.visuals_mut().text_cursor.stroke = Stroke::new(2.0, Color32::BLACK);
            let id = egui::Id::new(("text_command_source", editor.formula_id));
            if editor.request_focus || finish {
                ui.memory_mut(|m| m.request_focus(id));
            }
            let mut output = egui::TextEdit::singleline(&mut editor.source)
                .id(id)
                .desired_width(340.0)
                .show(ui);
            output.response.request_focus();
            if editor.request_focus {
                output
                    .state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(editor.source.chars().count()),
                    )));
                output.state.store(ctx, id);
                editor.request_focus = false;
            }
            let rect = output.response.rect;
            state.test_controls.insert(
                "Code formule".into(),
                [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
            );
            if let Some(example) = crate::math_input::command_example(&editor.source) {
                ui.label(
                    egui::RichText::new(example)
                        .size(10.0)
                        .color(Color32::from_gray(110)),
                );
            }
            if output.response.changed() || finish {
                action = Some(UiAction::EditTextCommand(
                    editor.source.clone(),
                    finish,
                    exit,
                ));
            }
            if finish {
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        });
    state.text_command_rect = window.map(|out| out.response.rect);

    action
}

fn math_font_picker(
    ui: &mut egui::Ui,
    id: &str,
    current: &TextFont,
    local: &[(String, String)],
    controls: &mut std::collections::BTreeMap<String, [f32; 4]>,
) -> Option<TextFont> {
    let mut picked = None;
    ui.label("Font Family");
    let label = current
        .custom
        .clone()
        .unwrap_or_else(|| current.family.display_name().into());
    let combo = egui::ComboBox::from_id_salt(id)
        .selected_text(label)
        .width(200.0)
        .show_ui(ui, |ui| {
            for family in FontFamily::all() {
                let response = ui.selectable_label(
                    current.custom.is_none() && current.family == *family,
                    family.display_name(),
                );
                controls.insert(
                    format!("{}:{}", id, family.name()),
                    [
                        response.rect.min.x,
                        response.rect.min.y,
                        response.rect.max.x,
                        response.rect.max.y,
                    ],
                );
                if response.clicked() {
                    picked = Some(TextFont::from_name(family.name(), ""));
                }
            }
            ui.separator();
            for (family, ps) in local {
                if ui
                    .selectable_label(
                        current.postscript.as_ref() == Some(ps),
                        format!("{} — {}", family, ps),
                    )
                    .clicked()
                {
                    picked = Some(TextFont::from_name(family, ps));
                }
            }
        });
    controls.insert(
        id.into(),
        [
            combo.response.rect.min.x,
            combo.response.rect.min.y,
            combo.response.rect.max.x,
            combo.response.rect.max.y,
        ],
    );
    picked
}
