//! A small visual vocabulary shared by every surface; no widget uses default accent fills.
use crate::theme::Tokens;
use iced::{
    Border, Theme,
    widget::{button, checkbox, container, pick_list, progress_bar, rule, scrollable, text_input},
};

pub const BUTTON_RADIUS: f32 = 7.;
pub const PANEL_RADIUS: f32 = 10.;
pub const MODAL_RADIUS: f32 = 16.;
#[derive(Clone, Copy)]
pub enum Button {
    Primary,
    Secondary,
    Quiet,
    Selected,
    Danger,
}
pub fn button_style(t: Tokens, kind: Button, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let disabled = status == button::Status::Disabled;
    let (background, text_color, border_color, border_width) = match kind {
        Button::Primary => (
            Some(if disabled {
                t.disabled
            } else if hovered {
                t.primary_hover
            } else {
                t.primary
            }),
            if disabled {
                t.disabled_text
            } else {
                t.on_primary
            },
            t.primary,
            0.,
        ),
        Button::Secondary => (
            Some(if hovered && !disabled {
                t.hover
            } else {
                t.panel
            }),
            if disabled { t.disabled_text } else { t.text },
            t.border,
            1.,
        ),
        Button::Quiet => (
            if hovered && !disabled {
                Some(t.hover)
            } else {
                None
            },
            if disabled {
                t.disabled_text
            } else {
                t.secondary
            },
            t.border,
            0.,
        ),
        Button::Selected => (
            Some(t.selected),
            if disabled { t.disabled_text } else { t.primary },
            t.primary,
            0.,
        ),
        Button::Danger => (
            if hovered && !disabled {
                Some(t.hover)
            } else {
                None
            },
            if disabled { t.disabled_text } else { t.error },
            t.border,
            0.,
        ),
    };
    button::Style {
        background: background.map(Into::into),
        text_color,
        border: Border {
            color: border_color,
            width: border_width,
            radius: BUTTON_RADIUS.into(),
        },
        ..Default::default()
    }
}
pub fn surface(t: Tokens, modal: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(t.panel.into()),
        text_color: Some(t.text),
        border: Border {
            color: t.border,
            width: 1.,
            radius: if modal { MODAL_RADIUS } else { PANEL_RADIUS }.into(),
        },
        ..Default::default()
    }
}
pub fn input(t: Tokens) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, status| text_input::Style {
        background: t.background.into(),
        border: Border {
            color: if matches!(status, text_input::Status::Focused { .. }) {
                t.focus
            } else {
                t.border
            },
            width: 1.,
            radius: BUTTON_RADIUS.into(),
        },
        icon: t.secondary,
        placeholder: t.secondary,
        value: if status == text_input::Status::Disabled {
            t.disabled_text
        } else {
            t.text
        },
        selection: t.selected,
    }
}
pub fn select(t: Tokens) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_, status| pick_list::Style {
        text_color: t.text,
        placeholder_color: t.secondary,
        handle_color: t.secondary,
        background: t.background.into(),
        border: Border {
            color: if matches!(status, pick_list::Status::Opened { .. }) {
                t.focus
            } else {
                t.border
            },
            width: 1.,
            radius: BUTTON_RADIUS.into(),
        },
    }
}
pub fn check(t: Tokens) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style {
    move |_, status| {
        let (checked, disabled) = match status {
            checkbox::Status::Active { is_checked } | checkbox::Status::Hovered { is_checked } => {
                (is_checked, false)
            }
            checkbox::Status::Disabled { is_checked } => (is_checked, true),
        };
        checkbox::Style {
            background: if disabled {
                t.disabled
            } else if checked {
                t.primary
            } else {
                t.panel
            }
            .into(),
            icon_color: t.on_primary,
            border: Border {
                color: if checked { t.primary } else { t.border },
                width: 1.,
                radius: 4.into(),
            },
            text_color: Some(if disabled {
                t.disabled_text
            } else {
                t.secondary
            }),
        }
    }
}
pub fn progress(t: Tokens, failed: bool) -> impl Fn(&Theme) -> progress_bar::Style {
    move |_| progress_bar::Style {
        background: t.hover.into(),
        bar: if failed { t.error } else { t.primary }.into(),
        border: Border {
            radius: 4.into(),
            ..Default::default()
        },
    }
}
pub fn divider(t: Tokens) -> impl Fn(&Theme) -> rule::Style {
    move |_| rule::Style {
        color: t.border,
        radius: 0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}
pub fn scroll(t: Tokens) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |theme, status| {
        let mut style = scrollable::default(theme, status);
        for rail in [&mut style.vertical_rail, &mut style.horizontal_rail] {
            rail.background = None;
            rail.border.width = 0.;
            rail.scroller.background = t.border.into();
            rail.scroller.border = Border {
                radius: 4.into(),
                ..Default::default()
            };
        }
        style
    }
}

pub fn menu(t: Tokens) -> impl Fn(&Theme) -> iced::widget::overlay::menu::Style {
    move |_| iced::widget::overlay::menu::Style {
        background: t.panel.into(),
        border: Border {
            color: t.border,
            width: 1.,
            radius: BUTTON_RADIUS.into(),
        },
        text_color: t.text,
        selected_text_color: t.primary,
        selected_background: t.selected.into(),
        shadow: Default::default(),
    }
}

pub fn scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(5)
            .scroller_width(5)
            .spacing(8),
    )
}
