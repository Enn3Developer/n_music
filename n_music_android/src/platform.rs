use async_trait::async_trait;
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

#[async_trait]
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
                )?;
                Ok(())
            })
            .unwrap();
    }

    async fn open_link(&self, link: String) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                let java_string = env.new_string(link)?;
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("openLink"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[(&java_string).into()],
                )?;
                Ok(())
            })
            .unwrap();
    }

    async fn internal_dir(&self) -> PathBuf {
        let path = self
            .app
            .external_data_path()
            .expect("can't get external data path")
            .join("config/");
        if !path.exists() {
            std::fs::create_dir(&path).unwrap();
        }
        path
    }

    async fn ask_music_dir(&self, tag: u64, _writer: n_event_bus::EventWriter) {
        self.jvm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(
                    self.callback.as_ref(),
                    jni::jni_str!("askDirectory"),
                    jni::jni_sig!("(J)V"),
                    &[jni::objects::JValue::Long(tag as i64)],
                )?;
                Ok(())
            })
            .unwrap();
    }
}
