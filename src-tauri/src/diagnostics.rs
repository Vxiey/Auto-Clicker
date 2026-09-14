use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

const MAX_SESSION_LOGS: usize = 8;
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 8_192;

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Clone)]
pub struct DiagnosticsState {
    inner: Arc<DiagnosticsInner>,
}

struct DiagnosticsInner {
    directory: PathBuf,
    session_log: PathBuf,
    crash_log: PathBuf,
    performance_log: PathBuf,
    session_id: String,
    write_lock: Mutex<()>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticsSnapshot {
    pub directory: String,
    pub session_log: String,
    pub crash_log: String,
    pub performance_log: String,
    pub session_id: String,
    pub session_log_bytes: u64,
    pub crash_log_bytes: u64,
    pub performance_log_bytes: u64,
}

impl DiagnosticsState {
    pub fn initialize(app: &AppHandle) -> Result<Self, String> {
        let directory = app
            .path()
            .app_log_dir()
            .map_err(|error| format!("failed to resolve log directory: {error}"))?;
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;

        prune_session_logs(&directory)?;
        rotate_if_needed(&directory.join("crash.log"), MAX_LOG_BYTES)?;
        rotate_if_needed(&directory.join("performance.log"), MAX_LOG_BYTES)?;

        let session_id = format!("{}-{}", unix_millis(), std::process::id());
        let session_log = directory.join(format!("session-{session_id}.log"));
        let crash_log = directory.join("crash.log");
        let performance_log = directory.join("performance.log");

        let state = Self {
            inner: Arc::new(DiagnosticsInner {
                directory,
                session_log,
                crash_log,
                performance_log,
                session_id,
                write_lock: Mutex::new(()),
            }),
        };

        state.log(
            LogLevel::Info,
            "app",
            &format!(
                "session started version={} pid={} debug_assertions={}",
                env!("CARGO_PKG_VERSION"),
                std::process::id(),
                cfg!(debug_assertions)
            ),
        );
        Ok(state)
    }

    pub fn install_panic_hook(&self) {
        let crash_path = self.inner.crash_log.clone();
        let session_id = self.inner.session_id.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("unnamed");
            let location = panic_info
                .location()
                .map(|location| {
                    format!(
                        "{}:{}:{}",
                        location.file(),
                        location.line(),
                        location.column()
                    )
                })
                .unwrap_or_else(|| "unknown".to_string());
            let payload = panic_info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic_info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string panic payload");
            let line = format!(
                "{} [CRASH] session={} thread={} location={} message={}\n",
                unix_millis(),
                session_id,
                sanitize(thread_name),
                sanitize(&location),
                sanitize(payload)
            );
            let _ = append_raw(&crash_path, &line);
            previous(panic_info);
        }));
    }

    pub fn log(&self, level: LogLevel, target: &str, message: &str) {
        let Ok(_guard) = self.inner.write_lock.lock() else {
            return;
        };
        let line = format!(
            "{} [{}] [{}] {}\n",
            unix_millis(),
            level.as_str(),
            sanitize(target),
            sanitize(message)
        );
        let _ = append_raw(&self.inner.session_log, &line);
    }

    pub fn performance(&self, target: &str, message: &str) {
        let Ok(_guard) = self.inner.write_lock.lock() else {
            return;
        };
        let line = format!(
            "{} [{}] {}\n",
            unix_millis(),
            sanitize(target),
            sanitize(message)
        );
        let _ = append_raw(&self.inner.performance_log, &line);
    }

    fn snapshot(&self) -> DiagnosticsSnapshot {
        DiagnosticsSnapshot {
            directory: display_path(&self.inner.directory),
            session_log: display_path(&self.inner.session_log),
            crash_log: display_path(&self.inner.crash_log),
            performance_log: display_path(&self.inner.performance_log),
            session_id: self.inner.session_id.clone(),
            session_log_bytes: file_size(&self.inner.session_log),
            crash_log_bytes: file_size(&self.inner.crash_log),
            performance_log_bytes: file_size(&self.inner.performance_log),
        }
    }

    fn clear_old_logs(&self) -> Result<(), String> {
        let _guard = self
            .inner
            .write_lock
            .lock()
            .map_err(|_| "diagnostics write mutex was poisoned".to_string())?;

        for entry in fs::read_dir(&self.inner.directory)
            .map_err(|error| format!("failed to read log directory: {error}"))?
        {
            let entry = entry.map_err(|error| format!("failed to inspect log file: {error}"))?;
            let path = entry.path();
            if path == self.inner.session_log {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("session-")
                || name == "crash.log"
                || name == "crash.log.1"
                || name == "performance.log"
                || name == "performance.log.1"
            {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }
}

#[tauri::command]
pub fn diagnostics_snapshot(state: State<'_, DiagnosticsState>) -> DiagnosticsSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn clear_diagnostics(state: State<'_, DiagnosticsState>) -> Result<(), String> {
    state.clear_old_logs()?;
    state.log(LogLevel::Info, "diagnostics", "old diagnostic logs cleared");
    Ok(())
}

#[tauri::command]
pub fn diagnostics_client_log(
    level: String,
    target: String,
    message: String,
    state: State<'_, DiagnosticsState>,
) {
    let level = match level.to_ascii_lowercase().as_str() {
        "debug" => LogLevel::Debug,
        "warn" | "warning" => LogLevel::Warn,
        "error" => LogLevel::Error,
        _ => LogLevel::Info,
    };
    state.log(level, &format!("ui:{target}"), &message);
}

fn append_raw(path: &Path, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    file.flush()
}

fn prune_session_logs(directory: &Path) -> Result<(), String> {
    let mut sessions = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("session-")
        })
        .collect::<Vec<_>>();

    sessions.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH)
    });

    let remove_count = sessions.len().saturating_sub(MAX_SESSION_LOGS.saturating_sub(1));
    for entry in sessions.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path());
    }
    Ok(())
}

fn rotate_if_needed(path: &Path, max_bytes: u64) -> Result<(), String> {
    let size = fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0);
    if size < max_bytes {
        return Ok(());
    }
    let backup = PathBuf::from(format!("{}.1", path.display()));
    let _ = fs::remove_file(&backup);
    fs::rename(path, &backup)
        .map_err(|error| format!("failed to rotate {}: {error}", path.display()))
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .take(MAX_MESSAGE_CHARS)
        .flat_map(|ch| match ch {
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\0' => "\\0".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0)
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
