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

/// A file to download. The URL is pinned to one upstream revision, and the size and SHA-256 are
/// checked after downloading so a truncated or changed file is never used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    pub url: &'static str,
    pub file_name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

impl ModelFile {
    /// Whether `path` holds this file at its full size. The checksum is verified when downloading.
    pub fn is_complete(&self, path: &Path) -> bool {
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == self.bytes)
    }
}

const TOKENIZER: ModelFile = ModelFile {
    url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/7ae557604adf67be50417f59c2c2f167def9a775/tokenizer.json",
    file_name: "qwen2.5-tokenizer.json",
    bytes: 7_031_645,
    sha256: "c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539",
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
                url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/9217f5db79a29953eb74d5343926648285ec7e67/qwen2.5-0.5b-instruct-q4_k_m.gguf",
                file_name: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
                bytes: 491_400_032,
                sha256: "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db",
            },
            AiModel::Accurate => ModelFile {
                url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/91cad51170dc346986eccefdc2dd33a9da36ead9/qwen2.5-1.5b-instruct-q4_k_m.gguf",
                file_name: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
                bytes: 1_117_320_736,
                sha256: "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
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
        self.files()
            .iter()
            .all(|f| f.is_complete(&dir.join(f.file_name)))
    }
}
