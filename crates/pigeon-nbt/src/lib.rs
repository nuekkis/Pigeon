mod decode;
mod encode;
mod value;

pub use decode::NbtDecodeError;
pub use decode::NbtReader;
pub use encode::NbtEncodeError;
pub use encode::NbtWriter;
pub use value::{Nbt, NbtCompound, NbtList, NbtTag, NbtValue};

pub const MAX_DEPTH: usize = 512;
