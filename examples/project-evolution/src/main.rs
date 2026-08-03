mod project;

use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task_list = project::messages::TaskList {
        tasks: vec![
            project::messages::Task {
                id: 1,
                title: "Implement sensor driver".to_string(),
                priority: project::types::Priority::High,
                created_at: project::types::Timestamp {
                    seconds: 1711500000,
                    nanos: 0,
                    _unknown: Vec::new(),
                },
                permissions: project::types::Permissions::READ
                    | project::types::Permissions::WRITE,
                state: project::types::TaskState::Assigned { user_id: 7 },
                _unknown: Vec::new(),
            },
            project::messages::Task {
                id: 2,
                title: "Write documentation".to_string(),
                priority: project::types::Priority::Medium,
                created_at: project::types::Timestamp {
                    seconds: 1711500060,
                    nanos: 500_000_000,
                    _unknown: Vec::new(),
                },
                permissions: project::types::Permissions::READ,
                state: project::types::TaskState::Pending {},
                _unknown: Vec::new(),
            },
        ],
        _unknown: Vec::new(),
    };

    let mut writer = BitWriter::new();
    task_list.pack(&mut writer)?;
    let bytes = writer.finish();
    println!(
        "TaskList with {} tasks: {} bytes",
        task_list.tasks.len(),
        bytes.len()
    );

    let mut reader = BitReader::new(&bytes);
    let decoded = project::messages::TaskList::unpack(&mut reader)?;
    assert_eq!(decoded.tasks.len(), 2);
    assert_eq!(decoded.tasks[0].title, "Implement sensor driver");
    println!("round trip: 2 tasks across 2 schema files");
    Ok(())
}
