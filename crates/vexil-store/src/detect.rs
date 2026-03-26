/// Auto-detected file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// `.vxb` — binary data file
    Vxb,
    /// `.vxc` — binary compiled schema file
    Vxc,
    /// `.vxbp` — binary data pack file
    Vxbp,
    /// `.vxcp` — binary compiled schema pack file
    Vxcp,
    /// `.vx` with `namespace` keyword — Vexil schema source
    VexilSchema,
    /// `.vx` with `@schema` directive — Vexil data file
    VxData,
    /// Unknown format
    Unknown,
}

/// Detect file format from the first bytes of a file.
pub fn detect_format(data: &[u8]) -> FileFormat {
    if data.len() >= 4 {
        match &data[0..4] {
            b"VXB\0" => return FileFormat::Vxb,
            b"VXC\0" => return FileFormat::Vxc,
            b"VXBP" => return FileFormat::Vxbp,
            b"VXCP" => return FileFormat::Vxcp,
            _ => {}
        }
    }

    // Text detection: scan for first non-whitespace, non-comment token
    if let Ok(text) = std::str::from_utf8(data) {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }
            if trimmed.starts_with("namespace") {
                return FileFormat::VexilSchema;
            }
            if trimmed.starts_with("@schema") {
                return FileFormat::VxData;
            }
            // First real token found, not a known marker
            break;
        }
    }

    FileFormat::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_vxb() {
        assert_eq!(detect_format(b"VXB\0\x01\x00"), FileFormat::Vxb);
    }

    #[test]
    fn detect_vxc() {
        assert_eq!(detect_format(b"VXC\0\x01\x00"), FileFormat::Vxc);
    }

    #[test]
    fn detect_vxbp() {
        assert_eq!(detect_format(b"VXBP\x01\x00"), FileFormat::Vxbp);
    }

    #[test]
    fn detect_vxcp() {
        assert_eq!(detect_format(b"VXCP\x01\x00"), FileFormat::Vxcp);
    }

    #[test]
    fn detect_vexil_schema() {
        let text = b"namespace test.simple\nmessage Foo { x @0 : u32 }";
        assert_eq!(detect_format(text), FileFormat::VexilSchema);
    }

    #[test]
    fn detect_vx_data() {
        let text = b"@schema \"test.simple\"\nFoo { x: 1 }";
        assert_eq!(detect_format(text), FileFormat::VxData);
    }

    #[test]
    fn detect_vexil_schema_with_comments() {
        let text = b"// This is a schema\n# another comment\nnamespace foo.bar";
        assert_eq!(detect_format(text), FileFormat::VexilSchema);
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect_format(b"garbage"), FileFormat::Unknown);
    }
}
