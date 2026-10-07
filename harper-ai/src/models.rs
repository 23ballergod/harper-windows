//! The open-weight models users can download. Both are Apache-2.0 licensed Qwen3 models, chosen
//! with `examples/benchmark.rs` (see the "Benchmark results" checks in CI).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AiModel {
    /// Qwen3 1.7B: the default. Fixes 79% of the benchmark's mistakes and leaves 91% of correct
    /// sentences alone, at the speed of the earlier Qwen2.5 1.5B with fewer false alarms.
    #[default]
    Fast,
    /// Qwen3 4B Instruct 2507: the best tested (83% fixed, 93% left alone), about twice as slow.
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
    // Qwen3 1.7B and Qwen3 4B Instruct 2507 ship byte-identical tokenizers.
    url: "https://huggingface.co/Qwen/Qwen3-1.7B/resolve/70d244cc86ccca08cf5af4e1e306ecf908b1ad5e/tokenizer.json",
    file_name: "qwen3-tokenizer.json",
    bytes: 11_422_654,
    sha256: "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4",
};

impl AiModel {
    pub fn display_name(self) -> &'static str {
        match self {
            AiModel::Fast => "Fast (Qwen3 1.7B)",
            AiModel::Accurate => "Most accurate (Qwen3 4B)",
        }
    }

    pub fn weights(self) -> ModelFile {
        match self {
            AiModel::Fast => ModelFile {
                url: "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/resolve/d7f544eead698dbd1f15126ef60b45a1e1933222/Qwen3-1.7B-Q4_K_M.gguf",
                file_name: "Qwen3-1.7B-Q4_K_M.gguf",
                bytes: 1_107_409_472,
                sha256: "b139949c5bd74937ad8ed8c8cf3d9ffb1e99c866c823204dc42c0d91fa181897",
            },
            AiModel::Accurate => ModelFile {
                url: "https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
                file_name: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
                bytes: 2_497_281_120,
                sha256: "3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597",
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
