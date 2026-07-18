mod decode;
mod encode;
mod value;

pub use decode::NbtDecodeError;
pub use decode::NbtReader;
pub use encode::NbtEncodeError;
pub use encode::NbtWriter;
pub use value::{Nbt, NbtTag, NbtValue};

pub const MAX_DEPTH: usize = 512;
