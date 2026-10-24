use std::sync::{Mutex, MutexGuard};

use super::session::{ClientInfo, SessionState};
use crate::options::Options;
use crate::pipeline::{Pipeline, Sink};
use crate::tools::cache::ToolCache;

pub(crate) struct Shared {
    pub pipeline: Pipeline,
    pub tools: ToolCache,
    session: Mutex<SessionState>,
}

impl Shared {
    pub(crate) fn new(sink: impl Sink, options: Options) -> Self {
        Self {
            pipeline: Pipeline::new(sink, options),
            tools: ToolCache::default(),
            session: Mutex::new(SessionState::new()),
        }
    }

    pub(crate) fn options(&self) -> &Options {
        self.pipeline.options()
    }

    pub(crate) fn touch_session(&self) -> String {
        self.session().touch()
    }

    pub(crate) fn remembered_client(&self) -> ClientInfo {
        self.session().client()
    }

    pub(crate) fn remember_client(&self, client: ClientInfo) {
        self.session().remember_client(client);
    }

    fn session(&self) -> MutexGuard<'_, SessionState> {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
