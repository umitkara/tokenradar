//! `[widget]`: the optional compact strip (Step 18, ADR-058).
//!
//! ```toml
//! [widget]
//! enabled = true
//! mode = "floating"          # floating | taskbar (Step 18b)
//! taskbar_align = "tray"     # taskbar mode: next to the notification area | at the left end
//! taskbar_offset = 0         # taskbar mode: logical px away from that anchor (a drag sets it)
//! opacity = 85               # background opacity in percent, 40..=100
//! click_through = false
//! hide_in_fullscreen = true
//! pace_mark = false
//! hotkey = "Ctrl+Alt+Shift+R" # toggles click-through while the widget is on
//!
//! [widget.position]          # written by the app after a drag, not a setting
//! monitor = '\\.\DISPLAY1'
//! x = 1.0                    # share of the free space: 0 = left/top, 1 = right/bottom
//! y = 1.0
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use specta::Type;

/// Lowest background opacity: below it the text loses its ground on a busy wallpaper.
pub const MIN_WIDGET_OPACITY: u8 = 40;
pub const DEFAULT_WIDGET_OPACITY: u8 = 85;
pub const DEFAULT_WIDGET_HOTKEY: &str = "Ctrl+Alt+Shift+R";
/// Largest taskbar offset in logical pixels: wider than any taskbar's free space, so it only
/// stops nonsense values; the widget is kept on the taskbar anyway.
pub const MAX_TASKBAR_OFFSET: u16 = 4000;

/// Where the widget lives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WidgetMode {
    /// On the desktop, always on top, draggable.
    #[default]
    Floating,
    /// Docked on the primary taskbar (Step 18b). Best effort: where docking fails the widget
    /// floats and tries again later (ADR-058).
    Taskbar,
}

/// Where a docked widget is anchored on the taskbar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarAlign {
    /// Just left of the notification area; the offset moves it further left.
    #[default]
    Tray,
    /// At the taskbar's left end; the offset moves it to the right. For centered icons.
    Left,
}

/// The widget settings the user changes; the position is kept apart ([`WidgetPosition`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // independent on/off settings, as in the config file
pub struct WidgetPrefs {
    /// Off by default (ADR-003): the tray is the only surface until the user asks for more.
    pub enabled: bool,
    pub mode: WidgetMode,
    /// Background opacity in percent; the text and bars stay opaque.
    pub opacity: u8,
    /// Mouse input passes through to the windows below.
    pub click_through: bool,
    /// Hide while a full-screen app, game or presentation runs.
    pub hide_in_fullscreen: bool,
    /// The time mark (ADR-044) across each bar.
    pub pace_mark: bool,
    /// The global shortcut that toggles click-through, as [`Hotkey`] text.
    pub hotkey: String,
    /// Taskbar mode: the anchor.
    pub taskbar_align: TaskbarAlign,
    /// Taskbar mode: logical pixels away from the anchor, `0..=MAX_TASKBAR_OFFSET`.
    pub taskbar_offset: u16,
}

impl Default for WidgetPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: WidgetMode::Floating,
            opacity: DEFAULT_WIDGET_OPACITY,
            click_through: false,
            hide_in_fullscreen: true,
            pace_mark: false,
            hotkey: DEFAULT_WIDGET_HOTKEY.to_owned(),
            taskbar_align: TaskbarAlign::Tray,
            taskbar_offset: 0,
        }
    }
}

impl WidgetPrefs {
    /// Opacity within `MIN_WIDGET_OPACITY..=100`, the taskbar offset within
    /// `0..=MAX_TASKBAR_OFFSET`; a shortcut that does not parse goes back to the default, a
    /// valid one is written in its canonical form.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.opacity = self.opacity.clamp(MIN_WIDGET_OPACITY, 100);
        self.taskbar_offset = self.taskbar_offset.min(MAX_TASKBAR_OFFSET);
        self.hotkey = Hotkey::parse(&self.hotkey)
            .unwrap_or_else(Hotkey::default_key)
            .to_string();
        self
    }

    /// The parsed shortcut; [`Self::normalized`] makes this always succeed.
    pub fn hotkey(&self) -> Hotkey {
        Hotkey::parse(&self.hotkey).unwrap_or_else(Hotkey::default_key)
    }
}

/// Where the widget was left: the monitor's device name and the top-left corner as a share
/// of the free space in that monitor's work area, so it survives resolution and scale changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub monitor: String,
    pub x: f64,
    pub y: f64,
}

impl WidgetPosition {
    #[must_use]
    pub fn normalized(mut self) -> Self {
        let unit = |v: f64| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                1.0
            }
        };
        self.x = unit(self.x);
        self.y = unit(self.y);
        self
    }
}

