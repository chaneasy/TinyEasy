#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod brand;
mod config;
mod credentials;
mod error;
mod files;
mod i18n;
mod model;
mod preview;
mod queue;
mod theme;
mod tinify;
mod ui;

fn main() -> iced::Result {
    ui::run()
}
