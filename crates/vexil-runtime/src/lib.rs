pub mod bit_reader;
pub mod bit_writer;
pub mod error;
pub mod leb128;
pub mod traits;
pub mod zigzag;

pub use bit_reader::BitReader;
pub use bit_writer::BitWriter;
pub use error::{DecodeError, EncodeError};
pub use traits::{Pack, Unpack};

pub const MAX_BYTES_LENGTH: u64 = 1 << 26;
pub const MAX_COLLECTION_COUNT: u64 = 1 << 24;
pub const MAX_LENGTH_PREFIX_BYTES: u8 = 4;
pub const MAX_RECURSION_DEPTH: u32 = 64;
