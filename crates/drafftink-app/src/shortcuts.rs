//! Keyboard shortcut registry and documentation.

/// A keyboard shortcut definition.
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub key: &'static str,
    pub ctrl: bool,
    pub shift: bool,
    pub description: &'static str,
}

impl Shortcut {
    pub const fn new(
        key: &'static str,
        ctrl: bool,
        shift: bool,
        description: &'static str,
    ) -> Self {
        Self {
            key,
            ctrl,
            shift,
            description,
        }
    }

    /// Format the shortcut for display (e.g., "Ctrl+S").
    pub fn format(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(self.key);
        parts.join("+")
    }
}

/// Registry of all keyboard shortcuts shown by the help dialog.
pub struct ShortcutRegistry;

impl ShortcutRegistry {
    /// Get all registered shortcuts.
    pub fn all() -> Vec<Shortcut> {
        vec![
            Shortcut::new("S / 1", false, false, "Selection tool"),
            Shortcut::new("H", false, false, "Pan tool"),
            Shortcut::new("R / 2", false, false, "Rectangle tool"),
            Shortcut::new("O / 4", false, false, "Ellipse tool"),
            Shortcut::new("A / 5", false, false, "Arrow tool"),
            Shortcut::new("L / 6", false, false, "Line tool"),
            Shortcut::new("D", false, false, "Draw tool"),
            Shortcut::new("K", false, false, "Highlighter tool"),
            Shortcut::new("E", false, false, "Cycle Eraser: Classic / Manual"),
            Shortcut::new("T / 8", false, false, "Text tool"),
            Shortcut::new("M", false, false, "Math formula tool"),
            Shortcut::new("Z", false, false, "Laser; press again to toggle Permanent"),
            Shortcut::new(
                "Space (hold)",
                false,
                false,
                "Temporary pan; release to return to previous tool",
            ),
            Shortcut::new("+ / =", false, false, "Zoom in"),
            Shortcut::new("-", false, false, "Zoom out"),
            Shortcut::new("0", false, false, "Reset zoom to 100%"),
            Shortcut::new(
                "F",
                false,
                false,
                "Fit selection or drawing to the viewport",
            ),
            Shortcut::new("A", true, false, "Select all shapes"),
            Shortcut::new("S", true, false, "Save Local"),
            Shortcut::new("O", true, false, "Open..."),
            Shortcut::new("E", true, false, "Export to PNG"),
            Shortcut::new("E", true, true, "Copy selection as PNG"),
            Shortcut::new("Z", true, false, "Undo"),
            Shortcut::new("Z", true, true, "Redo"),
            Shortcut::new("Y", true, false, "Redo"),
            Shortcut::new("G", true, false, "Group selected shapes"),
            Shortcut::new("G", true, true, "Ungroup selected shapes"),
            Shortcut::new("C", true, true, "Copy selection as PNG"),
            Shortcut::new("C", true, false, "Copy shapes"),
            Shortcut::new("X", true, false, "Cut shapes"),
            Shortcut::new("V", true, false, "Paste shapes or image"),
            Shortcut::new("D", true, false, "Duplicate selected shapes"),
            Shortcut::new("Delete", false, false, "Delete selected shapes"),
            Shortcut::new("Backspace", false, false, "Delete selected shapes"),
            Shortcut::new(
                "Escape",
                false,
                false,
                "Cancel current action / return to selection",
            ),
            Shortcut::new(
                "Shift+Drag",
                false,
                false,
                "Rectangle → square; Ellipse → circle; Line/Arrow → angle snap",
            ),
            Shortcut::new("^", false, false, "Literal caret in Text; exponent in Math"),
            Shortcut::new("P", true, false, "Presentation: hide/show panels"),
            Shortcut::new("F11", false, false, "Fullscreen"),
            Shortcut::new("B", true, false, "Bold selected text"),
            Shortcut::new("I", true, false, "Italic selected text"),
            Shortcut::new("U", true, false, "Underline selected text"),
            Shortcut::new("Ctrl+Arrow", false, false, "Move selected objects quickly"),
            Shortcut::new(
                "Shift+ArrowUp",
                false,
                false,
                "Text superscript / selected-object vertical move fallback",
            ),
            Shortcut::new(
                "Shift+ArrowDown",
                false,
                false,
                "Text subscript / selected-object vertical move fallback",
            ),
        ]
    }

    /// Print all shortcuts to console.
    pub fn print_all() {
        println!("\n=== Keyboard Shortcuts ===");
        for shortcut in Self::all() {
            println!("  {:20} {}", shortcut.format(), shortcut.description);
        }
        println!();
    }
}
