//! Text editing state using Parley's PlainEditor.

use parley::editing::{Generation, PlainEditor, PlainEditorDriver};
use parley::{FontContext, LayoutContext, StyleProperty};
use peniko::Brush;
use std::time::Duration;

// Use web_time for WASM compatibility
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

/// Keyboard key for text editing.
#[derive(Debug, Clone, PartialEq)]
pub enum TextKey {
    Character(String),
    /// Literal French dead-key caret, committed so expanders can erase it.
    DeadCaret,
    /// Browser/native key text, plus whether the physical caret key was pressed.
    ComposedCharacter(String, bool),
    Backspace,
    Delete,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
    Copy,
    Cut,
    Paste(String),
    ToggleSuperscript,
    ToggleSubscript,
}

/// Keyboard modifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl TextModifiers {
    /// Get the action modifier (Ctrl on Windows/Linux, Cmd on macOS).
    pub fn action_mod(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.meta
        } else {
            self.ctrl
        }
    }
}

/// Result of handling a text editing event.
#[derive(Debug, Clone, PartialEq)]
pub enum TextEditResult {
    /// Event was handled, text may have changed.
    Handled,
    /// Event was handled, user wants to exit editing.
    ExitEdit,
    /// Event was not handled (pass to other handlers).
    NotHandled,
    /// Copy requested - contains the selected text to copy.
    Copy(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptMode {
    Normal,
    Superscript,
    Subscript,
}

fn superscript_char(ch: char) -> char {
    match ch {
        '0' => '\u{2070}',
        '1' => '\u{00B9}',
        '2' => '\u{00B2}',
        '3' => '\u{00B3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        '+' => '\u{207A}',
        '-' => '\u{207B}',
        '=' => '\u{207C}',
        '(' => '\u{207D}',
        ')' => '\u{207E}',
        'n' | 'N' => '\u{207F}',
        'i' | 'I' => '\u{2071}',
        'x' | 'X' => '\u{02E3}',
        other => other,
    }
}

fn subscript_char(ch: char) -> char {
    match ch {
        '0' => '\u{2080}',
        '1' => '\u{2081}',
        '2' => '\u{2082}',
        '3' => '\u{2083}',
        '4' => '\u{2084}',
        '5' => '\u{2085}',
        '6' => '\u{2086}',
        '7' => '\u{2087}',
        '8' => '\u{2088}',
        '9' => '\u{2089}',
        '+' => '\u{208A}',
        '-' => '\u{208B}',
        '=' => '\u{208C}',
        '(' => '\u{208D}',
        ')' => '\u{208E}',
        'n' | 'N' => '\u{2099}',
        'i' | 'I' => '\u{1D62}',
        'x' | 'X' => '\u{2093}',
        other => other,
    }
}

fn convert_script_text(text: &str, mode: ScriptMode) -> String {
    text.chars()
        .map(|ch| match mode {
            ScriptMode::Normal => ch,
            ScriptMode::Superscript => superscript_char(ch),
            ScriptMode::Subscript => subscript_char(ch),
        })
        .collect()
}

/// Text editor state for a single text shape being edited.
pub struct TextEditState {
    /// The Parley PlainEditor for handling text editing.
    editor: PlainEditor<Brush>,
    /// Whether the cursor is currently visible (for blinking).
    cursor_visible: bool,
    /// Start time for cursor blinking.
    start_time: Option<Instant>,
    /// Blink period.
    blink_period: Duration,
    /// Whether a mouse drag is in progress for selection.
    is_dragging: bool,
    /// Cached layout width for bounds calculation.
    cached_width: f32,
    /// Cached layout height for bounds calculation.
    cached_height: f32,
    script_mode: ScriptMode,
    rich_layout: Option<parley::Layout<Brush>>,
    pending_dead_caret: bool,
    font_size: f32,
}

impl TextEditState {
    /// Create a new text edit state with the given text content.
    pub fn new(text: &str, font_size: f32) -> Self {
        let mut editor = PlainEditor::new(font_size);
        editor.set_text(text);
        editor.set_scale(1.0);

        // Set default styles - use SansSerif generic family
        // The renderer will set the specific font (GelPen) via styles
        let styles = editor.edit_styles();
        styles.insert(StyleProperty::FontStack(parley::FontStack::List(
            vec![
                parley::FontFamily::Named("Noto Sans".into()),
                parley::FontFamily::Named("STIX Two Math".into()),
            ]
            .into(),
        )));
        styles.insert(StyleProperty::Brush(Brush::Solid(peniko::Color::BLACK)));

        Self {
            editor,
            cursor_visible: true,
            start_time: None,
            blink_period: Duration::ZERO,
            is_dragging: false,
            cached_width: 0.0,
            cached_height: 0.0,
            script_mode: ScriptMode::Normal,
            rich_layout: None,
            pending_dead_caret: false,
            font_size,
        }
    }

    pub fn set_rich_layout(&mut self, layout: parley::Layout<Brush>) {
        self.rich_layout = Some(layout);
    }
    pub fn selection_geometry_with(&self, mut f: impl FnMut(parley::BoundingBox, usize)) {
        if let Some(layout) = &self.rich_layout {
            let selection = self.editor.raw_selection().refresh(layout);
            selection.geometry_with(layout, |mut rect, line_index| {
                if let Some(line) = layout.lines().nth(line_index) {
                    let shift = inline_baseline_shift(&line, self.font_size) as f64;
                    rect.y0 -= shift;
                    rect.y1 -= shift;
                }
                f(rect, line_index);
            });
            let range = selection.text_range();
            for (line_index, line) in layout.lines().enumerate() {
                for item in line.items() {
                    if let parley::PositionedLayoutItem::InlineBox(b) = item {
                        if layout.inline_boxes().iter().any(|raw| {
                            raw.id == b.id && range.start <= raw.index && range.end >= raw.index + 3
                        }) {
                            f(
                                parley::BoundingBox::new(
                                    b.x as f64,
                                    b.y as f64,
                                    (b.x + b.width) as f64,
                                    (b.y + b.height) as f64,
                                ),
                                line_index,
                            );
                        }
                    }
                }
            }
        } else {
            self.editor.selection_geometry_with(f);
        }
    }
    pub fn cursor_geometry(&self, size: f32) -> Option<parley::BoundingBox> {
        if let Some(layout) = &self.rich_layout {
            let cursor = self.editor.raw_selection().focus().refresh(layout);
            for line in layout.lines() {
                for item in line.items() {
                    if let parley::PositionedLayoutItem::InlineBox(b) = item {
                        if layout
                            .inline_boxes()
                            .iter()
                            .any(|raw| raw.id == b.id && raw.index == cursor.index())
                        {
                            return Some(parley::BoundingBox::new(
                                b.x as f64,
                                b.y as f64,
                                (b.x + size) as f64,
                                (b.y + b.height) as f64,
                            ));
                        }
                    }
                }
            }
            let mut rect = cursor.geometry(layout, size);
            if rect.height() < 1.0 {
                rect.y0 = rect.y1 - self.font_size as f64 * 1.2;
            }
            if let Some(line) = layout.lines().find(|l| {
                l.text_range().contains(&cursor.index()) || l.text_range().end == cursor.index()
            }) {
                let shift = inline_baseline_shift(&line, self.font_size) as f64;
                rect.y0 -= shift;
                rect.y1 -= shift;
            }
            Some(rect)
        } else {
            self.editor.cursor_geometry(size)
        }
    }

    fn inline_at(&self, x: f32, y: f32) -> Option<(usize, bool)> {
        let layout = self.rich_layout.as_ref()?;
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::InlineBox(b) = item {
                    if x >= b.x && x <= b.x + b.width && y >= b.y && y <= b.y + b.height {
                        if let Some(raw) = layout.inline_boxes().iter().find(|raw| raw.id == b.id) {
                            return Some((raw.index, x > b.x + b.width / 2.0));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn formula_byte_at(&self, x: f32, y: f32) -> Option<usize> {
        self.inline_at(x, y).map(|(byte, _)| byte)
    }
    pub fn formula_bounds(&self, byte: usize) -> Option<kurbo::Rect> {
        let layout = self.rich_layout.as_ref()?;
        let id = layout.inline_boxes().iter().find(|b| b.index == byte)?.id;
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::InlineBox(b) = item {
                    if b.id == id {
                        return Some(kurbo::Rect::new(
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
    /// Get a mutable reference to the PlainEditor.
    pub fn editor_mut(&mut self) -> &mut PlainEditor<Brush> {
        &mut self.editor
    }

    /// Get a reference to the PlainEditor.
    pub fn editor(&self) -> &PlainEditor<Brush> {
        &self.editor
    }

    /// Create a driver for performing edit operations.
    pub fn driver<'a>(
        &'a mut self,
        font_cx: &'a mut FontContext,
        layout_cx: &'a mut LayoutContext<Brush>,
    ) -> PlainEditorDriver<'a, Brush> {
        self.editor.driver(font_cx, layout_cx)
    }

    /// Get the current text content.
    pub fn text(&self) -> String {
        self.editor.text().to_string()
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: &str) {
        self.editor.set_text(text);
    }

    /// Set the text brush color.
    pub fn set_brush(&mut self, brush: Brush) {
        let styles = self.editor.edit_styles();
        styles.insert(StyleProperty::Brush(brush));
    }

    /// Set the font size.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
        let styles = self.editor.edit_styles();
        styles.insert(StyleProperty::FontSize(size));
    }

    /// Get the current selection range as byte offsets.
    /// Returns None if selection is collapsed (just a cursor).
    pub fn selection_range(&self) -> Option<std::ops::Range<usize>> {
        let range = self.editor.raw_selection().text_range();
        if range.start == range.end {
            None
        } else {
            Some(range)
        }
    }

    /// Get the cursor byte offset (start of selection or cursor position).
    pub fn cursor_byte_offset(&self) -> usize {
        self.editor.raw_selection().text_range().start
    }

    /// Set the text width constraint.
    pub fn set_width(&mut self, width: Option<f32>) {
        self.editor.set_width(width);
    }

    /// Reset cursor to visible state and start blinking.
    pub fn cursor_reset(&mut self) {
        self.start_time = Some(Instant::now());
        self.blink_period = Duration::from_millis(500);
        self.cursor_visible = true;
    }

    /// Disable cursor blinking.
    pub fn disable_blink(&mut self) {
        self.start_time = None;
    }

    /// Calculate the next blink time.
    pub fn next_blink_time(&self) -> Option<Instant> {
        self.start_time.map(|start_time| {
            let phase = Instant::now().duration_since(start_time);
            start_time
                + Duration::from_nanos(
                    ((phase.as_nanos() / self.blink_period.as_nanos() + 1)
                        * self.blink_period.as_nanos()) as u64,
                )
        })
    }

    /// Update cursor visibility based on blink state.
    pub fn cursor_blink(&mut self) {
        self.cursor_visible = self.start_time.is_some_and(|start_time| {
            let elapsed = Instant::now().duration_since(start_time);
            (elapsed.as_millis() / self.blink_period.as_millis()) % 2 == 0
        });
    }

    /// Check if cursor should be visible.
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Get the current generation (for change detection).
    pub fn generation(&self) -> Generation {
        self.editor.generation()
    }

    /// Check if the editor is composing (IME active).
    pub fn is_composing(&self) -> bool {
        self.editor.is_composing()
    }

    /// Get cached layout dimensions.
    pub fn layout_size(&self) -> (f32, f32) {
        (self.cached_width, self.cached_height)
    }

    /// Update cached layout dimensions from the current layout.
    pub fn update_layout_cache(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) {
        let layout = self.editor.layout(font_cx, layout_cx);
        self.cached_width = layout.width();
        self.cached_height = layout.height();
    }

    /// Handle a key press event.
    /// Returns whether the event was handled and if editing should exit.
    #[allow(clippy::drop_non_drop)]
    pub fn handle_key(
        &mut self,
        mut key: TextKey,
        modifiers: TextModifiers,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) -> TextEditResult {
        // Don't process keys while composing (IME)
        if self.editor.is_composing() {
            return TextEditResult::NotHandled;
        }

        key = match key {
            TextKey::DeadCaret => {
                self.pending_dead_caret = true;
                TextKey::Character("^".into())
            }
            TextKey::ComposedCharacter(mut value, physical_caret) => {
                if std::mem::take(&mut self.pending_dead_caret) {
                    let accent = value
                        .chars()
                        .next()
                        .is_some_and(|c| "âêîôûŷÂÊÎÔÛŶ".contains(c))
                        || value.contains('\u{0302}');
                    if accent && self.editor.raw_selection().is_collapsed() {
                        let cursor = self.editor.raw_selection().focus().index();
                        if self.editor.text().to_string()[..cursor].ends_with('^') {
                            self.editor.driver(font_cx, layout_cx).backdelete();
                        }
                    } else if value.starts_with('^') && !(physical_caret && value == "^") {
                        value.remove(0);
                    }
                }
                TextKey::Character(value)
            }
            other => {
                self.pending_dead_caret = false;
                other
            }
        };
        self.cursor_reset();
        let action_mod = modifiers.action_mod();
        let shift = modifiers.shift;

        if let Some(layout) = &self.rich_layout {
            let selection = self.editor.raw_selection().refresh(layout);
            let next = match key {
                TextKey::Left if !action_mod => Some(selection.previous_visual(layout, shift)),
                TextKey::Right if !action_mod && self.script_mode == ScriptMode::Normal => {
                    Some(selection.next_visual(layout, shift))
                }
                TextKey::Up => Some(selection.previous_line(layout, shift)),
                TextKey::Down => Some(selection.next_line(layout, shift)),
                TextKey::Home if !action_mod => Some(selection.line_start(layout, shift)),
                TextKey::End if !action_mod => Some(selection.line_end(layout, shift)),
                _ => None,
            };
            if let Some(next) = next {
                self.editor
                    .driver(font_cx, layout_cx)
                    .select_byte_range(next.anchor().index(), next.focus().index());
                return TextEditResult::Handled;
            }
        }
        self.rich_layout = None;
        let mut drv = self.editor.driver(font_cx, layout_cx);

        match key {
            TextKey::DeadCaret | TextKey::ComposedCharacter(_, _) => {
                unreachable!("input normalized before editing")
            }
            TextKey::Escape => {
                self.script_mode = ScriptMode::Normal;
                return TextEditResult::ExitEdit;
            }
            TextKey::Backspace => {
                if action_mod {
                    drv.backdelete_word();
                } else {
                    drv.backdelete();
                }
            }
            TextKey::Delete => {
                if action_mod {
                    drv.delete_word();
                } else {
                    drv.delete();
                }
            }
            TextKey::Enter => {
                drv.insert_or_replace_selection("\n");
            }
            TextKey::Left => {
                if action_mod {
                    if shift {
                        drv.select_word_left();
                    } else {
                        drv.move_word_left();
                    }
                } else if shift {
                    drv.select_left();
                } else {
                    drv.move_left();
                }
            }
            TextKey::Right => {
                if self.script_mode != ScriptMode::Normal && !action_mod && !shift {
                    self.script_mode = ScriptMode::Normal;
                    drop(drv);
                    self.update_layout_cache(font_cx, layout_cx);
                    return TextEditResult::Handled;
                }
                if action_mod {
                    if shift {
                        drv.select_word_right();
                    } else {
                        drv.move_word_right();
                    }
                } else if shift {
                    drv.select_right();
                } else {
                    drv.move_right();
                }
            }
            TextKey::Up => {
                if shift {
                    drv.select_up();
                } else {
                    drv.move_up();
                }
            }
            TextKey::Down => {
                if shift {
                    drv.select_down();
                } else {
                    drv.move_down();
                }
            }
            TextKey::Home => {
                if action_mod {
                    if shift {
                        drv.select_to_text_start();
                    } else {
                        drv.move_to_text_start();
                    }
                } else if shift {
                    drv.select_to_line_start();
                } else {
                    drv.move_to_line_start();
                }
            }
            TextKey::End => {
                if action_mod {
                    if shift {
                        drv.select_to_text_end();
                    } else {
                        drv.move_to_text_end();
                    }
                } else if shift {
                    drv.select_to_line_end();
                } else {
                    drv.move_to_line_end();
                }
            }
            TextKey::Copy => {
                // Return selected text for clipboard
                drop(drv);
                if let Some(text) = self.editor.selected_text() {
                    return TextEditResult::Copy(text.to_string());
                }
                return TextEditResult::Handled;
            }
            TextKey::Cut => {
                // Get selected text first, then delete
                drop(drv);
                let text_to_copy = self.editor.selected_text().map(|s| s.to_string());
                if let Some(text) = text_to_copy {
                    let mut drv = self.editor.driver(font_cx, layout_cx);
                    drv.delete();
                    drop(drv);
                    self.update_layout_cache(font_cx, layout_cx);
                    return TextEditResult::Copy(text);
                }
                return TextEditResult::Handled;
            }
            TextKey::Paste(ref text) => {
                drv.insert_or_replace_selection(text);
            }
            TextKey::ToggleSuperscript => {
                drop(drv);
                if let Some(selected) = self.editor.selected_text().map(str::to_string) {
                    let converted = convert_script_text(&selected, ScriptMode::Superscript);
                    let mut drv = self.editor.driver(font_cx, layout_cx);
                    drv.insert_or_replace_selection(&converted);
                    drop(drv);
                    self.script_mode = ScriptMode::Normal;
                } else {
                    self.script_mode = if self.script_mode == ScriptMode::Superscript {
                        ScriptMode::Normal
                    } else {
                        ScriptMode::Superscript
                    };
                }
                self.update_layout_cache(font_cx, layout_cx);
                return TextEditResult::Handled;
            }
            TextKey::ToggleSubscript => {
                drop(drv);
                if let Some(selected) = self.editor.selected_text().map(str::to_string) {
                    let converted = convert_script_text(&selected, ScriptMode::Subscript);
                    let mut drv = self.editor.driver(font_cx, layout_cx);
                    drv.insert_or_replace_selection(&converted);
                    drop(drv);
                    self.script_mode = ScriptMode::Normal;
                } else {
                    self.script_mode = if self.script_mode == ScriptMode::Subscript {
                        ScriptMode::Normal
                    } else {
                        ScriptMode::Subscript
                    };
                }
                self.update_layout_cache(font_cx, layout_cx);
                return TextEditResult::Handled;
            }
            TextKey::Character(ref c) => {
                // Handle Ctrl+A for select all
                if action_mod && (c == "a" || c == "A") {
                    if shift {
                        drv.collapse_selection();
                    } else {
                        drv.select_all();
                    }
                } else if !action_mod {
                    if self.script_mode != ScriptMode::Normal {
                        if c == " " {
                            self.script_mode = ScriptMode::Normal;
                            drv.insert_or_replace_selection(" ");
                        } else {
                            let converted = convert_script_text(c, self.script_mode);
                            drv.insert_or_replace_selection(&converted);
                        }
                    } else {
                        drv.insert_or_replace_selection(c);
                    }
                }
            }
        }

        // Update layout cache after changes
        drop(drv);
        self.update_layout_cache(font_cx, layout_cx);

        TextEditResult::Handled
    }

    /// Handle mouse press at the given local coordinates (relative to text position).
    pub fn handle_mouse_down(
        &mut self,
        local_x: f32,
        local_y: f32,
        shift: bool,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) {
        self.cursor_reset();
        self.is_dragging = true;
        self.pending_dead_caret = false;
        if let Some((start, after)) = self.inline_at(local_x, local_y) {
            let byte = start + if after { 3 } else { 0 };
            let anchor = if shift {
                self.editor.raw_selection().anchor().index()
            } else {
                byte
            };
            self.editor
                .driver(font_cx, layout_cx)
                .select_byte_range(anchor, byte);
            return;
        }

        if let Some(layout) = &self.rich_layout {
            let local_y = local_y
                + layout
                    .lines()
                    .find(|l| local_y >= l.metrics().min_coord && local_y <= l.metrics().max_coord)
                    .map(|l| inline_baseline_shift(&l, self.font_size))
                    .unwrap_or(0.0);
            let next = if shift {
                self.editor
                    .raw_selection()
                    .refresh(layout)
                    .extend_to_point(layout, local_x, local_y)
            } else {
                parley::editing::Selection::from_point(layout, local_x, local_y)
            };
            self.editor
                .driver(font_cx, layout_cx)
                .select_byte_range(next.anchor().index(), next.focus().index());
            return;
        }
        let mut drv = self.editor.driver(font_cx, layout_cx);
        if shift {
            drv.extend_selection_to_point(local_x, local_y);
        } else {
            drv.move_to_point(local_x, local_y);
        }
    }

    /// Handle mouse drag at the given local coordinates.
    pub fn handle_mouse_drag(
        &mut self,
        local_x: f32,
        local_y: f32,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) {
        if !self.is_dragging {
            return;
        }

        self.cursor_reset();
        if let Some(layout) = &self.rich_layout {
            let next = self
                .editor
                .raw_selection()
                .refresh(layout)
                .extend_to_point(layout, local_x, local_y);
            self.editor
                .driver(font_cx, layout_cx)
                .select_byte_range(next.anchor().index(), next.focus().index());
            return;
        }
        let mut drv = self.editor.driver(font_cx, layout_cx);
        drv.extend_selection_to_point(local_x, local_y);
    }

    /// Handle mouse release.
    pub fn handle_mouse_up(&mut self) {
        self.is_dragging = false;
    }

    /// Check if a drag is in progress.
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    /// Handle double-click to select word.
    pub fn handle_double_click(
        &mut self,
        local_x: f32,
        local_y: f32,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) {
        self.cursor_reset();
        self.pending_dead_caret = false;
        if let Some((start, _)) = self.inline_at(local_x, local_y) {
            self.editor
                .driver(font_cx, layout_cx)
                .select_byte_range(start, start + 3);
            return;
        }
        if let Some(layout) = &self.rich_layout {
            let next = parley::editing::Selection::word_from_point(layout, local_x, local_y);
            self.editor
                .driver(font_cx, layout_cx)
                .select_byte_range(next.anchor().index(), next.focus().index());
            return;
        }
        let mut drv = self.editor.driver(font_cx, layout_cx);
        drv.select_word_at_point(local_x, local_y);
    }

    /// Handle triple-click to select line.
    pub fn handle_triple_click(
        &mut self,
        local_x: f32,
        local_y: f32,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) {
        self.cursor_reset();
        if let Some(layout) = &self.rich_layout {
            let next = parley::editing::Selection::hard_line_from_point(layout, local_x, local_y);
            self.editor
                .driver(font_cx, layout_cx)
                .select_byte_range(next.anchor().index(), next.focus().index());
            return;
        }
        let mut drv = self.editor.driver(font_cx, layout_cx);
        drv.select_hard_line_at_point(local_x, local_y);
    }
}

impl Default for TextEditState {
    fn default() -> Self {
        Self::new("", 32.0)
    }
}

#[cfg(test)]
mod escape_regression {
    use super::*;
    #[test]
    fn escape_keeps_the_current_text() {
        let mut fonts = FontContext::new();
        let mut layouts = LayoutContext::new();
        let mut editor = TextEditState::new("Bonjour", 24.0);
        editor.handle_key(
            TextKey::End,
            TextModifiers::default(),
            &mut fonts,
            &mut layouts,
        );
        editor.handle_key(
            TextKey::Character(" monde".into()),
            TextModifiers::default(),
            &mut fonts,
            &mut layouts,
        );
        assert_eq!(
            editor.handle_key(
                TextKey::Escape,
                TextModifiers::default(),
                &mut fonts,
                &mut layouts
            ),
            TextEditResult::ExitEdit
        );
        assert_eq!(editor.text(), "Bonjour monde");
    }
}

#[cfg(test)]
mod expander_tests {
    use super::*;
    #[test]
    fn literal_caret_and_unicode_replacements_do_not_delete_prefix() {
        let mut fonts = FontContext::new();
        fonts.collection.register_fonts(
            peniko::Blob::new(std::sync::Arc::new(
                include_bytes!("../assets/NotoSans-Regular.ttf").as_slice(),
            )),
            None,
        );
        let mut layouts = LayoutContext::new();
        let mut editor = TextEditState::new("123", 20.0);
        editor
            .editor_mut()
            .edit_styles()
            .insert(parley::StyleProperty::FontStack(parley::FontStack::Single(
                parley::FontFamily::Named("Noto Sans".into()),
            )));
        editor.handle_key(
            TextKey::End,
            TextModifiers::default(),
            &mut fonts,
            &mut layouts,
        );
        for key in [
            TextKey::Character("^".into()),
            TextKey::Character("4".into()),
            TextKey::Backspace,
            TextKey::Backspace,
            TextKey::Character("⁴".into()),
        ] {
            editor.handle_key(key, TextModifiers::default(), &mut fonts, &mut layouts);
        }
        assert_eq!(editor.text(), "123⁴");
        for text in ["^^", "^p", "≥", "≤", "^3"] {
            editor.handle_key(
                TextKey::Character(text.into()),
                TextModifiers::default(),
                &mut fonts,
                &mut layouts,
            );
        }
        assert_eq!(editor.text(), "123⁴^^^p≥≤^3");
    }
}

#[cfg(test)]
mod dead_key_regressions {
    use super::*;
    #[test]
    fn french_caret_is_literal_and_external_replacement_keeps_prefix() {
        let mut renderer = crate::VelloRenderer::new();
        let (fonts, layouts) = renderer.contexts_mut();
        for (trigger, replacement) in [("4", "⁴"), (">", "≥"), ("<", "≤")] {
            let mut editor = TextEditState::new("123", 20.0);
            for key in [
                TextKey::End,
                TextKey::DeadCaret,
                TextKey::ComposedCharacter(trigger.into(), false),
                TextKey::Backspace,
                TextKey::Backspace,
                TextKey::ComposedCharacter(replacement.into(), false),
            ] {
                editor.handle_key(key, TextModifiers::default(), fonts, layouts);
            }
            assert_eq!(editor.text(), format!("123{replacement}"));
        }
        let mut editor = TextEditState::new("", 20.0);
        for key in [
            TextKey::DeadCaret,
            TextKey::ComposedCharacter("^".into(), true),
            TextKey::DeadCaret,
            TextKey::ComposedCharacter("^p".into(), false),
            TextKey::DeadCaret,
            TextKey::ComposedCharacter("â".into(), false),
        ] {
            editor.handle_key(key, TextModifiers::default(), fonts, layouts);
        }
        assert_eq!(editor.text(), "^^^pâ");
    }
}

/// Optical axis of an equals glyph, measured from the font baseline.
pub(crate) fn font_math_axis(bytes: &[u8], index: u32, size: f32) -> f32 {
    ttf_parser::Face::parse(bytes, index)
        .ok()
        .and_then(|face| {
            let glyph = face.glyph_index('=')?;
            let bounds = face.glyph_bounding_box(glyph)?;
            Some(
                (bounds.y_min as f32 + bounds.y_max as f32) * 0.5 * size
                    / face.units_per_em() as f32,
            )
        })
        .unwrap_or(size * 0.27)
}
/// Parley aligns boxes at their bottom; move ordinary text to their math axis.
pub(crate) fn inline_baseline_shift(line: &parley::layout::Line<'_, Brush>, size: f32) -> f32 {
    let height = line
        .items()
        .filter_map(|item| match item {
            parley::PositionedLayoutItem::InlineBox(b) => Some(b.height),
            _ => None,
        })
        .fold(0.0f32, f32::max);
    if height == 0.0 {
        return 0.0;
    }
    let axis = line
        .runs()
        .find(|run| run.font_size() > 0.0)
        .map(|run| {
            let font = run.font();
            font_math_axis(font.data.data(), font.index, run.font_size())
        })
        .unwrap_or(size * 0.27);
    height * 0.5 - axis
}
