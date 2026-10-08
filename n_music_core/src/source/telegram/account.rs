//! The Telegram account N Music signs in to, and its connection.
//!
//! Requests block their caller: they run on the account's own runtime, from the threads of jobs,
//! scans and playback, never from the bus.

use super::session::SessionStore;
use super::{TelegramChatInfo, TelegramChatKind, TelegramError, TelegramStatus};
use crate::source::Locator;
use grammers_client::client::{LoginToken, PasswordToken};
use grammers_client::peer::{Peer, User};
use grammers_client::sender::{ConnectionParams, SenderPool};
use grammers_client::{Client, InvocationError, SignInError};
use std::future::Future;
use std::io;
use std::mem;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::runtime::Runtime;

/// A step of signing in, or a page of a listing, that takes longer is given up.
pub(super) const TIMEOUT: Duration = Duration::from_secs(30);
/// How many of the account's chats are offered, the most recent first.
const CHATS: usize = 200;

/// What N Music is to Telegram: its API id and hash, registered at my.telegram.org.
#[derive(Clone, Debug)]
pub struct TelegramCredentials {
    pub api_id: i32,
    pub api_hash: String,
}

impl TelegramCredentials {
    /// The credentials a build passes as text, the way `option_env!` hands them over; `None`
    /// when either is missing or blank, or the id is not a number.
    pub fn new(api_id: Option<&str>, api_hash: Option<&str>) -> Option<Self> {
        let api_id = api_id?.trim().parse().ok().filter(|&id: &i32| id > 0)?;
        let api_hash = api_hash?.trim();
        (!api_hash.is_empty()).then(|| Self {
            api_id,
            api_hash: api_hash.to_string(),
        })
    }
}

/// The account, signed in or not, with its session and connection.
pub struct TelegramAccount {
    credentials: TelegramCredentials,
    store: Arc<SessionStore>,
    runtime: Runtime,
    /// `None` until a request needs it, and again after signing out.
    client: Mutex<Option<Client>>,
    login: Mutex<Login>,
}

/// Where signing in is, between steps.
enum Login {
    Idle,
    /// Telegram sent a code to `phone`.
    Code {
        phone: String,
        token: LoginToken,
    },
    /// The account has a password.
    Password {
        token: Box<PasswordToken>,
    },
}

