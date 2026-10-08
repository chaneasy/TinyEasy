mod animated_logo;
mod style;
mod toast;
mod view;
use crate::{
    brand::{self, Brand},
    config::{self, Settings},
    credentials::{self, Account, SystemVault},
    error::Error,
    files::{self, ImportEvent},
    i18n::{Language, tr},
    model::{ImageItem, SaveMode, SaveOptions, Status},
    preview::Transform,
    queue::{self, KeyPool},
    theme::{self, Mode},
    tinify::{Tinify, Usage},
};
use iced::{Element, Subscription, Task, Theme, Vector, widget::image::Handle};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use toast::{Context as ToastContext, Notifications};
use tokio::sync::{Semaphore, mpsc};

#[derive(Clone)]
pub enum Message {
    Loaded(Result<Vec<Account>, Error>),
    Validated(Vec<Usage>),
    OpenKeys,
    EditKey(usize, String),
    EditNote(usize, String),
    ToggleKey(usize),
    AddKey,
    RemoveKey(usize),
    SaveKeys,
    KeysSaved(Result<Vec<Account>, Error>),
    CheckKeys,
    KeyError(usize),
    OpenSettings,
    Theme(Mode),
    Language(Language),
    SettingsSaved(Settings, Result<(), Error>),
    SystemTheme(iced::theme::Mode),
    OpenImport,
    Import(bool),
    Selected(Vec<PathBuf>),
    ImportEvent(u64, ImportEvent),
    Thumbnail(u64, u64, Result<Handle, Error>),
    Compress,
    Retry(u64),
    Queue(u64, queue::Event),
    Filter(Option<Status>),
    Page(i32),
    Remove(u64),
    Clear,
    Mode(SaveMode),
    ChooseOutput,
    OutputSelected(Option<PathBuf>),
    Preserve(bool),
    OpenOutput,
    Opened(Result<(), Error>),
    Preview(u64),
    PreviewLoaded(u64, u64, Result<Handle, Error>),
    Zoom(f32),
    Pan(Vector),
    ResetPreview,
    Details(u64),
    CloseModal,
    DismissToast(u64),
    ToastTick,
    Drop(PathBuf),
    Hover(bool),
    Tick,
    Escape,
    FocusNext(bool),
    Shortcut(String),
    Brand(Option<Brand>),
    #[cfg(debug_assertions)]
    QaWindow(iced::window::Id),
    #[cfg(debug_assertions)]
    QaCapture(iced::window::Id),
    #[cfg(debug_assertions)]
    QaScreenshot(iced::window::Screenshot),
    #[cfg(debug_assertions)]
    QaSaved(Result<(), Error>),
}
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TinyEasy UI event [content redacted]")
    }
}
#[derive(Debug, Clone)]
pub struct Draft {
    pub account: Account,
    pub visible: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    Import,
    Keys,
    Settings,
    Preview,
    Details(u64),
}
pub struct State {
    settings: Settings,
    config_dir: Option<PathBuf>,
    preference_error: Option<Error>,
    settings_saving: bool,
    system_dark: bool,
    accounts: Vec<Account>,
    usage: Vec<Usage>,
    draft: Vec<Draft>,
    key_busy: bool,
    key_saving: bool,
    client: Option<Tinify>,
    pool: KeyPool,
    images: Vec<ImageItem>,
    next_id: u64,
    busy: bool,
    run_id: u64,
    run_items: HashSet<u64>,
    importing: bool,
    import_id: u64,
    pending_drops: Vec<PathBuf>,
    page: usize,
    filter: Option<Status>,
    mode: SaveMode,
    output: Option<PathBuf>,
    preserve: bool,
    modal: Option<Modal>,
    notifications: Notifications,
    preview_id: Option<u64>,
    preview_generation: u64,
    preview_handle: Option<Handle>,
    preview_transform: Transform,
    hover: bool,
    brand: Option<Brand>,
}
static DECODERS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(3)));

