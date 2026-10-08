use super::{
    Message, Modal, State,
    animated_logo::AnimatedLogo,
    style::{self, Button},
    toast::Context as ToastContext,
};
use crate::{
    brand::Icon,
    i18n::{self, tr},
    model::{ImageItem, SaveMode, Status, bytes},
    preview::Preview,
    theme::{self, Mode, Tokens},
};
use iced::{
    Border, Center, Element, Fill,
    font::Weight,
    widget::{
        self, button, center, checkbox, column, container, image, mouse_area, opaque, pick_list,
        progress_bar, row, rule, scrollable, space, stack, text, text_input, tooltip,
    },
};

pub(super) const PAGE_SIZE: usize = 18;
const SIDEBAR_WIDTH: f32 = 240.;
const HEADER_HEIGHT: f32 = 80.;
fn label(state: &State, key: &str) -> &'static str {
    tr(state.settings.language, key)
}
fn tokens(state: &State) -> Tokens {
    *theme::tokens(state.settings.theme.is_dark(state.system_dark))
}
fn title<'a>(value: impl iced::advanced::text::IntoFragment<'a>, size: u32) -> widget::Text<'a> {
    text(value).size(size).font(theme::font(Weight::Semibold))
}
fn action<'a>(
    state: &State,
    key: &str,
    message: Option<Message>,
    kind: Button,
) -> widget::Button<'a, Message> {
    let t = tokens(state);
    button(text(label(state, key)).size(13))
        .padding([9, 12])
        .on_press_maybe(message)
        .style(move |_, status| style::button_style(t, kind, status))
}
fn caption<'a>(state: &State, key: &str) -> widget::Text<'a> {
    text(label(state, key))
        .size(12)
        .color(tokens(state).secondary)
}
fn import_actions(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let unlocked = !state.busy && !state.importing;
    button(
        text(label(
            state,
            if state.importing {
                "status.importing"
            } else {
                "action.import"
            },
        ))
        .size(14)
        .font(theme::font(Weight::Semibold))
        .width(Fill)
        .align_x(Center),
    )
    .padding([11, 20])
    .width(132)
    .on_press_maybe(unlocked.then_some(Message::OpenImport))
    .style(move |_, status| style::button_style(t, Button::Primary, status))
    .into()
}
fn icon_action<'a>(
    state: &State,
    icon: Icon,
    key: &str,
    message: Option<Message>,
) -> Element<'a, Message> {
    let t = tokens(state);
    let enabled = message.is_some();
    let graphic: Element<'a, Message> = if let Some(assets) = &state.brand {
        image(
            assets
                .icon(
                    icon,
                    state.settings.theme.is_dark(state.system_dark),
                    enabled,
                )
                .clone(),
        )
        .width(20)
        .height(20)
        .into()
    } else {
        space().width(20).height(20).into()
    };
    mouse_area(
        tooltip(
            button(graphic)
                .padding(9)
                .on_press_maybe(message)
                .style(move |_, status| style::button_style(t, Button::Quiet, status)),
            text(label(state, key)).size(12),
            tooltip::Position::Top,
        )
        .style(style::surface(t, false)),
    )
    .interaction(iced::mouse::Interaction::Pointer)
    .into()
}
fn shortened(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_owned();
    }
    let head = value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    format!("{head}…")
}
pub(super) fn view(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let logo: Element<'_, Message> = if let Some(assets) = &state.brand {
        brand_scene(assets, 96., state.modal.is_none())
    } else {
        space().width(96).into()
    };
    let brand = container(logo).height(HEADER_HEIGHT).align_y(iced::Bottom);
    let header = container(
        row![
            brand,
            space().width(Fill),
            action(
                state,
                "settings.title",
                Some(Message::OpenSettings),
                Button::Quiet
            )
        ]
        .spacing(4)
        .align_y(Center),
    )
    .padding([0, 24])
    .width(Fill)
    .style(move |_| container::Style {
        background: Some(t.panel.into()),
        text_color: Some(t.text),
        ..Default::default()
    });
    let body = row![workspace(state), output(state)]
        .spacing(20)
        .height(Fill)
        .padding(20);
    let base: Element<'_, Message> =
        column![header, rule::horizontal(1).style(style::divider(t)), body]
            .height(Fill)
            .into();
    let content = if let Some(modal) = state.modal {
        let content = match modal {
            Modal::Import => import_dialog(state),
            Modal::Keys | Modal::Settings => settings_dialog(state),
            Modal::Preview => preview(state),
            Modal::Details(id) => details(state, id),
        };
        stack![
            base,
            opaque(
                center(opaque(content))
                    .padding(20)
                    .style(move |_| container::Style {
                        background: Some(t.scrim.into()),
                        ..Default::default()
                    })
            )
        ]
        .into()
    } else {
        base
    };
    stack![content, toast_layer(state)].into()
}

