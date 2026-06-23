use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    #[default]
    GlassSquid,
    Glacier,
    Nebula,
    Prismarine,
    Aurora,
    EmberMist,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 6] = [
        ThemePreset::GlassSquid,
        ThemePreset::Glacier,
        ThemePreset::Nebula,
        ThemePreset::Prismarine,
        ThemePreset::Aurora,
        ThemePreset::EmberMist,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemePreset::GlassSquid => "Glass Squid",
            ThemePreset::Glacier => "Glacier",
            ThemePreset::Nebula => "Nebula",
            ThemePreset::Prismarine => "Prismarine",
            ThemePreset::Aurora => "Aurora",
            ThemePreset::EmberMist => "Ember Mist",
        }
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Keybinds,
    Themes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeybindAction {
    Copy,
    Paste,
    Cut,
    Rename,
    NewFile,
    OpenSettings,
    GoBack,
    GoForward,
    UpOneLevel,
    Refresh,
    SelectAll,
}

impl KeybindAction {
    pub const ALL: [KeybindAction; 11] = [
        KeybindAction::Copy,
        KeybindAction::Paste,
        KeybindAction::Cut,
        KeybindAction::Rename,
        KeybindAction::NewFile,
        KeybindAction::OpenSettings,
        KeybindAction::GoBack,
        KeybindAction::GoForward,
        KeybindAction::UpOneLevel,
        KeybindAction::Refresh,
        KeybindAction::SelectAll,
    ];

