use std::collections::{HashMap, VecDeque};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::model::{MessagePinEvent, NotificationEvent};

use super::AppCore;

const AUTHORITATIVE_TTS_LIMIT: usize = 128;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePinStatus {
    pub current: Option<NotificationEvent>,
    pub pinned: bool,
}

#[derive(Default)]
pub(crate) struct MessagePinRuntime {
    visible: HashMap<u64, VisibleNotification>,
    authoritative: VecDeque<NotificationEvent>,
    next_client_id: u64,
    next_sequence: u64,
    pinned: Option<NotificationEvent>,
}

struct VisibleNotification {
    event: NotificationEvent,
    sequence: u64,
}

impl MessagePinRuntime {
    pub(super) fn remember_authoritative(&mut self, event: NotificationEvent) {
        if event.id.is_empty() {
            return;
        }
        self.authoritative.retain(|known| known.id != event.id);
        self.authoritative.push_back(event);
        while self.authoritative.len() > AUTHORITATIVE_TTS_LIMIT {
            self.authoritative.pop_front();
        }
    }

    fn authoritative_event(&self, id: &str) -> Option<NotificationEvent> {
        self.authoritative
            .iter()
            .rev()
            .find(|event| event.id == id)
            .cloned()
            .or_else(|| self.pinned.as_ref().filter(|event| event.id == id).cloned())
    }

    fn current(&self) -> Option<NotificationEvent> {
        self.visible
            .values()
            .max_by_key(|notification| notification.sequence)
            .map(|notification| notification.event.clone())
    }

    fn status(&self) -> MessagePinStatus {
        MessagePinStatus {
            current: self.current(),
            pinned: self.pinned.is_some(),
        }
    }

    fn pin_event(&self) -> MessagePinEvent {
        MessagePinEvent {
            pinned: self.pinned.is_some(),
            message: self.pinned.clone(),
        }
    }
}

impl AppCore {
    /// Register one notification output connection for visibility tracking.
    pub async fn register_notification_output(&self) -> u64 {
        let mut state = self.message_pin.lock().await;
        state.next_client_id = state.next_client_id.wrapping_add(1).max(1);
        state.next_client_id
    }

    /// Record the message currently displayed by one notification output.
    pub async fn report_notification_output(&self, client_id: u64, event: NotificationEvent) {
        let mut state = self.message_pin.lock().await;
        let Some(event) = state.authoritative_event(&event.id) else {
            return;
        };
        state.next_sequence = state.next_sequence.wrapping_add(1).max(1);
        let sequence = state.next_sequence;
        state
            .visible
            .insert(client_id, VisibleNotification { event, sequence });
    }

    pub(crate) async fn remember_authoritative_notification(&self, event: &NotificationEvent) {
        self.message_pin
            .lock()
            .await
            .remember_authoritative(event.clone());
    }

    /// Remove one output's visibility report. An optional ID prevents a stale
    /// end event from clearing a newer notification from the same connection.
    pub async fn clear_notification_output(&self, client_id: u64, message_id: Option<&str>) {
        let mut state = self.message_pin.lock().await;
        let should_clear = state
            .visible
            .get(&client_id)
            .is_some_and(|notification| message_id.is_none_or(|id| notification.event.id == id));
        if should_clear {
            state.visible.remove(&client_id);
        }
    }

    pub async fn message_status(&self) -> MessagePinStatus {
        self.message_pin.lock().await.status()
    }

    pub async fn message_pin_event(&self) -> MessagePinEvent {
        self.message_pin.lock().await.pin_event()
    }

    /// Pin the currently visible message, optionally checking an expected ID.
    /// The event is copied from an authenticated output report, so callers
    /// cannot inject an arbitrary Discord message into the output.
    pub async fn pin_current_message(
        &self,
        expected_id: Option<String>,
    ) -> Result<MessagePinStatus> {
        let (status, event) = {
            let mut state = self.message_pin.lock().await;
            let Some(current) = state.current() else {
                bail!("No notification is currently visible.");
            };
            if expected_id
                .as_deref()
                .is_some_and(|expected| expected != current.id)
            {
                bail!("The selected notification is no longer visible.");
            }
            let changed = state
                .pinned
                .as_ref()
                .is_none_or(|pinned| pinned.id != current.id);
            state.pinned = Some(current.clone());
            (state.status(), changed.then_some(current))
        };
        if let Some(message) = event {
            let limit = usize::from(self.config.read().await.notification_queue_limit);
            self.stage_scheduler.set_messages_pinned(true, limit).await;
            let _ = self
                .relay_tx
                .send(crate::model::RelayEvent::MessagePin(MessagePinEvent {
                    pinned: true,
                    message: Some(message),
                }));
        }
        Ok(status)
    }

