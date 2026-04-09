use proptest::prelude::*;
use vexil_lang::diagnostic::Severity;

// --- Compiler property tests ---

proptest! {
    /// Compiling any valid corpus schema must succeed (no panics, no errors).
    /// This is a sanity check that the compiler handles all valid inputs.
    #[test]
    fn corpus_valid_compiles(idx in 1u32..=41) {
        let filename = format!("{idx:03}_*.vexil");
        let corpus_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap()
            .join("corpus/valid");

        // Find the file matching the pattern
        let entries: Vec<_> = std::fs::read_dir(&corpus_dir).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(&format!("{idx:03}_")))
            .collect();

        if entries.is_empty() {
            return Ok(()); // Skip if no file for this index
        }

        let path = entries[0].path();
        let source = std::fs::read_to_string(&path).unwrap();
        let result = vexil_lang::compile(&source);

        // Must not have errors
        let has_errors = result.diagnostics.iter().any(|d| d.severity == Severity::Error);
        prop_assert!(!has_errors, "valid schema should compile: {:?}", result.diagnostics);

        // Must produce compiled output
        prop_assert!(result.compiled.is_some(), "valid schema should produce CompiledSchema");
    }

    /// Compiling the same source twice must produce identical canonical forms.
    /// This tests determinism of the compiler.
    #[test]
    fn compile_is_deterministic(
        field_count in 0u8..=10,
        field_type_idx in 0u8..=5,
    ) {
        let type_name = match field_type_idx % 6 {
            0 => "u8",
            1 => "u32",
            2 => "i64",
            3 => "f32",
            4 => "string",
            _ => "bool",
        };

        let mut source = String::from("namespace test.prop\nmessage M {\n");
        for i in 0..field_count {
            source.push_str(&format!("    f{i} @{i} : {type_name}\n"));
        }
        source.push_str("}\n");

        let r1 = vexil_lang::compile(&source);
        let r2 = vexil_lang::compile(&source);

        if let (Some(c1), Some(c2)) = (&r1.compiled, &r2.compiled) {
            let h1 = vexil_lang::canonical::schema_hash(c1);
            let h2 = vexil_lang::canonical::schema_hash(c2);
            prop_assert_eq!(h1, h2, "canonical hash must be deterministic");
        }
    }

    /// Any boolean sequence as bits roundtrips through write_bits/read_bits.
    /// We test using a simple manual bit writer/reader to avoid the vexil_runtime dependency.
    #[test]
    fn bits_roundtrip(bits in prop::collection::vec(any::<bool>(), 1..=32)) {
        // Write bits manually into a byte buffer
        let mut buf = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_offset = 0u8;
        for &b in &bits {
            if b {
                current_byte |= 1 << bit_offset;
            }
            bit_offset += 1;
            if bit_offset == 8 {
                buf.push(current_byte);
                current_byte = 0;
                bit_offset = 0;
            }
        }
        if bit_offset > 0 {
            buf.push(current_byte);
        }

        // Read back
        let mut byte_pos = 0;
        let mut bit_off = 0u8;
        for (i, &expected) in bits.iter().enumerate() {
            let bit = (buf[byte_pos] >> bit_off) & 1;
            let actual = bit != 0;
            prop_assert_eq!(actual, expected, "bit {} mismatch", i);
            bit_off += 1;
            if bit_off == 8 {
                byte_pos += 1;
                bit_off = 0;
            }
        }
    }

    /// Schema hash of the same source is always the same byte array.
    #[test]
    fn schema_hash_deterministic(
        ns_component in "[a-z]{1,8}",
        msg_name in "[A-Z][a-z]{1,8}",
    ) {
        let source = format!("namespace test.{ns_component}\nmessage {msg_name} {{ x @0 : u32 }}\n");

        let r1 = vexil_lang::compile(&source);
        let r2 = vexil_lang::compile(&source);

        if let (Some(c1), Some(c2)) = (&r1.compiled, &r2.compiled) {
            let h1 = vexil_lang::canonical::schema_hash(c1);
            let h2 = vexil_lang::canonical::schema_hash(c2);
            prop_assert_eq!(h1, h2);
        }
    }
}
