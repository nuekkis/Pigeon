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

/// Canonical resource locations for the Java Configuration phase.
pub mod configuration {
    // --- Clientbound (S -> C) ---
    /// S -> C, id 0 — request a previously-stored cookie from the client.
    pub const COOKIE_REQUEST: &str = "minecraft:cookie_request";
    /// S -> C, id 1 — server-to-client plugin message (brand, etc.).
    pub const CUSTOM_PAYLOAD_CB: &str = "minecraft:custom_payload";
    /// S -> C, id 2 — disconnect the client during configuration.
    pub const DISCONNECT: &str = "minecraft:disconnect";
    /// S -> C, id 3 — terminate configuration, transition to Play.
    pub const FINISH_CONFIGURATION_CB: &str = "minecraft:finish_configuration";
    /// S -> C, id 4 — periodic keep-alive ping.
    pub const KEEP_ALIVE_CB: &str = "minecraft:keep_alive";
    /// S -> C, id 5 — ping the client (expects `Pong`).
    pub const PING: &str = "minecraft:ping";
    /// S -> C, id 6 — tell the client to reset its chat state.
    pub const RESET_CHAT: &str = "minecraft:reset_chat";
    /// S -> C, id 7 — synchronized registry contents.
    pub const REGISTRY_DATA: &str = "minecraft:registry_data";
    /// S -> C, id 8 — pop a previously pushed resource pack.
    pub const RESOURCE_PACK_POP: &str = "minecraft:resource_pack_pop";
    /// S -> C, id 9 — push a resource pack to the client.
    pub const RESOURCE_PACK_PUSH: &str = "minecraft:resource_pack_push";
    /// S -> C, id 10 — store a cookie on the client.
    pub const STORE_COOKIE: &str = "minecraft:store_cookie";
    /// S -> C, id 11 — transfer the client to another host.
    pub const TRANSFER: &str = "minecraft:transfer";
    /// S -> C, id 12 — update the set of enabled gameplay features.
    pub const UPDATE_ENABLED_FEATURES: &str = "minecraft:update_enabled_features";
    /// S -> C, id 13 — synchronize all tag registries.
    pub const UPDATE_TAGS: &str = "minecraft:update_tags";
    /// S -> C, id 14 — ask the client which known-packs it has.
    pub const SELECT_KNOWN_PACKS: &str = "minecraft:select_known_packs";
    /// S -> C, id 15 — custom report metadata for telemetry.
    pub const CUSTOM_REPORT_DETAILS: &str = "minecraft:custom_report_details";
    /// S -> C, id 16 — server link graph (support URLs, etc).
    pub const SERVER_LINKS: &str = "minecraft:server_links";
    /// S -> C, id 17 — close a currently-open dialog.
    pub const CLEAR_DIALOG: &str = "minecraft:clear_dialog";
    /// S -> C, id 18 — open a client dialog.
    pub const SHOW_DIALOG: &str = "minecraft:show_dialog";
    /// S -> C, id 19 — server's code of conduct (1.21.6+).
    pub const CODE_OF_CONDUCT: &str = "minecraft:code_of_conduct";

    // --- Serverbound (C -> S) ---
    /// C -> S, id 0 — client settings (locale, view distance, chat mode, …).
    pub const CLIENT_INFORMATION: &str = "minecraft:client_information";
    /// C -> S, id 1 — reply to `CookieRequest`.
    pub const COOKIE_RESPONSE: &str = "minecraft:cookie_response";
    /// C -> S, id 2 — client-to-server plugin message.
    pub const CUSTOM_PAYLOAD_SB: &str = "minecraft:custom_payload";
    /// C -> S, id 3 — client acknowledges configuration finished.
    pub const FINISH_CONFIGURATION_SB: &str = "minecraft:finish_configuration";
    /// C -> S, id 4 — client keep-alive pong.
    pub const KEEP_ALIVE_SB: &str = "minecraft:keep_alive";
    /// C -> S, id 5 — reply to `Ping`.
    pub const PONG: &str = "minecraft:pong";
    /// C -> S, id 6 — resource-pack status update.
    pub const RESOURCE_PACK: &str = "minecraft:resource_pack";
    /// C -> S, id 7 — reply to `SelectKnownPacks`.
    pub const SELECT_KNOWN_PACKS_SB: &str = "minecraft:select_known_packs";
    /// C -> S, id 8 — custom click action (1.21.6+).
    pub const CUSTOM_CLICK_ACTION: &str = "minecraft:custom_click_action";
    /// C -> S, id 9 — client accepts the code of conduct.
    pub const ACCEPT_CODE_OF_CONDUCT: &str = "minecraft:accept_code_of_conduct";
}

/// Canonical resource locations for the Java Play phase.
///
/// Only the packets required to drive the initial play handshake and
/// keep the client alive are listed here; the full 139+66 packet
/// surface ships as later milestones wire gameplay.
pub mod play {
    // --- Clientbound (S -> C) ---
    /// S -> C, id 48 — `LoginPlay`: tells the client about the world
    /// it has joined (dimension, gamemode, spawn point, …).
    pub const LOGIN: &str = "minecraft:login";
    /// S -> C, id 43 — server keep-alive ping.
    pub const KEEP_ALIVE: &str = "minecraft:keep_alive";
    /// S -> C, id 32 — disconnect from play state with a chat reason.
    pub const DISCONNECT: &str = "minecraft:disconnect";
    /// S -> C, id 67 — remove the listed players by uuid.
    pub const PLAYER_INFO_REMOVE: &str = "minecraft:player_info_remove";
    /// S -> C, id 68 — push per-player updates (add/listed/gamemode/latency/display).
    pub const PLAYER_INFO_UPDATE: &str = "minecraft:player_info_update";

