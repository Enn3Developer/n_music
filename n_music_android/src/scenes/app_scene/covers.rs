use super::{AppScene, Changes};
use crate::ui::{decode_cover, CoverBuffer};
use n_event_bus::{job_emits, Ctx, EventWriter, Handle, Job, JobToken, Outbox, Tagged};
use std::collections::HashMap;
use std::path::PathBuf;

const BATCH: usize = 32;

/// Decodes covers off the event loop; a library loaded from the cache would otherwise stall it.
pub struct CoverJob(pub Vec<(usize, PathBuf)>);
pub struct CoversDecoded(pub Vec<(usize, PathBuf, CoverBuffer)>);
job_emits!(CoverJob => Tagged<CoversDecoded>);

impl Job for CoverJob {
    fn run(self, tag: u64, writer: EventWriter, token: Option<JobToken>) {
        let mut decoded: HashMap<PathBuf, Option<CoverBuffer>> = HashMap::new();
        for chunk in self.0.chunks(BATCH) {
            if token.as_ref().is_some_and(JobToken::is_cancelled) {
                return;
            }
            let batch = chunk
                .iter()
                .filter_map(|(index, path)| {
                    let buffer = decoded
                        .entry(path.clone())
                        .or_insert_with(|| decode_cover(path))
                        .clone()?;
                    Some((*index, path.clone(), buffer))
                })
                .collect();
            writer.emit_tagged(tag, CoversDecoded(batch));
        }
    }
}

impl Handle<Tagged<CoversDecoded>> for AppScene {
    fn handle(&mut self, msg: &Tagged<CoversDecoded>, ctx: &Ctx, _out: &mut Outbox) {
        if ctx.shutting_down {
            return;
        }
        let Some(decoded) = self.cover_job.as_ref().and_then(|job| job.open(msg)) else {
            return;
        };
        for (index, path, buffer) in &decoded.0 {
            self.covers.insert(path.clone(), buffer.clone());
            self.changes.push(Changes::Cover(*index, buffer.clone()));
        }
        self.apply_ui();
    }
}
