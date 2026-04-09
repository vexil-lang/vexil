//! Geometric types for Vexil runtime.
//!
//! Vec2, Vec3, Vec4, Quat, Mat3, Mat4 with basic operations and Pack/Unpack support.

use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::bit_reader::BitReader;
use crate::bit_writer::BitWriter;
use crate::error::{DecodeError, EncodeError};
use crate::traits::{Pack, Unpack};

/// 2-component vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

/// 3-component vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

/// 4-component vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

/// Quaternion.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Quat<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

/// 3x3 matrix (column-major).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Mat3<T> {
    pub cols: [Vec3<T>; 3],
}

/// 4x4 matrix (column-major).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Mat4<T> {
    pub cols: [Vec4<T>; 4],
}

// ==================== Vec2 Implementations ====================

impl<T> Vec2<T> {
    /// Create a new Vec2.
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Add<Output = T>> Add for Vec2<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vec2<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vec2<T> {
    type Output = Self;
    fn div(self, scalar: T) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl<T: Neg<Output = T>> Neg for Vec2<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<T: Pack> Pack for Vec2<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        self.x.pack(w)?;
        self.y.pack(w)
    }
}

impl<T: Unpack> Unpack for Vec2<T> {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            x: T::unpack(r)?,
            y: T::unpack(r)?,
        })
    }
}

// ==================== Vec3 Implementations ====================

impl<T> Vec3<T> {
    /// Create a new Vec3.
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T: Add<Output = T>> Add for Vec3<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vec3<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec3<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vec3<T> {
    type Output = Self;
    fn div(self, scalar: T) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl<T: Neg<Output = T>> Neg for Vec3<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl<T: Pack> Pack for Vec3<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        self.x.pack(w)?;
        self.y.pack(w)?;
        self.z.pack(w)
    }
}

impl<T: Unpack> Unpack for Vec3<T> {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            x: T::unpack(r)?,
            y: T::unpack(r)?,
            z: T::unpack(r)?,
        })
    }
}

// ==================== Vec4 Implementations ====================

impl<T> Vec4<T> {
    /// Create a new Vec4.
    pub const fn new(x: T, y: T, z: T, w: T) -> Self {
        Self { x, y, z, w }
    }
}

impl<T: Add<Output = T>> Add for Vec4<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vec4<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec4<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vec4<T> {
    type Output = Self;
    fn div(self, scalar: T) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
            w: self.w / scalar,
        }
    }
}

impl<T: Neg<Output = T>> Neg for Vec4<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }
}

impl<T: Pack> Pack for Vec4<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        self.x.pack(w)?;
        self.y.pack(w)?;
        self.z.pack(w)?;
        self.w.pack(w)
    }
}

impl<T: Unpack> Unpack for Vec4<T> {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            x: T::unpack(r)?,
            y: T::unpack(r)?,
            z: T::unpack(r)?,
            w: T::unpack(r)?,
        })
    }
}

// ==================== Quat Implementations ====================

impl<T> Quat<T> {
    /// Create a new Quaternion.
    pub const fn new(x: T, y: T, z: T, w: T) -> Self {
        Self { x, y, z, w }
    }
}

impl<T: Add<Output = T>> Add for Quat<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Quat<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Quat<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Quat<T> {
    type Output = Self;
    fn div(self, scalar: T) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
            w: self.w / scalar,
        }
    }
}

impl<T: Neg<Output = T>> Neg for Quat<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }
}

impl<T: Pack> Pack for Quat<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        self.x.pack(w)?;
        self.y.pack(w)?;
        self.z.pack(w)?;
        self.w.pack(w)
    }
}

impl<T: Unpack> Unpack for Quat<T> {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            x: T::unpack(r)?,
            y: T::unpack(r)?,
            z: T::unpack(r)?,
            w: T::unpack(r)?,
        })
    }
}

// ==================== Mat3 Implementations ====================

