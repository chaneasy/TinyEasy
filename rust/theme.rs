use crate::i18n::{Language, tr};
use iced::{Color, Theme, theme::Palette};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Light,
    Dark,
    #[default]
    System,
}
impl Mode {
    pub fn label(self, lang: Language) -> &'static str {
        tr(
            lang,
            match self {
                Self::Light => "theme.light",
                Self::Dark => "theme.dark",
                Self::System => "theme.system",
            },
        )
    }
    pub fn is_dark(self, system_dark: bool) -> bool {
        self == Self::Dark || (self == Self::System && system_dark)
    }
}
#[derive(Clone, Copy)]
pub struct Tokens {
    pub background: Color,
    pub panel: Color,
    pub card: Color,
    pub text: Color,
    pub secondary: Color,
    pub border: Color,
    pub primary: Color,
    pub primary_hover: Color,
    pub on_primary: Color,
    pub disabled_text: Color,
    pub hover: Color,
    pub focus: Color,
    pub selected: Color,
    pub disabled: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub scrim: Color,
}
const fn rgb(value: u32) -> Color {
    Color::from_rgb(
        (value >> 16 & 255) as f32 / 255.,
        (value >> 8 & 255) as f32 / 255.,
        (value & 255) as f32 / 255.,
    )
}
static LIGHT: LazyLock<Tokens> = LazyLock::new(|| Tokens {
    background: rgb(0xf4f7f9),
    panel: rgb(0xffffff),
    card: rgb(0xffffff),
    text: rgb(0x243744),
    secondary: rgb(0x637584),
    border: rgb(0xdfe7ec),
    primary: rgb(0x176b87),
    primary_hover: rgb(0x125970),
    on_primary: rgb(0xffffff),
    hover: rgb(0xeef3f6),
    focus: rgb(0x176b87),
    selected: rgb(0xe4f1f6),
    disabled: rgb(0xe6ecef),
    disabled_text: rgb(0x7d8d99),
    success: rgb(0x26764b),
    warning: rgb(0x936014),
    error: rgb(0xb6384b),
    scrim: Color::from_rgba(0.08, 0.15, 0.20, 0.45),
});
static DARK: LazyLock<Tokens> = LazyLock::new(|| Tokens {
    background: rgb(0x121c23),
    panel: rgb(0x19272f),
    card: rgb(0x1f303a),
    text: rgb(0xeaf2f5),
    secondary: rgb(0xa0b3be),
    border: rgb(0x30454f),
    primary: rgb(0x72c6da),
    primary_hover: rgb(0x93d9e7),
    on_primary: rgb(0x102b35),
    hover: rgb(0x263b46),
    focus: rgb(0x93d9e7),
    selected: rgb(0x244651),
    disabled: rgb(0x263a45),
    disabled_text: rgb(0x6e8793),
    success: rgb(0x7ace9e),
    warning: rgb(0xe9bb75),
    error: rgb(0xf194a3),
    scrim: Color::from_rgba(0.02, 0.05, 0.07, 0.65),
});
pub fn tokens(dark: bool) -> &'static Tokens {
    if dark { &DARK } else { &LIGHT }
}
static LIGHT_THEME: LazyLock<Theme> = LazyLock::new(|| make(false));
static DARK_THEME: LazyLock<Theme> = LazyLock::new(|| make(true));
fn make(dark: bool) -> Theme {
    let t = tokens(dark);
    Theme::custom_with_fn(
        if dark {
            "TinyEasy Dark"
        } else {
            "TinyEasy Light"
        },
        Palette {
            background: t.background,
            text: t.text,
            primary: t.primary,
            success: t.success,
            warning: t.warning,
            danger: t.error,
        },
        iced::theme::palette::Extended::generate,
    )
}
pub fn theme(dark: bool) -> Theme {
    if dark {
        DARK_THEME.clone()
    } else {
        LIGHT_THEME.clone()
    }
}
pub fn font(weight: iced::font::Weight) -> iced::Font {
    let name = if cfg!(target_os = "macos") {
        "PingFang SC"
    } else if cfg!(target_os = "windows") {
        "Microsoft YaHei"
    } else {
        "Noto Sans CJK SC"
    };
    iced::Font {
        weight,
        ..iced::Font::with_name(name)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_theme_ignores_system_changes() {
        assert!(!Mode::Light.is_dark(true));
        assert!(Mode::Dark.is_dark(false));
        assert!(Mode::System.is_dark(true));
        assert!(!Mode::System.is_dark(false));
    }
}