fn toast_layer(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let mut toasts = column![].spacing(8).width(Fill).max_width(440);
    for toast in state.notifications.items() {
        let mut message = column![].spacing(4).width(Fill);
        let context = match &toast.context {
            ToastContext::General => None,
            ToastContext::Key(index) => Some(format!("Key {}", index + 1)),
            ToastContext::Image(name) => Some(shortened(name, 40)),
        };
        if let Some(context) = context {
            message = message.push(title(context, 12));
        }
        message = message.push(
            text(toast.error.text(state.settings.language))
                .size(12)
                .color(t.text),
        );
        toasts = toasts.push(opaque(
            container(
                row![
                    text("!").size(18).color(t.error),
                    message,
                    button(text("×").size(18))
                        .padding([3, 8])
                        .on_press(Message::DismissToast(toast.id))
                        .style(move |_, status| style::button_style(t, Button::Quiet, status))
                ]
                .spacing(12)
                .align_y(Center),
            )
            .padding(12)
            .width(Fill)
            .style(move |_| container::Style {
                background: Some(t.panel.into()),
                text_color: Some(t.text),
                border: Border {
                    color: t.error,
                    width: 1.,
                    radius: style::PANEL_RADIUS.into(),
                },
                ..Default::default()
            }),
        ));
    }
    container(toasts)
        .padding(20)
        .width(Fill)
        .height(Fill)
        .align_x(Center)
        .into()
}
fn workspace(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let unlocked = !state.busy && !state.importing;
    let mut filters = row![].spacing(4);
    for status in [
        None,
        Some(Status::Pending),
        Some(Status::Processing),
        Some(Status::Done),
        Some(Status::Failed),
    ] {
        let name = status
            .map(|s| s.label(state.settings.language))
            .unwrap_or(label(state, "status.all"));
        filters = filters.push(
            button(text(name).size(12))
                .padding([7, 9])
                .on_press(Message::Filter(status))
                .style(move |_, s| {
                    style::button_style(
                        t,
                        if state.filter == status {
                            Button::Selected
                        } else {
                            Button::Quiet
                        },
                        s,
                    )
                }),
        );
    }
    let ids = state.visible();
    let visible_count = ids.len();
    let list: Element<'_, Message> = if visible_count == 0 {
        empty(state)
    } else {
        let mut gallery = column![].spacing(12);
        for chunk in ids
            .into_iter()
            .skip(state.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>()
            .chunks(3)
        {
            let mut line = row![].spacing(12);
            for id in chunk {
                if let Some(item) = state.images.iter().find(|i| i.id == *id) {
                    line = line.push(tile(state, item));
                }
            }
            for _ in chunk.len()..3 {
                line = line.push(space().width(Fill));
            }
            gallery = gallery.push(line);
        }
        scrollable(gallery)
            .height(Fill)
            .direction(style::scrollbar())
            .style(style::scroll(t))
            .into()
    };
    let footer = row![
        icon_action(
            state,
            Icon::Clear,
            "action.clear",
            (unlocked && !state.images.is_empty()).then_some(Message::Clear),
        ),
        space().width(Fill),
        icon_action(
            state,
            Icon::Previous,
            "action.previous",
            (state.page > 0).then_some(Message::Page(-1)),
        ),
        text(format!(
            "{} / {}",
            state.page + 1,
            visible_count.div_ceil(PAGE_SIZE).max(1)
        ))
        .size(12)
        .color(t.secondary),
        icon_action(
            state,
            Icon::Next,
            "action.next",
            ((state.page + 1) * PAGE_SIZE < visible_count).then_some(Message::Page(1)),
        )
    ]
    .spacing(5)
    .align_y(Center);
    column![filters, list, footer]
        .spacing(16)
        .width(Fill)
        .height(Fill)
        .into()
}
fn brand_scene(
    assets: &crate::brand::Brand,
    width: f32,
    playing: bool,
) -> Element<'static, Message> {
    // Match the original 82:96 panda/grass widths and share the same baseline.
    // Grass draws last so it overlaps the feet instead of leaving a vertical gap.
    let panda_width = width * 82. / 96.;
    let panda_height = panda_width * assets.panda.height as f32 / assets.panda.width as f32;
    // Report the visible scene's natural height; extra space belongs to its parent.
    let height = panda_height;
    stack![
        // Anchor the seat's center at source x=173 to the grass center.
        container(
            AnimatedLogo::new(&assets.panda, panda_width, panda_height)
                .offset_x(-panda_width * 34. / assets.panda.width as f32)
                .playing(playing)
        )
        .width(width)
        .height(height)
        .align_x(Center)
        .align_y(iced::Bottom),
        container(
            image(assets.grass.clone())
                .width(width)
                .height(width / 8.)
                .content_fit(iced::ContentFit::Contain)
        )
        .width(width)
        .height(height)
        .align_y(iced::Bottom)
    ]
    .into()
}

