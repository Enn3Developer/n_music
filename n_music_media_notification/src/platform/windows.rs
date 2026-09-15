use crate::state::{Backend, Change, Emit, MediaEvent, State};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use windows::core::{factory, w, HSTRING};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs,
    SystemMediaTransportControls, SystemMediaTransportControlsButton,
    SystemMediaTransportControlsButtonPressedEventArgs, SystemMediaTransportControlsDisplayUpdater,
    SystemMediaTransportControlsTimelineProperties,
};
use windows::Storage::StorageFile;
use windows::Storage::Streams::RandomAccessStreamReference;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::System::WinRT::{
    ISystemMediaTransportControlsInterop, RoInitialize, RO_INIT_MULTITHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, PostThreadMessageW,
    TranslateMessage, MSG, WINDOW_EX_STYLE, WM_APP, WM_QUIT, WS_OVERLAPPED,
};

/// Seconds the fast-forward/rewind buttons seek by.
const SEEK_STEP: f64 = 10.0;

const COMMAND_MESSAGE: u32 = WM_APP + 1;

enum Command {
    Status {
        playing: bool,
        position: f64,
        length: f64,
    },
    Timeline {
        position: f64,
        length: f64,
    },
    Metadata {
        title: String,
        artist: String,
        cover: Option<PathBuf>,
    },
}

type Startup = Result<(flume::Sender<Command>, u32), String>;

pub(crate) fn new_controls(emit: Emit, state: Arc<RwLock<State>>) -> Option<Box<dyn Backend>> {
    Some(Box::new(Smtc::new(emit, state)?))
}

/// Raw Windows System Media Transport Controls backend.
///
/// SMTC is thread-affine, so all WinRT calls live on a dedicated thread with a
/// message loop pumping a hidden window.
pub(crate) struct Smtc {
    commands: flume::Sender<Command>,
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Smtc {
    pub(crate) fn new(emit: Emit, _state: Arc<RwLock<State>>) -> Option<Self> {
        let (startup_tx, startup_rx) = flume::bounded::<Startup>(1);
        let thread = std::thread::Builder::new()
            .name(String::from("n_music_smtc"))
            .spawn(move || smtc_thread(emit, &startup_tx))
            .ok()?;

        match startup_rx.recv() {
            Ok(Ok((commands, thread_id))) => Some(Self {
                commands,
                thread_id,
                thread: Some(thread),
            }),
            _ => None,
        }
    }
}

impl Backend for Smtc {
    fn update(&self, state: &State, change: Change) {
        let command = match change {
            Change::Playback => Command::Status {
                playing: state.playing,
                position: state.position,
                length: state.length,
            },
            Change::Position {
                discontinuity: true,
            } => Command::Timeline {
                position: state.position,
                length: state.length,
            },
            Change::Position {
                discontinuity: false,
            } => return,
            Change::Metadata => Command::Metadata {
                title: state.title.clone(),
                artist: state.artist.clone(),
                cover: state.cover.clone(),
            },
            Change::Volume | Change::Loop => return,
        };

        if self.commands.send(command).is_ok() {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, COMMAND_MESSAGE, WPARAM(0), LPARAM(0));
            }
        }
    }
}

impl Drop for Smtc {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Context {
    controls: SystemMediaTransportControls,
    updater: SystemMediaTransportControlsDisplayUpdater,
    timeline: SystemMediaTransportControlsTimelineProperties,
    window: HWND,
}

fn smtc_thread(emit: Emit, startup: &flume::Sender<Startup>) {
    let (commands, receiver) = flume::unbounded::<Command>();
    let context = match initialize(&emit) {
        Ok(context) => context,
        Err(error) => {
            let _ = startup.send(Err(error));
            return;
        }
    };
    let thread_id = unsafe { GetCurrentThreadId() };
    if startup.send(Ok((commands, thread_id))).is_err() {
        unsafe {
            let _ = DestroyWindow(context.window);
        }
        return;
    }
    run_loop(&context, &receiver);
}

fn initialize(emit: &Emit) -> Result<Context, String> {
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED).map_err(|error| error.to_string())?;

        let module = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("N Music"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(HINSTANCE(module.0)),
            None,
        )
        .map_err(|error| error.to_string())?;

        match initialize_controls(window, emit) {
            Ok((controls, updater, timeline)) => Ok(Context {
                controls,
                updater,
                timeline,
                window,
            }),
            Err(error) => {
                let _ = DestroyWindow(window);
                Err(error)
            }
        }
    }
}

type Controls = (
    SystemMediaTransportControls,
    SystemMediaTransportControlsDisplayUpdater,
    SystemMediaTransportControlsTimelineProperties,
);

