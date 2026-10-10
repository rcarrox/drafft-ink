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
        crate::shortcut_bindings::definitions().into_iter().map(|d| Shortcut::new(d.default,false,false,d.description)).collect()
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
