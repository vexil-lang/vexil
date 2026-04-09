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

// ==================== Primitive Pack/Unpack Implementations ====================

impl Pack for bool {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_bool(*self);
        Ok(())
    }
}

impl Unpack for bool {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_bool()
    }
}

impl Pack for u8 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u8(*self);
        Ok(())
    }
}

impl Unpack for u8 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_u8()
    }
}

impl Pack for u16 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u16(*self);
        Ok(())
    }
}

impl Unpack for u16 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_u16()
    }
}

impl Pack for u32 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u32(*self);
        Ok(())
    }
}

impl Unpack for u32 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_u32()
    }
}

impl Pack for u64 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u64(*self);
        Ok(())
    }
}

impl Unpack for u64 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_u64()
    }
}

impl Pack for i8 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_i8(*self);
        Ok(())
    }
}

impl Unpack for i8 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_i8()
    }
}

impl Pack for i16 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_i16(*self);
        Ok(())
    }
}

impl Unpack for i16 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_i16()
    }
}

impl Pack for i32 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_i32(*self);
        Ok(())
    }
}

impl Unpack for i32 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_i32()
    }
}

impl Pack for i64 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_i64(*self);
        Ok(())
    }
}

impl Unpack for i64 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_i64()
    }
}

impl Pack for f32 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_f32(*self);
        Ok(())
    }
}

impl Unpack for f32 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_f32()
    }
}

impl Pack for f64 {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_f64(*self);
        Ok(())
    }
}

impl Unpack for f64 {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        r.read_f64()
    }
}