unsafe fn initialize_controls(window: HWND, emit: &Emit) -> Result<Controls, String> {
    unsafe {
        let interop: ISystemMediaTransportControlsInterop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
                .map_err(|error| error.to_string())?;
        let controls: SystemMediaTransportControls = interop
            .GetForWindow(window)
            .map_err(|error| error.to_string())?;

        for result in [
            controls.SetIsEnabled(true),
            controls.SetIsPlayEnabled(true),
            controls.SetIsPauseEnabled(true),
            controls.SetIsNextEnabled(true),
            controls.SetIsPreviousEnabled(true),
            controls.SetIsFastForwardEnabled(true),
            controls.SetIsRewindEnabled(true),
        ] {
            result.map_err(|error| error.to_string())?;
        }

        let updater = controls
            .DisplayUpdater()
            .map_err(|error| error.to_string())?;
        updater
            .SetType(MediaPlaybackType::Music)
            .map_err(|error| error.to_string())?;

        let button_emit = emit.clone();
        let button_handler = TypedEventHandler::new(
            move |_sender,
                  args: windows::core::Ref<
                '_,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >| {
                let button = args.ok()?.Button()?;
                let event = if button == SystemMediaTransportControlsButton::Play {
                    MediaEvent::Play
                } else if button == SystemMediaTransportControlsButton::Pause {
                    MediaEvent::Pause
                } else if button == SystemMediaTransportControlsButton::Next {
                    MediaEvent::Next
                } else if button == SystemMediaTransportControlsButton::Previous {
                    MediaEvent::Previous
                } else if button == SystemMediaTransportControlsButton::FastForward {
                    MediaEvent::SeekRelative(SEEK_STEP)
                } else if button == SystemMediaTransportControlsButton::Rewind {
                    MediaEvent::SeekRelative(-SEEK_STEP)
                } else {
                    return Ok(());
                };
                button_emit(event);
                Ok(())
            },
        );
        controls
            .ButtonPressed(&button_handler)
            .map_err(|error| error.to_string())?;

        let position_emit = emit.clone();
        let position_handler = TypedEventHandler::new(
            move |_sender,
                  args: windows::core::Ref<'_, PlaybackPositionChangeRequestedEventArgs>| {
                let position = Duration::from(args.ok()?.RequestedPlaybackPosition()?);
                position_emit(MediaEvent::SeekAbsolute(position.as_secs_f64()));
                Ok(())
            },
        );
        controls
            .PlaybackPositionChangeRequested(&position_handler)
            .map_err(|error| error.to_string())?;

        let timeline = SystemMediaTransportControlsTimelineProperties::new()
            .map_err(|error| error.to_string())?;

        Ok((controls, updater, timeline))
    }
}

fn run_loop(context: &Context, receiver: &flume::Receiver<Command>) {
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 == 0 || result.0 == -1 {
            break;
        }
        if message.message == COMMAND_MESSAGE {
            while let Ok(command) = receiver.try_recv() {
                apply(context, command);
            }
        } else {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    unsafe {
        let _ = DestroyWindow(context.window);
    }
}

fn apply(context: &Context, command: Command) {
    match command {
        Command::Status {
            playing,
            position,
            length,
        } => {
            let status = if playing {
                MediaPlaybackStatus::Playing
            } else {
                MediaPlaybackStatus::Paused
            };
            let _ = context.controls.SetPlaybackStatus(status);
            set_timeline(context, position, length);
        }
        Command::Timeline { position, length } => set_timeline(context, position, length),
        Command::Metadata {
            title,
            artist,
            cover,
        } => {
            let _ = context.updater.ClearAll();
            let _ = context.updater.SetType(MediaPlaybackType::Music);
            if let Ok(properties) = context.updater.MusicProperties() {
                let _ = properties.SetTitle(&HSTRING::from(title));
                if !artist.is_empty() {
                    let _ = properties.SetArtist(&HSTRING::from(artist));
                }
            }
            if let Some(cover) = cover {
                if let Ok(stream) = thumbnail(&cover) {
                    let _ = context.updater.SetThumbnail(&stream);
                }
            }
            let _ = context.updater.Update();
        }
    }
}

fn set_timeline(context: &Context, position: f64, length: f64) {
    let length = Duration::from_secs_f64(length.max(0.0));
    let position = Duration::from_secs_f64(position.max(0.0));
    let _ = context.timeline.SetStartTime(TimeSpan::default());
    let _ = context.timeline.SetMinSeekTime(TimeSpan::default());
    let _ = context.timeline.SetMaxSeekTime(TimeSpan::from(length));
    let _ = context.timeline.SetEndTime(TimeSpan::from(length));
    let _ = context.timeline.SetPosition(TimeSpan::from(position));
    let _ = context.controls.UpdateTimelineProperties(&context.timeline);
}

fn thumbnail(path: &Path) -> windows::core::Result<RandomAccessStreamReference> {
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path))?.join()?;
    RandomAccessStreamReference::CreateFromFile(&file)
}
