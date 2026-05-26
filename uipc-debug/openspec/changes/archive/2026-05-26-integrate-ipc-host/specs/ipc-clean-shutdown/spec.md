## ADDED Requirements

### Requirement: Graceful IPC shutdown on quit
When `q` is pressed in IPC mode, the tool SHALL send `IpcCommands::Shutdown` to the IPC thread and SHALL wait for the thread to finish (via `JoinHandle::join()`) before exiting. The tool SHALL handle the case where the IPC command channel has been closed or the thread has already exited.

#### Scenario: Shutdown sent on quit
- **WHEN** the user presses `q` in IPC mode
- **THEN** the tool sends `IpcCommands::Shutdown` to the IPC thread
- **THEN** the tool waits for the IPC thread to complete
- **THEN** the tool exits normally

#### Scenario: Shutdown with closed channel
- **WHEN** the IPC thread has already exited (channel closed)
- **THEN** the tool detects the closed channel and exits without error

### Requirement: IPC thread JoinHandle management
The tool SHALL store the IPC thread's `JoinHandle` and command channel `Sender<IpcCommands>` in the `App` struct for lifecycle management. These SHALL be wrapped in `Option` to support IPC/offline mode and to allow safe `take()` on shutdown.

#### Scenario: JoinHandle stored on IPC start
- **WHEN** the tool starts in IPC mode
- **THEN** the `App` struct contains a `Some(JoinHandle)` and a `Some(Sender<IpcCommands>)`

#### Scenario: JoinHandle is None in offline mode
- **WHEN** the tool starts with `--no-ipc`
- **THEN** the `App` struct has `None` for both the `JoinHandle` and the `Sender`

### Requirement: Shutdown on error
If the IPC thread panics or returns an error, the tool SHALL log the error and fall back to offline mode. The TUI SHALL remain usable for offline operations.

#### Scenario: IPC thread failure logged
- **WHEN** the IPC thread fails to create the window
- **THEN** the error is logged to the trace log pane
- **THEN** the tool transitions to offline mode and continues running
