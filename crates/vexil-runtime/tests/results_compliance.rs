use serde::Deserialize;
use serde_json::Value;
use vexil_runtime::{BitReader, BitWriter};

#[derive(Deserialize)]
struct ResultVector {
    name: String,
    value: Value,
    expected_bytes: String,
}

fn decode_hex(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("vector hex is ASCII");
            u8::from_str_radix(text, 16).expect("vector hex is valid")
        })
        .collect()
}

#[test]
fn result_vectors_encode_and_decode_with_published_discriminants() {
    let vectors: Vec<ResultVector> =
        serde_json::from_str(include_str!("../../../compliance/vectors/results.json"))
            .expect("results vectors parse");

    for vector in vectors {
        let result = &vector.value["value"];
        let mut writer = BitWriter::new();
        match vector.name.as_str() {
            "result_ok_u8" => {
                writer.write_bool(true);
                writer.write_u8(result["ok"].as_u64().expect("u8 value") as u8);
            }
            "result_err_string" => {
                writer.write_bool(false);
                writer.write_string(result["err"].as_str().expect("string value"));
            }
            "result_ok_void" => writer.write_bool(true),
            "result_packed_bool_adjacency" => {
                writer.write_bool(true);
                writer.write_bool(result["ok"].as_bool().expect("bool value"));
                writer.write_bool(vector.value["tail"].as_bool().expect("tail value"));
            }
            name => panic!("unhandled result vector {name}"),
        }
        let expected = decode_hex(&vector.expected_bytes);
        assert_eq!(writer.finish(), expected, "{} encode", vector.name);

        let mut reader = BitReader::new(&expected);
        let is_ok = reader.read_bool().expect("result discriminant");
        match vector.name.as_str() {
            "result_ok_u8" => {
                assert!(is_ok);
                assert_eq!(reader.read_u8().expect("ok payload"), 42);
            }
            "result_err_string" => {
                assert!(!is_ok);
                assert_eq!(reader.read_string().expect("err payload"), "oops");
            }
            "result_ok_void" => assert!(is_ok),
            "result_packed_bool_adjacency" => {
                assert!(is_ok);
                assert!(reader.read_bool().expect("bool payload"));
                assert!(reader.read_bool().expect("tail"));
            }
            _ => unreachable!(),
        }
    }
}
