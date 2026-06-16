use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tracing::Level;
use tracing_subscriber;

use ipc_host::value_table::{Entry, Value, create_table_with_entries, set_value_table};

static CANCELLED: AtomicBool = AtomicBool::new(false);

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    {
        let entries = vec![
            (
                0x3304,
                Entry {
                    // value: Value::UnsignedInteger16(0),
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
                    // value: Value::UnsignedInteger8(110),
                    value: Value::UnsignedInteger32(0),
                    source: 0,
                    destination: 0,
                    writable: false,
                },
            ),
            (
                0x320c,
                Entry {
                    // value: Value::UnsignedInteger32(56),
                    value: Value::UnsignedInteger32(0),
                    source: 0,
                    destination: 0,
                    writable: false,
                },
            ),
        ];
        let table = create_table_with_entries(&entries);
        set_value_table(table);
    }

    tracing::info!("Press Ctrl+C to stop...");
    let (tx, rx) = std::sync::mpsc::channel::<ipc_host::IpcCommands>();

    let capture_config: ipc_host::CaptureConfig = ipc_host::CaptureConfig {
        path: Some("./scratch".into()),
        max: Some(20),
    };

    let join_handle = thread::spawn(|| unsafe {
        ipc_host::create_ipc_window_and_run(rx, capture_config)
            .expect("Failed to create IPC window");
    });

    tx.send(ipc_host::IpcCommands::StartCapture).unwrap();

    let _ = ctrlc::set_handler(move || {
        tracing::info!("Shutting down...");
        tx.send(ipc_host::IpcCommands::Shutdown)
            .expect("Failed to send cancel command");
        CANCELLED.store(true, Ordering::SeqCst);
    });

    while !CANCELLED.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(10));
    }

    join_handle.join().expect("Failed to join IPC thread");

    tracing::info!("Server stopped.");
}
