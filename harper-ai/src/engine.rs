//! Runs a small quantized Qwen model on the CPU to correct one sentence at a time.

use std::path::Path;

use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use tokenizers::Tokenizer;

/// Anything that can turn a sentence into its corrected form.
///
/// The checker only depends on this trait so it can be tested without model weights.
pub trait Corrector: Send {
    fn correct(&mut self, sentence: &str) -> Result<String, String>;
}

const SYSTEM_PROMPT: &str = "You are a meticulous copy editor. Fix grammar, spelling, \
punctuation, capitalization, and word-choice mistakes in the user's text. Make the fewest \
changes possible. Keep the author's meaning, tone, wording, and style, including \
contractions, slang, and casual phrasing. Do not rephrase sentences that are already correct, do not add explanations, and do not wrap the \
answer in quotes. Reply with only the corrected text.";

/// Few-shot examples teach a small model the expected output format far more reliably than
/// instructions alone.
const EXAMPLES: &[(&str, &str)] = &[
    (
        "me and him goes to the libary every weekends",
        "He and I go to the library every weekend",
    ),
    ("The weather is nice today.", "The weather is nice today."),
    (
        "i dont think its gonna work tbh",
        "I don't think it's gonna work tbh",
    ),
    (
        "Their are alot of reasons why I could of done better.",
        "There are a lot of reasons why I could have done better.",
    ),
];

/// The supported model architectures, which share one interface.
enum Model {
    Qwen2(crate::qwen2::ModelWeights),
    Qwen3(crate::qwen3::ModelWeights),
}

macro_rules! each_model {
    ($self:expr, $m:ident => $body:expr) => {
        match $self {
            Model::Qwen2($m) => $body,
            Model::Qwen3($m) => $body,
        }
    };
}

impl Model {
    fn forward(&mut self, x: &Tensor, pos: usize) -> candle_core::Result<Tensor> {
        each_model!(self, m => m.forward(x, pos))
    }
    fn forward_all(&mut self, x: &Tensor, pos: usize) -> candle_core::Result<Tensor> {
        each_model!(self, m => m.forward_all(x, pos))
    }
    fn clear_kv_cache(&mut self) {
        each_model!(self, m => m.clear_kv_cache())
    }
    fn snapshot_kv_cache(&self) -> Vec<Option<(Tensor, Tensor)>> {
        each_model!(self, m => m.snapshot_kv_cache())
    }
    fn restore_kv_cache(&mut self, snapshot: &[Option<(Tensor, Tensor)>]) {
        each_model!(self, m => m.restore_kv_cache(snapshot))
    }
    fn truncate_kv_cache(&mut self, len: usize) -> candle_core::Result<()> {
        each_model!(self, m => m.truncate_kv_cache(len))
    }
}

pub struct QwenCorrector {
    model: Model,
    /// Hybrid Qwen3 models think out loud unless the answer starts with an empty thinking block.
    skip_thinking: bool,
    tokenizer: Tokenizer,
    device: Device,
    eos_tokens: Vec<u32>,
    /// The attention cache after reading the instructions and examples, which never change.
    prefix_cache: Vec<Option<(Tensor, Tensor)>>,
    prefix_len: usize,
}

impl QwenCorrector {
    /// Loads a Qwen2 or Qwen3 GGUF model and its `tokenizer.json`.
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self, String> {
        let device = Device::Cpu;
        let mut file = std::fs::File::open(model_path)
            .map_err(|e| format!("cannot open {}: {e}", model_path.display()))?;
        let content = gguf_file::Content::read(&mut file).map_err(|e| e.to_string())?;
        let metadata_str = |key: &str| {
            content
                .metadata
                .get(key)
                .and_then(|v| v.to_string().ok())
                .cloned()
                .unwrap_or_default()
        };
        let architecture = metadata_str("general.architecture");
        let skip_thinking = metadata_str("tokenizer.chat_template").contains("enable_thinking");
        let model = match architecture.as_str() {
            "qwen2" => Model::Qwen2(
                crate::qwen2::ModelWeights::from_gguf(content, &mut file, &device)
                    .map_err(|e| e.to_string())?,
            ),
            "qwen3" => Model::Qwen3(
                crate::qwen3::ModelWeights::from_gguf(content, &mut file, &device)
                    .map_err(|e| e.to_string())?,
            ),
            other => return Err(format!("unsupported model architecture {other:?}")),
        };
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| e.to_string())?;

        let eos_tokens = ["<|im_end|>", "<|endoftext|>"]
            .iter()
            .filter_map(|t| tokenizer.token_to_id(t))
            .collect();

