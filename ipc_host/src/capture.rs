use std::path::PathBuf;
use std::sync::Mutex;

pub struct CaptureConfig {
    pub path: Option<PathBuf>,
    pub max: Option<usize>,
}

impl CaptureConfig {
    pub fn none() -> Self {
        Self {
            path: None,
            max: None,
        }
    }
}

pub struct CaptureState {
    pub enabled: bool,
    pub path: PathBuf,
    pub count: usize,
    pub max: usize,
}

pub static CAPTURE_STATE: std::sync::LazyLock<Mutex<Option<CaptureState>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));
