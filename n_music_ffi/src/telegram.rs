//! Telegram's errors and chat kinds as the desktop's strings name them.

use n_music_core::source::telegram::{TelegramChatKind, TelegramError};

/// Why a Telegram step failed, as a string key and what fills its `%1`.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TelegramErrorText {
    /// The string's key, like `telegram_error_code_invalid`.
    pub key: String,
    /// What fills its `%1`: the minutes to wait, or why Telegram failed; empty otherwise.
    pub detail: String,
}

/// `error` as the desktop's string key for it and what fills its `%1`.
#[uniffi::export]
pub fn telegram_error_text(error: TelegramError) -> TelegramErrorText {
    let (code, detail) = match error {
        TelegramError::PhoneInvalid => ("phone_invalid", String::new()),
        TelegramError::PhoneBanned => ("phone_banned", String::new()),
        TelegramError::CodeInvalid => ("code_invalid", String::new()),
        TelegramError::PasswordInvalid => ("password_invalid", String::new()),
        TelegramError::SignUpRequired => ("sign_up_required", String::new()),
        // In whole minutes, rounded up.
        TelegramError::Wait { seconds } => ("wait", seconds.div_ceil(60).max(1).to_string()),
        TelegramError::SignedOut => ("signed_out", String::new()),
        TelegramError::NotFound => ("not_found", String::new()),
        TelegramError::Offline(why) => ("offline", why),
        TelegramError::Failed(why) => ("failed", why),
    };
    TelegramErrorText {
        key: format!("telegram_error_{code}"),
        detail,
    }
}

/// The desktop's string key for a chat of `kind`, like `telegram_kind_channel`.
#[uniffi::export]
pub fn telegram_chat_kind_key(kind: TelegramChatKind) -> String {
    let kind = match kind {
        TelegramChatKind::SavedMessages => "saved",
        TelegramChatKind::User => "user",
        TelegramChatKind::Bot => "bot",
        TelegramChatKind::Group => "group",
        TelegramChatKind::Channel => "channel",
    };
    format!("telegram_kind_{kind}")
}
