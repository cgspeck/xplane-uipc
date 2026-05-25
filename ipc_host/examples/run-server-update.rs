use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tracing::Level;
use tracing_subscriber;

use ipc_host::value_table::{
    Entry, Value, create_table_with_entries, get_value_table, set_value_table,
};

static CANCELLED: AtomicBool = AtomicBool::new(false);

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    {
        let entries = vec![
            (
                0x3304,
                Entry {
                    value: Value::UnsignedInteger32(0x50000008),
                    source: 0,
                    destination: 0,
                    writable: false,
                },
            ),
            (
                0x3308,
                Entry {
                    value: Value::UnsignedInteger32(0xFADEFFFF),
                    source: 0,
                    destination: 0,
                    writable: false,
                },
            ),
            (
                0x3124,
                Entry {
                    value: Value::UnsignedInteger32(0x00000000),
                    source: 0,
                    destination: 0,
                    writable: false,
                },
            ),
            (
                0x320c,
                Entry {
                    value: Value::UnsignedInteger32(0x00000000),
                    source: 0,
                    destination: 0,
                    writable: false,
                },
            ),
            (
                0x02CC,
                Entry {
                    value: Value::UnsignedInteger32(0),
                    source: 0,
                    destination: 0,
                    writable: true,
                },
            ),
        ];
        let table = create_table_with_entries(&entries);
        set_value_table(table);
    }

    let (tx, rx) = std::sync::mpsc::channel::<ipc_host::IpcCommands>();

    let join_handle = thread::spawn(|| unsafe {
        ipc_host::create_ipc_window_and_run(rx, ipc_host::CaptureConfig::none())
            .expect("Failed to create IPC window");
    });

    let _ = ctrlc::set_handler(move || {
        tracing::info!("Shutting down...");
        tx.send(ipc_host::IpcCommands::Shutdown)
            .expect("Failed to send cancel command");
        CANCELLED.store(true, Ordering::SeqCst);
    });

    let mut heading: u32 = 0;
    while !CANCELLED.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));

        heading = (heading + 1) % 360;

        let table = get_value_table();
        if let Ok(mut table) = table.write() {
            if let Some(entry) = table.entries.get_mut(0x02CC as usize) {
                if let Some(e) = entry {
                    e.value = Value::UnsignedInteger32(heading);
                }
            }
        }
    }

    join_handle.join().expect("Failed to join IPC thread");

    tracing::info!("Server stopped.");
}
