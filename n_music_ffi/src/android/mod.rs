//! What the core needs from Android besides the bindings: the JVM and the application context,
//! for cpal, which reads them through `ndk-context`, and for the Storage Access Framework.

mod documents;

use documents::AndroidDocumentProvider;
use jni::objects::{Global, JClass, JObject};
use jni::refs::Reference;
use jni::{EnvUnowned, JavaVM};
use n_music_core::source::Providers;
use std::ffi::{c_char, c_int, CStr, CString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;
use std::sync::{Arc, Mutex, OnceLock};

struct Android {
    jvm: Arc<JavaVM>,
    context: Global<JObject<'static>>,
}

static ANDROID: OnceLock<Android> = OnceLock::new();

/// Held while `NativeLibrary.init` runs: `ndk-context` panics when it is initialized twice.
static INIT: Mutex<()> = Mutex::new(());

/// `NativeLibrary.init(context)`: keeps the JVM and the application context for the native
/// side. Without `android_main` nothing else hands them over, and cpal needs them to list the
/// output devices. Later calls do nothing.
#[no_mangle]
pub extern "system" fn Java_com_enn3developer_n_1music_NativeLibrary_init<'local>(
    mut env: EnvUnowned<'local>,
    _: JClass<'local>,
    context: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let _init = INIT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if ANDROID.get().is_some() {
            return Ok(());
        }
        forward_stdio_to_logcat();
        let jvm = env.get_java_vm()?;
        let context = env.new_global_ref(&context)?;
        // SAFETY: both pointers stay valid for the life of the process: the JVM never unloads
        // and the global reference is kept in `ANDROID`, never deleted.
        unsafe {
            ndk_context::initialize_android_context(jvm.get_raw().cast(), context.as_raw().cast());
        }
        let _ = ANDROID.set(Android {
            jvm: Arc::new(jvm),
            context,
        });
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

/// Attaches the calling thread to the JVM until it ends.
pub fn attach_current_thread() {
    let Some(android) = ANDROID.get() else {
        return;
    };
    if let Err(error) = android
        .jvm
        .attach_current_thread(|_| Ok::<_, jni::errors::Error>(()))
    {
        log::warn!("Could not attach the bus thread to the JVM: {error:?}");
    }
}

#[link(name = "log")]
extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

const ANDROID_LOG_INFO: c_int = 4;

/// Sends what the native side writes to stdout and stderr to logcat, under the tag the Slint
/// app's android-activity used: the logger's copy of each line, panics, `eprintln!`. Android
/// points both at /dev/null.
fn forward_stdio_to_logcat() {
    let mut fds: [c_int; 2] = [0; 2];
    // SAFETY: plain calls on descriptors this function creates. The write end lives on as
    // stdout and stderr, the read end in the `File` the forwarding thread owns.
    let output = unsafe {
        if libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) != 0 {
            return;
        }
        libc::dup2(fds[1], libc::STDOUT_FILENO);
        libc::dup2(fds[1], libc::STDERR_FILENO);
        libc::close(fds[1]);
        File::from_raw_fd(fds[0])
    };
    let forwarding = std::thread::Builder::new()
        .name(String::from("stdio to logcat"))
        .spawn(move || {
            const TAG: &CStr = c"RustStdoutStderr";
            let mut output = BufReader::new(output);
            let mut line = Vec::new();
            loop {
                line.clear();
                match output.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                if line.last() == Some(&b'\n') {
                    line.pop();
                }
                line.retain(|&byte| byte != 0);
                if let Ok(text) = CString::new(std::mem::take(&mut line)) {
                    // SAFETY: both are valid NUL-terminated strings for the call.
                    unsafe { __android_log_write(ANDROID_LOG_INFO, TAG.as_ptr(), text.as_ptr()) };
                }
            }
        });
    if let Err(error) = forwarding {
        log::warn!("Could not forward stdout and stderr to logcat: {error}");
    }
}

/// The document provider for the folders picked through the Storage Access Framework.
pub fn providers() -> Providers {
    let android = ANDROID
        .get()
        .expect("NativeLibrary.init must run before Core.start");
    match AndroidDocumentProvider::new(android.jvm.clone(), &android.context) {
        Ok(documents) => Providers::default().with_documents(documents),
        Err(error) => {
            log::error!("Could not access the Android content resolver: {error:?}");
            Providers::default()
        }
    }
}
