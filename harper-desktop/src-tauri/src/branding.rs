//! The app's user-facing name, kept in one place so it can be changed easily.

pub const APP_NAME: &str = "Shah Re-Writer";

/// The folder under `%APPDATA%` and `%LOCALAPPDATA%` that holds settings and AI models. It matches
/// the bundle identifier so the uninstaller's "Delete the application data" option removes it.
pub const DATA_FOLDER: &str = "com.shahrewriter.app";
