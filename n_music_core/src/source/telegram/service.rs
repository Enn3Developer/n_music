//! Signing in to Telegram and finding chats, on the bus. Requests run as jobs: the bus never
//! waits on Telegram.

use super::account::TelegramAccount;
use super::{TelegramChatInfo, TelegramError};
use crate::messages::{
    FindTelegramChats, ScanRequested, TelegramChatsFound, TelegramCode, TelegramPassword,
    TelegramSignIn, TelegramSignOut, TelegramStatusChanged,
};
use crate::settings::{LibrarySettings, Options};
use crate::source::Locator;
use n_event_bus::{
    job_emits, Ctx, EventWriter, Handle, Job, JobToken, Message, Outbox, Registrar, RunningJob,
    Subscriber, Tagged,
};
use std::any::Any;
use std::sync::Arc;

/// Checks at launch that the account is still signed in. Queued by the engine.
pub(crate) struct VerifyTelegram;

impl Message for VerifyTelegram {}

pub(crate) struct TelegramService {
    account: Arc<TelegramAccount>,
    /// The Telegram chats among them are listed again once signed in.
    libraries: Options<LibrarySettings>,
    /// The step of signing in or out running: one runs at a time.
    step: Option<RunningJob>,
    /// The search for chats running: a new one replaces it.
    search: Option<RunningJob>,
}

impl TelegramService {
    pub(crate) fn new(account: Arc<TelegramAccount>, libraries: Options<LibrarySettings>) -> Self {
        Self {
            account,
            libraries,
            step: None,
            search: None,
        }
    }

    /// Where signing in is, with no step running.
    pub(crate) fn status(account: &TelegramAccount) -> TelegramStatusChanged {
        TelegramStatusChanged {
            status: account.status(),
            busy: false,
            error: None,
        }
    }

    /// Runs `step`, unless one runs already.
    fn start(&mut self, step: Step, ctx: &Ctx, out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        if self.step.is_some() {
            log::debug!("Not starting a Telegram step: one is running");
            return;
        }
        out.emit(TelegramStatusChanged {
            status: self.account.status(),
            busy: true,
            error: None,
        });
        self.step = Some(ctx.jobs.spawn_oneshot(StepJob {
            account: self.account.clone(),
            step,
        }));
    }

    /// Lists the Telegram chats among the libraries again.
    fn rescan(&self, out: &mut Outbox) {
        let libraries = self.libraries.get().libraries.clone();
        for library in libraries {
            if matches!(library, Locator::TelegramChat(_)) {
                out.emit(ScanRequested {
                    library: Some(library),
                    check_cache: true,
                });
            }
        }
    }
}

impl Subscriber for TelegramService {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn register(reg: &mut Registrar<Self>) {
        reg.on::<VerifyTelegram>();
        reg.on::<TelegramSignIn>();
        reg.on::<TelegramCode>();
        reg.on::<TelegramPassword>();
        reg.on::<TelegramSignOut>();
        reg.on::<FindTelegramChats>();
        StepJob::subscribe(reg);
        SearchJob::subscribe(reg);
    }
}

impl Handle<VerifyTelegram> for TelegramService {
    fn handle(&mut self, _msg: &VerifyTelegram, ctx: &Ctx, out: &mut Outbox) {
        if self.account.is_signed_in() {
            self.start(Step::Verify, ctx, out);
        }
    }
}

impl Handle<TelegramSignIn> for TelegramService {
    fn handle(&mut self, msg: &TelegramSignIn, ctx: &Ctx, out: &mut Outbox) {
        self.start(Step::SendCode(msg.phone.clone()), ctx, out);
    }
}

impl Handle<TelegramCode> for TelegramService {
    fn handle(&mut self, msg: &TelegramCode, ctx: &Ctx, out: &mut Outbox) {
        self.start(Step::Code(msg.0.clone()), ctx, out);
    }
}

impl Handle<TelegramPassword> for TelegramService {
    fn handle(&mut self, msg: &TelegramPassword, ctx: &Ctx, out: &mut Outbox) {
        self.start(Step::Password(msg.0.clone()), ctx, out);
    }
}

impl Handle<TelegramSignOut> for TelegramService {
    fn handle(&mut self, _msg: &TelegramSignOut, ctx: &Ctx, out: &mut Outbox) {
        self.start(Step::SignOut, ctx, out);
    }
}

impl Handle<FindTelegramChats> for TelegramService {
    fn handle(&mut self, msg: &FindTelegramChats, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        // Dropping the search running cancels it: its result is not reported.
        self.search = Some(ctx.jobs.spawn_stream(SearchJob {
            account: self.account.clone(),
            query: msg.query.clone(),
        }));
    }
}

impl Handle<Tagged<StepDone>> for TelegramService {
    fn handle(&mut self, msg: &Tagged<StepDone>, _ctx: &Ctx, out: &mut Outbox) {
        let Some(done) = self.step.as_ref().and_then(|step| step.open(msg)) else {
            return;
        };
        let signed_in = done.signed_in;
        out.emit(TelegramStatusChanged {
            status: self.account.status(),
            busy: false,
            error: done.error.clone(),
        });
        self.step = None;
        if signed_in {
            self.rescan(out);
        }
    }
}

impl Handle<Tagged<Found>> for TelegramService {
    fn handle(&mut self, msg: &Tagged<Found>, _ctx: &Ctx, out: &mut Outbox) {
        let Some(found) = self.search.as_ref().and_then(|search| search.open(msg)) else {
            return;
        };
        let (chats, error) = match &found.chats {
            Ok(chats) => (chats.clone(), None),
            Err(error) => (vec![], Some(error.clone())),
        };
        out.emit(TelegramChatsFound {
            query: found.query.clone(),
            chats,
            error,
        });
        self.search = None;
    }
}