fn empty(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let content: Element<'_, Message> = if state.images.is_empty() && state.filter.is_none() {
        column![
            title(label(state, "empty.title"), 18),
            import_actions(state),
            text(label(state, "empty.formats"))
                .size(11)
                .color(t.secondary)
        ]
        .spacing(14)
        .align_x(Center)
        .into()
    } else if let Some(assets) = &state.brand {
        image(
            assets
                .icon(
                    Icon::Empty,
                    state.settings.theme.is_dark(state.system_dark),
                    true,
                )
                .clone(),
        )
        .width(64)
        .height(64)
        .content_fit(iced::ContentFit::Contain)
        .into()
    } else {
        space().width(64).height(64).into()
    };
    container(center(content))
        .width(Fill)
        .height(Fill)
        .style(move |_| container::Style {
            background: Some(if state.hover { t.selected } else { t.panel }.into()),
            text_color: Some(t.text),
            border: Border {
                color: if state.hover { t.focus } else { t.border },
                width: 1.,
                radius: style::PANEL_RADIUS.into(),
            },
            ..Default::default()
        })
        .into()
}
fn tile<'a>(state: &'a State, item: &'a ImageItem) -> Element<'a, Message> {
    let t = tokens(state);
    let unlocked = !state.busy && !state.importing;
    let thumbnail: Element<'_, Message> = if let Some(handle) = &item.thumbnail {
        image(handle.clone())
            .width(Fill)
            .height(84)
            .content_fit(iced::ContentFit::Contain)
            .into()
    } else {
        center(text("JPG / PNG / WebP").size(11).color(t.secondary))
            .width(Fill)
            .height(84)
            .into()
    };
    let image_area = container(thumbnail)
        .padding(8)
        .width(Fill)
        .style(move |_| container::Style {
            background: Some(t.background.into()),
            border: Border {
                radius: 8.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let preview = button(image_area)
        .on_press(Message::Preview(item.id))
        .padding(0)
        .width(Fill)
        .style(move |_, s| style::button_style(t, Button::Quiet, s));
    let size = match &item.result {
        Some(Ok(o)) => format!("{} → {}", bytes(o.before), bytes(o.after)),
        _ => format!("{} × {}  {}", item.width, item.height, bytes(item.bytes)),
    };
    let color = match item.status {
        Status::Done => t.success,
        Status::Failed => t.error,
        Status::Processing => t.primary,
        _ => t.secondary,
    };
    let name = item.name();
    let filename = container(
        title(shortened(&name, 27), 13)
            .width(Fill)
            .height(18)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Fill)
    .clip(true);
    let status = row![
        text(item.status.label(state.settings.language))
            .size(11)
            .color(color),
        space().width(Fill),
        text(if item.status == Status::Pending {
            String::new()
        } else {
            format!("{:.0}%", item.progress)
        })
        .size(11)
        .color(t.secondary)
    ]
    .align_y(Center);
    let content = column![
        preview,
        filename,
        text(size).size(11).color(t.secondary),
        status,
        progress_bar(0.0..=100., item.progress)
            .girth(3)
            .style(style::progress(t, item.status == Status::Failed))
    ]
    .spacing(8);
    let mut actions = row![].spacing(2);
    if item.status != Status::Done {
        actions = actions.push(
            action(
                state,
                "action.remove",
                unlocked.then_some(Message::Remove(item.id)),
                Button::Quiet,
            )
            .padding([5, 6]),
        );
    }
    if item.status == Status::Failed {
        actions = actions.push(
            action(
                state,
                "action.retry",
                state.can_compress().then_some(Message::Retry(item.id)),
                Button::Selected,
            )
            .padding([5, 6]),
        );
    }
    actions = actions.push(space().width(Fill)).push(
        action(
            state,
            "action.details",
            Some(Message::Details(item.id)),
            Button::Quiet,
        )
        .padding([5, 6]),
    );
    container(content.push(actions))
        .padding(10)
        .width(Fill)
        .style(move |_| container::Style {
            background: Some(t.card.into()),
            text_color: Some(t.text),
            border: Border {
                color: t.border,
                width: 1.,
                radius: style::PANEL_RADIUS.into(),
            },
            ..Default::default()
        })
        .into()
}
fn output(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let mut options = column![].spacing(8);
    for mode in [SaveMode::Overwrite, SaveMode::Directory] {
        let (key, hint) = if mode == SaveMode::Overwrite {
            ("save.overwrite", "save.overwrite_hint")
        } else {
            ("save.directory", "save.directory_hint")
        };
        let selected = state.mode == mode;
        options = options.push(
            button(
                row![
                    text(if selected { "●" } else { "○" }).size(14),
                    column![
                        title(label(state, key), 13),
                        text(label(state, hint)).size(11).color(if state.busy {
                            t.disabled_text
                        } else {
                            t.secondary
                        })
                    ]
                    .spacing(4)
                ]
                .spacing(9),
            )
            .padding(10)
            .width(Fill)
            .on_press_maybe((!state.busy).then_some(Message::Mode(mode)))
            .style(move |_, status| {
                style::button_style(
                    t,
                    if selected {
                        Button::Selected
                    } else {
                        Button::Secondary
                    },
                    status,
                )
            }),
        );
    }
    if state.mode == SaveMode::Directory {
        let mut folder_label = column![
            text(label(
                state,
                if state.output.is_some() {
                    "save.change"
                } else {
                    "save.choose"
                },
            ))
            .size(12)
        ]
        .spacing(4);
        if let Some(path) = &state.output {
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy();
            folder_label = folder_label.push(text(shortened(&name, 24)).size(11));
        }
        let choose = button(folder_label)
            .padding(10)
            .width(Fill)
            .on_press_maybe((!state.busy).then_some(Message::ChooseOutput))
            .style(move |_, status| style::button_style(t, Button::Secondary, status));
        let mut keep = checkbox(state.preserve)
            .label(label(state, "save.structure"))
            .size(14)
            .text_size(11)
            .spacing(8)
            .style(style::check(t));
        if !state.busy {
            keep = keep.on_toggle(Message::Preserve);
        }
        options = options.push(choose).push(keep);
    }
    if state.output_folder().is_some() {
        options = options.push(
            action(state, "save.open", Some(Message::OpenOutput), Button::Quiet)
                .width(Fill)
                .padding([6, 10]),
        );
    }
    let total = state.run_items.len();
    let done = state
        .images
        .iter()
        .filter(|i| {
            state.run_items.contains(&i.id) && matches!(i.status, Status::Done | Status::Failed)
        })
        .count();
    let progress = if total > 0 {
        state
            .images
            .iter()
            .filter(|i| state.run_items.contains(&i.id))
            .map(|i| i.progress)
            .sum::<f32>()
            / total as f32
    } else {
        0.
    };
    let before = state
        .images
        .iter()
        .filter_map(|i| i.result.as_ref()?.as_ref().ok())
        .map(|o| o.before)
        .sum::<u64>();
    let after = state
        .images
        .iter()
        .filter_map(|i| i.result.as_ref()?.as_ref().ok())
        .map(|o| o.after)
        .sum::<u64>();
    if total > 0 {
        options = options.push(
            column![
                rule::horizontal(1).style(style::divider(t)),
                row![
                    caption(state, "status.progress"),
                    space().width(Fill),
                    text(format!("{progress:.0}%")).size(12)
                ]
                .align_y(Center),
                progress_bar(0.0..=100., progress)
                    .girth(5)
                    .style(style::progress(t, false)),
                text(i18n::format(
                    state.settings.language,
                    "stats.processed",
                    &[("done", done.to_string()), ("total", total.to_string())]
                ))
                .size(11)
                .color(t.secondary),
                metric(state, "stats.original", bytes(before), t.text),
                metric(state, "stats.result", bytes(after), t.text),
                metric(
                    state,
                    "stats.saved_label",
                    bytes(before.saturating_sub(after)),
                    t.success
                )
            ]
            .spacing(8),
        );
    }
    let quota = i18n::format(
        state.settings.language,
        "output.quota",
        &[("count", state.remaining().to_string())],
    );
    let content = column![
        title(label(state, "output.title"), 18),
        scrollable(options)
            .height(Fill)
            .direction(style::scrollbar())
            .style(style::scroll(t)),
        rule::horizontal(1).style(style::divider(t)),
        text(quota).size(11).color(if state.remaining() > 0 {
            t.secondary
        } else {
            t.warning
        }),
        button(
            text(label(
                state,
                if state.busy {
                    "status.processing"
                } else {
                    "output.compress"
                },
            ))
            .size(14)
            .font(theme::font(Weight::Semibold))
            .width(Fill)
            .align_x(Center)
        )
        .padding([11, 12])
        .on_press_maybe(state.can_compress().then_some(Message::Compress))
        .style(move |_, status| style::button_style(t, Button::Primary, status))
        .width(Fill)
    ]
    .spacing(16);
    container(content)
        .padding(18)
        .width(SIDEBAR_WIDTH)
        .height(Fill)
        .style(style::surface(t, false))
        .into()
}
fn metric<'a>(state: &State, key: &str, value: String, color: iced::Color) -> Element<'a, Message> {
    row![
        caption(state, key),
        space().width(Fill),
        text(value).size(12).color(color)
    ]
    .align_y(Center)
    .into()
}

fn modal_header<'a>(state: &State, key: &str, suffix: String) -> Element<'a, Message> {
    row![
        title(label(state, key), 20),
        space().width(Fill),
        text(suffix).size(12).color(tokens(state).secondary)
    ]
    .align_y(Center)
    .into()
}
fn import_dialog(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let unlocked = !state.busy && !state.importing;
    container(
        column![
            modal_header(state, "action.import", String::new()),
            rule::horizontal(1).style(style::divider(t)),
            row![
                action(
                    state,
                    "action.images",
                    unlocked.then_some(Message::Import(false)),
                    Button::Primary
                )
                .width(Fill),
                action(
                    state,
                    "action.folder",
                    unlocked.then_some(Message::Import(true)),
                    Button::Secondary
                )
                .width(Fill),
            ]
            .spacing(12),
            row![
                space().width(Fill),
                action(
                    state,
                    "action.cancel",
                    Some(Message::CloseModal),
                    Button::Quiet
                ),
            ]
        ]
        .spacing(20),
    )
    .padding(24)
    .width(360)
    .style(style::surface(t, true))
    .into()
}
fn settings_dialog(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let editing_keys = state.modal == Some(Modal::Keys);
    let tabs = row![
        action(
            state,
            "settings.general",
            Some(Message::OpenSettings),
            if editing_keys {
                Button::Quiet
            } else {
                Button::Selected
            }
        ),
        action(
            state,
            "keys.tab",
            (!state.busy && !state.key_busy).then_some(Message::OpenKeys),
            if editing_keys {
                Button::Selected
            } else {
                Button::Quiet
            }
        )
    ]
    .spacing(6);
    let content = if editing_keys {
        keys(state)
    } else {
        settings(state)
    };
    container(
        column![
            modal_header(
                state,
                "settings.title",
                if editing_keys {
                    format!("{} / 50", state.draft.len())
                } else {
                    String::new()
                }
            ),
            rule::horizontal(1).style(style::divider(t)),
            tabs,
            content
        ]
        .spacing(16),
    )
    .padding(24)
    .width(620)
    .style(style::surface(t, true))
    .into()
}

fn keys(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let mut entries = column![].spacing(16);
    for (index, draft) in state.draft.iter().enumerate() {
        let mut note = text_input(label(state, "keys.note"), &draft.account.note)
            .padding(10)
            .size(13)
            .style(style::input(t));
        let mut key = text_input("API Key", &draft.account.key)
            .secure(!draft.visible)
            .padding(10)
            .size(13)
            .style(style::input(t));
        if !state.key_busy {
            note = note.on_input(move |v| Message::EditNote(index, v));
            key = key.on_input(move |v| Message::EditKey(index, v));
        }
        let top = row![
            title(format!("Key {}", index + 1), 13),
            space().width(Fill),
            action(
                state,
                "action.remove",
                (!state.key_busy).then_some(Message::RemoveKey(index)),
                Button::Danger
            )
        ]
        .align_y(Center);
        let field = row![
            key,
            icon_action(
                state,
                if draft.visible {
                    Icon::Hide
                } else {
                    Icon::Show
                },
                if draft.visible {
                    "action.hide"
                } else {
                    "action.show"
                },
                Some(Message::ToggleKey(index)),
            )
        ]
        .spacing(8)
        .align_y(Center);
        let mut entry = column![top, note, field].spacing(8);
        if state
            .accounts
            .get(index)
            .is_some_and(|a| a.key == draft.account.key)
            && let Some(usage) = state.usage.get(index)
        {
            if usage.error.is_some() {
                entry = entry.push(
                    button(text(label(state, "status.failed")).size(12))
                        .padding([2, 0])
                        .on_press(Message::KeyError(index))
                        .style(move |_, status| style::button_style(t, Button::Danger, status)),
                );
            } else {
                entry = entry.push(
                    text(i18n::format(
                        state.settings.language,
                        "keys.remaining",
                        &[("count", usage.remaining().to_string())],
                    ))
                    .size(12)
                    .color(t.secondary),
                );
            }
        }
        entries = entries.push(entry);
        if index + 1 < state.draft.len() {
            entries = entries.push(rule::horizontal(1).style(style::divider(t)));
        }
    }
    let scroll_height = (state.draft.len().max(1) * 166).min(240) as f32;
    let content = column![
        scrollable(entries)
            .height(scroll_height)
            .direction(style::scrollbar())
            .style(style::scroll(t)),
        icon_action(
            state,
            Icon::Add,
            "keys.add",
            (!state.key_busy && state.draft.len() < 50).then_some(Message::AddKey),
        )
    ]
    .spacing(16);
    let footer = row![
        action(
            state,
            "keys.check",
            (!state.key_busy && !state.accounts.is_empty()).then_some(Message::CheckKeys),
            Button::Quiet
        ),
        space().width(Fill),
        action(
            state,
            "action.cancel",
            (!state.key_busy).then_some(Message::CloseModal),
            Button::Quiet
        ),
        action(
            state,
            if state.key_saving {
                "keys.saving"
            } else if state.key_busy {
                "keys.validating"
            } else {
                "action.save"
            },
            (!state.key_busy).then_some(Message::SaveKeys),
            Button::Primary
        )
    ]
    .spacing(8);
    content
        .push(rule::horizontal(1).style(style::divider(t)))
        .push(footer)
        .into()
}
fn settings(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let mut themes = row![].spacing(6);
    for mode in [Mode::Light, Mode::Dark, Mode::System] {
        themes = themes.push(
            button(text(mode.label(state.settings.language)).size(13))
                .padding([10, 12])
                .width(Fill)
                .on_press(Message::Theme(mode))
                .style(move |_, s| {
                    style::button_style(
                        t,
                        if mode == state.settings.theme {
                            Button::Selected
                        } else {
                            Button::Secondary
                        },
                        s,
                    )
                }),
        );
    }
    let language = pick_list(
        [i18n::Language::English, i18n::Language::Chinese],
        Some(state.settings.language),
        Message::Language,
    )
    .padding(10)
    .text_size(13)
    .width(Fill)
    .style(style::select(t))
    .menu_style(style::menu(t));
    column![
        column![caption(state, "settings.language"), language].spacing(10),
        column![caption(state, "settings.theme"), themes].spacing(10),
        rule::horizontal(1).style(style::divider(t)),
        row![
            space().width(Fill),
            action(
                state,
                "action.close",
                (!state.key_saving).then_some(Message::CloseModal),
                Button::Primary
            )
        ]
    ]
    .spacing(20)
    .into()
}
fn preview(state: &State) -> Element<'_, Message> {
    let t = tokens(state);
    let stage: Element<'_, Message> = state
        .preview_handle
        .as_ref()
        .map(|handle| {
            Preview {
                handle: handle.clone(),
                transform: state.preview_transform,
            }
            .into()
        })
        .unwrap_or_else(|| {
            center(
                text(label(state, "status.loading"))
                    .size(13)
                    .color(t.secondary),
            )
            .into()
        });
    let name = state
        .images
        .iter()
        .find(|i| Some(i.id) == state.preview_id)
        .map(|i| i.name())
        .unwrap_or_default();
    let heading = row![
        column![
            title(shortened(&name, 64), 16),
            caption(state, "preview.original")
        ]
        .spacing(4),
        space().width(Fill),
        action(
            state,
            "action.close",
            Some(Message::CloseModal),
            Button::Quiet
        )
    ]
    .spacing(12)
    .align_y(Center);
    let tools = row![
        action(
            state,
            "action.zoom_out",
            (state.preview_transform.scale > 0.1).then_some(Message::Zoom(1. / 1.2)),
            Button::Secondary
        ),
        text(format!("{:.0}%", state.preview_transform.scale * 100.)).size(13),
        action(
            state,
            "action.zoom_in",
            (state.preview_transform.scale < 10.).then_some(Message::Zoom(1.2)),
            Button::Secondary
        ),
        action(
            state,
            "action.reset",
            Some(Message::ResetPreview),
            Button::Quiet
        )
    ]
    .spacing(12)
    .align_y(Center);
    container(
        column![
            heading,
            rule::horizontal(1).style(style::divider(t)),
            stage,
            container(tools).center_x(Fill)
        ]
        .spacing(16),
    )
    .width(Fill)
    .height(Fill)
    .padding(20)
    .style(style::surface(t, true))
    .into()
}
fn details(state: &State, id: u64) -> Element<'_, Message> {
    let t = tokens(state);
    let mut content = column![].spacing(14);
    if let Some(item) = state.images.iter().find(|i| i.id == id) {
        content = content.push(title(item.name(), 16)).push(
            text(item.path.to_string_lossy().into_owned())
                .size(12)
                .color(t.secondary),
        );
        if let Some(result) = &item.result {
            match result {
                Ok(o) => {
                    content = content
                        .push(text(o.path.to_string_lossy().into_owned()).size(12))
                        .push(text(format!("{} → {}", bytes(o.before), bytes(o.after))).size(13));
                }
                Err(_) => content = content.push(caption(state, "status.failed")),
            }
        }
    }
    container(
        column![
            modal_header(state, "action.details", String::new()),
            rule::horizontal(1).style(style::divider(t)),
            scrollable(content)
                .height(220)
                .direction(style::scrollbar())
                .style(style::scroll(t)),
            row![
                space().width(Fill),
                action(
                    state,
                    "action.close",
                    Some(Message::CloseModal),
                    Button::Primary
                )
            ]
        ]
        .spacing(18),
    )
    .width(560)
    .padding(24)
    .style(style::surface(t, true))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{
        Event, Point, Rectangle, Size, Vector,
        advanced::{Layout, Shell, layout, mouse, widget::Tree},
    };

    fn menu_leaves(layout: Layout<'_>, bounds: &mut Vec<Rectangle>) {
        let children = layout.children().collect::<Vec<_>>();
        if children.is_empty() {
            bounds.push(layout.bounds());
        } else {
            for child in children {
                menu_leaves(child, bounds);
            }
        }
    }

    fn content_bounds(state: &State, size: Size) -> Vec<Rectangle> {
        fn collect(layout: Layout<'_>, bounds: &mut Vec<Rectangle>) {
            bounds.push(layout.bounds());
            for child in layout.children() {
                collect(child, bounds);
            }
        }
        let renderer = iced::Renderer::new(theme::font(Weight::Normal), 16.into());
        let mut view = view(state);
        let mut tree = Tree::new(&view);
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let content = Layout::new(&node).children().next().unwrap();
        let mut bounds = Vec::new();
        collect(content, &mut bounds);
        bounds
    }

    #[test]
    fn toasts_do_not_resize_the_workspace_or_settings_dialogs() {
        let mut state = State::boot(None).0;
        state.key_busy = false;
        for modal in [None, Some(Modal::Settings), Some(Modal::Keys)] {
            state.modal = modal;
            state.notifications = Default::default();
            let before = content_bounds(&state, Size::new(860., 620.));
            state.notify(crate::error::Error::Network, ToastContext::General);
            state.notify(crate::error::Error::KeyFormat, ToastContext::Key(0));
            state.notify(
                crate::error::Error::File,
                ToastContext::Image("test.png".into()),
            );
            assert_eq!(content_bounds(&state, Size::new(860., 620.)), before);
        }
    }

    #[test]
    fn toast_controls_are_clickable_without_blocking_the_import_button() {
        let mut state = State::boot(None).0;
        state.key_busy = false;
        state.notifications = Default::default();
        state.notify(crate::error::Error::Network, ToastContext::General);
        let toast_id = state.notifications.items()[0].id;
        let size = Size::new(860., 620.);
        let viewport = Rectangle::new(Point::ORIGIN, size);
        let renderer = iced::Renderer::new(theme::font(Weight::Normal), 16.into());
        let mut view = view(&state);
        let mut tree = Tree::new(&view);
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let root = Layout::new(&node);
        let content = root.children().next().unwrap();
        let body = content.children().nth(2).unwrap();
        let workspace = body.children().next().unwrap();
        let empty = workspace.children().nth(1).unwrap();
        let center = empty.children().next().unwrap();
        let empty_content = center.children().next().unwrap();
        let import_point = empty_content.children().nth(1).unwrap().bounds().center();

        let toast_layer = root.children().nth(1).unwrap();
        let toast_column = toast_layer.children().next().unwrap();
        let toast = toast_column.children().next().unwrap();
        let toast_row = toast.children().next().unwrap();
        let close_point = toast_row.children().nth(2).unwrap().bounds().center();
        let mut click = |point| {
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                view.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    root,
                    mouse::Cursor::Available(point),
                    &renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &viewport,
                );
            }
            messages
        };
        assert!(
            click(import_point)
                .iter()
                .any(|message| matches!(message, Message::OpenImport))
        );
        assert!(
            click(close_point)
                .iter()
                .any(|message| matches!(message, Message::DismissToast(id) if *id == toast_id))
        );
    }

    #[test]
    fn language_menu_opens_below_the_selector_in_supported_windows() {
        let renderer = iced::Renderer::new(theme::font(Weight::Normal), 16.into());
        for size in [Size::new(860., 620.), Size::new(1080., 760.)] {
            for language in [i18n::Language::English, i18n::Language::Chinese] {
                for theme in [Mode::Light, Mode::Dark, Mode::System] {
                    let mut state = State::boot(None).0;
                    state.modal = Some(Modal::Settings);
                    state.settings.language = language;
                    state.settings.theme = theme;
                    let mut dialog = settings_dialog(&state);
                    let mut tree = Tree::new(&dialog);
                    let node = dialog.as_widget_mut().layout(
                        &mut tree,
                        &renderer,
                        &layout::Limits::new(Size::ZERO, size),
                    );
                    let origin = Point::new(
                        (size.width - node.size().width) / 2.,
                        (size.height - node.size().height) / 2.,
                    );
                    let node = node.move_to(origin);
                    let root = Layout::new(&node);
                    let content = root.children().next().unwrap();
                    let settings = content.children().nth(3).unwrap();
                    let language_section = settings.children().next().unwrap();
                    let selector = language_section.children().nth(1).unwrap().bounds();
                    let viewport = Rectangle::new(Point::ORIGIN, size);
                    let mut messages = Vec::new();
                    dialog.as_widget_mut().update(
                        &mut tree,
                        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        root,
                        mouse::Cursor::Available(selector.center()),
                        &renderer,
                        &mut iced::advanced::clipboard::Null,
                        &mut Shell::new(&mut messages),
                        &viewport,
                    );
                    let mut overlay = dialog
                        .as_widget_mut()
                        .overlay(&mut tree, root, &renderer, &viewport, Vector::ZERO)
                        .expect("Clicking the language selector opens its menu");
                    let menu = overlay.as_overlay_mut().layout(&renderer, size);
                    let mut leaves = Vec::new();
                    menu_leaves(Layout::new(&menu), &mut leaves);
                    assert!(!leaves.is_empty());
                    for bounds in leaves {
                        assert!(
                            bounds.y >= selector.y + selector.height - 0.01,
                            "Menu must open downward: {bounds:?}, selector {selector:?}"
                        );
                        assert!(
                            bounds.y + bounds.height <= size.height,
                            "Menu must fit the window"
                        );
                    }
                }
            }
        }
    }
}