impl<T> Mat3<T> {
    /// Create a new Mat3 from three column vectors.
    pub const fn new(c0: Vec3<T>, c1: Vec3<T>, c2: Vec3<T>) -> Self {
        Self { cols: [c0, c1, c2] }
    }

    /// Create an identity matrix (requires T: Default + From<u8>).
    pub fn identity() -> Self
    where
        T: Default + From<u8>,
    {
        Self {
            cols: [
                Vec3::new(T::from(1), T::default(), T::default()),
                Vec3::new(T::default(), T::from(1), T::default()),
                Vec3::new(T::default(), T::default(), T::from(1)),
            ],
        }
    }
}

impl<T: Pack> Pack for Mat3<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        self.cols[0].pack(w)?;
        self.cols[1].pack(w)?;
        self.cols[2].pack(w)
    }
}

impl<T: Unpack> Unpack for Mat3<T> {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            cols: [Vec3::unpack(r)?, Vec3::unpack(r)?, Vec3::unpack(r)?],
        })
    }
}

// ==================== Mat4 Implementations ====================

impl<T> Mat4<T> {
    /// Create a new Mat4 from four column vectors.
    pub const fn new(c0: Vec4<T>, c1: Vec4<T>, c2: Vec4<T>, c3: Vec4<T>) -> Self {
        Self {
            cols: [c0, c1, c2, c3],
        }
    }

    /// Create an identity matrix (requires T: Default + From<u8>).
    pub fn identity() -> Self
    where
        T: Default + From<u8>,
    {
        Self {
            cols: [
                Vec4::new(T::from(1), T::default(), T::default(), T::default()),
                Vec4::new(T::default(), T::from(1), T::default(), T::default()),
                Vec4::new(T::default(), T::default(), T::from(1), T::default()),
                Vec4::new(T::default(), T::default(), T::default(), T::from(1)),
            ],
        }
    }
}

impl<T: Pack> Pack for Mat4<T> {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        self.cols[0].pack(w)?;
        self.cols[1].pack(w)?;
        self.cols[2].pack(w)?;
        self.cols[3].pack(w)
    }
}

