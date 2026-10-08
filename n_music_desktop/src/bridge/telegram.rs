//! Telegram: signing in and out, and finding the chats to add as sources.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// This build signs in to Telegram: it has N Music's API credentials.
        #[qproperty(bool, available)]
        /// Where signing in is: `signedOut`, `codeSent`, `passwordNeeded` or `signedIn`.
        #[qproperty(QString, status)]
        /// The account signed in to, while signed in.
        #[qproperty(QString, account)]
        /// The phone number the code went to, while one is awaited.
        #[qproperty(QString, phone)]
        /// What the account's password hint says, while the password is awaited.
        #[qproperty(QString, hint)]
        /// A step of signing in or out runs: the next waits for it.
        #[qproperty(bool, busy)]
        /// Why the last step failed, as the end of a `telegram_error_` string key, with
        /// `errorDetail` for its `%1`; empty when it did not.
        #[qproperty(QString, error)]
        #[qproperty(QString, error_detail)]
        /// The chats found for `query`, as `{ location, title, kind, username }`: `location`
        /// as `Sources.items` writes it, `kind` one of `saved`, `user`, `bot`, `group` and
        /// `channel`, `username` without the `@`, or empty.
        #[qproperty(QVariant, chats)]
        /// What `chats` were found for.
        #[qproperty(QString, query)]
        /// A search runs: `chats` still shows the answer before.
        #[qproperty(bool, searching)]
        /// Why the search found nothing, like `error`.
        #[qproperty(QString, search_error)]
        #[qproperty(QString, search_error_detail)]
        type Telegram = super::TelegramRust;

        /// Asks Telegram to send a code to sign in with to `phone`.
        #[qinvokable]
        fn send_phone(self: &Telegram, phone: &QString);
        /// Signs in with the `code` Telegram sent.
        #[qinvokable]
        fn send_code(self: &Telegram, code: &QString);
        /// Signs in with the account's `password`.
        #[qinvokable]
        fn send_password(self: &Telegram, password: &QString);
        /// Signs out, or stops signing in.
        #[qinvokable]
        fn sign_out(self: &Telegram);
        /// Looks for chats: the account's with `query` in their name, and the public one it
        /// names as `@name` or a `t.me` link; an empty query lists the account's chats.
        #[qinvokable]
        fn find(self: Pin<&mut Telegram>, query: &QString);
    }

    impl cxx_qt::Threading for Telegram {}
    impl cxx_qt::Initialize for Telegram {}
}

use crate::bus;
use crate::hub::{hub, Changed, TelegramChats};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use n_music_core::messages::{
    FindTelegramChats, TelegramCode, TelegramPassword, TelegramSignIn, TelegramSignOut,
};
use n_music_core::source::telegram::{TelegramChatKind, TelegramError, TelegramStatus};
use std::sync::Arc;

pub struct TelegramRust {
    available: bool,
    status: QString,
    account: QString,
    phone: QString,
    hint: QString,
    busy: bool,
    error: QString,
    error_detail: QString,
    chats: QVariant,
    query: QString,
    searching: bool,
    search_error: QString,
    search_error_detail: QString,
    /// The query of the search asked for last.
    asked: Option<String>,
    /// A search was asked for that no answer came for yet.
    pending: bool,
    /// The answer `chats` shows, to tell a new one from it.
    shown: Option<Arc<TelegramChats>>,
}

impl Default for TelegramRust {
    fn default() -> Self {
        Self {
            available: false,
            status: QString::from("signedOut"),
            account: QString::default(),
            phone: QString::default(),
            hint: QString::default(),
            busy: false,
            error: QString::default(),
            error_detail: QString::default(),
            // A list from the start, so QML can go through it before chats arrive.
            chats: QVariant::from(&QList::<QVariant>::default()),
            query: QString::default(),
            searching: false,
            search_error: QString::default(),
            search_error_detail: QString::default(),
            asked: None,
            pending: false,
            shown: None,
        }
    }
}

impl cxx_qt::Initialize for qobject::Telegram {
    fn initialize(mut self: Pin<&mut Self>) {
        hub().watch(self.qt_thread(), Changed::TELEGRAM, |telegram, _| {
            telegram.load()
        });
        self.as_mut().load();
    }
}

