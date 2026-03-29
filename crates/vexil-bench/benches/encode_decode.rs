use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vexil_bench::messages::{DrawText, Envelope, OutputChunk};
use vexil_runtime::{BitReader, BitWriter};

use prost::Message;
use vexil_bench::pb::{PbDrawText, PbEnvelope, PbOutputChunk};

fn bench_envelope(c: &mut Criterion) {
    let env = Envelope {
        version: 1,
        domain: 3,
        msg_type: 42,
        session_id: 1,
        timestamp: 1_234_567_890_123,
        msg_id: Some(99),
    };
    let mut w = BitWriter::new();
    env.encode(&mut w).unwrap();
    let bytes = w.finish();

    c.bench_function("Envelope encode", |b| {
        b.iter(|| {
            let mut w = BitWriter::new();
            black_box(&env).encode(&mut w).unwrap();
            black_box(w.finish());
        })
    });
    c.bench_function("Envelope decode", |b| {
        b.iter(|| {
            let mut r = BitReader::new(black_box(&bytes));
            black_box(Envelope::decode(&mut r).unwrap());
        })
    });
}

fn bench_draw_text(c: &mut Criterion) {
    let dt = DrawText {
        x: 80,
        y: 24,
        fg: [255, 128, 0],
        bg: [0, 0, 0],
        bold: true,
        italic: false,
        text: "Hello, Vexil! This is a medium-length terminal draw command.".into(),
    };
    let mut w = BitWriter::new();
    dt.encode(&mut w).unwrap();
    let bytes = w.finish();

    c.bench_function("DrawText encode", |b| {
        b.iter(|| {
            let mut w = BitWriter::new();
            black_box(&dt).encode(&mut w).unwrap();
            black_box(w.finish());
        })
    });
    c.bench_function("DrawText decode", |b| {
        b.iter(|| {
            let mut r = BitReader::new(black_box(&bytes));
            black_box(DrawText::decode(&mut r).unwrap());
        })
    });
}

fn bench_output_chunk(c: &mut Criterion) {
    let chunk = OutputChunk {
        session_id: 42,
        pane_id: 7,
        sequence: 1_000_000,
        data: vec![0xAB; 4096], // 4 KiB payload
        command_tag: Some("cargo build --release".into()),
    };
    let mut w = BitWriter::new();
    chunk.encode(&mut w).unwrap();
    let bytes = w.finish();

    c.bench_function("OutputChunk encode (4KiB)", |b| {
        b.iter(|| {
            let mut w = BitWriter::new();
            black_box(&chunk).encode(&mut w).unwrap();
            black_box(w.finish());
        })
    });
    c.bench_function("OutputChunk decode (4KiB)", |b| {
        b.iter(|| {
            let mut r = BitReader::new(black_box(&bytes));
            black_box(OutputChunk::decode(&mut r).unwrap());
        })
    });
}

fn bench_batch(c: &mut Criterion) {
    // Simulate a realistic batch: 1 Envelope + 50 DrawText commands
    let env = Envelope {
        version: 1,
        domain: 3,
        msg_type: 42,
        session_id: 1,
        timestamp: 1_234_567_890_123,
        msg_id: Some(99),
    };
    let commands: Vec<DrawText> = (0..50)
        .map(|i| DrawText {
            x: i * 2,
            y: i,
            fg: [255, 200, 100],
            bg: [0, 0, 0],
            bold: i % 3 == 0,
            italic: i % 5 == 0,
            text: format!("line {i}: some terminal output text here"),
        })
        .collect();

    // Pre-encode to get the bytes for decode benchmark
    let mut w = BitWriter::new();
    env.encode(&mut w).unwrap();
    for cmd in &commands {
        cmd.encode(&mut w).unwrap();
    }
    let bytes = w.finish();

    c.bench_function("Batch encode (1 Envelope + 50 DrawText)", |b| {
        b.iter(|| {
            let mut w = BitWriter::new();
            black_box(&env).encode(&mut w).unwrap();
            for cmd in black_box(&commands) {
                cmd.encode(&mut w).unwrap();
            }
            black_box(w.finish());
        })
    });
    c.bench_function("Batch decode (1 Envelope + 50 DrawText)", |b| {
        b.iter(|| {
            let mut r = BitReader::new(black_box(&bytes));
            black_box(Envelope::decode(&mut r).unwrap());
            for _ in 0..50 {
                black_box(DrawText::decode(&mut r).unwrap());
            }
        })
    });
}

