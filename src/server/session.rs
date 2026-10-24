//! Calls without a conversation handle share the instance's generated session,
//! which rolls over after 30 idle minutes. This is what groups stdio calls.

use std::time::{Duration, Instant};

use crate::contract::SESSION_INACTIVITY_MINUTES;
use crate::support::ids::new_session_id;

const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(SESSION_INACTIVITY_MINUTES * 60);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClientInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub protocol_version: Option<String>,
}

pub(crate) struct SessionState {
    session_id: String,
    last_activity: Instant,
    client: ClientInfo,
}

impl SessionState {
    pub(crate) fn new() -> Self {
        Self {
            session_id: new_session_id(),
            last_activity: Instant::now(),
            client: ClientInfo::default(),
        }
    }

    pub(crate) fn touch(&mut self) -> String {
        if self.last_activity.elapsed() > INACTIVITY_TIMEOUT {
            self.session_id = new_session_id();
        }
        self.last_activity = Instant::now();
        self.session_id.clone()
    }

    pub(crate) fn client(&self) -> ClientInfo {
        self.client.clone()
    }

    pub(crate) fn remember_client(&mut self, client: ClientInfo) {
        self.client = client;
    }
}
