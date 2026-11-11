//! App-owned palettes. Named variants follow Oak's published palette families;
//! selection is local to FolderLens and scoped to each Rune view build.

use std::cell::Cell;

use rune::Color;

// Default colors also name the shared roles used by all view builders.
pub const PANEL: &str = "#010105";
pub const INK: &str = "#ffffff";
pub const MUTED: &str = "#aaaab8";
pub const BLUE: &str = "#66a7ff";
pub const RED: &str = "#ff666c";
pub const TRACK: &str = "#1b1b23";
pub const CARD: &str = "#111116";
pub const BORDER: &str = "#454552";
pub const CHECK_BORDER: &str = "#636366";
pub const CHECK_FILL: &str = "#2a2a2c";
pub const TOGGLE_OFF: &str = "#3a3a3c";
pub const SECONDARY_TEXT: &str = "#d1d1d6";
pub const DANGER_TEXT: &str = "#ffaca5";
pub const DANGER_FILL: &str = "#3b2a30";
pub const DANGER_BORDER: &str = "#88504e";
pub const WARNING: &str = "#ff9f0a";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Default,
    Kanagawa,
    Lotus,
    Dracula,
    Alucard,
    TokyoNight,
    TokyoDay,
}

impl Theme {
    pub const ALL: [Self; 7] = [
        Self::Default,
        Self::Kanagawa,
        Self::Lotus,
        Self::Dracula,
        Self::Alucard,
        Self::TokyoNight,
        Self::TokyoDay,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Kanagawa => "kanagawa",
            Self::Lotus => "kanagawa-lotus",
            Self::Dracula => "dracula",
            Self::Alucard => "alucard",
            Self::TokyoNight => "tokyo-night",
            Self::TokyoDay => "tokyo-night-day",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Kanagawa => "Kanagawa",
            Self::Lotus => "Kanagawa Lotus",
            Self::Dracula => "Dracula",
            Self::Alucard => "Alucard (Light)",
            Self::TokyoNight => "Tokyo Night",
            Self::TokyoDay => "Tokyo Night Day",
        }
    }

    pub fn parse(value: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|theme| theme.as_str() == value)
            .unwrap_or_default()
    }

    fn palette(self) -> Palette {
        let (background, surface, foreground, muted, accent, error, warning) = match self {
            Self::Default => (PANEL, TRACK, INK, MUTED, BLUE, RED, WARNING),
            Self::Kanagawa => (
                "#1f1f28", "#2a2a37", "#dcd7ba", "#c8c3a8", "#7e9cd8", "#ff7777", "#ff9e3b",
            ),
            Self::Lotus => (
                "#f2ecbc", "#e5ddb0", "#43436c", "#43436c", "#455f8e", "#a83247", "#665b26",
            ),
            Self::Dracula => (
                "#282a36", "#44475a", "#f8f8f2", "#f8f8f2", "#8be9fd", "#ffa0a0", "#ffb86c",
            ),
            Self::Alucard => (
                "#fffbeb", "#cfcfde", "#1f1f1f", "#343434", "#005e87", "#a32c20", "#79380e",
            ),
            Self::TokyoNight => (
                "#222436", "#2f334d", "#c8d3f5", "#c8d3f5", "#82aaff", "#ff9098", "#ff966c",
            ),
            Self::TokyoDay => (
                "#e1e2e7", "#d0d5e3", "#2e5857", "#2e5857", "#005f76", "#a33232", "#75552b",
            ),
        };
        Palette {
            background,
            surface,
            foreground,
            muted,
            accent,
            error,
            warning,
        }
    }

    fn paint(self, token: &str) -> Color {
        if self == Self::Default {
            return rune::color::hex(token).unwrap_or(Color::WHITE);
        }
        let p = self.palette();
        let hex = match token {
            PANEL => p.background,
            INK | SECONDARY_TEXT => p.foreground,
            MUTED | CHECK_BORDER | BORDER => p.muted,
            TRACK | CARD | CHECK_FILL | TOGGLE_OFF | DANGER_FILL => p.surface,
            BLUE => p.accent,
            RED | DANGER_TEXT | DANGER_BORDER => p.error,
            WARNING => p.warning,
            _ => token,
        };
        rune::color::hex(hex).unwrap_or(Color::WHITE)
    }
}

struct Palette {
    background: &'static str,
    surface: &'static str,
    foreground: &'static str,
    muted: &'static str,
    accent: &'static str,
    error: &'static str,
    warning: &'static str,
}

thread_local! {
    static CURRENT: Cell<Theme> = const { Cell::new(Theme::Default) };
}

// Shared view helpers read this scope instead of a process-wide theme setting.
// Restoring it keeps different windows and parallel headless tests independent.
pub struct Scope(Theme);

pub fn scope(theme: Theme) -> Scope {
    Scope(CURRENT.with(|current| current.replace(theme)))
}

impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|current| current.set(self.0));
    }
}

pub fn paint(token: &str) -> Color {
    CURRENT.with(|current| current.get().paint(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_keeps_text_and_actions_readable_on_both_surfaces() {
        let contrast = |a: Color, b: Color| {
            let (a, b) = (a.luminance(), b.luminance());
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        };
        for theme in Theme::ALL {
            for surface in [crate::PANEL, crate::TRACK] {
                for text in [crate::INK, crate::MUTED, crate::BLUE, crate::RED, WARNING] {
                    assert!(
                        contrast(theme.paint(text), theme.paint(surface)) >= 4.5,
                        "{theme:?}: {text} on {surface}"
                    );
                }
                assert!(contrast(theme.paint(crate::BLUE), theme.paint(surface)) >= 3.0);
            }
            assert!(contrast(theme.paint(crate::PANEL), theme.paint(crate::BLUE)) >= 4.5);
        }
    }
}
