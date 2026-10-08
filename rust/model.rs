use crate::{
    error::Error,
    i18n::{Language, tr},
};
use iced::widget::image::Handle;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Pending,
    Processing,
    Done,
    Failed,
}
impl Status {
    pub fn label(self, lang: Language) -> &'static str {
        tr(
            lang,
            match self {
                Self::Pending => "status.pending",
                Self::Processing => "status.processing",
                Self::Done => "status.done",
                Self::Failed => "status.failed",
            },
        )
    }
}
#[derive(Debug, Clone)]
pub struct ImageItem {
    pub id: u64,
    pub revision: u64,
    pub path: PathBuf,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    pub thumbnail: Option<Handle>,
    pub thumbnail_requested: bool,
    pub status: Status,
    pub progress: f32,
    pub result: Option<Result<Output, Error>>,
}
impl ImageItem {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
}
#[derive(Debug, Clone)]
pub struct Output {
    pub path: PathBuf,
    pub before: u64,
    pub after: u64,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SaveMode {
    #[default]
    Overwrite,
    Directory,
}
#[derive(Debug, Clone)]
pub struct SaveOptions {
    pub mode: SaveMode,
    pub directory: Option<PathBuf>,
    pub common_root: Option<PathBuf>,
    pub preserve: bool,
}
pub fn bytes(value: u64) -> String {
    if value < 1024 {
        format!("{value} B")
    } else if value < 1024 * 1024 {
        format!("{:.1} KiB", value as f64 / 1024.)
    } else {
        format!("{:.2} MiB", value as f64 / (1024. * 1024.))
    }
}