impl TelegramAccount {
    /// Opens the account kept in `dir`, signed in or not. Nothing connects until a request
    /// needs Telegram.
    pub fn open(dir: &Path, credentials: TelegramCredentials) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let store = SessionStore::open(&dir.join("telegram.db")).map_err(io::Error::other)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("telegram")
            .enable_all()
            .build()?;
        Ok(Self {
            credentials,
            store: Arc::new(store),
            runtime,
            client: Mutex::new(None),
            login: Mutex::new(Login::Idle),
        })
    }

    /// Where signing in is, as last known: nothing is asked of Telegram.
    pub fn status(&self) -> TelegramStatus {
        match &*self.login() {
            Login::Code { phone, .. } => {
                return TelegramStatus::CodeSent {
                    phone: phone.clone(),
                }
            }
            Login::Password { token } => {
                return TelegramStatus::PasswordNeeded {
                    hint: token.hint().map(str::to_string),
                }
            }
            Login::Idle => {}
        }
        match self.store.account() {
            Ok(Some(name)) => TelegramStatus::SignedIn { name },
            Ok(None) => TelegramStatus::SignedOut,
            Err(error) => {
                log::error!("Could not read the Telegram session: {error}");
                TelegramStatus::SignedOut
            }
        }
    }

    /// An account is signed in, as last known.
    pub fn is_signed_in(&self) -> bool {
        matches!(self.store.account(), Ok(Some(_)))
    }

    /// Asks Telegram to send a code to `phone`, in the Telegram app or by SMS.
    pub(super) fn send_code(&self, phone: &str) -> Result<(), TelegramError> {
        let phone = phone_number(phone).ok_or(TelegramError::PhoneInvalid)?;
        let client = self.client();
        let api_hash = &self.credentials.api_hash;
        let token = self.block_on(TIMEOUT, async {
            Ok(client.request_login_code(&phone, api_hash).await?)
        })?;
        *self.login() = Login::Code { phone, token };
        Ok(())
    }

    /// Signs in with the `code` Telegram sent; an account with a password asks for it next.
    pub(super) fn enter_code(&self, code: &str) -> Result<(), TelegramError> {
        let Login::Code { phone, token } = mem::replace(&mut *self.login(), Login::Idle) else {
            return Err(TelegramError::Failed(String::from("No code was asked for")));
        };
        let code: String = code.chars().filter(char::is_ascii_digit).collect();
        let client = self.client();
        let signed = self.block_on(TIMEOUT, async { Ok(client.sign_in(&token, &code).await) });
        let again = |token| Login::Code { phone, token };
        match signed {
            Ok(Ok(user)) => self.signed_in(&user),
            Ok(Err(SignInError::PasswordRequired(token))) => {
                *self.login() = Login::Password {
                    token: Box::new(token),
                };
                Ok(())
            }
            Ok(Err(SignInError::InvalidCode)) => {
                *self.login() = again(token);
                Err(TelegramError::CodeInvalid)
            }
            Ok(Err(SignInError::SignUpRequired)) => Err(TelegramError::SignUpRequired),
            Ok(Err(SignInError::Other(error))) => {
                *self.login() = again(token);
                Err(error.into())
            }
            Ok(Err(SignInError::InvalidPassword(_))) => Err(TelegramError::Failed(String::from(
                "Telegram asked for a password",
            ))),
            Err(error) => {
                *self.login() = again(token);
                Err(error)
            }
        }
    }

    /// Signs in with the account's `password`.
    pub(super) fn enter_password(&self, password: &str) -> Result<(), TelegramError> {
        let Login::Password { token } = mem::replace(&mut *self.login(), Login::Idle) else {
            return Err(TelegramError::Failed(String::from(
                "No password was asked for",
            )));
        };
        let client = self.client();
        let checked = self.block_on(TIMEOUT, async {
            Ok(client.check_password(*token, password.as_bytes()).await)
        });
        match checked {
            Ok(Ok(user)) => self.signed_in(&user),
            Ok(Err(SignInError::InvalidPassword(token))) => {
                *self.login() = Login::Password {
                    token: Box::new(token),
                };
                Err(TelegramError::PasswordInvalid)
            }
            // The password token went with the request: signing in starts over.
            Ok(Err(error)) => Err(match error {
                SignInError::Other(error) => error.into(),
                error => TelegramError::Failed(error.to_string()),
            }),
            Err(error) => Err(error),
        }
    }

    /// Signs out of the account, or stops signing in. Signed out here whatever Telegram
    /// answers: the session is forgotten.
    pub(super) fn sign_out(&self) -> Result<(), TelegramError> {
        *self.login() = Login::Idle;
        if !self.is_signed_in() {
            return Ok(());
        }
        let client = self.client();
        let signed_out = self.block_on(TIMEOUT, async {
            client.sign_out().await?;
            Ok(())
        });
        if let Err(error) = signed_out {
            log::warn!("Telegram did not hear of signing out: {error}");
        }
        self.forget()
    }

    /// Checks the account is still signed in, and takes its name again. One signed out
    /// elsewhere is forgotten.
    pub(super) fn verify(&self) -> Result<(), TelegramError> {
        if !self.is_signed_in() {
            return Ok(());
        }
        let client = self.client();
        match self.block_on(TIMEOUT, async { Ok(client.get_me().await?) }) {
            Ok(user) => self.signed_in(&user),
            Err(TelegramError::SignedOut) => {
                self.forget()?;
                Err(TelegramError::SignedOut)
            }
            Err(error) => Err(error),
        }
    }

    /// Chats to make libraries of: those of the account with `query` in their name, the most
    /// recently active first, after the public one `query` names as `@name` or a `t.me` link.
    pub(super) fn find_chats(&self, query: &str) -> Result<Vec<TelegramChatInfo>, TelegramError> {
        if !self.is_signed_in() {
            return Err(TelegramError::SignedOut);
        }
        let client = self.client();
        let username = username(query);
        let words = query.trim().to_lowercase();
        self.block_on(TIMEOUT, async {
            let mut chats: Vec<TelegramChatInfo> = vec![];
            if let Some(username) = &username {
                match client.resolve_username(username).await {
                    Ok(Some(peer)) => chats.extend(chat_info(&peer)),
                    Ok(None) => {}
                    Err(error) if error.is("USERNAME_INVALID") => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let mut dialogs = client.iter_dialogs().limit(CHATS);
            while let Some(dialog) = dialogs.next().await? {
                let Some(chat) = chat_info(dialog.peer()) else {
                    continue;
                };
                let named = words.is_empty()
                    || chat.title.to_lowercase().contains(&words)
                    || chat
                        .username
                        .as_deref()
                        .is_some_and(|name| Some(name.to_lowercase()) == username);
                if named && !chats.iter().any(|known| known.locator == chat.locator) {
                    chats.push(chat);
                }
            }
            Ok(chats)
        })
    }

    /// The client, connected when it was not.
    pub(super) fn client(&self) -> Client {
        lock(&self.client)
            .get_or_insert_with(|| self.connect())
            .clone()
    }

    /// Runs `future` on the account's runtime and waits for it, for `timeout` at most. A panic
    /// in it fails the request, not the thread waiting.
    pub(super) fn block_on<T>(
        &self,
        timeout: Duration,
        future: impl Future<Output = Result<T, TelegramError>>,
    ) -> Result<T, TelegramError> {
        let run = std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(tokio::time::timeout(timeout, future))
        }));
        match run {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TelegramError::Offline(format!(
                "Telegram did not answer in {}s",
                timeout.as_secs()
            ))),
            Err(_) => Err(TelegramError::Failed(String::from(
                "The Telegram client failed",
            ))),
        }
    }

    fn connect(&self) -> Client {
        // Telegram lists the session as N Music on this system, at this version.
        let params = ConnectionParams {
            app_version: String::from(env!("CARGO_PKG_VERSION")),
            ..ConnectionParams::default()
        };
        let pool =
            SenderPool::with_configuration(self.store.clone(), self.credentials.api_id, params);
        self.runtime.spawn(pool.runner.run());
        // Updates are never asked for: those Telegram sends anyway are dropped.
        drop(pool.updates);
        Client::new(pool.handle)
    }

    /// Remembers the account `user` signed in.
    fn signed_in(&self, user: &User) -> Result<(), TelegramError> {
        self.store
            .set_account(&account_name(user))
            .map_err(|error| TelegramError::Failed(error.to_string()))
    }

    /// Forgets the session and drops the connection: the next sign in starts afresh.
    fn forget(&self) -> Result<(), TelegramError> {
        *self.login() = Login::Idle;
        if let Some(client) = lock(&self.client).take() {
            client.disconnect();
        }
        self.store
            .clear()
            .map_err(|error| TelegramError::Failed(error.to_string()))
    }

    fn login(&self) -> MutexGuard<'_, Login> {
        lock(&self.login)
    }
}

