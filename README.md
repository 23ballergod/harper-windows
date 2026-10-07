# Shah Re-Writer

A Windows grammar checker that works in any app you type in: Chrome, the Claude desktop app, Word, Notepad and more. It runs entirely on your PC, with no account, no subscription and no API costs.

- **Works everywhere**: underlines mistakes in whatever text box has focus and shows fixes in a popup. A tray icon lets you pause, open settings or quit.
- **Offline AI suggestions**: a free, open-weight language model runs on your PC for deeper, Grammarly-style fixes. Nothing you write leaves your computer.
- **Windows installer** built automatically by GitHub Actions.

## Install

1. Open the latest successful [Windows installer](../../actions/workflows/windows_installer.yml) run and download the `harper-windows-installer` artifact.
2. Unzip it and run the `Shah Re-Writer_*_x64-setup.exe` inside. Windows SmartScreen may warn that the app is unrecognized because it isn't code-signed yet; choose **More info → Run anyway**.
3. Follow the short setup. Shah Re-Writer then sits in the system tray and checks whatever you type.
4. Optional: open **Settings → AI Suggestions** and download a model for deeper suggestions.

## How it works

- **Rule checks** (instant): a rule-based grammar engine checks the focused text box through Windows UI Automation and draws underlines over mistakes. Clicking an underline shows the fixes; picking one selects the text in the app and types the correction, so it works in Chrome, Electron apps and Office, and Ctrl+Z undoes it.
- **AI suggestions** (a second or two after you pause): a quantized Qwen3 model runs on your CPU through [candle](https://github.com/huggingface/candle), one sentence at a time. Its correction is diffed against what you wrote, and each changed word becomes its own underline. Heavy rewrites are discarded, and finished sentences are cached so only what you edit is re-checked.

| Model | Download | Speed on a typical laptop | Best for |
|---|---|---|---|
| Fast (Qwen3 1.7B, default) | about 1.1 GB | a second or two per sentence | any PC with 8 GB of RAM |
| Most accurate (Qwen3 4B Instruct 2507) | about 2.5 GB | about twice as slow | PCs with 16 GB of RAM |

The models are downloaded from Hugging Face on first use; they are not bundled with the installer.

## Quality

`harper-ai/tests/data/tricky.jsonl` holds 288 sentences: the mistakes Grammarly is known for catching, plus correct sentences that must be left alone. Every pull request runs `cargo run -p harper-ai --release --example benchmark` against both models and posts the scores as a "Benchmark results" check.

## Building

- AI checker tests: `cargo test -p harper-ai`
- Desktop unit tests: `cd harper-desktop/src-tauri && cargo test --lib`
- Windows installer: `just build-desktop-windows` (cross-compiles from Linux with `cargo-xwin`; this is what CI runs)

## Licenses and credits

Shah Re-Writer is licensed under Apache-2.0; see [LICENSE](LICENSE). It is built on open-source software whose licenses require credit:

- [Harper](https://github.com/Automattic/harper), the rule-based grammar engine, copyright The Harper Contributors and Automattic, Apache-2.0.
- [candle](https://github.com/huggingface/candle), which runs the AI model, MIT or Apache-2.0. `harper-ai/src/qwen2.rs` is adapted from candle-transformers.
- Qwen3 models by the Qwen team at Alibaba Cloud, Apache-2.0 (GGUF conversions by Unsloth).
- Tauri, egui, wgpu and other libraries, MIT or Apache-2.0.
