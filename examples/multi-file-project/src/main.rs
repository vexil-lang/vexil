mod project;

use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};

fn main() {
    let task_list = project::messages::TaskList {
        tasks: vec![
            project::messages::Task {
                id: 1,
                title: "Implement sensor driver".to_string(),
                priority: project::types::Priority::High,
                created_at: project::types::Timestamp {
                    seconds: 1711500000,
                    nanos: 0,
                },
            },
            project::messages::Task {
                id: 2,
                title: "Write documentation".to_string(),
                priority: project::types::Priority::Medium,
                created_at: project::types::Timestamp {
                    seconds: 1711500060,
                    nanos: 500_000_000,
                },
            },
        ],
    };

    let mut writer = BitWriter::new();
    task_list.pack(&mut writer).expect("encode failed");
    let bytes = writer.finish();
    println!(
        "TaskList with {} tasks: {} bytes",
        task_list.tasks.len(),
        bytes.len()
    );

    let mut reader = BitReader::new(&bytes);
    let decoded = project::messages::TaskList::unpack(&mut reader).expect("decode failed");
    assert_eq!(decoded.tasks.len(), 2);
    assert_eq!(decoded.tasks[0].title, "Implement sensor driver");
    println!("Roundtrip OK");
}
