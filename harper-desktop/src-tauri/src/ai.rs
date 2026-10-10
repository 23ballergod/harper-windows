//! Offline AI suggestions: settings, model downloads, and the highlighter-side checker.
//!
//! Settings live in their own file next to `config.json` so the highlighter process can pick up
//! changes by re-reading it, without growing the IPC protocol.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use harper_ai::{AI_RULE_NAME, AiChecker, AiModel, CheckerStatus, QwenCorrector};
use harper_core::linting::{FlatConfig, Lint};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    pub enabled: bool,
    pub model: AiModel,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            model: AiModel::Fast,
        }
    }
}

impl AiSettings {
    fn path() -> Option<PathBuf> {
        crate::config::Config::folder_path().map(|p| p.join("ai.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("no config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}

/// Where downloaded model files are stored.
pub fn models_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join(crate::branding::DATA_FOLDER).join("models"))
}

/// Download progress for one model, reported to the settings page.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelState {
    pub downloaded: bool,
    pub downloading: bool,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

/// Download state for every model, shared by Tauri commands.
#[derive(Default)]
pub struct AiDownloads {
    states: std::sync::Mutex<HashMap<AiModel, ModelState>>,
}

impl AiDownloads {
    pub fn state(&self, model: AiModel) -> ModelState {
        let downloaded = models_dir().is_some_and(|dir| model.is_downloaded(&dir));
        let mut state = self
            .states
            .lock()
            .unwrap()
            .get(&model)
            .cloned()
            .unwrap_or_default();
        state.downloaded = downloaded && !state.downloading;
        if state.total_bytes == 0 {
            state.total_bytes = model.files().iter().map(|f| f.bytes).sum();
        }
        state
    }

    fn update(&self, model: AiModel, f: impl FnOnce(&mut ModelState)) {
        let mut states = self.states.lock().unwrap();
        f(states.entry(model).or_default());
    }

    /// Downloads every file the model needs, skipping ones already on disk. Files are written to
    /// a `.part` file first so an interrupted download is never mistaken for a finished one.
    pub async fn download(&self, model: AiModel) -> Result<(), String> {
        {
            let mut states = self.states.lock().unwrap();
            let state = states.entry(model).or_default();
            if state.downloading {
                return Ok(());
            }
            *state = ModelState {
                downloading: true,
                total_bytes: model.files().iter().map(|f| f.bytes).sum(),
                ..Default::default()
            };
        }

        let result = self.download_files(model).await;
        self.update(model, |state| {
            state.downloading = false;
            state.error = result.as_ref().err().cloned();
        });
        result
    }

    async fn download_files(&self, model: AiModel) -> Result<(), String> {
        let dir = models_dir().ok_or("no local data directory")?;
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| e.to_string())?;

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            // Without this a stalled connection would leave the download stuck forever.
            .read_timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;

        let mut done_before: u64 = 0;
        for file in model.files() {
            let target = dir.join(file.file_name);
            if file.is_complete(&target) {
                done_before += file.bytes;
                self.update(model, |s| s.downloaded_bytes = done_before);
                continue;
            }

            let mut response = client
                .get(file.url)
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(|e| format!("Could not download {}: {e}", file.file_name))?;

            let part = dir.join(format!("{}.part", file.file_name));
            let mut out = tokio::fs::File::create(&part)
                .await
                .map_err(|e| e.to_string())?;
            let mut written: u64 = 0;
            let mut hasher = Sha256::new();

            use tokio::io::AsyncWriteExt;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| format!("Download interrupted: {e}"))?
            {
                out.write_all(&chunk).await.map_err(|e| e.to_string())?;
                hasher.update(&chunk);
                written += chunk.len() as u64;
                self.update(model, |s| s.downloaded_bytes = done_before + written);
            }
            out.flush().await.map_err(|e| e.to_string())?;
            drop(out);

            let digest = hex::encode(hasher.finalize());
            if written != file.bytes || digest != file.sha256 {
                let _ = tokio::fs::remove_file(&part).await;
                return Err(format!(
                    "The download of {} was incomplete or damaged. Please try again.",
                    file.file_name
                ));
            }

            tokio::fs::rename(&part, &target)
                .await
                .map_err(|e| e.to_string())?;
            done_before += written;
        }

        Ok(())
    }

    pub fn delete(&self, model: AiModel) -> Result<(), String> {
        let dir = models_dir().ok_or("no local data directory")?;
        // The tokenizer is shared between models, so only the weights are removed.
        let path = model.weights_path(&dir);
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        self.update(model, |s| *s = ModelState::default());
        Ok(())
    }
}

/// How often the highlighter re-reads `ai.json`.
const SETTINGS_POLL: Duration = Duration::from_secs(2);

/// Owns the AI checker inside the highlighter process and keeps it in sync with the settings.
pub struct AiRuntime {
    settings: Option<AiSettings>,
    checker: Option<AiChecker>,
    last_poll: Option<Instant>,
}

impl AiRuntime {
    pub fn new() -> Self {
        Self {
            settings: None,
            checker: None,
            last_poll: None,
        }
    }

    fn sync_settings(&mut self) {
        if self.last_poll.is_some_and(|t| t.elapsed() < SETTINGS_POLL) {
            return;
        }
        self.last_poll = Some(Instant::now());

        let settings = AiSettings::load();
        let Some(dir) = models_dir() else { return };
        let ready = settings.enabled && settings.model.is_downloaded(&dir);

        if !ready {
            self.checker = None;
            self.settings = Some(settings);
            return;
        }

        let failed = self
            .checker
            .as_ref()
            .is_some_and(|c| matches!(c.status(), CheckerStatus::Failed(_)));

        if self.settings != Some(settings) || (self.checker.is_none() && !failed) {
            let model = settings.model;
            self.checker = Some(AiChecker::spawn(move || {
                QwenCorrector::load(&model.weights_path(&dir), &model.tokenizer_path(&dir))
            }));
        }
        self.settings = Some(settings);
    }

    /// Adds AI lints for `text` to `lints`, replacing rule-based lints on the same words, and
    /// honoring the rule being turned off.
    pub fn add_lints(
        &mut self,
        text: &str,
        lint_config: &FlatConfig,
        lints: &mut BTreeMap<String, Vec<Lint>>,
    ) {
        if lint_config.has_rule(AI_RULE_NAME) && !lint_config.is_rule_enabled(AI_RULE_NAME) {
            return;
        }

        self.sync_settings();
        let Some(checker) = &self.checker else { return };

        let ai_lints: Vec<Lint> = checker.lints(text);

        // The AI wins where both flag the same words: the benchmark shows its fix is more often
        // the right one (79% vs 75% of sentences fixed with Qwen3 1.7B).
        for rule_lints in lints.values_mut() {
            rule_lints.retain(|l| !ai_lints.iter().any(|ai| overlaps(l, ai)));
        }
        lints.retain(|_, rule_lints| !rule_lints.is_empty());

        if !ai_lints.is_empty() {
            lints.insert(AI_RULE_NAME.to_string(), ai_lints);
        }
    }
}

/// Whether two lints touch the same characters. An insertion (an empty span) counts as covering
/// the character it is attached to.
fn overlaps(a: &Lint, b: &Lint) -> bool {
    a.span.start < b.span.end.max(b.span.start + 1)
        && b.span.start < a.span.end.max(a.span.start + 1)
}
