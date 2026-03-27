use crate::bit_reader::BitReader;
use crate::bit_writer::BitWriter;
use crate::error::{DecodeError, EncodeError};

/// Serialize a value into a Vexil wire-format byte stream.
///
/// Generated code implements this trait for every Vexil message, enum, union,
/// and newtype. You typically call [`Pack::pack`] indirectly through the
/// generated `encode()` convenience method rather than using it directly.
pub trait Pack {
    /// Write this value's fields into `writer` using LSB-first bit packing.
    fn pack(&self, writer: &mut BitWriter) -> Result<(), EncodeError>;
}

/// Deserialize a value from a Vexil wire-format byte stream.
///
/// Generated code implements this trait for every Vexil message, enum, union,
/// and newtype. You typically call [`Unpack::unpack`] indirectly through the
/// generated `decode()` convenience method rather than using it directly.
pub trait Unpack: Sized {
    /// Read this value's fields from `reader` using LSB-first bit packing.
    fn unpack(reader: &mut BitReader<'_>) -> Result<Self, DecodeError>;
}

impl<T: Pack> Pack for Box<T> {
    fn pack(&self, writer: &mut BitWriter) -> Result<(), EncodeError> {
        (**self).pack(writer)
    }
}

impl<T: Unpack> Unpack for Box<T> {
    fn unpack(reader: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Box::new(T::unpack(reader)?))
    }
}
