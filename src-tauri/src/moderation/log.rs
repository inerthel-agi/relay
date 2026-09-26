//! Local decision log. It keeps who, where, which rule and what happened,
//! never the message text or the media, for at most 7 days and 500 entries.

use std::collections::VecDeque;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Lane;

pub const LOG_LIMIT: usize = 500;
const LOG_RETENTION_MS: u64 = 7 * 86_400_000;
const SUMMARY_WINDOW_MS: u64 = 86_400_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogAction {
    Blocked,
    Held,
    Ignored,
    Approved,
    Rejected,
    Expired,
    Evicted,
    Released,
    Warned,
    TimedOut,
    Banned,
    Kicked,
    MessageDeleted,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<Lane>,
    pub action: LogAction,
    /// Stable reason code such as "forbidden_concept" or "cooldown".
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSummary {
    pub blocked: usize,
    pub held: usize,
    pub ignored: usize,
    pub approved: usize,
    pub rejected: usize,
    pub sanctions: usize,
    /// Distinct authors with at least one block, hold or ignore.
    pub flagged_members: usize,
}

#[derive(Default)]
pub struct ModerationLog {
    entries: VecDeque<LogEntry>,
    path: Option<PathBuf>,
}

impl ModerationLog {
    /// Loads the saved log; a missing or damaged file starts an empty log.
    pub fn open(path: PathBuf, now_ms: u64) -> Self {
        let entries = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<VecDeque<LogEntry>>(&bytes).ok())
            .unwrap_or_default();
        let mut log = Self {
            entries,
            path: Some(path),
        };
        log.prune(now_ms);
        log
    }

    pub fn record(&mut self, entry: LogEntry) {
        let now_ms = entry.at;
        self.entries.push_back(entry);
        self.prune(now_ms);
        self.persist();
    }

    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.iter().rev().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.persist();
    }

    pub fn summary(&self, now_ms: u64) -> LogSummary {
        let mut summary = LogSummary::default();
        let mut flagged = std::collections::HashSet::new();
        for entry in self
            .entries
            .iter()
            .filter(|entry| now_ms.saturating_sub(entry.at) < SUMMARY_WINDOW_MS)
        {
            match entry.action {
                LogAction::Blocked => summary.blocked += 1,
                LogAction::Held => summary.held += 1,
                LogAction::Ignored => summary.ignored += 1,
                LogAction::Approved | LogAction::Released => summary.approved += 1,
                LogAction::Rejected | LogAction::Expired | LogAction::Evicted => {
                    summary.rejected += 1
                }
                LogAction::Warned
                | LogAction::TimedOut
                | LogAction::Banned
                | LogAction::Kicked
                | LogAction::MessageDeleted => summary.sanctions += 1,
            }
            if matches!(
                entry.action,
                LogAction::Blocked | LogAction::Held | LogAction::Ignored
            ) && let Some(author) = &entry.author_id
            {
                flagged.insert(author.clone());
            }
        }
        summary.flagged_members = flagged.len();
        summary
    }

    fn prune(&mut self, now_ms: u64) {
        self.entries
            .retain(|entry| now_ms.saturating_sub(entry.at) < LOG_RETENTION_MS);
        while self.entries.len() > LOG_LIMIT {
            self.entries.pop_front();
        }
    }

    /// Best effort: a failed write never blocks moderation itself.
    fn persist(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Ok(bytes) = serde_json::to_vec(&self.entries) {
            let temporary = path.with_extension("json.tmp");
            if std::fs::write(&temporary, bytes).is_ok() {
                let _ = std::fs::rename(&temporary, path);
            }
        }
    }
}
