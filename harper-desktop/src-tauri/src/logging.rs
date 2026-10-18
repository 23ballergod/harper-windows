//! Writes each process's log to a file, since a Windows app started from the Start menu has no
//! console. Logs live in `%LOCALAPPDATA%\<data folder>\logs`: `app.log` for the main process and
//! `highlighter.log` for the overlay. The previous run's log is kept as `*.previous.log`.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

pub fn log_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join(crate::branding::DATA_FOLDER).join("logs"))
}

fn open_log_file(name: &str) -> Option<File> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{name}.log"));
    let _ = std::fs::rename(&path, dir.join(format!("{name}.previous.log")));
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Sends this process's logs, including anything printed to stderr, to `<name>.log`.
pub fn init(name: &str) {
    let file = open_log_file(name);

    #[cfg(target_os = "windows")]
    if let Some(file) = &file {
        redirect_stderr(file);
    }

    let filter = Targets::new()
        .with_target("harper_desktop_lib", Level::INFO)
        .with_target("harper_ai", Level::INFO)
        .with_default(Level::WARN);
    let layer = tracing_subscriber::fmt::layer().with_ansi(false);

    let result = match file {
        Some(file) => tracing_subscriber::registry()
            .with(filter)
            .with(layer.with_writer(Mutex::new(file)))
            .try_init(),
        None => tracing_subscriber::registry()
            .with(filter)
            .with(layer.with_writer(std::io::stderr))
            .try_init(),
    };
    if let Err(error) = result {
        eprintln!("Unable to set up logging: {error}");
    }

    tracing::info!(
        "{} {} started ({name})",
        crate::branding::APP_NAME,
        env!("CARGO_PKG_VERSION")
    );
}

/// Points this process's stderr at the log file so `eprintln!` messages are kept too.
#[cfg(target_os = "windows")]
fn redirect_stderr(file: &File) {
    use std::os::windows::io::AsRawHandle as _;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};

    let Ok(clone) = file.try_clone() else { return };
    let handle = HANDLE(clone.as_raw_handle());
    if unsafe { SetStdHandle(STD_ERROR_HANDLE, handle) }.is_ok() {
        // The handle must stay open for the rest of the process.
        std::mem::forget(clone);
    }
}

/// Logs `message` under `key` only when it differs from the last message logged for that key, so
/// state checked many times a second shows up once per change.
pub fn note_change(key: &'static str, message: String) {
    static LAST: OnceLock<Mutex<HashMap<&'static str, String>>> = OnceLock::new();
    let mut last = LAST
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if last.get(key) != Some(&message) {
        tracing::info!("{key}: {message}");
        last.insert(key, message);
    }
}