impl qobject::Telegram {
    fn load(mut self: Pin<&mut Self>) {
        let (telegram, found) = {
            let state = hub().state();
            (state.telegram.clone(), state.telegram_chats.clone())
        };
        let Some(telegram) = telegram else {
            return;
        };
        let (status, account, phone, hint) = match &telegram.status {
            TelegramStatus::SignedOut => ("signedOut", "", "", ""),
            TelegramStatus::CodeSent { phone } => ("codeSent", "", phone.as_str(), ""),
            TelegramStatus::PasswordNeeded { hint } => (
                "passwordNeeded",
                "",
                "",
                hint.as_deref().unwrap_or_default(),
            ),
            TelegramStatus::SignedIn { name } => ("signedIn", name.as_str(), "", ""),
        };
        let (error, error_detail) = describe(telegram.error.as_ref());
        self.as_mut().set_available(true);
        self.as_mut().set_status(QString::from(status));
        self.as_mut().set_account(QString::from(account));
        self.as_mut().set_phone(QString::from(phone));
        self.as_mut().set_hint(QString::from(hint));
        self.as_mut().set_error(QString::from(error));
        self.as_mut().set_error_detail(QString::from(&error_detail));
        self.as_mut().set_busy(telegram.busy);
        let new = found.filter(|found| {
            let shown = self.shown.as_ref();
            !shown.is_some_and(|shown| Arc::ptr_eq(shown, found))
        });
        if let Some(found) = new {
            let mut chats = QList::<QVariant>::default();
            for chat in &found.chats {
                let mut item = QMap::<QMapPair_QString_QVariant>::default();
                let text = |value: &str| QVariant::from(&QString::from(value));
                item.insert(QString::from("location"), text(&chat.locator.to_string()));
                item.insert(QString::from("title"), text(&chat.title));
                item.insert(QString::from("kind"), text(kind(chat.kind)));
                item.insert(
                    QString::from("username"),
                    text(chat.username.as_deref().unwrap_or_default()),
                );
                chats.append(QVariant::from(&item));
            }
            let (search_error, search_error_detail) = describe(found.error.as_ref());
            self.as_mut().set_chats(QVariant::from(&chats));
            self.as_mut().set_query(QString::from(&found.query));
            self.as_mut().set_search_error(QString::from(search_error));
            self.as_mut()
                .set_search_error_detail(QString::from(&search_error_detail));
            let mut rust = self.as_mut().rust_mut();
            if rust.asked.as_ref() == Some(&found.query) {
                rust.pending = false;
            }
            rust.shown = Some(found);
            let pending = rust.pending;
            self.set_searching(pending);
        }
    }

    fn send_phone(&self, phone: &QString) {
        bus::emit(TelegramSignIn {
            phone: phone.to_string(),
        });
    }

    fn send_code(&self, code: &QString) {
        bus::emit(TelegramCode(code.to_string()));
    }

    fn send_password(&self, password: &QString) {
        bus::emit(TelegramPassword(password.to_string()));
    }

    fn sign_out(&self) {
        bus::emit(TelegramSignOut);
    }

    fn find(mut self: Pin<&mut Self>, query: &QString) {
        let query = query.to_string().trim().to_string();
        let mut rust = self.as_mut().rust_mut();
        rust.asked = Some(query.clone());
        rust.pending = true;
        self.set_searching(true);
        bus::emit(FindTelegramChats { query });
    }
}

/// `error` as the end of a `telegram_error_` string key, and what fills its `%1`.
fn describe(error: Option<&TelegramError>) -> (&'static str, String) {
    let Some(error) = error else {
        return ("", String::new());
    };
    match error {
        TelegramError::PhoneInvalid => ("phone_invalid", String::new()),
        TelegramError::PhoneBanned => ("phone_banned", String::new()),
        TelegramError::CodeInvalid => ("code_invalid", String::new()),
        TelegramError::PasswordInvalid => ("password_invalid", String::new()),
        TelegramError::SignUpRequired => ("sign_up_required", String::new()),
        TelegramError::Wait { seconds } => ("wait", minutes(*seconds)),
        TelegramError::SignedOut => ("signed_out", String::new()),
        TelegramError::NotFound => ("not_found", String::new()),
        TelegramError::Offline(why) => ("offline", why.clone()),
        TelegramError::Failed(why) => ("failed", why.clone()),
    }
}

/// `seconds` rounded up to whole minutes.
fn minutes(seconds: u32) -> String {
    seconds.div_ceil(60).max(1).to_string()
}

fn kind(kind: TelegramChatKind) -> &'static str {
    match kind {
        TelegramChatKind::SavedMessages => "saved",
        TelegramChatKind::User => "user",
        TelegramChatKind::Bot => "bot",
        TelegramChatKind::Group => "group",
        TelegramChatKind::Channel => "channel",
    }
}
