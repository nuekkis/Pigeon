//! Java Edition protocol implementation.
//!
//! The Java protocol is divided into four connection states:
//! [`Status`], [`Login`], [`Configuration`], and [`Play`]. Each state
//! has a separate set of `C→S` (clientbound-to-server) and `S→C`
//! (serverbound-to-client) packets.

pub mod client;
pub mod server;
pub mod state;
pub mod status;
pub mod login;

pub use state::ProtocolState;
