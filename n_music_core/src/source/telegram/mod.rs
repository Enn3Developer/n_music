//! Chats on Telegram: a channel, a group or Saved Messages is a library, the audio files sent in
//! it its tracks.
//!
//! Chats are known by their Bot API style id, which tells users, groups and channels apart:
//! their names change, and private ones have none.
//!
//! With the `telegram` feature, a [`TelegramAccount`] signs in to Telegram; the engine signs in
//! and out on the bus when [`crate::source::Providers::with_telegram`] was given one.

#[cfg(feature = "telegram")]
mod account;
#[cfg(feature = "telegram")]
pub(crate) mod service;
#[cfg(feature = "telegram")]
mod session;

#[cfg(feature = "telegram")]
pub use account::{TelegramAccount, TelegramCredentials};

use super::Locator;
use std::fmt::{self, Display, Formatter};

/// Where signing in to Telegram is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelegramStatus {
    /// No account: signing in starts with a phone number.
    SignedOut,
    /// Telegram sent a code to `phone`, in the Telegram app or by SMS.
    CodeSent { phone: String },
    /// The account has a password, which `hint` may help remember.
    PasswordNeeded { hint: Option<String> },
    /// Signed in to the account called `name`.
    SignedIn { name: String },
}

/// Why a request to Telegram failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelegramError {
    /// The phone number is not one Telegram knows.
    PhoneInvalid,
    /// Telegram banned the phone number.
    PhoneBanned,
    /// The code is wrong, or it expired.
    CodeInvalid,
    /// The password is wrong.
    PasswordInvalid,
    /// The phone number has no account yet: it is made in a Telegram app first.
    SignUpRequired,
    /// Too many attempts: Telegram takes the next one in `seconds`.
    Wait { seconds: u32 },
    /// Not signed in, or signed out elsewhere.
    SignedOut,
    /// The chat or the message is not there, or not any more.
    NotFound,
    /// Telegram could not be reached.
    Offline(String),
    /// Anything else, as Telegram tells it.
    Failed(String),
}

impl Display for TelegramError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TelegramError::PhoneInvalid => f.write_str("The phone number is invalid"),
            TelegramError::PhoneBanned => f.write_str("The phone number is banned"),
            TelegramError::CodeInvalid => f.write_str("The code is invalid or expired"),
            TelegramError::PasswordInvalid => f.write_str("The password is wrong"),
            TelegramError::SignUpRequired => f.write_str("The phone number has no account"),
            TelegramError::Wait { seconds } => write!(f, "Too many attempts: wait {seconds}s"),
            TelegramError::SignedOut => f.write_str("Not signed in to Telegram"),
            TelegramError::NotFound => f.write_str("Not found on Telegram"),
            TelegramError::Offline(why) | TelegramError::Failed(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for TelegramError {}

impl From<TelegramError> for std::io::Error {
    /// Of a kind that a download gives up on when trying again would not help.
    fn from(error: TelegramError) -> Self {
        use std::io::ErrorKind;
        let kind = match &error {
            TelegramError::SignedOut => ErrorKind::PermissionDenied,
            TelegramError::NotFound => ErrorKind::NotFound,
            TelegramError::Offline(_) => ErrorKind::TimedOut,
            _ => ErrorKind::Other,
        };
        std::io::Error::new(kind, error)
    }
}

/// A Telegram chat to make a library of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelegramChatInfo {
    /// A [`Locator::TelegramChat`].
    pub locator: Locator,
    /// Its name; empty for Saved Messages, which has none of its own.
    pub title: String,
    pub kind: TelegramChatKind,
    /// Its public username, without the `@`.
    pub username: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelegramChatKind {
    /// The account's own chat.
    SavedMessages,
    User,
    Bot,
    Group,
    Channel,
}

/// What the locators of chats and their audio files start with.
const SCHEME: &str = "telegram:";
/// Message ids are written this wide, so that the text order of a chat's tracks is the order
/// they were sent in.
const MESSAGE_WIDTH: usize = 10;

/// The text of the [`super::Locator::TelegramChat`] of `chat`: `telegram:<chat>`.
pub(super) fn chat_uri(chat: i64) -> String {
    format!("{SCHEME}{chat}")
}

/// The text of the [`super::Locator::TelegramAudio`] sent as `message` in `chat`:
/// `telegram:<chat>/<message>`.
pub(super) fn audio_uri(chat: i64, message: i32) -> String {
    format!("{SCHEME}{chat}/{message:0MESSAGE_WIDTH$}")
}

/// The chat `uri` is of, and its message when it is an audio file's.
pub(super) fn parse(uri: &str) -> Option<(i64, Option<i32>)> {
    let rest = uri.strip_prefix(SCHEME)?;
    let (chat, message) = match rest.split_once('/') {
        Some((chat, message)) => (chat, Some(message)),
        None => (rest, None),
    };
    let chat = number(chat)?;
    let message = match message {
        Some(message) => Some(number(message).filter(|&message| message > 0)?),
        None => None,
    };
    Some((chat, message))
}

/// `text` as a number written in ASCII digits, with a leading `-` for a negative one.
fn number<T: std::str::FromStr>(text: &str) -> Option<T> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_round_trip() {
        let chat = -1001234567890;
        assert_eq!(chat_uri(chat), "telegram:-1001234567890");
        assert_eq!(parse(&chat_uri(chat)), Some((chat, None)));
        let audio = audio_uri(chat, 456);
        assert_eq!(audio, "telegram:-1001234567890/0000000456");
        assert_eq!(parse(&audio), Some((chat, Some(456))));
    }

    #[test]
    fn text_order_is_sending_order() {
        let mut uris = vec![audio_uri(7, 100), audio_uri(7, 9), audio_uri(7, i32::MAX)];
        uris.sort();
        assert_eq!(
            uris,
            [audio_uri(7, 9), audio_uri(7, 100), audio_uri(7, i32::MAX)]
        );
    }

    #[test]
    fn rejects_what_is_not_a_chat() {
        for uri in [
            "telegram:",
            "telegram:abc",
            "telegram:+5",
            "telegram:5/",
            "telegram:5/0",
            "telegram:5/-3",
            "telegram:5/x",
            "telegram:5/1/2",
            "https://t.me/channel",
            "/music/telegram:5",
        ] {
            assert_eq!(parse(uri), None, "{uri}");
        }
    }
}