        let mut corrector = Self {
            model,
            skip_thinking,
            tokenizer,
            device,
            eos_tokens,
            prefix_cache: Vec::new(),
            prefix_len: 0,
        };
        corrector.warm_prefix()?;
        Ok(corrector)
    }

    /// The part of the prompt that is the same for every sentence.
    fn prefix(&self) -> String {
        let mut prompt = format!("<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n");
        for (input, output) in EXAMPLES {
            prompt.push_str(&format!(
                "<|im_start|>user\n{input}<|im_end|>\n{}{output}<|im_end|>\n",
                self.answer_start()
            ));
        }
        prompt.push_str("<|im_start|>user\n");
        prompt
    }

    fn answer_start(&self) -> &'static str {
        if self.skip_thinking {
            "<|im_start|>assistant\n<think>\n\n</think>\n\n"
        } else {
            "<|im_start|>assistant\n"
        }
    }

    fn suffix(&self, sentence: &str) -> String {
        format!("{sentence}<|im_end|>\n{}", self.answer_start())
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        self.tokenizer
            .encode(text, false)
            .map(|e| e.get_ids().to_vec())
            .map_err(|e| e.to_string())
    }

    /// Runs the shared prefix through the model once and keeps its attention cache.
    fn warm_prefix(&mut self) -> Result<(), String> {
        let tokens = self.encode(&self.prefix())?;
        self.model.clear_kv_cache();
        let input = Tensor::new(tokens.as_slice(), &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| e.to_string())?;
        self.model.forward(&input, 0).map_err(|e| e.to_string())?;
        self.prefix_cache = self.model.snapshot_kv_cache();
        self.prefix_len = tokens.len();
        Ok(())
    }
}

/// How many tokens of the user's own sentence to propose at once.
const DRAFT_LEN: usize = 10;

/// Proposes the tokens that follow the generated text in the original sentence.
///
/// A correction mostly repeats the input, so the next tokens can usually be predicted by finding
/// where the output currently is in the input ("prompt lookup decoding"). The model then checks the
/// whole guess in one pass, which is much faster than producing one token at a time.
fn draft_from_source(source: &[u32], generated: &[u32], max: usize) -> Vec<u32> {
    if generated.is_empty() {
        return source.iter().take(max).copied().collect();
    }
    for n in (1..=3usize).rev() {
        if generated.len() < n {
            continue;
        }
        let tail = &generated[generated.len() - n..];
        // Prefer the latest match: corrections move forward through the sentence.
        if let Some(pos) = source.windows(n).rposition(|w| w == tail) {
            let from = pos + n;
            if from < source.len() {
                return source[from..].iter().take(max).copied().collect();
            }
        }
    }
    Vec::new()
}

impl QwenCorrector {
    fn argmax_rows(logits: &Tensor) -> Result<Vec<u32>, String> {
        logits
            .argmax(candle_core::D::Minus1)
            .and_then(|t| t.to_vec1::<u32>())
            .map_err(|e| e.to_string())
    }
}

impl Corrector for QwenCorrector {
    fn correct(&mut self, sentence: &str) -> Result<String, String> {
        let prompt_tokens = self.encode(&self.suffix(sentence))?;
        let source = self.encode(sentence)?;
        let max_new = source.len() * 3 / 2 + 16;

        self.model.restore_kv_cache(&self.prefix_cache);
        let mut pos = self.prefix_len;

        // Greedy decoding: corrections should be deterministic.
        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| e.to_string())?;
        let logits = self
            .model
            .forward(&input, pos)
            .and_then(|l| l.squeeze(0))
            .map_err(|e| e.to_string())?;
        pos += prompt_tokens.len();
        let mut next = logits
            .argmax(candle_core::D::Minus1)
            .and_then(|t| t.to_scalar::<u32>())
            .map_err(|e| e.to_string())?;

        let mut output: Vec<u32> = Vec::new();
        while output.len() < max_new && !self.eos_tokens.contains(&next) {
            output.push(next);

            let draft = draft_from_source(&source, &output, DRAFT_LEN);
            let mut input_ids = vec![next];
            input_ids.extend_from_slice(&draft);

            let input = Tensor::new(input_ids.as_slice(), &self.device)
                .and_then(|t| t.unsqueeze(0))
                .map_err(|e| e.to_string())?;
            let logits = self
                .model
                .forward_all(&input, pos)
                .map_err(|e| e.to_string())?;
            let predicted = Self::argmax_rows(&logits)?;

            // predicted[i] is the model's choice after input_ids[..=i]. Accept draft tokens for
            // as long as they match what the model would have produced anyway.
            let mut accepted = 0;
            while accepted < draft.len()
                && predicted[accepted] == draft[accepted]
                && !self.eos_tokens.contains(&draft[accepted])
                && output.len() < max_new
            {
                output.push(draft[accepted]);
                accepted += 1;
            }

            pos += 1 + accepted;
            if accepted < draft.len() {
                self.model
                    .truncate_kv_cache(pos)
                    .map_err(|e| e.to_string())?;
            }
            next = predicted[accepted];
        }

        self.tokenizer
            .decode(&output, true)
            .map(|s| s.trim().to_string())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::draft_from_source;

    #[test]
    fn drafts_start_of_sentence_first() {
        assert_eq!(draft_from_source(&[1, 2, 3, 4], &[], 2), vec![1, 2]);
    }

    #[test]
    fn drafts_continue_after_matching_tail() {
        // Output so far "1 9 3" after a substitution; continue after "3" in the source.
        assert_eq!(
            draft_from_source(&[1, 2, 3, 4, 5], &[1, 9, 3], 5),
            vec![4, 5]
        );
    }

    #[test]
    fn no_draft_when_tail_is_unknown() {
        assert!(draft_from_source(&[1, 2, 3], &[7, 8], 5).is_empty());
    }
}
