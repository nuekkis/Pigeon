//! Packet id lookup backed by the embedded `packets.json` report.
//!
//! Instead of hand-curating `const ID: i32 = ...` values across the protocol
//! crates, every packet type carries its canonical Mojang resource location
//! here and resolves its wire id through [`pigeon_data::packets`].
//!
//! This keeps drift between the typed Rust packets and the official
//! data-generator output detectable: if Mojang renumbers a packet, the
//! build-time `pigeon-data` tests will detect the divergence before runtime
//! ever sees a malformed frame.

/// Canonical resource locations for the Java Status phase.
pub mod status {
    /// C -> S, id 0 — empty status request.
    pub const STATUS_REQUEST: &str = "minecraft:status_request";
    /// S -> C, id 0 — JSON server-list response.
    pub const STATUS_RESPONSE: &str = "minecraft:status_response";
    /// C -> S, id 1 — ping probe with an arbitrary payload.
    pub const PING_REQUEST: &str = "minecraft:ping_request";
    /// S -> C, id 1 — pong echo carrying the same payload.
    pub const PONG_RESPONSE: &str = "minecraft:pong_response";
}

/// Canonical resource locations for the Java Handshake phase.
pub mod handshake {
    /// C -> S, id 0 — `Intention` packet (the only handshake packet).
    /// Selects the next state (status or login).
    pub const INTENTION: &str = "minecraft:intention";
}

/// Canonical resource locations for the Java Login phase.
pub mod login {
    /// C -> S, id 0 — `Login Start` (player name + uuid).
    pub const HELLO: &str = "minecraft:hello";
    /// S -> C, id 0 — `Disconnect (Login)`.
    pub const LOGIN_DISCONNECT: &str = "minecraft:login_disconnect";
    /// S -> C, id 1 — `Encryption Request`.
    pub const HELLO_CB: &str = "minecraft:hello";
    /// C -> S, id 1 — `Encryption Response`.
    pub const KEY: &str = "minecraft:key";
    /// S -> C, id 2 — `Login Success`.
    pub const LOGIN_FINISHED: &str = "minecraft:login_finished";
    /// S -> C, id 3 — `Set Compression`.
    pub const LOGIN_COMPRESSION: &str = "minecraft:login_compression";
    /// C -> S, id 2 — `Login Plugin Response`.
    pub const CUSTOM_QUERY_ANSWER: &str = "minecraft:custom_query_answer";
    /// S -> C, id 4 — `Login Plugin Request`.
    pub const CUSTOM_QUERY: &str = "minecraft:custom_query";
    /// C -> S, id 3 — `Login Acknowledged`.
    pub const LOGIN_ACKNOWLEDGED: &str = "minecraft:login_acknowledged";
    /// S -> C, id 5 — `Cookie Request` (1.20.5+).
    pub const COOKIE_REQUEST: &str = "minecraft:cookie_request";
    /// C -> S, id 4 — `Cookie Response` (1.20.5+).
    pub const COOKIE_RESPONSE: &str = "minecraft:cookie_response";
}

/// Resolve a clientbound packet id in the given phase.
pub fn clientbound(phase: &str, packet: &str) -> i32 {
    pigeon_data::packets::clientbound_id(phase, packet)
        .unwrap_or_else(|| panic!("clientbound packet {packet} missing from {phase} report"))
}

/// Resolve a serverbound packet id in the given phase.
pub fn serverbound(phase: &str, packet: &str) -> i32 {
    pigeon_data::packets::serverbound_id(phase, packet)
        .unwrap_or_else(|| panic!("serverbound packet {packet} missing from {phase} report"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As a safety net against silent renumberings in vanilla data, every
    /// packet constant referenced by the typed protocol modules must resolve
    /// to the expected 1.21.11 wire id from `packets.json`.
    #[test]
    fn status_ids_match_data_report() {
        assert_eq!(serverbound("status", status::STATUS_REQUEST), 0);
        assert_eq!(clientbound("status", status::STATUS_RESPONSE), 0);
        assert_eq!(serverbound("status", status::PING_REQUEST), 1);
        assert_eq!(clientbound("status", status::PONG_RESPONSE), 1);
    }

    #[test]
    fn handshake_ids_match_data_report() {
        assert_eq!(serverbound("handshake", handshake::INTENTION), 0);
    }

    #[test]
    fn login_ids_match_data_report() {
        // C -> S side.
        assert_eq!(serverbound("login", login::HELLO), 0);
        assert_eq!(serverbound("login", login::KEY), 1);
        assert_eq!(serverbound("login", login::CUSTOM_QUERY_ANSWER), 2);
        assert_eq!(serverbound("login", login::LOGIN_ACKNOWLEDGED), 3);
        assert_eq!(serverbound("login", login::COOKIE_RESPONSE), 4);
        // S -> C side.
        assert_eq!(clientbound("login", login::LOGIN_DISCONNECT), 0);
        assert_eq!(clientbound("login", login::HELLO_CB), 1);
        assert_eq!(clientbound("login", login::LOGIN_FINISHED), 2);
        assert_eq!(clientbound("login", login::LOGIN_COMPRESSION), 3);
        assert_eq!(clientbound("login", login::CUSTOM_QUERY), 4);
        assert_eq!(clientbound("login", login::COOKIE_REQUEST), 5);
    }
}