impl<T: Unpack> Unpack for Mat4<T> {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            cols: [
                Vec4::unpack(r)?,
                Vec4::unpack(r)?,
                Vec4::unpack(r)?,
                Vec4::unpack(r)?,
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2_new() {
        let v = Vec2::new(1.0f32, 2.0f32);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn vec2_add() {
        let a = Vec2::new(1, 2);
        let b = Vec2::new(3, 4);
        let c = a + b;
        assert_eq!(c.x, 4);
        assert_eq!(c.y, 6);
    }

    #[test]
    fn vec2_sub() {
        let a = Vec2::new(5, 7);
        let b = Vec2::new(2, 3);
        let c = a - b;
        assert_eq!(c.x, 3);
        assert_eq!(c.y, 4);
    }

    #[test]
    fn vec2_mul_scalar() {
        let a = Vec2::new(2.0f32, 3.0f32);
        let b = a * 2.0f32;
        assert_eq!(b.x, 4.0);
        assert_eq!(b.y, 6.0);
    }

    #[test]
    fn vec2_neg() {
        let a = Vec2::new(1.0f32, -2.0f32);
        let b = -a;
        assert_eq!(b.x, -1.0);
        assert_eq!(b.y, 2.0);
    }

    #[test]
    fn vec3_new() {
        let v = Vec3::new(1.0f32, 2.0f32, 3.0f32);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn vec3_add() {
        let a = Vec3::new(1, 2, 3);
        let b = Vec3::new(4, 5, 6);
        let c = a + b;
        assert_eq!(c.x, 5);
        assert_eq!(c.y, 7);
        assert_eq!(c.z, 9);
    }

    #[test]
    fn vec4_new() {
        let v = Vec4::new(1.0f32, 2.0f32, 3.0f32, 4.0f32);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
        assert_eq!(v.w, 4.0);
    }

    #[test]
    fn quat_new() {
        let q = Quat::new(1.0f32, 2.0f32, 3.0f32, 4.0f32);
        assert_eq!(q.x, 1.0);
        assert_eq!(q.y, 2.0);
        assert_eq!(q.z, 3.0);
        assert_eq!(q.w, 4.0);
    }

    #[test]
    fn mat3_identity() {
        let m = Mat3::<f32>::identity();
        assert_eq!(m.cols[0].x, 1.0);
        assert_eq!(m.cols[0].y, 0.0);
        assert_eq!(m.cols[0].z, 0.0);
        assert_eq!(m.cols[1].x, 0.0);
        assert_eq!(m.cols[1].y, 1.0);
        assert_eq!(m.cols[1].z, 0.0);
        assert_eq!(m.cols[2].x, 0.0);
        assert_eq!(m.cols[2].y, 0.0);
        assert_eq!(m.cols[2].z, 1.0);
    }

    #[test]
    fn mat4_identity() {
        let m = Mat4::<f32>::identity();
        assert_eq!(m.cols[0].x, 1.0);
        assert_eq!(m.cols[0].y, 0.0);
        assert_eq!(m.cols[0].z, 0.0);
        assert_eq!(m.cols[0].w, 0.0);
        assert_eq!(m.cols[1].x, 0.0);
        assert_eq!(m.cols[1].y, 1.0);
        assert_eq!(m.cols[2].z, 1.0);
        assert_eq!(m.cols[3].w, 1.0);
    }

    #[test]
    fn vec2_pack_unpack() {
        let v = Vec2::new(1.0f32, 2.0f32);
        let mut w = BitWriter::new();
        v.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let v2 = Vec2::<f32>::unpack(&mut r).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn vec3_pack_unpack() {
        let v = Vec3::new(1.0f32, 2.0f32, 3.0f32);
        let mut w = BitWriter::new();
        v.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let v2 = Vec3::<f32>::unpack(&mut r).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn vec4_pack_unpack() {
        let v = Vec4::new(1.0f32, 2.0f32, 3.0f32, 4.0f32);
        let mut w = BitWriter::new();
        v.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let v2 = Vec4::<f32>::unpack(&mut r).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn quat_pack_unpack() {
        let q = Quat::new(1.0f32, 2.0f32, 3.0f32, 4.0f32);
        let mut w = BitWriter::new();
        q.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let q2 = Quat::<f32>::unpack(&mut r).unwrap();
        assert_eq!(q, q2);
    }

    #[test]
    fn mat3_pack_unpack() {
        let m = Mat3::new(
            Vec3::new(1.0f32, 0.0f32, 0.0f32),
            Vec3::new(0.0f32, 1.0f32, 0.0f32),
            Vec3::new(0.0f32, 0.0f32, 1.0f32),
        );
        let mut w = BitWriter::new();
        m.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let m2 = Mat3::<f32>::unpack(&mut r).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn mat4_pack_unpack() {
        let m = Mat4::new(
            Vec4::new(1.0f32, 0.0f32, 0.0f32, 0.0f32),
            Vec4::new(0.0f32, 1.0f32, 0.0f32, 0.0f32),
            Vec4::new(0.0f32, 0.0f32, 1.0f32, 0.0f32),
            Vec4::new(0.0f32, 0.0f32, 0.0f32, 1.0f32),
        );
        let mut w = BitWriter::new();
        m.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let m2 = Mat4::<f32>::unpack(&mut r).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn vec2_int_pack_unpack() {
        let v = Vec2::new(10i32, 20i32);
        let mut w = BitWriter::new();
        v.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let v2 = Vec2::<i32>::unpack(&mut r).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn vec3_int_pack_unpack() {
        let v = Vec3::new(1i16, 2i16, 3i16);
        let mut w = BitWriter::new();
        v.pack(&mut w).unwrap();
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        let v2 = Vec3::<i16>::unpack(&mut r).unwrap();
        assert_eq!(v, v2);
    }
}