fn bench_pb_envelope(c: &mut Criterion) {
    let env = PbEnvelope {
        version: 1,
        domain: 3,
        msg_type: 42,
        session_id: 1,
        timestamp: 1_234_567_890_123,
        msg_id: Some(99),
    };
    let bytes = env.encode_to_vec();

    c.bench_function("Envelope encode (Protobuf)", |b| {
        b.iter(|| {
            black_box(black_box(&env).encode_to_vec());
        })
    });
    c.bench_function("Envelope decode (Protobuf)", |b| {
        b.iter(|| {
            black_box(PbEnvelope::decode(black_box(bytes.as_slice())).unwrap());
        })
    });
}

fn bench_pb_draw_text(c: &mut Criterion) {
    let dt = PbDrawText {
        x: 80,
        y: 24,
        fg: vec![255, 128, 0],
        bg: vec![0, 0, 0],
        bold: true,
        italic: false,
        text: "Hello, Vexil! This is a medium-length terminal draw command.".into(),
    };
    let bytes = dt.encode_to_vec();

    c.bench_function("DrawText encode (Protobuf)", |b| {
        b.iter(|| {
            black_box(black_box(&dt).encode_to_vec());
        })
    });
    c.bench_function("DrawText decode (Protobuf)", |b| {
        b.iter(|| {
            black_box(PbDrawText::decode(black_box(bytes.as_slice())).unwrap());
        })
    });
}

fn bench_pb_output_chunk(c: &mut Criterion) {
    let chunk = PbOutputChunk {
        session_id: 42,
        pane_id: 7,
        sequence: 1_000_000,
        data: vec![0xAB; 4096],
        command_tag: Some("cargo build --release".into()),
    };
    let bytes = chunk.encode_to_vec();

    c.bench_function("OutputChunk encode (4KiB) (Protobuf)", |b| {
        b.iter(|| {
            black_box(black_box(&chunk).encode_to_vec());
        })
    });
    c.bench_function("OutputChunk decode (4KiB) (Protobuf)", |b| {
        b.iter(|| {
            black_box(PbOutputChunk::decode(black_box(bytes.as_slice())).unwrap());
        })
    });
}

fn bench_pb_batch(c: &mut Criterion) {
    let env = PbEnvelope {
        version: 1,
        domain: 3,
        msg_type: 42,
        session_id: 1,
        timestamp: 1_234_567_890_123,
        msg_id: Some(99),
    };
    let commands: Vec<PbDrawText> = (0..50)
        .map(|i| PbDrawText {
            x: i * 2,
            y: i,
            fg: vec![255, 200, 100],
            bg: vec![0, 0, 0],
            bold: i % 3 == 0,
            italic: i % 5 == 0,
            text: format!("line {i}: some terminal output text here"),
        })
        .collect();

    c.bench_function("Batch encode (1+50) (Protobuf)", |b| {
        b.iter(|| {
            let mut buf = black_box(&env).encode_to_vec();
            for cmd in black_box(&commands) {
                cmd.encode(&mut buf).unwrap();
            }
            black_box(buf);
        })
    });
}

criterion_group!(
    benches,
    bench_envelope,
    bench_draw_text,
    bench_output_chunk,
    bench_batch,
    bench_pb_envelope,
    bench_pb_draw_text,
    bench_pb_output_chunk,
    bench_pb_batch,
);
criterion_main!(benches);
