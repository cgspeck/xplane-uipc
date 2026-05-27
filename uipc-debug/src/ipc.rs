use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use ipc_host::{IpcCommands, create_ipc_window_and_run};

pub fn spawn() -> anyhow::Result<(JoinHandle<()>, Sender<IpcCommands>)> {
    let (ipc_tx, ipc_rx) = std::sync::mpsc::channel::<IpcCommands>();
    let (write_tx, write_rx) = std::sync::mpsc::channel::<ipc_host::WriteRequest>();

    // Drain any incoming write requests (not used in debug tool)
    std::thread::spawn(move || while write_rx.recv().is_ok() {});

    ipc_host::set_write_channel(write_tx);

    let thread_handle =
        std::thread::Builder::new()
            .name("ipc-host".into())
            .spawn(move || unsafe {
                if let Err(e) = create_ipc_window_and_run(
                    ipc_rx,
                    ipc_host::CaptureConfig {
                        max: None,
                        path: None,
                    },
                ) {
                    tracing::error!("IPC thread exited with error: {}", e);
                }
            })?;

    Ok((thread_handle, ipc_tx))
}
