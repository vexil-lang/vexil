mod generated;

use generated::{Command, Permissions, Request, Response, Status};
use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};

fn main() {
    // Build an auth request
    let request = Request {
        id: 1,
        command: Command::Auth {
            token: "sk-abc123".to_string(),
            perms: Permissions::READ | Permissions::WRITE,
        },
    };

    // Encode
    let mut writer = BitWriter::new();
    request.pack(&mut writer).expect("encode failed");
    let bytes = writer.finish();
    println!("Auth request: {} bytes", bytes.len());

    // Decode
    let mut reader = BitReader::new(&bytes);
    let decoded = Request::unpack(&mut reader).expect("decode failed");
    assert_eq!(decoded.id, 1);
    match &decoded.command {
        Command::Auth { token, perms } => {
            assert_eq!(token, "sk-abc123");
            assert!(perms.contains(Permissions::READ));
            assert!(perms.contains(Permissions::WRITE));
            assert!(!perms.contains(Permissions::ADMIN));
            println!("Auth OK: token={}, perms={:?}", token, perms);
        }
        _ => panic!("expected Auth command"),
    }

    // Build a query request
    let query = Request {
        id: 2,
        command: Command::Query {
            table: "sensors".to_string(),
            limit: 100,
            offset: 0,
        },
    };

    let mut writer = BitWriter::new();
    query.pack(&mut writer).expect("encode failed");
    let bytes = writer.finish();
    println!("Query request: {} bytes", bytes.len());

    // Build a response
    let response = Response {
        id: 2,
        status: Status::Ok,
        body: b"[{\"id\":1}]".to_vec(),
    };

    let mut writer = BitWriter::new();
    response.pack(&mut writer).expect("encode failed");
    let bytes = writer.finish();
    println!("Response: {} bytes", bytes.len());

    let mut reader = BitReader::new(&bytes);
    let decoded = Response::unpack(&mut reader).expect("decode failed");
    assert_eq!(decoded.id, 2);
    println!("Roundtrip OK");
}
