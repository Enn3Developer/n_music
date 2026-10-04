use n_event_bus::{EventWriter, Job, JobToken};
use std::path::PathBuf;
use std::sync::Arc;

pub type NativePlatform = AndroidPlatform;

pub struct AndroidPlatform {
    app: slint::android::AndroidApp,
    jvm: std::sync::Arc<jni::JavaVM>,
    callback: std::sync::Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
}

impl AndroidPlatform {
    pub fn new(
        app: slint::android::AndroidApp,
        jvm: std::sync::Arc<jni::JavaVM>,
        callback: std::sync::Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    ) -> Self {
        Self { app, jvm, callback }
    }

    pub fn jni_handles(
        &self,
    ) -> (
        std::sync::Arc<jni::JavaVM>,
        std::sync::Arc<jni::objects::Global<jni::objects::JObject<'static>>>,
    ) {
        (self.jvm.clone(), self.callback.clone())
    }

    pub fn open_link(&self, link: String) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                let java_string = env.new_string(link)?;
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("openLink"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[(&java_string).into()],
                )
                .inspect_err(|error| log_jni_error(env, "MainActivity.openLink", error))?;
                Ok(())
            })
            .expect("JNI call MainActivity.openLink failed");
    }

    /// Where the app keeps its data.
    pub fn internal_dir(&self) -> PathBuf {
        let path = self
            .app
            .external_data_path()
            .or_else(|| self.app.internal_data_path())
            .expect("Android provided neither an external nor an internal data directory")
            .join("config/");
        if let Err(error) = std::fs::create_dir_all(&path) {
            log::error!(
                "Could not create Android configuration directory {}: {error}",
                path.display()
            );
        }
        path
    }

    /// Where the app keeps what can be rebuilt (covers).
    pub fn cache_dir(&self) -> PathBuf {
        let cache_dir = self
            .jvm
            .attach_current_thread(|env| -> jni::errors::Result<String> {
                let dir = env
                    .call_method(
                        self.callback.as_ref(),
                        jni::jni_str!("getCacheDir"),
                        jni::jni_sig!("()Ljava/io/File;"),
                        &[],
                    )
                    .inspect_err(|error| log_jni_error(env, "Context.getCacheDir", error))?
                    .l()?;
                let path = env
                    .call_method(
                        &dir,
                        jni::jni_str!("getAbsolutePath"),
                        jni::jni_sig!("()Ljava/lang/String;"),
                        &[],
                    )?
                    .l()?;
                jni::objects::JString::cast_local(env, path)?.try_to_string(env)
            })
            .map(PathBuf::from)
            .unwrap_or_else(|error| {
                log::error!("Could not get the Android cache directory: {error:?}");
                self.internal_dir().join("cache")
            });
        if let Err(error) = std::fs::create_dir_all(&cache_dir) {
            log::error!(
                "Could not create Android cache directory {}: {error}",
                cache_dir.display()
            );
        }
        cache_dir
    }

    /// Opens the folder picker; the activity answers through `gotDirectory`.
    fn pick_music_folder(&self) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("askDirectory"),
                    jni::jni_sig!("()V"),
                    &[],
                )
                .inspect_err(|error| log_jni_error(env, "MainActivity.askDirectory", error))?;
                Ok(())
            })
            .expect("JNI call MainActivity.askDirectory failed");
    }
}

/// Asks for a music folder; the pick makes it the library (see `gotDirectory`).
pub struct PickFolderJob(pub Arc<NativePlatform>);
impl Job for PickFolderJob {
    fn run(self, _: u64, _: EventWriter, _: Option<JobToken>) {
        self.0.pick_music_folder();
    }
}

pub struct OpenLinkJob(pub Arc<NativePlatform>, pub String);
impl Job for OpenLinkJob {
    fn run(self, _: u64, _: EventWriter, _: Option<JobToken>) {
        self.0.open_link(self.1);
    }
}

pub(crate) fn log_jni_error(env: &mut jni::Env<'_>, operation: &str, error: &jni::errors::Error) {
    let Some(exception) = env.exception_occurred() else {
        log::error!("{operation} failed: {error:?}");
        return;
    };
    env.exception_clear();
    let details = (|| -> jni::errors::Result<String> {
        let stack = env
            .call_static_method(
                jni::jni_str!("android/util/Log"),
                jni::jni_str!("getStackTraceString"),
                jni::jni_sig!("(Ljava/lang/Throwable;)Ljava/lang/String;"),
                &[(&exception).into()],
            )?
            .l()?;
        jni::objects::JString::cast_local(env, stack)?.try_to_string(env)
    })();
    env.exception_clear();
    match details {
        Ok(stack) => log::error!("{operation} failed: {error:?}\n{stack}"),
        Err(details_error) => log::error!(
            "{operation} failed: {error:?}; cannot read Java exception: {details_error:?}"
        ),
    }
    // Keep the original exception pending for the existing JNI error handling.
    let _ = env.throw(&exception);
}
