//! Server-wide player registry (M6).
//!
//! Tracks every play-phase player: uuid, name, gamemode, latency,
//! listed/tab-list visibility. The registry is `Arc<PlayerRegistry>`
//! shared between the accept loop (per-connection tasks) and the
//! status ping handler (so the `online` count reflects real players).
//!
//! M6 only wires the data side + `PlayerInfoUpdate`/`PlayerInfoRemove`
//! packet construction helpers; pushing those updates to every other
//! player (broadcast) ships in M6.5 alongside the chunk-streaming
//! broadcast fan-out.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use uuid::Uuid;

/// Default gamemode the server assigns on Play entry. 1 = survival.
pub const DEFAULT_GAMEMODE: i32 = 1;

/// Per-player snapshot kept in the registry. Fields mirror the
/// `PlayerInfoUpdate` payload so the registry can be encoded into a
/// `PlayerInfoUpdate` packet with no intermediate copying.
#[derive(Debug, Clone)]
pub struct PlayerRecord {
    pub uuid: Uuid,
    pub username: String,
    pub gamemode: i32,
    /// Whether the player shows up in the tab list (vanilla default: true).
    pub listed: bool,
    /// Last measured network latency in milliseconds (-1 = unknown).
    pub latency: i32,
    pub entity_id: i32,
}

impl PlayerRecord {
    fn new(uuid: Uuid, username: String, entity_id: i32) -> Self {
        Self {
            uuid,
            username,
            gamemode: DEFAULT_GAMEMODE,
            listed: true,
            latency: 0,
            entity_id,
        }
    }
}

/// Process-wide player registry.
#[derive(Debug, Default)]
pub struct PlayerRegistry {
    players: HashMap<Uuid, PlayerRecord>,
}

impl PlayerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new player; if a record with the same uuid exists, it
    /// is overwritten and the old record returned.
    pub fn insert(&mut self, record: PlayerRecord) -> Option<PlayerRecord> {
        self.players.insert(record.uuid, record)
    }

    pub fn remove(&mut self, uuid: &Uuid) -> Option<PlayerRecord> {
        self.players.remove(uuid)
    }

    pub fn get(&self, uuid: &Uuid) -> Option<&PlayerRecord> {
        self.players.get(uuid)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PlayerRecord> {
        self.players.values()
    }

    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    /// Update the per-player latency field. Returns the previous
    /// latency so the caller can detect changes.
    pub fn update_latency(&mut self, uuid: &Uuid, latency: i32) -> Option<i32> {
        let record = self.players.get_mut(uuid)?;
        let prev = record.latency;
        record.latency = latency;
        Some(prev)
    }
}

/// A shareable handle wrapping a [`PlayerRegistry`] with locking.
/// Clones cheaply; the inner registry is the single source of truth
/// across per-connection tasks.
#[derive(Debug, Clone)]
pub struct PlayerRegistryHandle {
    inner: Arc<Mutex<PlayerRegistry>>,
}

impl PlayerRegistryHandle {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlayerRegistry::new())),
        }
    }

    pub fn with_lock<R>(&self, f: impl FnOnce(&mut PlayerRegistry) -> R) -> R {
        let mut guard = self.inner.lock();
        f(&mut guard)
    }

    pub fn read<R>(&self, f: impl FnOnce(&PlayerRegistry) -> R) -> R {
        let guard = self.inner.lock();
        f(&guard)
    }

    /// Convenience: insert a freshly-connected player.
    pub fn admit(&self, uuid: Uuid, username: String, entity_id: i32) -> PlayerRecord {
        let record = PlayerRecord::new(uuid, username, entity_id);
        self.with_lock(|r| r.insert(record.clone()));
        record
    }

    /// Convenience: remove a parting player.
    pub fn depart(&self, uuid: &Uuid) -> Option<PlayerRecord> {
        self.with_lock(|r| r.remove(uuid))
    }

    /// Convenience: current online count.
    pub fn online(&self) -> usize {
        self.read(|r| r.len())
    }
}

impl Default for PlayerRegistryHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn admit_depart_roundtrip() {
        let reg = PlayerRegistryHandle::new();
        assert_eq!(reg.online(), 0);

        let r = reg.admit(uuid(1), "alice".to_string(), 1);
        assert_eq!(r.gamemode, DEFAULT_GAMEMODE);
        assert!(r.listed);
        assert_eq!(reg.online(), 1);

        let _ = reg.admit(uuid(2), "bob".to_string(), 2);
        assert_eq!(reg.online(), 2);

        let removed = reg.depart(&uuid(1));
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().username, "alice");
        assert_eq!(reg.online(), 1);
    }

    #[test]
    fn snapshot_for_status_ping() {
        let reg = PlayerRegistryHandle::new();
        for i in 0..5 {
            reg.admit(uuid(i as u128), format!("p{}", i), i);
        }
        // The status ping reads only the count, not the names.
        assert_eq!(reg.online(), 5);
    }

    #[test]
    fn update_latency_returns_previous() {
        let reg = PlayerRegistryHandle::new();
        reg.admit(uuid(42), "carol".to_string(), 7);
        let prev = reg.with_lock(|r| r.update_latency(&uuid(42), 100));
        assert_eq!(prev, Some(0));
        let prev = reg.with_lock(|r| r.update_latency(&uuid(42), 250));
        assert_eq!(prev, Some(100));
    }

    #[test]
    fn handle_clones_share_state() {
        let reg = PlayerRegistryHandle::new();
        let reg2 = reg.clone();
        reg.admit(uuid(1), "alice".to_string(), 1);
        assert_eq!(reg2.online(), 1);
    }
}
