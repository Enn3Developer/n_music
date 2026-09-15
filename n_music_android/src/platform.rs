use n_music_core::platform::Platform;
use std::path::PathBuf;

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
}

impl Platform for AndroidPlatform {
    fn set_clipboard_text(&self, text: String) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                let java_string = env.new_string(text)?;
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("set_clipboard_text"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[(&java_string).into()],
                )
                .inspect_err(|error| {
                    log_jni_error(env, "MainActivity.set_clipboard_text", error)
                })?;
                Ok(())
            })
            .expect("JNI call MainActivity.set_clipboard_text failed");
    }

    fn open_link(&self, link: String) {
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

    fn internal_dir(&self) -> PathBuf {
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

    fn ask_music_dir(&self, tag: u64, _writer: n_event_bus::EventWriter) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("askDirectory"),
                    jni::jni_sig!("(J)V"),
                    &[jni::objects::JValue::Long(tag as i64)],
                )
                .inspect_err(|error| log_jni_error(env, "MainActivity.askDirectory", error))?;
                Ok(())
            })
            .expect("JNI call MainActivity.askDirectory failed");
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