impl Drop for TelegramAccount {
    fn drop(&mut self) {
        if let Some(client) = lock(&self.client).take() {
            client.disconnect();
        }
    }
}

impl From<InvocationError> for TelegramError {
    fn from(error: InvocationError) -> Self {
        match &error {
            InvocationError::Rpc(rpc) if rpc.code == 420 => TelegramError::Wait {
                seconds: rpc.value.unwrap_or(0),
            },
            // AUTH_KEY_UNREGISTERED, SESSION_REVOKED, USER_DEACTIVATED and the like.
            InvocationError::Rpc(rpc) if rpc.code == 401 => TelegramError::SignedOut,
            InvocationError::Rpc(rpc) if rpc.name == "PHONE_NUMBER_INVALID" => {
                TelegramError::PhoneInvalid
            }
            InvocationError::Rpc(rpc) if rpc.name == "PHONE_NUMBER_BANNED" => {
                TelegramError::PhoneBanned
            }
            InvocationError::Io(_) | InvocationError::Transport(_) | InvocationError::Dropped => {
                TelegramError::Offline(error.to_string())
            }
            _ => TelegramError::Failed(error.to_string()),
        }
    }
}

/// `peer` as a chat to pick; `None` for one that cannot hold music, like a deleted account.
fn chat_info(peer: &Peer) -> Option<TelegramChatInfo> {
    let (kind, title) = match peer {
        Peer::User(user) if user.is_self() => (TelegramChatKind::SavedMessages, String::new()),
        Peer::User(user) if user.deleted() => return None,
        Peer::User(user) if user.is_bot() => (TelegramChatKind::Bot, user.full_name()),
        Peer::User(user) => (TelegramChatKind::User, user.full_name()),
        Peer::Group(_) => (TelegramChatKind::Group, peer.name()?.to_string()),
        Peer::Channel(_) => (TelegramChatKind::Channel, peer.name()?.to_string()),
    };
    Some(TelegramChatInfo {
        locator: Locator::telegram_chat(peer.id().bot_api_dialog_id()?),
        title: title.trim().to_string(),
        kind,
        username: peer.username().map(str::to_string),
    })
}

