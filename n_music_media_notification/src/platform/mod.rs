#[cfg(target_os = "linux")]
mod mpris;
#[cfg(target_os = "linux")]
pub(crate) use mpris::new_controls;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::new_controls;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use windows::new_controls;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub(crate) async fn new_controls(
    _emit: crate::state::Emit,
    _state: std::sync::Arc<std::sync::RwLock<crate::state::State>>,
) -> Option<Box<dyn crate::state::Backend>> {
    None
}
