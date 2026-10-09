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

#[cfg(target_arch = "wasm32")]
use crate::app::file_ops;
use crate::math_input::{
    friendly_math_to_latex, normalize_friendly_math_input, open_structured_depth,
};
use crate::settings::UserSettings;

// Re-export from widgets crate for consistent styling
use drafftink_widgets::{
    ColorGrid, ColorSwatch, ColorSwatchWithWheel, FontSizeButton, IconButton, IconButtonStyle, NoColorSwatch,
    StrokeWidthButton, TAILWIND_COLORS, ToggleButton, default_btn, input_text,
    menu_item as widgets_menu_item, menu_item_enabled as widgets_menu_item_enabled,
    menu_separator as widgets_menu_separator, panel_frame as widgets_panel_frame, primary_btn,
    secondary_btn, section_label as widgets_section_label,
    vertical_separator as widgets_vertical_separator,
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
            path_style: 0,      // Direct by default
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
                    ui.label(egui::RichText::new(&ui_state.save_status).color(Color32::BLACK.gamma_multiply(opacity)));
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
        .or(tab_action)
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
        "toolbar" => Vec2::new(58.0, 530.0),
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
        Pos2::new(12.0, (screen.height() - 530.0).max(24.0) / 2.0),
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
            });
        });
    });

    remember_panel(ui_state, "toolbar", &output.response);
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
            !ui_state.laser_color_rect.is_some_and(|rect| rect.contains(pos))
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
            pa