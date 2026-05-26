use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};

use chrono::Local;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

/// Shared ring buffer for trace entries. Read by the TUI trace pane.
pub struct TraceBuffer {
    buffer: VecDeque<String>,
    capacity: usize,
}

impl TraceBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, msg: String) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(msg);
    }

    pub fn entries(&self) -> Vec<String> {
        self.buffer.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

struct AppLayer {
    buffer: Arc<Mutex<TraceBuffer>>,
    file: Option<Arc<Mutex<std::fs::File>>>,
}

struct FmtVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for FmtVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        let _ = write!(self.0, " {}={:?}", field.name(), value);
    }
}

fn format_event(event: &tracing::Event<'_>) -> String {
    let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
    let meta = event.metadata();
    let mut msg = String::new();
    msg.push_str(&timestamp);
    msg.push(' ');
    msg.push_str(meta.level().as_str());
    msg.push(' ');
    msg.push_str(meta.target());
    let mut visitor = FmtVisitor(&mut msg);
    event.record(&mut visitor);
    msg
}

impl<S> Layer<S> for AppLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let msg = format_event(event);

        if let Ok(mut buf) = self.buffer.lock() {
            buf.push(msg.clone());
        }

        if let Some(ref file) = self.file {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{}", msg);
            }
        }
    }
}

/// Initialize tracing with the given log level and optional file output.
/// Returns the shared trace buffer, which the TUI reads for display.
pub fn init_tracing(level_str: &str, log_file: Option<String>) -> Arc<Mutex<TraceBuffer>> {
    let level: LevelFilter = match level_str.to_lowercase().as_str() {
        "error" => LevelFilter::ERROR,
        "warn" => LevelFilter::WARN,
        "info" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        "trace" => LevelFilter::TRACE,
        _ => LevelFilter::TRACE,
    };

    let buffer = Arc::new(Mutex::new(TraceBuffer::new(1000)));

    let file = log_file.map(|path| {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap_or_else(|e| {
                eprintln!("Warning: cannot create log file '{}': {}", path, e);
                std::fs::OpenOptions::new().write(true).open("NUL").unwrap()
            });
        Arc::new(Mutex::new(f))
    });

    let layer = AppLayer {
        buffer: buffer.clone(),
        file,
    };

    tracing_subscriber::registry()
        .with(level)
        .with(layer)
        .init();

    buffer
}
