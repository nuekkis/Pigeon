//! Pigeon network protocol implementation.
//!
//! Currently Java-only. Bedrock support will be added behind the `bedrock`
//! module boundary in a future milestone without refactoring the Java side.

pub mod codec;
pub mod java;
pub mod ser;

pub use codec::framed::{
    DecodedPacket, EncodedPacket, PacketCodec, PacketDecodeError, PacketEncodeError,
};