pub fn run() -> iced::Result {
    let icon = image::load_from_memory(include_bytes!("../../assets/app-icons/icon-64.png"))
        .ok()
        .and_then(|i| {
            let rgba = i.to_rgba8();
            iced::window::icon::from_rgba(rgba.as_raw().clone(), rgba.width(), rgba.height()).ok()
        });
    let window_size = iced::Size::new(1080., 760.);
    #[cfg(debug_assertions)]
    let window_size = if std::env::var_os("TINYEASY_QA_DIR").is_some()
        && std::env::var_os("TINYEASY_QA_SMALL").is_some()
    {
        iced::Size::new(860., 620.)
    } else {
        window_size
    };
    iced::application(State::new, State::update, State::view)
        .title("TinyEasy")
        .theme(State::theme)
        .subscription(State::subscription)
        .window(iced::window::Settings {
            size: window_size,
            min_size: Some(iced::Size::new(860., 620.)),
            icon,
            position: iced::window::Position::Centered,
            ..Default::default()
        })
        .default_font(theme::font(iced::font::Weight::Normal))
        .run()
}
impl State {
    fn new() -> (Self, Task<Message>) {
        let directory = config::directory().ok();
        #[cfg(debug_assertions)]
        let directory = std::env::var_os("TINYEASY_QA_DIR")
            .map(PathBuf::from)
            .or(directory);
        Self::boot(directory)
    }
    fn boot(config_dir: Option<PathBuf>) -> (Self, Task<Message>) {
        let settings = config_dir
            .as_ref()
            .map(|d| config::load(&d.join("settings.json")))
            .unwrap_or(Err(Error::Configuration));
        let preference_error = settings.as_ref().err().copied();
        let mut notifications = Notifications::default();
        if let Some(error) = preference_error {
            notifications.push(error, ToastContext::General, Instant::now());
        }
        let client = Tinify::new().ok();
        #[cfg(debug_assertions)]
        let client = if std::env::var_os("TINYEASY_QA_DIR").is_some() {
            None
        } else {
            client
        };
        let state = Self {
            settings: settings.unwrap_or_default(),
            config_dir: config_dir.clone(),
            preference_error,
            settings_saving: false,
            system_dark: false,
            accounts: vec![],
            usage: vec![],
            draft: vec![],
            key_busy: true,
            key_saving: false,
            client,
            pool: KeyPool::default(),
            images: vec![],
            next_id: 1,
            busy: false,
            run_id: 0,
            run_items: HashSet::new(),
            importing: false,
            import_id: 0,
            pending_drops: vec![],
            page: 0,
            filter: None,
            mode: SaveMode::Overwrite,
            output: None,
            preserve: false,
            modal: None,
            notifications,
            preview_id: None,
            preview_generation: 0,
            preview_handle: None,
            preview_transform: Transform::default(),
            hover: false,
            brand: None,
        };
        let load = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    #[cfg(debug_assertions)]
                    if std::env::var_os("TINYEASY_QA_DIR").is_some() {
                        return Ok(Vec::new());
                    }
                    credentials::load_system(
                        &config_dir.ok_or(Error::Configuration)?.join("keys.enc"),
                    )
                })
                .await
                .map_err(|_| Error::SecureStorage)?
            },
            Message::Loaded,
        );
        let brand = Task::perform(
            async {
                tokio::task::spawn_blocking(brand::load)
                    .await
                    .ok()
                    .and_then(Result::ok)
            },
            Message::Brand,
        );
        (
            state,
            Task::batch([load, brand, iced::system::theme().map(Message::SystemTheme)]),
        )
    }
    fn theme(&self) -> Theme {
        theme::theme(self.settings.theme.is_dark(self.system_dark))
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::system::theme_changes().map(Message::SystemTheme),
            iced::event::listen_with(|event, status, _window| match event {
                #[cfg(debug_assertions)]
                iced::Event::Window(iced::window::Event::Opened { .. })
                    if std::env::var_os("TINYEASY_QA_DIR").is_some()
                        && std::env::var_os("TINYEASY_QA_SCREENSHOT").is_some() =>
                {
                    Some(Message::QaWindow(_window))
                }
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    Some(Message::Drop(path))
                }
                iced::Event::Window(iced::window::Event::FileHovered(_)) => {
                    Some(Message::Hover(true))
                }
                iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
                    Some(Message::Hover(false))
                }
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                }) => Some(Message::Escape),
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) if status == iced::event::Status::Ignored => {
                    if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) {
                        Some(Message::FocusNext(modifiers.shift()))
                    } else if modifiers.command() {
                        if let iced::keyboard::Key::Character(value) = key {
                            Some(Message::Shortcut(value.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }),
            if !self.pending_drops.is_empty() && !self.importing {
                iced::time::every(Duration::from_millis(120)).map(|_| Message::Tick)
            } else {
                Subscription::none()
            },
            if self.notifications.is_empty() {
                Subscription::none()
            } else {
                iced::time::every(Duration::from_millis(250)).map(|_| Message::ToastTick)
            },
        ])
    }
    fn view(&self) -> Element<'_, Message> {
        view::view(self)
    }
    fn notify(&mut self, error: Error, context: ToastContext) {
        self.notifications.push(error, context, Instant::now());
    }
    fn remaining(&self) -> u32 {
        self.usage.iter().map(Usage::remaining).sum()
    }
    fn output_folder(&self) -> Option<&Path> {
        if self.mode == SaveMode::Directory {
            self.output.as_deref()
        } else {
            self.images
                .iter()
                .find_map(|item| item.result.as_ref()?.as_ref().ok()?.path.parent())
        }
    }
    fn can_compress(&self) -> bool {
        !self.busy
            && !self.key_busy
            && !self.importing
            && self.remaining() > 0
            && self
                .images
                .iter()
                .any(|image| matches!(image.status, Status::Pending | Status::Failed))
            && (self.mode == SaveMode::Overwrite || self.output.is_some())
    }
    fn open_keys(&mut self) {
        self.draft = self
            .accounts
            .iter()
            .cloned()
            .map(|account| Draft {
                account,
                visible: false,
            })
            .collect();
        if self.draft.is_empty() {
            self.draft.push(Draft {
                account: Account {
                    key: String::new(),
                    note: String::new(),
                },
                visible: false,
            });
        }
        self.modal = Some(Modal::Keys);
    }
    fn validate(&mut self) -> Task<Message> {
        let Some(client) = self.client.clone() else {
            self.key_busy = false;
            self.notify(Error::Network, ToastContext::General);
            return Task::none();
        };
        self.key_busy = true;
        Task::perform(
            queue::validate(client, self.accounts.clone()),
            Message::Validated,
        )
    }
    fn persist_settings(&mut self) -> Task<Message> {
        if self.settings_saving {
            return Task::none();
        }
        if let Some(e) = self.preference_error {
            self.notify(e, ToastContext::General);
            return Task::none();
        }
        let Some(dir) = self.config_dir.clone() else {
            self.notify(Error::Configuration, ToastContext::General);
            return Task::none();
        };
        self.settings_saving = true;
        let settings = self.settings.clone();
        let result_settings = settings.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    config::save(&dir.join("settings.json"), &settings)
                })
                .await
                .unwrap_or(Err(Error::Configuration))
            },
            move |r| Message::SettingsSaved(result_settings.clone(), r),
        )
    }
    fn import(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        if paths.is_empty() || self.busy {
            return Task::none();
        }
        if self.importing {
            self.pending_drops.extend(paths);
            return Task::none();
        }
        self.importing = true;
        self.import_id += 1;
        let generation = self.import_id;
        let known = self.images.iter().map(|i| i.path.clone()).collect();
        let (tx, rx) = mpsc::channel(4);
        tokio::task::spawn_blocking(move || files::scan(paths, known, tx));
        Task::run(
            futures_util::stream::unfold(rx, |mut rx| async { rx.recv().await.map(|e| (e, rx)) }),
            move |e| Message::ImportEvent(generation, e),
        )
    }
    fn visible(&self) -> Vec<u64> {
        self.images
            .iter()
            .filter(|i| self.filter.is_none_or(|s| s == i.status))
            .map(|i| i.id)
            .collect()
    }
    fn load_thumbnails(&mut self) -> Task<Message> {
        let visible = self.visible();
        let pages = visible.len().div_ceil(view::PAGE_SIZE).max(1);
        self.page = self.page.min(pages - 1);
        let ids: HashSet<_> = visible
            .into_iter()
            .skip(self.page * view::PAGE_SIZE)
            .take(view::PAGE_SIZE)
            .collect();
        let mut tasks = vec![];
        for image in &mut self.images {
            if !ids.contains(&image.id) {
                image.thumbnail = None;
                image.thumbnail_requested = false;
                continue;
            }
            if image.thumbnail_requested {
                continue;
            }
            image.thumbnail_requested = true;
            let (id, revision, path) = (image.id, image.revision, image.path.clone());
            let semaphore = DECODERS.clone();
            tasks.push(Task::perform(
                async move {
                    let _permit = semaphore.acquire_owned().await.map_err(|_| Error::Image)?;
                    tokio::task::spawn_blocking(move || files::decode(&path, true))
                        .await
                        .map_err(|_| Error::Image)?
                },
                move |r| Message::Thumbnail(id, revision, r),
            ));
        }
        Task::batch(tasks)
    }
    fn start(&mut self, only: Option<u64>) -> Task<Message> {
        if !self.can_compress() {
            return Task::none();
        }
        let Some(client) = self.client.clone() else {
            return Task::none();
        };
        self.run_id += 1;
        self.busy = true;
        self.run_items.clear();
        let all = self
            .images
            .iter()
            .map(|i| i.path.clone())
            .collect::<Vec<_>>();
        let mut items = vec![];
        for image in &mut self.images {
            if matches!(image.status, Status::Pending | Status::Failed)
                && only.is_none_or(|id| id == image.id)
            {
                image.status = Status::Pending;
                image.progress = 0.;
                image.result = None;
                self.run_items.insert(image.id);
                items.push((image.id, image.path.clone()));
            }
        }
        let options = SaveOptions {
            mode: self.mode,
            directory: self.output.clone(),
            common_root: files::common_root(&all),
            preserve: self.preserve,
        };
        let (tx, rx) = mpsc::channel(64);
        let pool = self.pool.clone();
        let run_id = self.run_id;
        tokio::spawn(queue::run(client, pool, items, options, tx));
        Task::run(
            futures_util::stream::unfold(rx, |mut rx| async { rx.recv().await.map(|e| (e, rx)) }),
            move |e| Message::Queue(run_id, e),
        )
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(result) => {
                self.key_busy = false;
                #[cfg(debug_assertions)]
                if std::env::var_os("TINYEASY_QA_DIR").is_some() {
                    let scene = std::env::var("TINYEASY_QA_SCENE").unwrap_or_default();
                    if scene == "empty" || scene == "error" {
                        self.modal = None;
                        if scene == "error" {
                            self.notify(Error::Network, ToastContext::General);
                        }
                        return Task::none();
                    }
                    if scene == "settings" {
                        self.modal = Some(Modal::Settings);
                        return Task::none();
                    }
                    if scene == "settings-error" {
                        self.modal = Some(Modal::Settings);
                        self.notify(Error::Configuration, ToastContext::General);
                        return Task::none();
                    }
                    if scene == "keys-error" {
                        self.open_keys();
                        return self.update(Message::SaveKeys);
                    }
                    if scene == "keys-usage-error" {
                        self.accounts = vec![Account {
                            key: String::new(),
                            note: String::new(),
                        }];
                        self.open_keys();
                        return self.update(Message::Validated(vec![Usage {
                            valid: false,
                            used: 0,
                            error: Some(Error::InvalidKey),
                        }]));
                    }
                    if scene == "toast-many" {
                        self.modal = None;
                        self.notify(Error::File, ToastContext::General);
                        self.notify(Error::Network, ToastContext::General);
                        self.notify(Error::Unsupported, ToastContext::General);
                        return Task::none();
                    }
                    if scene == "import" {
                        self.modal = Some(Modal::Import);
                        return Task::none();
                    }
                    if scene == "keys-many" {
                        self.open_keys();
                        self.draft = (0..50)
                            .map(|_| Draft {
                                account: Account {
                                    key: String::new(),
                                    note: String::new(),
                                },
                                visible: false,
                            })
                            .collect();
                        return Task::none();
                    }
                    if let Some(path) = std::env::var_os("TINYEASY_QA_IMPORT") {
                        self.modal = None;
                        return self.import(vec![PathBuf::from(path)]);
                    }
                }
                match result {
                    Ok(accounts) => {
                        self.accounts = accounts;
                        if self.accounts.is_empty() {
                            self.open_keys();
                        } else {
                            return self.validate();
                        }
                    }
                    Err(e) => {
                        self.notify(e, ToastContext::General);
                        self.open_keys();
                    }
                }
            }
            Message::Validated(usage) => {
                for (index, result) in usage.iter().enumerate() {
                    if let Some(error) = result.error {
                        self.notify(error, ToastContext::Key(index));
                    }
                }
                self.usage = usage;
                self.pool = KeyPool::new(self.accounts.clone(), self.usage.clone());
                self.key_busy = false;
            }
            Message::OpenKeys if !self.busy && !self.key_busy => match self.modal {
                Some(Modal::Settings) => self.modal = Some(Modal::Keys),
                Some(Modal::Keys) => {}
                _ => self.open_keys(),
            },
            Message::EditKey(i, value) if !self.key_busy => {
                if let Some(d) = self.draft.get_mut(i) {
                    d.account.key = value;
                }
            }
            Message::EditNote(i, value) if !self.key_busy => {
                if let Some(d) = self.draft.get_mut(i) {
                    d.account.note = value;
                }
            }
            Message::ToggleKey(i) => {
                if let Some(d) = self.draft.get_mut(i) {
                    d.visible = !d.visible;
                }
            }
            Message::AddKey if !self.key_busy && self.draft.len() < 50 => self.draft.push(Draft {
                account: Account {
                    key: String::new(),
                    note: String::new(),
                },
                visible: false,
            }),
            Message::RemoveKey(i) if !self.key_busy => {
                if i < self.draft.len() {
                    self.draft.remove(i);
                }
            }
            Message::SaveKeys if !self.busy && !self.key_busy => {
                let accounts = self
                    .draft
                    .iter()
                    .map(|d| d.account.clone())
                    .collect::<Vec<_>>();
                match credentials::normalize(&accounts) {
                    Err((index, error)) => self.notify(error, ToastContext::Key(index)),
                    Ok(accounts) => {
                        let Some(dir) = self.config_dir.clone() else {
                            self.notify(Error::Configuration, ToastContext::General);
                            return Task::none();
                        };
                        self.key_busy = true;
                        self.key_saving = true;
                        return Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    credentials::save(
                                        &dir.join("keys.enc"),
                                        &accounts,
                                        &SystemVault,
                                    )
                                })
                                .await
                                .unwrap_or(Err(Error::SecureStorage))
                            },
                            Message::KeysSaved,
                        );
                    }
                }
            }
            Message::KeysSaved(result) => {
                self.key_busy = false;
                self.key_saving = false;
                match result {
                    Ok(accounts) => {
                        self.accounts = accounts;
                        self.open_keys();
                        return self.validate();
                    }
                    Err(e) => self.notify(e, ToastContext::General),
                }
            }
            Message::CheckKeys if !self.busy && !self.key_busy => return self.validate(),
            Message::KeyError(index) => {
                if let Some(error) = self.usage.get(index).and_then(|usage| usage.error) {
                    self.notify(error, ToastContext::Key(index));
                }
            }
            Message::OpenSettings => {
                // Start a draft once per settings session; tab changes preserve edits.
                if !matches!(self.modal, Some(Modal::Keys | Modal::Settings)) {
                    self.open_keys();
                }
                self.modal = Some(Modal::Settings);
            }
            Message::Theme(mode) => {
                self.settings.theme = mode;
                return self.persist_settings();
            }
            Message::Language(lang) => {
                self.settings.language = lang;
                return self.persist_settings();
            }
            Message::SettingsSaved(saved, result) => {
                self.settings_saving = false;
                match result {
                    Err(e) => self.notify(e, ToastContext::General),
                    Ok(()) if saved != self.settings => return self.persist_settings(),
                    _ => {}
                }
            }
            Message::SystemTheme(mode) => self.system_dark = mode == iced::theme::Mode::Dark,
            Message::OpenImport if !self.busy && !self.importing && self.modal.is_none() => {
                self.modal = Some(Modal::Import);
            }
            Message::Import(folder) if !self.busy && !self.importing => {
                if self.modal == Some(Modal::Import) {
                    self.modal = None;
                }
                let lang = self.settings.language;
                return Task::perform(
                    async move {
                        let dialog = rfd::AsyncFileDialog::new().set_title(tr(
                            lang,
                            if folder {
                                "dialog.folder"
                            } else {
                                "dialog.images"
                            },
                        ));
                        if folder {
                            dialog
                                .pick_folder()
                                .await
                                .map(|h| vec![h.path().to_owned()])
                                .unwrap_or_default()
                        } else {
                            dialog
                                .add_filter("JPG · PNG · WebP", &["jpg", "jpeg", "png", "webp"])
                                .pick_files()
                                .await
                                .unwrap_or_default()
                                .into_iter()
                                .map(|h| h.path().to_owned())
                                .collect()
                        }
                    },
                    Message::Selected,
                );
            }
            Message::Selected(paths) => return self.import(paths),
            Message::ImportEvent(id, event) if id == self.import_id => match event {
                ImportEvent::Batch(batch) => {
                    let known = self
                        .images
                        .iter()
                        .map(|i| i.path.clone())
                        .collect::<HashSet<_>>();
                    for mut item in batch {
                        if !known.contains(&item.path) {
                            item.id = self.next_id;
                            self.next_id += 1;
                            self.images.push(item);
                        }
                    }
                    return self.load_thumbnails();
                }
                ImportEvent::Error(e) => self.notify(e, ToastContext::General),
                ImportEvent::Finished => {
                    self.importing = false;
                    #[cfg(debug_assertions)]
                    if std::env::var_os("TINYEASY_QA_DIR").is_some()
                        && std::env::var("TINYEASY_QA_SCENE").is_ok_and(|s| {
                            matches!(
                                s.as_str(),
                                "gallery" | "states" | "directory" | "filter-empty" | "done"
                            )
                        })
                    {
                        // Visual fixtures contain usage only, never credentials or live network access.
                        self.usage = vec![Usage {
                            valid: true,
                            used: 10,
                            error: None,
                        }];
                        if std::env::var("TINYEASY_QA_SCENE").is_ok_and(|s| s == "filter-empty") {
                            self.filter = Some(Status::Done);
                        }
                        if std::env::var("TINYEASY_QA_SCENE").is_ok_and(|s| s == "done") {
                            for item in &mut self.images {
                                self.run_items.insert(item.id);
                                item.status = Status::Done;
                                item.progress = 100.;
                                item.result = Some(Ok(crate::model::Output {
                                    path: item.path.clone(),
                                    before: item.bytes,
                                    after: item.bytes / 2,
                                }));
                            }
                        }
                        if std::env::var("TINYEASY_QA_SCENE").is_ok_and(|s| s == "directory") {
                            self.mode = SaveMode::Directory;
                            self.output = self
                                .config_dir
                                .as_ref()
                                .and_then(|p| p.parent())
                                .map(|p| p.join("Output 中文 images"));
                        }
                        if std::env::var("TINYEASY_QA_SCENE").is_ok_and(|s| s == "states") {
                            for (index, item) in self.images.iter_mut().enumerate() {
                                self.run_items.insert(item.id);
                                if index == 1 {
                                    item.status = Status::Done;
                                    item.progress = 100.;
                                    item.result = Some(Ok(crate::model::Output {
                                        path: item.path.clone(),
                                        before: item.bytes,
                                        after: item.bytes / 2,
                                    }));
                                }
                                if index == 2 {
                                    item.status = Status::Processing;
                                    item.progress = 63.;
                                }
                                if index == 0 {
                                    item.status = Status::Failed;
                                    item.progress = 100.;
                                    item.result = Some(Err(Error::Network));
                                }
                            }
                            let context = self
                                .images
                                .first()
                                .map(|item| ToastContext::Image(item.name()));
                            if let Some(context) = context {
                                self.notify(Error::Network, context);
                            }
                        }
                    }
                    #[cfg(debug_assertions)]
                    if std::env::var_os("TINYEASY_QA_DIR").is_some()
                        && std::env::var("TINYEASY_QA_SCENE").is_ok_and(|s| s == "preview")
                        && let Some(item) = self.images.first()
                    {
                        return self.update(Message::Preview(item.id));
                    }
                    let paths = std::mem::take(&mut self.pending_drops);
                    return Task::batch([self.import(paths), self.load_thumbnails()]);
                }
            },
            Message::Thumbnail(id, revision, result) => {
                let visible = self.visible();
                if visible
                    .iter()
                    .skip(self.page * view::PAGE_SIZE)
                    .take(view::PAGE_SIZE)
                    .any(|v| *v == id)
                    && let Some(item) = self
                        .images
                        .iter_mut()
                        .find(|i| i.id == id && i.revision == revision)
                {
                    item.thumbnail = result.ok();
                }
            }
            Message::Compress => return self.start(None),
            Message::Retry(id)
                if self
                    .images
                    .iter()
                    .any(|i| i.id == id && i.status == Status::Failed) =>
            {
                return self.start(Some(id));
            }
            Message::Queue(id, event) if id == self.run_id => {
                self.usage = self.pool.usage();
                match event {
                    queue::Event::Started(id) => {
                        if let Some(i) = self.images.iter_mut().find(|i| i.id == id) {
                            i.status = Status::Processing;
                            i.progress = 1.;
                        }
                    }
                    queue::Event::Progress(id, p) => {
                        if let Some(i) = self
                            .images
                            .iter_mut()
                            .find(|i| i.id == id && i.status == Status::Processing)
                        {
                            i.progress = i.progress.max(p);
                        }
                    }
                    queue::Event::Finished(id, result) => {
                        let mut failure = None;
                        if let Some(i) = self.images.iter_mut().find(|i| i.id == id) {
                            if let Err(error) = result {
                                failure = Some((error, ToastContext::Image(i.name())));
                            }
                            i.status = if result.is_ok() {
                                Status::Done
                            } else {
                                Status::Failed
                            };
                            i.progress = 100.;
                            if self.mode == SaveMode::Overwrite
                                && let Ok(output) = &result
                            {
                                i.revision += 1;
                                i.bytes = output.after;
                                i.thumbnail = None;
                                i.thumbnail_requested = false;
                            }
                            i.result = Some(result);
                        }
                        if let Some((error, context)) = failure {
                            self.notify(error, context);
                        }
                    }
                    queue::Event::Complete => self.busy = false,
                }
                return self.load_thumbnails();
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.page = 0;
                return self.load_thumbnails();
            }
            Message::Page(delta) => {
                self.page = self.page.saturating_add_signed(delta as isize);
                return self.load_thumbnails();
            }
            Message::Remove(id) if !self.busy && !self.importing => {
                self.images
                    .retain(|i| i.id != id || i.status == Status::Done);
                return self.load_thumbnails();
            }
            Message::Clear if !self.busy && !self.importing => {
                self.images.clear();
                self.run_items.clear();
                self.page = 0;
            }
            Message::Mode(mode) if !self.busy => {
                if mode == SaveMode::Directory && self.output.is_none() {
                    return self.choose_output();
                }
                self.mode = mode;
            }
            Message::ChooseOutput if !self.busy => return self.choose_output(),
            Message::OutputSelected(Some(path)) => {
                self.output = Some(path);
                self.mode = SaveMode::Directory;
            }
            Message::Preserve(value) if !self.busy => self.preserve = value,
            Message::OpenOutput => {
                if let Some(path) = self.output_folder().map(Path::to_path_buf) {
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                open::that(path).map_err(|_| Error::File)
                            })
                            .await
                            .unwrap_or(Err(Error::File))
                        },
                        Message::Opened,
                    );
                }
            }
            Message::Opened(Err(e)) => self.notify(e, ToastContext::General),
            Message::Preview(id) => {
                if let Some(item) = self.images.iter().find(|i| i.id == id) {
                    self.preview_generation += 1;
                    let generation = self.preview_generation;
                    self.preview_id = Some(id);
                    self.preview_handle = None;
                    self.preview_transform = Transform::default();
                    self.modal = Some(Modal::Preview);
                    let path = item.path.clone();
                    let semaphore = DECODERS.clone();
                    return Task::perform(
                        async move {
                            let _permit =
                                semaphore.acquire_owned().await.map_err(|_| Error::Image)?;
                            tokio::task::spawn_blocking(move || files::decode(&path, false))
                                .await
                                .unwrap_or(Err(Error::Image))
                        },
                        move |r| Message::PreviewLoaded(generation, id, r),
                    );
                }
            }
            Message::PreviewLoaded(generation, id, result)
                if generation == self.preview_generation
                    && self.preview_id == Some(id)
                    && self.modal == Some(Modal::Preview) =>
            {
                match result {
                    Ok(h) => self.preview_handle = Some(h),
                    Err(e) => {
                        let context = self
                            .images
                            .iter()
                            .find(|item| item.id == id)
                            .map(|item| ToastContext::Image(item.name()))
                            .unwrap_or(ToastContext::General);
                        self.notify(e, context);
                        self.modal = None;
                    }
                }
            }
            Message::Zoom(factor) => self.preview_transform.zoom(factor),
            Message::Pan(delta) => self.preview_transform.offset += delta,
            Message::ResetPreview => self.preview_transform = Transform::default(),
            Message::Details(id) => {
                if let Some(item) = self.images.iter().find(|item| item.id == id)
                    && let Some(Err(error)) = item.result
                {
                    let context = ToastContext::Image(item.name());
                    self.notify(error, context);
                } else {
                    self.modal = Some(Modal::Details(id));
                }
            }
            Message::CloseModal | Message::Escape
                if self.modal == Some(Modal::Import)
                    || !self.key_busy
                    || (self.modal == Some(Modal::Settings) && !self.key_saving) =>
            {
                self.modal = None;
                self.preview_generation += 1;
                self.preview_id = None;
                self.preview_handle = None;
                self.draft.clear();
            }
            Message::DismissToast(id) => self.notifications.dismiss(id),
            Message::ToastTick => self.notifications.expire(Instant::now()),
            Message::Drop(path) if !self.busy && self.modal.is_none() => {
                self.hover = false;
                self.pending_drops.push(path);
            }
            Message::Hover(value) => self.hover = value,
            Message::Tick if !self.pending_drops.is_empty() && !self.importing => {
                let paths = std::mem::take(&mut self.pending_drops);
                return self.import(paths);
            }
            Message::FocusNext(backward) => {
                return if backward {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::Shortcut(key) => {
                let message = match key.as_str() {
                    "," => Some(Message::OpenSettings),
                    "k" => Some(Message::OpenKeys),
                    "o" if self.modal.is_none() => Some(Message::Import(false)),
                    _ => None,
                };
                if let Some(message) = message {
                    return self.update(message);
                }
            }
            #[cfg(debug_assertions)]
            Message::QaWindow(id) => {
                return Task::perform(
                    async move {
                        let delay = std::env::var("TINYEASY_QA_CAPTURE_MS")
                            .ok()
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(800);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        id
                    },
                    Message::QaCapture,
                );
            }
            #[cfg(debug_assertions)]
            Message::QaCapture(id) => {
                return iced::window::screenshot(id).map(Message::QaScreenshot);
            }
            #[cfg(debug_assertions)]
            Message::QaScreenshot(screenshot) => {
                if let Some(path) = std::env::var_os("TINYEASY_QA_SCREENSHOT") {
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                image::save_buffer(
                                    PathBuf::from(path),
                                    &screenshot.rgba,
                                    screenshot.size.width,
                                    screenshot.size.height,
                                    image::ColorType::Rgba8,
                                )
                                .map_err(|_| Error::File)
                            })
                            .await
                            .unwrap_or(Err(Error::File))
                        },
                        Message::QaSaved,
                    );
                }
            }
            #[cfg(debug_assertions)]
            Message::QaSaved(result) => {
                if result.is_err() {
                    eprintln!("QA screenshot failed");
                } else {
                    println!("QA screenshot saved");
                }
                if std::env::var_os("TINYEASY_QA_EXIT").is_some() {
                    return iced::exit();
                }
            }
            Message::Brand(brand) => self.brand = brand,
            _ => {}
        }
        Task::none()
    }
    fn choose_output(&self) -> Task<Message> {
        let lang = self.settings.language;
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title(tr(lang, "dialog.output"))
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_owned())
            },
            Message::OutputSelected,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Output;
    fn state() -> State {
        State::boot(None).0
    }
    fn item(id: u64, status: Status) -> ImageItem {
        ImageItem {
            id,
            revision: 0,
            path: PathBuf::from(format!("test-{id}.png")),
            bytes: 100,
            width: 1,
            height: 1,
            thumbnail: None,
            thumbnail_requested: true,
            status,
            progress: 0.,
            result: None,
        }
    }
    #[test]
    fn output_folder_follows_save_mode_and_successful_results() {
        let mut s = state();
        let folder = PathBuf::from("chosen-output");
        s.output = Some(folder.clone());
        assert!(s.output_folder().is_none());

        s.mode = SaveMode::Directory;
        assert_eq!(s.output_folder(), Some(folder.as_path()));

        let original = PathBuf::from("original-images");
        let mut image = item(1, Status::Done);
        image.result = Some(Ok(Output {
            path: original.join("image.png"),
            before: 100,
            after: 50,
        }));
        s.images.push(image);
        s.mode = SaveMode::Overwrite;
        assert_eq!(s.output_folder(), Some(original.as_path()));

        s.images[0].result = Some(Err(Error::File));
        assert!(s.output_folder().is_none());
    }
    #[test]
    fn import_choice_can_be_cancelled_during_background_key_loading() {
        let mut s = state();
        assert!(s.key_busy);
        drop(s.update(Message::OpenImport));
        assert_eq!(s.modal, Some(Modal::Import));
        drop(s.update(Message::Escape));
        assert_eq!(s.modal, None);
        assert!(
            s.key_busy,
            "Cancelling import must not cancel credential loading"
        );
        assert!(!s.importing);
    }
    #[test]
    fn settings_tabs_preserve_unsaved_notes_and_an_empty_key_list() {
        let mut s = state();
        s.key_busy = false;
        s.accounts = vec![crate::tinify::tests::random_account()];
        drop(s.update(Message::OpenSettings));
        drop(s.update(Message::OpenKeys));
        drop(s.update(Message::EditNote(0, "Unsaved note".into())));
        drop(s.update(Message::OpenSettings));
        drop(s.update(Message::OpenKeys));
        assert_eq!(s.draft[0].account.note, "Unsaved note");

        drop(s.update(Message::RemoveKey(0)));
        drop(s.update(Message::OpenSettings));
        drop(s.update(Message::OpenKeys));
        assert!(s.draft.is_empty());
        assert_eq!(s.accounts.len(), 1, "Only Save changes active accounts");
    }

    #[test]
    fn pending_key_save_cannot_be_dismissed_from_the_general_tab() {
        let mut s = state();
        s.key_busy = false;
        drop(s.update(Message::OpenKeys));
        s.key_busy = true;
        s.key_saving = true;
        drop(s.update(Message::OpenSettings));
        assert_eq!(s.modal, Some(Modal::Settings));
        drop(s.update(Message::CloseModal));
        drop(s.update(Message::Escape));
        assert_eq!(s.modal, Some(Modal::Settings));
        assert_eq!(s.draft.len(), 1);
    }
    #[test]
    fn theme_and_language_switch_preserve_running_task_and_usage() {
        let mut s = state();
        s.busy = true;
        s.run_id = 7;
        s.images.push(item(1, Status::Processing));
        s.images[0].progress = 42.;
        s.usage = vec![Usage {
            valid: true,
            used: 10,
            error: None,
        }];
        drop(s.update(Message::Theme(Mode::Dark)));
        drop(s.update(Message::Language(Language::Chinese)));
        assert!(s.busy);
        assert_eq!(s.run_id, 7);
        assert_eq!(s.images[0].progress, 42.);
        assert_eq!(s.remaining(), 490);
        assert_eq!(s.settings.language, Language::Chinese);
    }
    #[test]
    fn stale_events_cannot_overwrite_newer_task_results() {
        let mut s = state();
        s.run_id = 2;
        s.images.push(item(1, Status::Done));
        drop(s.update(Message::Queue(
            1,
            queue::Event::Finished(1, Err(Error::Network)),
        )));
        assert_eq!(s.images[0].status, Status::Done);
        drop(s.update(Message::Queue(2, queue::Event::Progress(1, 3.))));
        assert_eq!(s.images[0].status, Status::Done);
    }
    #[test]
    fn failed_key_save_keeps_active_accounts_and_quota() {
        let mut s = state();
        let a = crate::tinify::tests::random_account();
        s.accounts = vec![a.clone()];
        s.usage = vec![Usage {
            valid: true,
            used: 11,
            error: None,
        }];
        drop(s.update(Message::KeysSaved(Err(Error::SecureStorage))));
        assert_eq!(s.accounts[0].key, a.key);
        assert_eq!(s.remaining(), 489);
    }
    #[test]
    fn filtering_does_not_change_task_states_and_completed_items_cannot_be_removed() {
        let mut s = state();
        s.images = vec![item(1, Status::Pending), item(2, Status::Done)];
        drop(s.update(Message::Filter(Some(Status::Done))));
        assert_eq!(s.visible(), vec![2]);
        assert_eq!(s.images[0].status, Status::Pending);
        drop(s.update(Message::Remove(2)));
        assert_eq!(s.images.len(), 2);
        s.busy = true;
        drop(s.update(Message::Clear));
        assert_eq!(s.images.len(), 2);
    }
    #[test]
    fn invalid_key_edits_and_storage_failures_notify_without_losing_the_draft() {
        let mut s = state();
        s.key_busy = false;
        drop(s.update(Message::OpenKeys));
        drop(s.update(Message::SaveKeys));
        assert!(
            s.notifications
                .items()
                .iter()
                .any(|toast| toast.error == Error::KeyFormat
                    && toast.context == ToastContext::Key(0))
        );
        assert_eq!(s.draft.len(), 1);

        drop(s.update(Message::KeysSaved(Err(Error::SecureStorage))));
        assert!(
            s.notifications
                .items()
                .iter()
                .any(|toast| toast.error == Error::SecureStorage)
        );
        assert_eq!(s.modal, Some(Modal::Keys));
        assert_eq!(s.draft.len(), 1);
    }
    #[test]
    fn validation_and_compression_failures_notify_with_the_affected_item() {
        let mut s = state();
        s.notifications = Default::default();
        drop(s.update(Message::Validated(vec![Usage {
            valid: false,
            used: 0,
            error: Some(Error::InvalidKey),
        }])));
        assert_eq!(s.notifications.items()[0].context, ToastContext::Key(0));
        s.notifications = Default::default();
        drop(s.update(Message::KeyError(0)));
        assert_eq!(s.notifications.items()[0].error, Error::InvalidKey);

        s.run_id = 1;
        s.images.push(item(2, Status::Processing));
        drop(s.update(Message::Queue(
            1,
            queue::Event::Finished(2, Err(Error::Network)),
        )));
        assert!(
            s.notifications
                .items()
                .iter()
                .any(|toast| toast.error == Error::Network
                    && toast.context == ToastContext::Image("test-2.png".into()))
        );
        assert_eq!(s.images[0].status, Status::Failed);

        s.notifications = Default::default();
        drop(s.update(Message::Details(2)));
        assert_eq!(s.notifications.items()[0].error, Error::Network);
        assert_eq!(s.modal, None);
    }
    #[test]
    fn completed_images_disable_compression_until_new_work_is_available() {
        let mut s = state();
        s.key_busy = false;
        s.usage = vec![Usage {
            valid: true,
            used: 10,
            error: None,
        }];
        s.images = vec![item(1, Status::Done), item(2, Status::Done)];
        assert!(
            !s.can_compress(),
            "Remaining quota must not re-enable completed images"
        );

        s.images.push(item(3, Status::Pending));
        assert!(s.can_compress());
        drop(s.update(Message::Filter(Some(Status::Done))));
        assert!(
            s.can_compress(),
            "Filtering must not hide queued work from the button"
        );

        s.images[2].status = Status::Failed;
        assert!(s.can_compress(), "Failed images remain retryable");
        s.images[2].status = Status::Done;
        assert!(!s.can_compress());
    }
    #[tokio::test]
    async fn a_new_batch_preserves_completed_images_and_only_queues_unfinished_work() {
        let mut s = state();
        s.key_busy = false;
        s.usage = vec![Usage {
            valid: true,
            used: 10,
            error: None,
        }];
        // The empty KeyPool cannot authorize a network request.
        s.pool = KeyPool::default();
        let mut completed = item(1, Status::Done);
        completed.progress = 100.;
        completed.result = Some(Ok(Output {
            path: PathBuf::from("completed.min.png"),
            before: 100,
            after: 50,
        }));
        s.images = vec![completed, item(2, Status::Pending), item(3, Status::Failed)];
        drop(s.start(None));
        assert_eq!(s.run_items, HashSet::from([2, 3]));
        assert_eq!(s.images[0].status, Status::Done);
        assert_eq!(s.images[0].progress, 100.);
        assert_eq!(
            s.images[0].result.as_ref().unwrap().as_ref().unwrap().after,
            50
        );
    }
    #[test]
    fn single_item_result_keeps_other_results() {
        let mut s = state();
        s.run_id = 9;
        s.images = vec![item(1, Status::Done), item(2, Status::Processing)];
        s.images[0].result = Some(Ok(Output {
            path: PathBuf::from("first.min.png"),
            before: 100,
            after: 50,
        }));
        drop(s.update(Message::Queue(
            9,
            queue::Event::Finished(2, Err(Error::Network)),
        )));
        assert!(s.images[0].result.as_ref().unwrap().is_ok());
        assert_eq!(s.images[1].status, Status::Failed);
    }
    #[test]
    fn first_launch_defaults_and_manual_system_theme_priority() {
        let mut s = state();
        assert_eq!(s.settings.language, Language::English);
        assert_eq!(s.settings.theme, Mode::System);
        drop(s.update(Message::SystemTheme(iced::theme::Mode::Dark)));
        assert!(s.settings.theme.is_dark(s.system_dark));
        drop(s.update(Message::Theme(Mode::Light)));
        drop(s.update(Message::SystemTheme(iced::theme::Mode::Dark)));
        assert!(!s.settings.theme.is_dark(s.system_dark));
    }
}

