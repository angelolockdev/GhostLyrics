pub mod matcher;
pub mod watcher;

pub use watcher::CurrentMediaState;

#[cfg(windows)]
pub use watcher::windows_gsmtc::get_current_session_state;

#[cfg(not(windows))]
pub use watcher::fallback::get_current_session_state;