    pub fn label(self) -> &'static str {
        match self {
            KeybindAction::Copy => "Copy",
            KeybindAction::Paste => "Paste",
            KeybindAction::Cut => "Cut",
            KeybindAction::Rename => "Rename",
            KeybindAction::NewFile => "New File",
            KeybindAction::OpenSettings => "Open Settings",
            KeybindAction::GoBack => "Go Back",
            KeybindAction::GoForward => "Go Forward",
            KeybindAction::UpOneLevel => "Up One Level",
            KeybindAction::Refresh => "Refresh",
            KeybindAction::SelectAll => "Select All",
        }
    }

    pub fn default_binding(self) -> Option<Keybind> {
        match self {
            KeybindAction::Copy => Some(Keybind::command(ShortcutKey::C)),
            KeybindAction::Paste => Some(Keybind::command(ShortcutKey::V)),
            KeybindAction::Cut => Some(Keybind::command(ShortcutKey::X)),
            KeybindAction::Rename => Some(Keybind::plain(ShortcutKey::F2)),
            KeybindAction::NewFile => Some(Keybind::command(ShortcutKey::N)),
            KeybindAction::OpenSettings => Some(Keybind {
                command: true,
                shift: true,
                alt: false,
                key: ShortcutKey::S,
            }),
            KeybindAction::GoBack => None,
            KeybindAction::GoForward => None,
            KeybindAction::UpOneLevel => None,
            KeybindAction::Refresh => Some(Keybind::command(ShortcutKey::R)),
            KeybindAction::SelectAll => Some(Keybind::command(ShortcutKey::A)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutKey {
    A,
    C,
    N,
    R,
    S,
    V,
    X,
    Delete,
    F2,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
}

impl ShortcutKey {
    pub fn to_egui(self) -> Key {
        match self {
            ShortcutKey::A => Key::A,
            ShortcutKey::C => Key::C,
            ShortcutKey::N => Key::N,
            ShortcutKey::R => Key::R,
            ShortcutKey::S => Key::S,
            ShortcutKey::V => Key::V,
            ShortcutKey::X => Key::X,
            ShortcutKey::Delete => Key::Delete,
            ShortcutKey::F2 => Key::F2,
            ShortcutKey::ArrowLeft => Key::ArrowLeft,
            ShortcutKey::ArrowRight => Key::ArrowRight,
            ShortcutKey::ArrowUp => Key::ArrowUp,
        }
    }

    pub fn from_egui(key: Key) -> Option<Self> {
        Some(match key {
            Key::A => ShortcutKey::A,
            Key::C => ShortcutKey::C,
            Key::N => ShortcutKey::N,
            Key::R => ShortcutKey::R,
            Key::S => ShortcutKey::S,
            Key::V => ShortcutKey::V,
            Key::X => ShortcutKey::X,
            Key::Delete => ShortcutKey::Delete,
            Key::F2 => ShortcutKey::F2,
            Key::ArrowLeft => ShortcutKey::ArrowLeft,
            Key::ArrowRight => ShortcutKey::ArrowRight,
            Key::ArrowUp => ShortcutKey::ArrowUp,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            ShortcutKey::A => "A",
            ShortcutKey::C => "C",
            ShortcutKey::N => "N",
            ShortcutKey::R => "R",
            ShortcutKey::S => "S",
            ShortcutKey::V => "V",
            ShortcutKey::X => "X",
            ShortcutKey::Delete => "Delete",
            ShortcutKey::F2 => "F2",
            ShortcutKey::ArrowLeft => "Left",
            ShortcutKey::ArrowRight => "Right",
            ShortcutKey::ArrowUp => "Up",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keybind {
    #[serde(default)]
    pub command: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub key: ShortcutKey,
}

impl Keybind {
    pub fn plain(key: ShortcutKey) -> Self {
        Self {
            command: false,
            shift: false,
            alt: false,
            key,
        }
    }

    pub fn command(key: ShortcutKey) -> Self {
        Self {
            command: true,
            shift: false,
            alt: false,
            key,
        }
    }

    pub fn to_shortcut(self) -> KeyboardShortcut {
        KeyboardShortcut::new(
            Modifiers {
                command: self.command,
                shift: self.shift,
                alt: self.alt,
                ..Default::default()
            },
            self.key.to_egui(),
        )
    }

    pub fn label(self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.command {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        parts.push(self.key.label());
        parts.join(" + ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeybindSettings {
    #[serde(default = "default_copy")]
    pub copy: Option<Keybind>,
    #[serde(default = "default_paste")]
    pub paste: Option<Keybind>,
    #[serde(default = "default_cut")]
    pub cut: Option<Keybind>,
    #[serde(default = "default_rename")]
    pub rename: Option<Keybind>,
    #[serde(default = "default_new_file")]
    pub new_file: Option<Keybind>,
    #[serde(default = "default_open_settings")]
    pub open_settings: Option<Keybind>,
    #[serde(default)]
    pub go_back: Option<Keybind>,
    #[serde(default)]
    pub go_forward: Option<Keybind>,
    #[serde(default)]
    pub up_one_level: Option<Keybind>,
    #[serde(default = "default_refresh")]
    pub refresh: Option<Keybind>,
    #[serde(default = "default_select_all")]
    pub select_all: Option<Keybind>,
}

fn default_copy() -> Option<Keybind> {
    KeybindAction::Copy.default_binding()
}
fn default_paste() -> Option<Keybind> {
    KeybindAction::Paste.default_binding()
}
fn default_cut() -> Option<Keybind> {
    KeybindAction::Cut.default_binding()
}
fn default_rename() -> Option<Keybind> {
    KeybindAction::Rename.default_binding()
}
fn default_new_file() -> Option<Keybind> {
    KeybindAction::NewFile.default_binding()
}
fn default_open_settings() -> Option<Keybind> {
    KeybindAction::OpenSettings.default_binding()
}
fn default_refresh() -> Option<Keybind> {
    KeybindAction::Refresh.default_binding()
}
fn default_select_all() -> Option<Keybind> {
    KeybindAction::SelectAll.default_binding()
}

impl Default for KeybindSettings {
    fn default() -> Self {
        Self {
            copy: default_copy(),
            paste: default_paste(),
            cut: default_cut(),
            rename: default_rename(),
            new_file: default_new_file(),
            open_settings: default_open_settings(),
            go_back: None,
            go_forward: None,
            up_one_level: None,
            refresh: default_refresh(),
            select_all: default_select_all(),
        }
    }
}

impl KeybindSettings {
    pub fn get(&self, action: KeybindAction) -> Option<Keybind> {
        match action {
            KeybindAction::Copy => self.copy,
            KeybindAction::Paste => self.paste,
            KeybindAction::Cut => self.cut,
            KeybindAction::Rename => self.rename,
            KeybindAction::NewFile => self.new_file,
            KeybindAction::OpenSettings => self.open_settings,
            KeybindAction::GoBack => self.go_back,
            KeybindAction::GoForward => self.go_forward,
            KeybindAction::UpOneLevel => self.up_one_level,
            KeybindAction::Refresh => self.refresh,
            KeybindAction::SelectAll => self.select_all,
        }
    }

    pub fn set(&mut self, action: KeybindAction, binding: Option<Keybind>) {
        match action {
            KeybindAction::Copy => self.copy = binding,
            KeybindAction::Paste => self.paste = binding,
            KeybindAction::Cut => self.cut = binding,
            KeybindAction::Rename => self.rename = binding,
            KeybindAction::NewFile => self.new_file = binding,
            KeybindAction::OpenSettings => self.open_settings = binding,
            KeybindAction::GoBack => self.go_back = binding,
            KeybindAction::GoForward => self.go_forward = binding,
            KeybindAction::UpOneLevel => self.up_one_level = binding,
            KeybindAction::Refresh => self.refresh = binding,
            KeybindAction::SelectAll => self.select_all = binding,
        }
    }

    pub fn is_default_binding(&self, action: KeybindAction) -> bool {
        self.get(action) == action.default_binding()
    }

    pub fn consume_action(&self, input: &mut egui::InputState, action: KeybindAction) -> bool {
        self.get(action)
            .is_some_and(|binding| input.consume_shortcut(&binding.to_shortcut()))
    }
}
