//! The open-weight models users can download. Both are Apache-2.0 licensed Qwen2.5 models.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AiModel {
    /// Qwen2.5 0.5B Instruct: quick on any PC, catches the common mistakes.
    #[default]
    Fast,
    /// Qwen2.5 1.5B Instruct: noticeably better suggestions, about three times slower.
    Accurate,
}

/// A file to download, with a rough size for progress display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    pub url: &'static str,
    pub file_name: &'static str,
    pub approx_bytes: u64,
}

const TOKENIZER: ModelFile = ModelFile {
    url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/main/tokenizer.json",
    file_name: "qwen2.5-tokenizer.json",
    approx_bytes: 7_000_000,
};

impl AiModel {
    pub fn display_name(self) -> &'static str {
        match self {
            AiModel::Fast => "Fast (Qwen2.5 0.5B)",
            AiModel::Accurate => "Accurate (Qwen2.5 1.5B)",
        }
    }

    pub fn weights(self) -> ModelFile {
        match self {
            AiModel::Fast => ModelFile {
                url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
                file_name: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
                approx_bytes: 491_000_000,
            },
            AiModel::Accurate => ModelFile {
                url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
                file_name: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
                approx_bytes: 1_120_000_000,
            },
        }
    }

    pub fn tokenizer(self) -> ModelFile {
        TOKENIZER
    }

    /// Every file the model needs.
    pub fn files(self) -> [ModelFile; 2] {
        [self.weights(), self.tokenizer()]
    }

    pub fn weights_path(self, dir: &Path) -> PathBuf {
        dir.join(self.weights().file_name)
    }

    pub fn tokenizer_path(self, dir: &Path) -> PathBuf {
        dir.join(self.tokenizer().file_name)
    }

    pub fn is_downloaded(self, dir: &Path) -> bool {
        self.files().iter().all(|f| dir.join(f.file_name).is_file())
    }
}
