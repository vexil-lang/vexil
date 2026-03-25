use crate::bit_reader::BitReader;
use crate::bit_writer::BitWriter;
use crate::error::{DecodeError, EncodeError};

pub trait Pack {
    fn pack(&self, writer: &mut BitWriter) -> Result<(), EncodeError>;
}

pub trait Unpack: Sized {
    fn unpack(reader: &mut BitReader<'_>) -> Result<Self, DecodeError>;
}