/// What the account is called: its name, else its username or phone number.
fn account_name(user: &User) -> String {
    let name = user.full_name();
    let name = name.trim();
    if !name.is_empty() {
        return name.to_string();
    }
    match (user.username(), user.phone()) {
        (Some(username), _) => format!("@{username}"),
        (None, Some(phone)) => format!("+{phone}"),
        (None, None) => String::from("Telegram"),
    }
}

/// `text` as Telegram takes a phone number: its digits, the country code first; `None` when it
/// has too few or too many to be an international number.
fn phone_number(text: &str) -> Option<String> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    (8..=15).contains(&digits.len()).then_some(digits)
}

/// The username `query` names as `@name`, `t.me/name` or `telegram.me/name`, lowercased.
fn username(query: &str) -> Option<String> {
    let query = query.trim();
    let name = match query.strip_prefix('@') {
        Some(name) => name,
        None => {
            let link = query
                .strip_prefix("https://")
                .or_else(|| query.strip_prefix("http://"))
                .unwrap_or(query);
            let link = link.strip_prefix("www.").unwrap_or(link);
            link.strip_prefix("t.me/")
                .or_else(|| link.strip_prefix("telegram.me/"))?
                .split(['/', '?', '#'])
                .next()?
        }
    };
    let valid = (4..=32).contains(&name.len())
        && name.starts_with(|first: char| first.is_ascii_alphabetic())
        && name
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '_');
    valid.then(|| name.to_lowercase())
}

/// A poisoned lock still holds a usable value: the account's state is written whole.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_need_both_parts() {
        let credentials = TelegramCredentials::new(Some(" 12345 "), Some("abc")).unwrap();
        assert_eq!(credentials.api_id, 12345);
        assert_eq!(credentials.api_hash, "abc");
        assert!(TelegramCredentials::new(None, Some("abc")).is_none());
        assert!(TelegramCredentials::new(Some("12345"), Some(" ")).is_none());
        assert!(TelegramCredentials::new(Some("id"), Some("abc")).is_none());
        assert!(TelegramCredentials::new(Some("0"), Some("abc")).is_none());
    }

    #[test]
    fn phone_numbers_keep_their_digits() {
        assert_eq!(
            phone_number("+39 345 678 9012").as_deref(),
            Some("393456789012")
        );
        assert_eq!(phone_number("(555) 0132"), None);
        assert_eq!(phone_number("+1 234 567 890 123 456"), None);
    }

    #[test]
    fn usernames_come_from_names_and_links() {
        assert_eq!(username("@Music_Hub").as_deref(), Some("music_hub"));
        assert_eq!(
            username("https://t.me/musichub/123").as_deref(),
            Some("musichub")
        );
        assert_eq!(
            username("t.me/musichub?start=1").as_deref(),
            Some("musichub")
        );
        assert_eq!(
            username("telegram.me/musichub").as_deref(),
            Some("musichub")
        );
        assert_eq!(username("musichub"), None);
        assert_eq!(username("@abc"), None);
        assert_eq!(username("@1music"), None);
        assert_eq!(username("https://t.me/+AbCdEf"), None);
    }
}
