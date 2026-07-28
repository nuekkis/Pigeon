//! Server-wide player registry (M6+M6.5).
//!
//! Tracks every play-phase player: uuid, name, gamemode, latency,
//! listed/tab-list visibility. The registry is `Arc<PlayerRegistry>`
//! shared between the accept loop (per-connection tasks) and the
//! status ping handler (so the `online` count reflects real players).
//!
//! M6 wires the data side and admits/removes a player from the count.
//! M6.5 adds an outbound broadcast bus: each admitted player owns an
//! `mpsc::Sender<EncodedPacket>`; when a new player joins, the
//! registry broadcasts the joiner's `PlayerInfoUpdate` to every other
//! online player and (separately) hands the joiner a snapshot of the
//! currently-online player list so the joiner's tab list is correct.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use pigeon_protocol::EncodedPacket;
use tokio::sync::mpsc;
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
    /// Per-player outbound mpsc sender. Each connection's [`handle_play`]
    /// task owns the matching `mpsc::Receiver` and forwards anything
    /// pushed here to its framed socket — so a broadcast is a single
    /// `try_send` per peer.
    senders: HashMap<Uuid, mpsc::Sender<EncodedPacket>>,
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
        self.senders.remove(uuid);
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

    /// Register an outbound mpsc channel for a freshly admitted player.
    /// Future [`broadcast`] / [`send_to`] calls pushed onto this
    /// registry will route through this sender. Returns the matching
    /// receiver the caller owns for the player's connection task.
    pub fn attach_channel(&mut self, uuid: Uuid, capacity: usize) -> mpsc::Receiver<EncodedPacket> {
        let (tx, rx) = mpsc::channel(capacity);
        self.senders.insert(uuid, tx);
        rx
    }

    /// Send a single packet to one specific online player. Returns
    /// `Err` if the channel is closed (player departed between the
    /// snapshot and the send).
    pub fn send_to(&self, uuid: &Uuid, packet: EncodedPacket) -> Result<(), EncodedPacket> {
        match self.senders.get(uuid) {
            Some(tx) => match tx.try_send(packet) {
                Ok(()) => Ok(()),
                Err(mpsc::error::TrySendError::Full(p)) => Err(p),
                Err(mpsc::error::TrySendError::Closed(p)) => Err(p),
            },
            None => Err(packet),
        }
    }

    /// Broadcast a single packet to every online player **except** the
    /// one identified by `skip`. Packets that fail to enqueue (full /
    /// closed) are logged at debug level and dropped — a slow or
    /// recently-departed peer must never block the broadcast fan-out.
    pub fn broadcast_except(&self, skip: &Uuid, packet: &EncodedPacket) {
        for (uuid, tx) in &self.senders {
            if uuid == skip {
                continue;
            }
            if let Err(err) = tx.try_send(packet.clone()) {
                tracing::debug!(%uuid, %err, "broadcast drop");
            }
        }
    }

    /// Snapshot of every online player's record **excluding** the one
    /// identified by `skip` — used to build the joiner's initial tab
    /// list when it enters play.
    pub fn others(&self, skip: &Uuid) -> Vec<PlayerRecord> {
        self.players
            .values()
            .filter(|r| r.uuid != *skip)
            .cloned()
            .collect()
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

    /// Convenience: remove a parting player. Also drops the cached
    /// outbound sender so subsequent broadcasts skip them.
    pub fn depart(&self, uuid: &Uuid) -> Option<PlayerRecord> {
        self.with_lock(|r| r.remove(uuid))
    }

    /// Convenience: current online count.
    pub fn online(&self) -> usize {
        self.read(|r| r.len())
    }

    /// M6.5: attach an outbound channel to an already-admitted player,
    /// broadcast the joiner's encoded `PlayerInfoUpdate` packet to every
    /// other online peer, and return the joiner's `mpsc::Receiver`
    /// paired with a `Vec<PlayerRecord>` snapshot of every other online
    /// player so the caller can build a single `PlayerInfoUpdate`
    /// describing everyone currently online.
    ///
    /// The caller is expected to have called [`admit`] earlier (during
    /// login) so the registry already has the `PlayerRecord`.
    pub fn attach_and_snapshot(
        &self,
        uuid: Uuid,
        joiner_packet: EncodedPacket,
    ) -> (mpsc::Receiver<EncodedPacket>, Vec<PlayerRecord>) {
        let mut snapshot = Vec::new();
        let rx = self.with_lock(|r| {
            snapshot = r.others(&uuid);
            r.broadcast_except(&uuid, &joiner_packet);
            // Attach the joiner's outbound channel — done last so the
            // broadcast above did not loop back into the joiner's own
            // receiver.
            r.attach_channel(uuid, 64)
        });
        (rx, snapshot)
    }

    /// M6.5: remove a parting player from the registry **and** broadcast
    /// the supplied `PlayerInfoRemove` packet to every still-online peer.
    /// The channel for the parting player is dropped atomically so no
    /// later broadcast reaches them. Returns the removed record if the
    /// player was actually registered.
    pub fn depart_and_broadcast(
        &self,
        uuid: &Uuid,
        remove_packet: EncodedPacket,
    ) -> Option<PlayerRecord> {
        self.with_lock(|r| {
            let removed = r.remove(uuid);
            // `broadcast_except` would skip the departing peer (using
            // the uuid filter) — but `remove` already dropped their
            // sender from the map. Either way, broadcasting to
            // *everyone current* is exactly what we want here; there's
            // no risk of looping back into the departed receiver.
            r.broadcast_except(uuid, &remove_packet);
            removed
        })
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

    /// Helper: build an `EncodedPacket` stand-in so the broadcast path
    /// can carry arbitrary payloads through `mpsc::Sender<EncodedPacket>`
    /// in tests without pulling the full protocol encoder.
    fn dummy_packet(tag: u8) -> EncodedPacket {
        EncodedPacket::new(tag as i32, bytes::Bytes::from_static(&[]))
    }

    #[tokio::test]
    async fn attach_and_snapshot_relays_joiner_broadcast() {
        let reg = PlayerRegistryHandle::new();
        // Alice admits first; her snapshot is empty (nobody else online).
        reg.admit(uuid(1), "alice".to_string(), 1);
        let joiner_pkt = dummy_packet(0x44);
        let (mut alice_rx, others) = reg.attach_and_snapshot(uuid(1), joiner_pkt);
        assert!(others.is_empty(), "no snapshot when alone");

        // Bob admits next. His joiner broadcast goes OUT to Alice (the
        // only other online peer), and his snapshot contains Alice.
        reg.admit(uuid(2), "bob".to_string(), 2);
        let bob_joiner_pkt = dummy_packet(0x44);
        let (mut bob_rx, others) = reg.attach_and_snapshot(uuid(2), bob_joiner_pkt.clone());
        assert_eq!(others.len(), 1);
        assert_eq!(others[0].username, "alice");

        // Both receivers should have the *opposite* player's joiner packet.
        let received_by_alice = alice_rx.recv().await;
        assert!(received_by_alice.is_some(), "alice got the broadcast");
        assert_eq!(received_by_alice.unwrap().id, bob_joiner_pkt.id);
        // Bob's own receiver must be empty — he never broadcast to himself.
        let bob_drained = bob_rx.try_recv();
        assert!(bob_drained.is_err(), "bob's own channel is empty");

        // Charlie joins and should receive BOTH Alice and Bob as a snapshot.
        reg.admit(uuid(3), "charlie".to_string(), 3);
        let charlie_pkt = dummy_packet(0x44);
        let (mut charlie_rx, others) = reg.attach_and_snapshot(uuid(3), charlie_pkt.clone());
        assert_eq!(others.len(), 2);
        assert!(others.iter().any(|r| r.username == "alice"));
        assert!(others.iter().any(|r| r.username == "bob"));

        // Alice and Bob each should have received Charlie's broadcast.
        let received_by_alice_for_charlie = alice_rx.recv().await;
        assert!(received_by_alice_for_charlie.is_some());
        assert_eq!(received_by_alice_for_charlie.unwrap().id, charlie_pkt.id);
        let received_by_bob_for_charlie = bob_rx.recv().await;
        assert!(received_by_bob_for_charlie.is_some());
        assert_eq!(received_by_bob_for_charlie.unwrap().id, charlie_pkt.id);
        // Charlie's own channel must be empty.
        assert!(
            charlie_rx.try_recv().is_err(),
            "charlie's own channel is empty"
        );
    }

    #[tokio::test]
    async fn depart_and_broadcast_fans_out_remove() {
        let reg = PlayerRegistryHandle::new();
        // Three players attach their channels — keep their receivers.
        reg.admit(uuid(1), "alice".to_string(), 1);
        let (mut alice_rx, _) = reg.attach_and_snapshot(uuid(1), dummy_packet(0x44));
        reg.admit(uuid(2), "bob".to_string(), 2);
        let (mut bob_rx, _) = reg.attach_and_snapshot(uuid(2), dummy_packet(0x44));
        reg.admit(uuid(3), "charlie".to_string(), 3);
        let (mut charlie_rx, _) = reg.attach_and_snapshot(uuid(3), dummy_packet(0x44));

        // Drain the per-joiner broadcasts so receivers are clean before
        // the departure test; otherwise Bob's joiner packet (still
        // queued for Alice) would shadow the remove broadcast below.
        let _ = alice_rx.recv().await;
        let _ = alice_rx.recv().await;
        let _ = bob_rx.recv().await;

        // Bob (uuid=2) departs — broadcast PlayerInfoRemove to Alice & Charlie.
        let remove_pkt = dummy_packet(0x43);
        let removed = reg.depart_and_broadcast(&uuid(2), remove_pkt);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().username, "bob");
        assert_eq!(reg.online(), 2);

        // Alice and Charlie should each have exactly one queued remove packet.
        let received_by_alice = alice_rx.recv().await;
        assert!(
            received_by_alice.is_some(),
            "alice got the remove broadcast"
        );
        assert_eq!(received_by_alice.unwrap().id, 0x43);
        let received_by_charlie = charlie_rx.recv().await;
        assert!(
            received_by_charlie.is_some(),
            "charlie got the remove broadcast"
        );
        assert_eq!(received_by_charlie.unwrap().id, 0x43);

        // Depart when only one peer remains — broadcast is a no-op but
        // removes cleanly.
        let _ = reg.depart_and_broadcast(&uuid(1), dummy_packet(0x43));
        assert_eq!(reg.online(), 1);
        let r = reg.depart_and_broadcast(&uuid(3), dummy_packet(0x43));
        assert!(r.is_some());
        assert_eq!(reg.online(), 0);
    }
}
