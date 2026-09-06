use n_audio::player::{PlaybackEvent, PlaybackTask};
use n_event_bus::{job_emits, EventWriter, Job, JobToken, Tagged};

pub struct PlaybackJob(pub PlaybackTask);

job_emits!(PlaybackJob => Tagged<PlaybackEvent>);

impl Job for PlaybackJob {
    async fn run(self, tag: u64, writer: EventWriter, _token: Option<JobToken>) {
        let events = writer.clone();
        let result =
            tokio::task::spawn_blocking(move || self.0.run(|event| events.emit_tagged(tag, event)))
                .await;
        let error = match result {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        writer.emit_tagged(tag, PlaybackEvent::Failed(error));
    }
}
