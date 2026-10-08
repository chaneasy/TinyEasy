//! Bounded, credential-free notifications, rendered above the current view.
use crate::error::Error;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(7);
const MAX_VISIBLE: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Context {
    General,
    Key(usize),
    Image(String),
}

pub(super) struct Toast {
    pub id: u64,
    pub error: Error,
    pub context: Context,
    expires_at: Instant,
}

#[derive(Default)]
pub(super) struct Notifications {
    items: Vec<Toast>,
    next_id: u64,
}

impl Notifications {
    pub fn items(&self) -> &[Toast] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn push(&mut self, error: Error, context: Context, now: Instant) {
        self.expire(now);
        if let Some(toast) = self
            .items
            .iter_mut()
            .find(|toast| toast.error == error && toast.context == context)
        {
            toast.expires_at = now + TIMEOUT;
            return;
        }
        if self.items.len() == MAX_VISIBLE {
            self.items.remove(0);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.items.push(Toast {
            id,
            error,
            context,
            expires_at: now + TIMEOUT,
        });
    }

    pub fn dismiss(&mut self, id: u64) {
        self.items.retain(|toast| toast.id != id);
    }

    pub fn expire(&mut self, now: Instant) {
        self.items.retain(|toast| toast.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeating_an_error_refreshes_its_timeout_without_duplicate_toasts() {
        let mut toasts = Notifications::default();
        let now = Instant::now();
        toasts.push(Error::Network, Context::General, now);
        let id = toasts.items()[0].id;
        toasts.push(
            Error::Network,
            Context::General,
            now + Duration::from_secs(3),
        );
        assert_eq!(toasts.items().len(), 1);
        assert_eq!(toasts.items()[0].id, id);
        toasts.expire(now + TIMEOUT);
        assert!(!toasts.is_empty());
        toasts.expire(now + TIMEOUT + Duration::from_secs(3));
        assert!(toasts.is_empty());
    }

    #[test]
    fn an_old_close_event_does_not_dismiss_a_newer_notification() {
        let mut toasts = Notifications::default();
        let now = Instant::now();
        toasts.push(Error::File, Context::General, now);
        let old_id = toasts.items()[0].id;
        toasts.expire(now + TIMEOUT);
        toasts.push(Error::Network, Context::General, now + TIMEOUT);
        toasts.dismiss(old_id);
        assert_eq!(toasts.items()[0].error, Error::Network);
        toasts.dismiss(toasts.items()[0].id);
        assert!(toasts.is_empty());
    }

    #[test]
    fn batch_failures_keep_the_number_of_visible_notifications_bounded() {
        let mut toasts = Notifications::default();
        let now = Instant::now();
        for index in 0..50 {
            toasts.push(Error::InvalidKey, Context::Key(index), now);
        }
        assert_eq!(toasts.items().len(), MAX_VISIBLE);
        assert_eq!(toasts.items()[0].context, Context::Key(47));
    }
}
