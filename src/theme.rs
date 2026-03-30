use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use crossterm::style::Stylize;

/// Available built-in colour presets.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    /// Moco signature green palette — the default.
    #[default]
    Moco,
    /// Neutral light-on-dark theme.
    Default,
    /// Dracula-inspired dark theme.
    Dracula,
    /// Nord-inspired cool blue theme.
    Nord,
    /// Solarized Dark theme.
    SolarizedDark,
}

/// Per-element colour overrides from `[theme.colors]` in `config.toml`.
///
/// Any `None` field falls back to the active preset value.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorConfig {
    /// Colour for open task identifiers.
    pub open: Option<String>,
    /// Colour for completed task identifiers.
    pub complete: Option<String>,
    /// Colour for deferred task identifiers.
    pub defer: Option<String>,
    /// Accent colour — headings, labels, selected text.
    pub accent: Option<String>,
    /// Background colour for the selected row in TUI browsers.
    pub selection_bg: Option<String>,
    /// Colour of the filled portion of a progress bar.
    pub progress_filled: Option<String>,
    /// Colour of the empty portion of a progress bar.
    pub progress_empty: Option<String>,
}

/// The `[theme]` section of `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeConfig {
    /// Named colour preset to use as the base.
    #[serde(default)]
    pub preset: Preset,
    /// Per-element overrides applied on top of the preset.
    #[serde(default)]
    pub colors: ColorConfig,
}

/// Resolved theme — all colours ready for rendering.
#[derive(Debug, Clone)]
pub struct Theme {
    pub open: Color,
    pub complete: Color,
    pub defer: Color,
    pub accent: Color,
    pub selection_bg: Color,
    pub progress_filled: Color,
    pub progress_empty: Color,
}

/// Convert a ratatui [`Color`] to the equivalent crossterm color for terminal output.
fn to_crossterm(color: Color) -> crossterm::style::Color {
    use crossterm::style::Color as C;
    match color {
        Color::Black => C::Black,
        Color::Red => C::DarkRed,
        Color::Green => C::DarkGreen,
        Color::Yellow => C::DarkYellow,
        Color::Blue => C::DarkBlue,
        Color::Magenta => C::DarkMagenta,
        Color::Cyan => C::DarkCyan,
        Color::White | Color::Gray => C::Grey,
        Color::DarkGray => C::DarkGrey,
        Color::LightRed => C::Red,
        Color::LightGreen => C::Green,
        Color::LightYellow => C::Yellow,
        Color::LightBlue => C::Blue,
        Color::LightMagenta => C::Magenta,
        Color::LightCyan => C::Cyan,
        Color::Indexed(i) => C::AnsiValue(i),
        Color::Rgb(r, g, b) => C::Rgb { r, g, b },
        _ => C::Reset,
    }
}

/// Returns `true` if ANSI color output should be emitted to stdout.
///
/// Checks the `NO_COLOR` environment variable and whether stdout is a terminal.
pub fn is_color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

/// Convert a ratatui [`Color`] into an `anstyle` [`Style`] for use with clap's help renderer.
fn ratatui_color_to_anstyle(color: Color) -> clap::builder::styling::Style {
    use clap::builder::styling::{Ansi256Color, AnsiColor, Color as SColor, RgbColor, Style};
    let fg = match color {
        Color::Black => SColor::Ansi(AnsiColor::Black),
        Color::Red => SColor::Ansi(AnsiColor::Red),
        Color::Green => SColor::Ansi(AnsiColor::Green),
        Color::Yellow => SColor::Ansi(AnsiColor::Yellow),
        Color::Blue => SColor::Ansi(AnsiColor::Blue),
        Color::Magenta => SColor::Ansi(AnsiColor::Magenta),
        Color::Cyan => SColor::Ansi(AnsiColor::Cyan),
        Color::Gray | Color::White => SColor::Ansi(AnsiColor::White),
        Color::DarkGray => SColor::Ansi(AnsiColor::BrightBlack),
        Color::LightRed => SColor::Ansi(AnsiColor::BrightRed),
        Color::LightGreen => SColor::Ansi(AnsiColor::BrightGreen),
        Color::LightYellow => SColor::Ansi(AnsiColor::BrightYellow),
        Color::LightBlue => SColor::Ansi(AnsiColor::BrightBlue),
        Color::LightMagenta => SColor::Ansi(AnsiColor::BrightMagenta),
        Color::LightCyan => SColor::Ansi(AnsiColor::BrightCyan),
        Color::Indexed(n) => SColor::Ansi256(Ansi256Color(n)),
        Color::Rgb(r, g, b) => SColor::Rgb(RgbColor(r, g, b)),
        _ => SColor::Ansi(AnsiColor::White),
    };
    Style::new().fg_color(Some(fg))
}