/// `[widget]` as read: the settings and, under them, the position.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct WidgetSection {
    #[serde(flatten)]
    pub prefs: WidgetPrefs,
    pub position: Option<WidgetPosition>,
}

/// The key of a global shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyKey {
    /// `A`..=`Z`.
    Letter(char),
    /// `0`..=`9`.
    Digit(u8),
    /// `F1`..=`F24`.
    F(u8),
}

/// A global shortcut such as `Ctrl+Alt+Shift+R`. It needs Ctrl, Alt or Win, so it never takes
/// a key the user types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // the four modifier keys
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: HotkeyKey,
}

impl Hotkey {
    pub const fn default_key() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: true,
            win: false,
            key: HotkeyKey::Letter('R'),
        }
    }

    /// Parses `+`-separated parts in any order and case; the key comes last.
    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let (key, modifiers) = parts.split_last()?;
        let mut hk = Self {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            key: parse_key(key)?,
        };
        for m in modifiers {
            let flag = match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => &mut hk.ctrl,
                "alt" => &mut hk.alt,
                "shift" => &mut hk.shift,
                "win" | "super" | "meta" => &mut hk.win,
                _ => return None,
            };
            if *flag {
                return None;
            }
            *flag = true;
        }
        (hk.ctrl || hk.alt || hk.win).then_some(hk)
    }
}

fn parse_key(key: &str) -> Option<HotkeyKey> {
    let upper = key.to_ascii_uppercase();
    let mut chars = upper.chars();
    match (chars.next()?, chars.as_str()) {
        (c @ 'A'..='Z', "") => Some(HotkeyKey::Letter(c)),
        (c @ '0'..='9', "") => Some(HotkeyKey::Digit(c as u8 - b'0')),
        ('F', n) => n
            .parse::<u8>()
            .ok()
            .filter(|n| (1..=24).contains(n))
            .map(HotkeyKey::F),
        _ => None,
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.ctrl, "Ctrl"),
            (self.alt, "Alt"),
            (self.shift, "Shift"),
            (self.win, "Win"),
        ] {
            if on {
                write!(f, "{name}+")?;
            }
        }
        match self.key {
            HotkeyKey::Letter(c) => write!(f, "{c}"),
            HotkeyKey::Digit(d) => write!(f, "{d}"),
            HotkeyKey::F(n) => write!(f, "F{n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_parse_in_any_order_and_print_canonically() {
        let hk = Hotkey::parse("shift + ctrl+ALT+r").expect("valid");
        assert_eq!(hk, Hotkey::default_key());
        assert_eq!(hk.to_string(), DEFAULT_WIDGET_HOTKEY);
        assert_eq!(
            Hotkey::parse("Win+F12").map(|h| h.to_string()).as_deref(),
            Some("Win+F12")
        );
        assert_eq!(
            Hotkey::parse("Ctrl+7").map(|h| h.key),
            Some(HotkeyKey::Digit(7))
        );
    }

    #[test]
    fn hotkeys_need_a_modifier_that_typing_does_not_use() {
        for bad in [
            "R",
            "Shift+R",
            "Ctrl+",
            "Ctrl+Ctrl+R",
            "Ctrl+F25",
            "Ctrl+Ğ",
            "Hyper+R",
            "",
        ] {
            assert_eq!(Hotkey::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn prefs_normalize_opacity_and_shortcut() {
        let p = WidgetPrefs {
            opacity: 5,
            hotkey: "nonsense".into(),
            ..WidgetPrefs::default()
        }
        .normalized();
        assert_eq!(p.opacity, MIN_WIDGET_OPACITY);
        assert_eq!(p.hotkey, DEFAULT_WIDGET_HOTKEY);
        let p = WidgetPrefs {
            opacity: 200,
            hotkey: "alt+shift+w".into(),
            ..WidgetPrefs::default()
        }
        .normalized();
        assert_eq!((p.opacity, p.hotkey.as_str()), (100, "Alt+Shift+W"));
        let p = WidgetPrefs {
            taskbar_offset: u16::MAX,
            ..WidgetPrefs::default()
        }
        .normalized();
        assert_eq!(p.taskbar_offset, MAX_TASKBAR_OFFSET);
    }

    #[test]
    fn positions_stay_inside_the_free_space() {
        let p = WidgetPosition {
            monitor: "m".into(),
            x: -1.0,
            y: f64::NAN,
        }
        .normalized();
        assert_eq!((p.x, p.y), (0.0, 1.0));
    }
}