    pub async fn unpin_message(&self) -> MessagePinStatus {
        let (status, changed) = {
            let mut state = self.message_pin.lock().await;
            let changed = state.pinned.take().is_some();
            (state.status(), changed)
        };
        if changed {
            let _ = self
                .relay_tx
                .send(crate::model::RelayEvent::MessagePin(MessagePinEvent {
                    pinned: false,
                    message: None,
                }));
            self.stage_scheduler.set_messages_pinned(false, 50).await;
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthorIdentity, RelayEvent, VisualSegment};

    fn notification(id: &str) -> NotificationEvent {
        NotificationEvent {
            id: id.into(),
            text: format!("Message {id}"),
            author: AuthorIdentity {
                username: "tester".into(),
                display_avatar_url: String::new(),
            },
            guild_tag: None,
            timestamp: 1,
            segments: vec![VisualSegment {
                kind: "text".into(),
                value: format!("Message {id}"),
                url: None,
                animated: false,
            }],
        }
    }

    #[tokio::test]
    async fn pins_only_an_authenticated_visible_message_and_broadcasts_state() {
        let directory = tempfile::tempdir().unwrap();
        let core = AppCore::load(directory.path().join("config.json")).unwrap();
        let mut events = core.relay_tx.subscribe();
        let client = core.register_notification_output().await;
        core.remember_authoritative_notification(&notification("visible"))
            .await;
        core.report_notification_output(client, notification("visible"))
            .await;

        let status = core
            .pin_current_message(Some("visible".into()))
            .await
            .unwrap();
        assert_eq!(
            status.current.as_ref().map(|event| event.id.as_str()),
            Some("visible")
        );
        assert!(status.pinned);
        let RelayEvent::MessagePin(event) = events.recv().await.unwrap() else {
            panic!("expected message pin event");
        };
        assert!(event.pinned);
        assert_eq!(
            event.message.as_ref().map(|value| value.id.as_str()),
            Some("visible")
        );
    }

    #[tokio::test]
    async fn stale_or_missing_visibility_cannot_be_pinned() {
        let directory = tempfile::tempdir().unwrap();
        let core = AppCore::load(directory.path().join("config.json")).unwrap();
        assert!(core.pin_current_message(None).await.is_err());
        let client = core.register_notification_output().await;
        core.remember_authoritative_notification(&notification("visible"))
            .await;
        core.report_notification_output(client, notification("visible"))
            .await;
        assert!(
            core.pin_current_message(Some("other".into()))
                .await
                .is_err()
        );
        core.clear_notification_output(client, Some("visible"))
            .await;
        assert!(core.pin_current_message(None).await.is_err());
    }

    #[tokio::test]
    async fn unpin_is_ephemeral_and_does_not_restore_after_new_core_load() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let core = AppCore::load(path.clone()).unwrap();
        let client = core.register_notification_output().await;
        core.remember_authoritative_notification(&notification("visible"))
            .await;
        core.report_notification_output(client, notification("visible"))
            .await;
        core.pin_current_message(None).await.unwrap();
        let status = core.unpin_message().await;
        assert!(!status.pinned);
        assert!(!AppCore::load(path).unwrap().message_status().await.pinned);
    }

    #[tokio::test]
    async fn output_reports_must_match_authoritative_message_content() {
        let directory = tempfile::tempdir().unwrap();
        let core = AppCore::load(directory.path().join("config.json")).unwrap();
        let client = core.register_notification_output().await;
        let trusted = notification("trusted");
        core.remember_authoritative_notification(&trusted).await;

        core.report_notification_output(client, notification("unknown"))
            .await;
        assert!(core.message_status().await.current.is_none());

        let mut tampered = trusted.clone();
        tampered.text = "injected text".into();
        tampered.author.username = "attacker".into();
        tampered.segments[0].value = "injected text".into();
        core.report_notification_output(client, tampered).await;
        let current = core
            .message_status()
            .await
            .current
            .expect("known notification should be visible");
        assert_eq!(current.id, trusted.id);
        assert_eq!(current.text, trusted.text);
        assert_eq!(current.author.username, trusted.author.username);
        assert_eq!(current.segments[0].value, trusted.segments[0].value);
    }

    #[tokio::test]
    async fn pinned_message_remains_authoritative_after_registry_eviction() {
        let directory = tempfile::tempdir().unwrap();
        let core = AppCore::load(directory.path().join("config.json")).unwrap();
        let client = core.register_notification_output().await;
        let trusted = notification("pinned");
        core.remember_authoritative_notification(&trusted).await;
        core.report_notification_output(client, trusted.clone())
            .await;
        core.pin_current_message(None).await.unwrap();

        for index in 0..AUTHORITATIVE_TTS_LIMIT {
            core.remember_authoritative_notification(&notification(&format!("other-{index}")))
                .await;
        }
        core.clear_notification_output(client, Some("pinned")).await;
        let mut tampered = trusted.clone();
        tampered.text = "injected after eviction".into();
        core.report_notification_output(client, tampered).await;
        let current = core
            .message_status()
            .await
            .current
            .expect("pinned fallback should resolve after eviction");
        assert_eq!(current.id, trusted.id);
        assert_eq!(current.text, trusted.text);
    }
}
