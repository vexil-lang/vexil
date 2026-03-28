//! Schema-driven encoder/decoder with `.vx` text and `.vxb` binary file formats.
//!
//! Provides `encode`/`decode` for `Value` trees, human-readable `.vx` text format,
//! and typed `.vxb` binary format with schema hash verification.

pub mod convert;
pub(crate) mod decoder;
pub(crate) mod detect;
pub(crate) mod encoder;
pub(crate) mod error;
pub(crate) mod formatter;
pub(crate) mod header;
pub(crate) mod lexer;
pub(crate) mod meta;
pub(crate) mod parser;
pub(crate) mod validate;
pub(crate) mod value;

pub use convert::{compiled_schema_to_value, schema_store_to_value};
pub use decoder::decode;
pub use detect::{detect_format, FileFormat};
pub use encoder::encode;
pub use error::{StoreDecodeError, StoreEncodeError, VxError, VxbError};
pub use formatter::{format, FormatOptions};
pub use header::{read_header, write_header, Magic, VxbHeader, FORMAT_VERSION};
pub use lexer::{Lexer, Span, Spanned, Token};
pub use meta::{meta_schema, pack_schema};
pub use parser::parse;
pub use validate::validate;
pub use value::Value;
