use crate::platform::Platform;
use n_event_bus::{job_emits, EventWriter, Job, JobToken, Tagged};
use std::path::PathBuf;
use std::sync::Arc;

pub struct DirectoryJob(pub Arc<dyn Platform>);
pub struct DirectoryChosen(pub PathBuf);
job_emits!(DirectoryJob => Tagged<DirectoryChosen>);
impl Job for DirectoryJob {
    fn run(self, tag: u64, writer: EventWriter, _: Option<JobToken>) {
        self.0.ask_music_dir(tag, writer);
    }
}

pub struct OpenLinkJob(pub Arc<dyn Platform>, pub String);
impl Job for OpenLinkJob {
    fn run(self, _: u64, _: EventWriter, _: Option<JobToken>) {
        self.0.open_link(self.1);
    }
}