#[cfg(test)]
mod invalidation_tests {
    use super::*;
    #[test]
    fn obsolete_preview_failure_cannot_close_a_new_preview() {
        let mut state = State::boot(None).0;
        state.modal = Some(Modal::Preview);
        state.preview_id = Some(1);
        state.preview_generation = 2;
        drop(state.update(Message::PreviewLoaded(1, 1, Err(Error::Image))));
        assert_eq!(state.modal, Some(Modal::Preview));
        assert!(
            !state
                .notifications
                .items()
                .iter()
                .any(|toast| toast.error == Error::Image)
        );
    }
    #[test]
    fn obsolete_thumbnail_cannot_replace_new_image_revision() {
        let mut state = State::boot(None).0;
        state.images.push(ImageItem {
            id: 1,
            revision: 2,
            path: PathBuf::from("test.png"),
            bytes: 1,
            width: 1,
            height: 1,
            thumbnail: None,
            thumbnail_requested: true,
            status: Status::Done,
            progress: 100.,
            result: None,
        });
        let handle = Handle::from_rgba(1, 1, vec![0, 0, 0, 255]);
        drop(state.update(Message::Thumbnail(1, 1, Ok(handle.clone()))));
        assert!(state.images[0].thumbnail.is_none());
        drop(state.update(Message::Thumbnail(1, 2, Ok(handle))));
        assert!(state.images[0].thumbnail.is_some());
    }
}