impl Theme {
    /// Resolve a [`ThemeConfig`] into a concrete [`Theme`].
    ///
    /// Applies the named preset as a base, then layers any per-element overrides.
    pub fn resolve(config: &ThemeConfig) -> Self {
        let base = Self::from_preset(&config.preset);
        let c = &config.colors;

        Self {
            open: c.open.as_deref().and_then(parse_color).unwrap_or(base.open),
            complete: c.complete.as_deref().and_then(parse_color).unwrap_or(base.complete),
            defer: c.defer.as_deref().and_then(parse_color).unwrap_or(base.defer),
            accent: c.accent.as_deref().and_then(parse_color).unwrap_or(base.accent),
            selection_bg: c
                .selection_bg
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(base.selection_bg),
            progress_filled: c
                .progress_filled
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(base.progress_filled),
            progress_empty: c
                .progress_empty
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(base.progress_empty),
        }
    }

    /// Wrap `text` in ANSI color codes for terminal (non-TUI) output.
    ///
    /// Returns plain text when color is disabled (non-TTY stdout or `NO_COLOR` set).
    pub fn paint(&self, text: impl std::fmt::Display, color: Color) -> String {
        if is_color_enabled() {
            text.to_string().with(to_crossterm(color)).to_string()
        } else {
            text.to_string()
        }
    }

    /// Build clap [`Styles`] from this theme so that `--help` output matches the active palette.
    ///
    /// Section headings (Usage, Commands, Options), option literals, and placeholders
    /// all derive their colours from the theme's `accent` and `open` fields.
    pub fn to_clap_styles(&self) -> clap::builder::Styles {
        use clap::builder::styling::{AnsiColor, Color as SColor, Effects, Style};
        let header = ratatui_color_to_anstyle(self.accent) | Effects::BOLD;
        let literal = ratatui_color_to_anstyle(self.accent) | Effects::BOLD;
        let placeholder = ratatui_color_to_anstyle(self.open);
        let error_style = Style::new()
            .fg_color(Some(SColor::Ansi(AnsiColor::BrightRed)))
            | Effects::BOLD;
        let valid_style = ratatui_color_to_anstyle(self.open);
        let invalid_style = Style::new()
            .fg_color(Some(SColor::Ansi(AnsiColor::BrightYellow)))
            | Effects::BOLD;

        clap::builder::Styles::styled()
            .header(header)
            .usage(header)
            .literal(literal)
            .placeholder(placeholder)
            .error(error_style)
            .valid(valid_style)
            .invalid(invalid_style)
    }

    fn from_preset(preset: &Preset) -> Self {
        match preset {
            Preset::Moco => Self::preset_moco(),
            Preset::Default => Self::preset_default(),
            Preset::Dracula => Self::preset_dracula(),
            Preset::Nord => Self::preset_nord(),
            Preset::SolarizedDark => Self::preset_solarized_dark(),
        }
    }

    /// Moco signature theme — a palette of greens inspired by the tool's name.
    ///
    /// Fresh lime for open tasks, solid green for completed, yellow-green for
    /// deferred, and a dark forest green as the selection background.
    fn preset_moco() -> Self {
        Self {
            open: Color::LightGreen,
            complete: Color::Green,
            defer: Color::Indexed(148), // yellow-green (#afd700)
            accent: Color::LightGreen,
            selection_bg: Color::Indexed(22), // dark forest green (#005f00)
            progress_filled: Color::Green,
            progress_empty: Color::Indexed(22),
        }
    }

    fn preset_default() -> Self {
        Self {
            open: Color::White,
            complete: Color::Green,
            defer: Color::Yellow,
            accent: Color::Blue,
            selection_bg: Color::DarkGray,
            progress_filled: Color::White,
            progress_empty: Color::DarkGray,
        }
    }

    fn preset_dracula() -> Self {
        Self {
            open: Color::White,
            complete: Color::Green,
            defer: Color::Magenta,
            accent: Color::Cyan,
            selection_bg: Color::Indexed(236),
            progress_filled: Color::Cyan,
            progress_empty: Color::Indexed(236),
        }
    }

    fn preset_nord() -> Self {
        Self {
            open: Color::White,
            complete: Color::Cyan,
            defer: Color::Yellow,
            accent: Color::LightBlue,
            selection_bg: Color::DarkGray,
            progress_filled: Color::LightBlue,
            progress_empty: Color::DarkGray,
        }
    }

    fn preset_solarized_dark() -> Self {
        Self {
            open: Color::Cyan,
            complete: Color::Green,
            defer: Color::Yellow,
            accent: Color::Blue,
            selection_bg: Color::DarkGray,
            progress_filled: Color::Blue,
            progress_empty: Color::DarkGray,
        }
    }
}

