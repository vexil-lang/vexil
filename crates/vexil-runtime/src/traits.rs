use crate::bit_reader::BitReader;
use crate::bit_writer::BitWriter;
use crate::error::{DecodeError, EncodeError};

pub trait Pack {
    fn pack(&self, writer: &mut BitWriter) -> Result<(), EncodeError>;
}

pub trait Unpack: Sized {
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
