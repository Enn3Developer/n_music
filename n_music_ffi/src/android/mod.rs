//! What the core needs from Android besides the bindings: the JVM and the application context,
//! for cpal, which reads them through `ndk-context`, and for the Storage Access Framework.

mod documents;

use documents::AndroidDocumentProvider;
use jni::objects::{Global, JClass, JObject};
use jni::refs::Reference;
use jni::{EnvUnowned, JavaVM};
use n_music_core::source::Providers;
use std::sync::{Arc, OnceLock};

struct Android {
    jvm: Arc<JavaVM>,
    context: Global<JObject<'static>>,
}

static ANDROID: OnceLock<Android> = OnceLock::new();

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
        if ANDROID.get().is_some() {
            return Ok(());
        }
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
