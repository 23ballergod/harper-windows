//! Runs a small quantized Qwen model on the CPU to correct one sentence at a time.

use std::path::Path;

use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen2::ModelWeights;
use tokenizers::Tokenizer;

/// Anything that can turn a sentence into its corrected form.
///
/// The checker only depends on this trait so it can be tested without model weights.
pub trait Corrector: Send {
    fn correct(&mut self, sentence: &str) -> Result<String, String>;
}

const SYSTEM_PROMPT: &str = "You are a meticulous copy editor. Fix grammar, spelling, \
punctuation, capitalization, and word-choice mistakes in the user's text. Make the fewest \
changes possible. Keep the author's meaning, tone, wording, and style. Do not \
rephrase sentences that are already correct, do not add explanations, and do not wrap the \
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
        "Their are alot of reasons why I could of done better.",
        "There are a lot of reasons why I could have done better.",
    ),
];

pub struct QwenCorrector {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    eos_tokens: Vec<u32>,
}

impl QwenCorrector {
    /// Loads a Qwen2-architecture GGUF model and its `tokenizer.json`.
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self, String> {
        let device = Device::Cpu;
        let mut file = std::fs::File::open(model_path)
            .map_err(|e| format!("cannot open {}: {e}", model_path.display()))?;
        let content = gguf_file::Content::read(&mut file).map_err(|e| e.to_string())?;
        let model =
            ModelWeights::from_gguf(content, &mut file, &device).map_err(|e| e.to_string())?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| e.to_string())?;

        let eos_tokens = ["<|im_end|>", "<|endoftext|>"]
            .iter()
            .filter_map(|t| tokenizer.token_to_id(t))
            .collect();

        Ok(Self {
            model,
            tokenizer,
            device,
            eos_tokens,
        })
    }

    fn prompt(sentence: &str) -> String {
        let mut prompt = format!("<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n");
        for (input, output) in EXAMPLES {
            prompt.push_str(&format!(
                "<|im_start|>user\n{input}<|im_end|>\n<|im_start|>assistant\n{output}<|im_end|>\n"
            ));
        }
        prompt.push_str(&format!(
            "<|im_start|>user\n{sentence}<|im_end|>\n<|im_start|>assistant\n"
        ));
        prompt
    }
}

impl Corrector for QwenCorrector {
    fn correct(&mut self, sentence: &str) -> Result<String, String> {
        let prompt = Self::prompt(sentence);
        let encoding = self
            .tokenizer
            .encode(prompt, false)
            .map_err(|e| e.to_string())?;
        let prompt_tokens = encoding.get_ids().to_vec();

        let input_len = self
            .tokenizer
            .encode(sentence, false)
            .map(|e| e.get_ids().len())
            .unwrap_or(64);
        let max_new = input_len * 3 / 2 + 16;

        self.model.clear_kv_cache();
        // Greedy decoding: corrections should be deterministic.
        let mut sampler = LogitsProcessor::from_sampling(0, Sampling::ArgMax);

        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| e.to_string())?;
        let logits = self
            .model
            .forward(&input, 0)
            .and_then(|l| l.squeeze(0))
            .map_err(|e| e.to_string())?;
        let mut next = sampler.sample(&logits).map_err(|e| e.to_string())?;

        let mut output = Vec::new();
        for index in 0..max_new {
            if self.eos_tokens.contains(&next) {
                break;
            }
            output.push(next);

            let input = Tensor::new(&[next], &self.device)
                .and_then(|t| t.unsqueeze(0))
                .map_err(|e| e.to_string())?;
            let logits = self
                .model
                .forward(&input, prompt_tokens.len() + index)
                .and_then(|l| l.squeeze(0))
                .map_err(|e| e.to_string())?;
            next = sampler.sample(&logits).map_err(|e| e.to_string())?;
        }

        self.tokenizer
            .decode(&output, true)
            .map(|s| s.trim().to_string())
            .map_err(|e| e.to_string())
    }
}