    // --- Serverbound (C -> S) ---
    /// C -> S, id 27 — client keep-alive pong.
    pub const KEEP_ALIVE_SB: &str = "minecraft:keep_alive";
    /// C -> S, id 29 — `ServerboundMovePlayerPosPacket`.
    pub const MOVE_PLAYER_POS: &str = "minecraft:move_player_pos";
    /// C -> S, id 30 — `ServerboundMovePlayerPosRotPacket`.
    pub const MOVE_PLAYER_POS_ROT: &str = "minecraft:move_player_pos_rot";
    /// C -> S, id 31 — `ServerboundMovePlayerRotPacket`.
    pub const MOVE_PLAYER_ROT: &str = "minecraft:move_player_rot";
    /// C -> S, id 32 — `ServerboundMovePlayerStatusOnlyPacket` (on-ground).
    pub const MOVE_PLAYER_STATUS_ONLY: &str = "minecraft:move_player_status_only";
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

    #[test]
    fn configuration_ids_match_data_report() {
        // S -> C (clientbound)
        assert_eq!(
            clientbound("configuration", configuration::COOKIE_REQUEST),
            0
        );
        assert_eq!(
            clientbound("configuration", configuration::CUSTOM_PAYLOAD_CB),
            1
        );
        assert_eq!(clientbound("configuration", configuration::DISCONNECT), 2);
        assert_eq!(
            clientbound("configuration", configuration::FINISH_CONFIGURATION_CB),
            3
        );
        assert_eq!(
            clientbound("configuration", configuration::KEEP_ALIVE_CB),
            4
        );
        assert_eq!(clientbound("configuration", configuration::PING), 5);
        assert_eq!(clientbound("configuration", configuration::RESET_CHAT), 6);
        assert_eq!(
            clientbound("configuration", configuration::REGISTRY_DATA),
            7
        );
        assert_eq!(
            clientbound("configuration", configuration::RESOURCE_PACK_POP),
            8
        );
        assert_eq!(
            clientbound("configuration", configuration::RESOURCE_PACK_PUSH),
            9
        );
        assert_eq!(
            clientbound("configuration", configuration::STORE_COOKIE),
            10
        );
        assert_eq!(clientbound("configuration", configuration::TRANSFER), 11);
        assert_eq!(
            clientbound("configuration", configuration::UPDATE_ENABLED_FEATURES),
            12
        );
        assert_eq!(clientbound("configuration", configuration::UPDATE_TAGS), 13);
        assert_eq!(
            clientbound("configuration", configuration::SELECT_KNOWN_PACKS),
            14
        );
        assert_eq!(
            clientbound("configuration", configuration::CUSTOM_REPORT_DETAILS),
            15
        );
        assert_eq!(
            clientbound("configuration", configuration::SERVER_LINKS),
            16
        );
        assert_eq!(
            clientbound("configuration", configuration::CLEAR_DIALOG),
            17
        );
        assert_eq!(clientbound("configuration", configuration::SHOW_DIALOG), 18);
        assert_eq!(
            clientbound("configuration", configuration::CODE_OF_CONDUCT),
            19
        );

        // C -> S (serverbound)
        assert_eq!(
            serverbound("configuration", configuration::CLIENT_INFORMATION),
            0
        );
        assert_eq!(
            serverbound("configuration", configuration::COOKIE_RESPONSE),
            1
        );
        assert_eq!(
            serverbound("configuration", configuration::CUSTOM_PAYLOAD_SB),
            2
        );
        assert_eq!(
            serverbound("configuration", configuration::FINISH_CONFIGURATION_SB),
            3
        );
        assert_eq!(
            serverbound("configuration", configuration::KEEP_ALIVE_SB),
            4
        );
        assert_eq!(serverbound("configuration", configuration::PONG), 5);
        assert_eq!(
            serverbound("configuration", configuration::RESOURCE_PACK),
            6
        );
        assert_eq!(
            serverbound("configuration", configuration::SELECT_KNOWN_PACKS_SB),
            7
        );
        assert_eq!(
            serverbound("configuration", configuration::CUSTOM_CLICK_ACTION),
            8
        );
        assert_eq!(
            serverbound("configuration", configuration::ACCEPT_CODE_OF_CONDUCT),
            9
        );
    }

    #[test]
    fn play_ids_match_data_report() {
        // S -> C (clientbound)
        assert_eq!(clientbound("play", play::LOGIN), 48);
        assert_eq!(clientbound("play", play::KEEP_ALIVE), 43);
        assert_eq!(clientbound("play", play::DISCONNECT), 32);
        assert_eq!(clientbound("play", play::PLAYER_INFO_REMOVE), 67);
        assert_eq!(clientbound("play", play::PLAYER_INFO_UPDATE), 68);
        // C -> S (serverbound)
        assert_eq!(serverbound("play", play::KEEP_ALIVE_SB), 27);
        assert_eq!(serverbound("play", play::MOVE_PLAYER_POS), 29);
        assert_eq!(serverbound("play", play::MOVE_PLAYER_POS_ROT), 30);
        assert_eq!(serverbound("play", play::MOVE_PLAYER_ROT), 31);
        assert_eq!(serverbound("play", play::MOVE_PLAYER_STATUS_ONLY), 32);
    }
}