enum Step {
    Verify,
    SendCode(String),
    Code(String),
    Password(String),
    SignOut,
}

struct StepJob {
    account: Arc<TelegramAccount>,
    step: Step,
}

struct StepDone {
    error: Option<TelegramError>,
    /// The step signed the account in.
    signed_in: bool,
}

job_emits!(StepJob => Tagged<StepDone>);

impl Job for StepJob {
    fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let account = &self.account;
        let was_signed_in = account.is_signed_in();
        let result = match &self.step {
            Step::Verify => account.verify(),
            Step::SendCode(phone) => account.send_code(phone),
            Step::Code(code) => account.enter_code(code),
            Step::Password(password) => account.enter_password(password),
            Step::SignOut => account.sign_out(),
        };
        if let Err(error) = &result {
            log::info!("Telegram: {error}");
        }
        writer.emit_tagged(
            tag,
            StepDone {
                error: result.err(),
                signed_in: !was_signed_in && account.is_signed_in(),
            },
        );
    }
}

struct SearchJob {
    account: Arc<TelegramAccount>,
    query: String,
}

struct Found {
    query: String,
    chats: Result<Vec<TelegramChatInfo>, TelegramError>,
}

job_emits!(SearchJob => Tagged<Found>);

impl Job for SearchJob {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let chats = self.account.find_chats(&self.query);
        if token.as_ref().is_some_and(JobToken::is_cancelled) {
            return;
        }
        writer.emit_tagged(
            tag,
            Found {
                query: self.query,
                chats,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::telegram::{TelegramCredentials, TelegramStatus};
    use n_event_bus::{App, Envelope, EventReceiver, JobControl};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    type Status = (TelegramStatus, bool, Option<TelegramError>);
    /// A query, with why it found nothing.
    type Search = (String, Option<TelegramError>);

    /// Keeps what the service reports.
    #[derive(Clone, Default)]
    struct Heard {
        statuses: Arc<Mutex<Vec<Status>>>,
        found: Arc<Mutex<Vec<Search>>>,
    }

    impl Subscriber for Heard {
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn register(reg: &mut Registrar<Self>) {
            reg.on::<TelegramStatusChanged>();
            reg.on::<TelegramChatsFound>();
        }
    }

    impl Handle<TelegramStatusChanged> for Heard {
        fn handle(&mut self, msg: &TelegramStatusChanged, _ctx: &Ctx, _out: &mut Outbox) {
            let status = (msg.status.clone(), msg.busy, msg.error.clone());
            self.statuses.lock().unwrap().push(status);
        }
    }

    impl Handle<TelegramChatsFound> for Heard {
        fn handle(&mut self, msg: &TelegramChatsFound, _ctx: &Ctx, _out: &mut Outbox) {
            assert!(msg.chats.is_empty());
            let found = (msg.query.clone(), msg.error.clone());
            self.found.lock().unwrap().push(found);
        }
    }

    /// A bus with the service of an account that never signed in.
    fn bus(dir: &std::path::Path) -> (App, EventReceiver, Heard) {
        let (writer, rx) = EventWriter::channel();
        let mut app = App::new(JobControl::new(writer));
        let credentials = TelegramCredentials::new(Some("1"), Some("hash")).unwrap();
        let account = Arc::new(TelegramAccount::open(dir, credentials).unwrap());
        app.register_subscriber(TelegramService::new(account, Options::in_memory()));
        let heard = Heard::default();
        app.register_subscriber(heard.clone());
        (app, rx, heard)
    }

    /// Dispatches what comes until `done`.
    fn run_until(app: &mut App, rx: &EventReceiver, done: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        app.dispatch_all();
        while !done() {
            let left = deadline.saturating_duration_since(Instant::now());
            let event = rx.recv_timeout(left).expect("the service went quiet");
            app.enqueue_event(event);
            app.dispatch_all();
        }
    }

    #[test]
    fn signing_out_while_signed_out_reports_it() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, rx, heard) = bus(dir.path());
        app.enqueue(Envelope::new(TelegramSignOut));
        run_until(&mut app, &rx, || heard.statuses.lock().unwrap().len() == 2);
        assert_eq!(
            *heard.statuses.lock().unwrap(),
            [
                (TelegramStatus::SignedOut, true, None),
                (TelegramStatus::SignedOut, false, None),
            ]
        );
    }

    #[test]
    fn a_code_before_asking_for_one_fails() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, rx, heard) = bus(dir.path());
        app.enqueue(Envelope::new(TelegramCode(String::from("12345"))));
        run_until(&mut app, &rx, || heard.statuses.lock().unwrap().len() == 2);
        let statuses = heard.statuses.lock().unwrap();
        assert_eq!(statuses[1].0, TelegramStatus::SignedOut);
        assert!(!statuses[1].1);
        assert!(matches!(statuses[1].2, Some(TelegramError::Failed(_))));
    }

    #[test]
    fn finding_chats_needs_an_account() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, rx, heard) = bus(dir.path());
        app.enqueue(Envelope::new(FindTelegramChats {
            query: String::from("music"),
        }));
        run_until(&mut app, &rx, || !heard.found.lock().unwrap().is_empty());
        assert_eq!(
            *heard.found.lock().unwrap(),
            [(String::from("music"), Some(TelegramError::SignedOut))]
        );
    }
}
