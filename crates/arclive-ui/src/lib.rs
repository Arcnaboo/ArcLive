//! ArcLive desktop UI. Depends on `arclive-core` only.

mod app;
mod theme;
mod widgets;

pub use app::{UiAction, UiApp, UiSnapshot};
pub use theme::ArcLiveTheme;