/// Parse a colour name string to a [`Color`], returning `None` if unrecognised.
///
/// Accepts lowercase names with either `-` or `_` as word separators.
pub fn parse_color(s: &str) -> Option<Color> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "light_red" => Some(Color::LightRed),
        "light_green" => Some(Color::LightGreen),
        "light_yellow" => Some(Color::LightYellow),
        "light_blue" => Some(Color::LightBlue),
        "light_magenta" => Some(Color::LightMagenta),
        "light_cyan" => Some(Color::LightCyan),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_basic_names() {
        assert_eq!(parse_color("green"), Some(Color::Green));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("white"), Some(Color::White));
    }

    #[test]
    fn parse_color_case_insensitive() {
        assert_eq!(parse_color("GREEN"), Some(Color::Green));
        assert_eq!(parse_color("DarkGray"), None); // only lowercase accepted
    }

    #[test]
    fn parse_color_dash_separator() {
        assert_eq!(parse_color("dark-gray"), Some(Color::DarkGray));
        assert_eq!(parse_color("light-blue"), Some(Color::LightBlue));
    }

    #[test]
    fn parse_color_unknown_returns_none() {
        assert_eq!(parse_color("neon-pink"), None);
    }

    #[test]
    fn default_preset_resolves() {
        let config = ThemeConfig::default();
        let theme = Theme::resolve(&config);
        // Default is now the moco preset
        assert_eq!(theme.open, Color::LightGreen);
        assert_eq!(theme.complete, Color::Green);
    }

    #[test]
    fn moco_preset() {
        let config = ThemeConfig {
            preset: Preset::Moco,
            colors: ColorConfig::default(),
        };
        let theme = Theme::resolve(&config);
        assert_eq!(theme.open, Color::LightGreen);
        assert_eq!(theme.complete, Color::Green);
        assert_eq!(theme.defer, Color::Indexed(148));
        assert_eq!(theme.selection_bg, Color::Indexed(22));
    }

    #[test]
    fn neutral_default_preset() {
        let config = ThemeConfig {
            preset: Preset::Default,
            colors: ColorConfig::default(),
        };
        let theme = Theme::resolve(&config);
        assert_eq!(theme.open, Color::White);
        assert_eq!(theme.accent, Color::Blue);
    }

    #[test]
    fn override_applies_on_top_of_preset() {
        let config = ThemeConfig {
            preset: Preset::Default,
            colors: ColorConfig {
                open: Some("cyan".to_string()),
                ..Default::default()
            },
        };
        let theme = Theme::resolve(&config);
        assert_eq!(theme.open, Color::Cyan);
        assert_eq!(theme.complete, Color::Green); // unchanged
    }

    #[test]
    fn unknown_override_color_falls_back_to_preset() {
        let config = ThemeConfig {
            preset: Preset::Default,
            colors: ColorConfig {
                open: Some("__unknown__".to_string()),
                ..Default::default()
            },
        };
        let theme = Theme::resolve(&config);
        assert_eq!(theme.open, Color::White); // preset default
    }

    #[test]
    fn dracula_preset() {
        let config = ThemeConfig {
            preset: Preset::Dracula,
            colors: ColorConfig::default(),
        };
        let theme = Theme::resolve(&config);
        assert_eq!(theme.accent, Color::Cyan);
        assert_eq!(theme.defer, Color::Magenta);
    }

    #[test]
    fn nord_preset() {
        let config = ThemeConfig {
            preset: Preset::Nord,
            colors: ColorConfig::default(),
        };
        let theme = Theme::resolve(&config);
        assert_eq!(theme.complete, Color::Cyan);
        assert_eq!(theme.accent, Color::LightBlue);
    }

    #[test]
    fn to_clap_styles_does_not_panic() {
        // Smoke test: all presets should produce valid Styles without panicking.
        for preset in [Preset::Moco, Preset::Default, Preset::Dracula, Preset::Nord, Preset::SolarizedDark] {
            let theme = Theme::resolve(&ThemeConfig { preset, colors: ColorConfig::default() });
            let _styles = theme.to_clap_styles();
        }
    }

    #[test]
    fn ratatui_color_to_anstyle_covers_indexed() {
        // Indexed colors (used by Moco preset) must not hit the fallback branch.
        let style = ratatui_color_to_anstyle(Color::Indexed(22));
        // Just ensure it produces a style with a foreground color set.
        assert!(style.get_fg_color().is_some());
    }

    #[test]
    fn paint_returns_plain_text_when_no_color_set() {
        // Set NO_COLOR — paint() must return undecorated text.
        // We only set NO_COLOR; stdout may or may not be a TTY in CI so we
        // test the env-var branch specifically.
        // SAFETY: test is single-threaded from Rust's test runner perspective;
        //         env mutation is acceptable here.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let theme = Theme::resolve(&ThemeConfig::default());
        let result = theme.paint("hello", Color::Green);
        unsafe { std::env::remove_var("NO_COLOR") };
        assert_eq!(result, "hello");
    }
}
