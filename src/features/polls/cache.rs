use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct PollCache {
    inner: Arc<DashMap<Uuid, DateTime<Utc>>>,
}

#[allow(dead_code)] // i don't care, im setting up a structure for it now
impl PollCache {
    /// creates a new, empty `PollCache`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// insert or update a poll in the cache.
    pub fn insert(&self, id: Uuid, expires_at: DateTime<Utc>) {
        self.inner.insert(id, expires_at);
    }

    /// get a poll's expiration time, returning `None` if not cached.
    pub fn get(&self, id: &Uuid) -> Option<DateTime<Utc>> {
        self.inner.get(id).map(|entry| *entry.value())
    }

    /// check if a poll is present in the cache.
    pub fn contains(&self, id: &Uuid) -> bool {
        self.inner.contains_key(id)
    }

    /// remove a poll from the cache manually (e.g., when closed or deleted).
    pub fn remove(&self, id: &Uuid) -> Option<DateTime<Utc>> {
        self.inner.remove(id).map(|(_, v)| v)
    }

    /// atomically replace the entire cache with a fresh set of polls.
    pub fn sync_all(&self, fresh_polls: impl IntoIterator<Item = (Uuid, DateTime<Utc>)>) {
        self.inner.clear();
        for (id, expires_at) in fresh_polls {
            self.inner.insert(id, expires_at);
        }
    }

    /// remove all polls whose expiration timestamp is in the past.
    /// returns the number of evicted polls.
    pub fn evict_expired(&self) -> usize {
        let now = Utc::now();
        let initial_len = self.inner.len();

        self.inner.retain(|_, expires_at| *expires_at > now);

        initial_len - self.inner.len()
    }

    /// get the current total count of cached polls.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// extracts and returns all IDs whose expiration time is past
    pub fn retain_expired(&self) -> Vec<Uuid> {
        let now = Utc::now();
        let mut expired = Vec::new();

        // dashmap retain lets us filter while building our list of expired keys
        self.inner.retain(|id, ends_at| {
            if *ends_at <= now {
                expired.push(*id);
                false // remove from map
            } else {
                true // keep in map
            }
        });

        expired
    }
}
