//! Chats on Telegram: a channel, a group or Saved Messages is a library, the audio files sent in
//! it its tracks.
//!
//! Chats are known by their Bot API style id, which tells users, groups and channels apart:
//! their names change, and private ones have none.

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
