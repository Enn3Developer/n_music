pub mod loader;
pub mod updater;
pub mod window;

use crate::app::{Messenger, Platform, Runner, Settings};
use flume::Receiver;

pub trait ModuleMessage {
    /// The Message enum the modules understand must always have a `Quit` option and should be handled gracefully
    fn quit() -> Self;
}

#[async_trait::async_trait]
/// Module trait
///
/// The graphical app uses various modules to handle everything: window, update, metadata loading, etc...
/// The core principle is: the core [crate::app::run_app] is a "glue" between all modules:
/// - it starts them by [Module::setup]
/// - handles communication between modules by exposing its [Messenger]
/// - handles communication from modules to [crate::runner::Runner]
/// - sends `Quit` messages when the app closes or a module request it
/// Every module has to register its Message enum, and it must have a `Quit` option available and the modules should handle it gracefully.
/// The core provides every module a [Receiver] with its correct Message enum, and [crate::platform::Platform], [crate::settings::Settings] and [crate::runner::Runner] to easily access necessary data
/// [window::WindowModule] is the only exception that needs [Module::start_sync] because of limitations caused by some OSes, all the other modules can ignore this function
pub trait Module<P: crate::platform::Platform + Send> {
    /// Message enum that the module understands
    type Message: ModuleMessage;
    /// Initial module setup
    async fn setup(
        platform: Platform<P>,
        settings: Settings,
        runner: Runner,
        messenger: Messenger<P>,
        rx: Receiver<Self::Message>,
    ) -> Self;
    fn is_async() -> bool {
        true
    }
    /// Starts the module, it should handle the loop, if necessary, and the quit message
    async fn start(&self);
    /// As above, sync version, used only by [window::WindowModule] because of Slint limitations
    fn start_sync(&self);
}
