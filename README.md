# Harper for Windows

A Windows grammar checker that works in any app you type in (Chrome, the Claude desktop app, Word, Notepad and more), built on the open-source [Harper](https://github.com/Automattic/harper) engine by Automattic.

What this fork adds on top of Harper:

- **Windows-first desktop app**: underlines mistakes in whatever text box has focus and shows fixes in a popup, with a tray icon.
- **Offline AI suggestions**: an optional, free, open-weight language model (Qwen2.5, Apache-2.0) that runs entirely on your PC for deeper, Grammarly-style fixes. No API key, no subscription, nothing leaves your computer.
- **Windows installer** built automatically by GitHub Actions.

## Install

1. Open the latest successful [Windows installer](../../actions/workflows/windows_installer.yml) run and download the `harper-windows-installer` artifact.
2. Unzip it and run the `Harper for Windows_*_x64-setup.exe` inside. Windows SmartScreen may warn that the app is unrecognized because it isn't code-signed yet; choose **More info → Run anyway**.
3. Follow the short setup. Harper then sits in the system tray and checks whatever you type.
4. Optional: open **Settings → AI Suggestions** and download a model for deeper suggestions.

## How it works

- **Harper rules** (instant): Harper's rule engine checks the focused text box through Windows UI Automation and draws underlines over mistakes. Clicking an underline shows the fixes; picking one selects the text in the app and types the correction, so it works in Chrome, Electron apps and Office, and Ctrl+Z undoes it.
- **AI suggestions** (a second or two after you pause): a quantized Qwen2.5 model runs on your CPU through [candle](https://github.com/huggingface/candle), one sentence at a time. Its correction is diffed against what you wrote, and each changed word becomes its own underline. Heavy rewrites are discarded, and finished sentences are cached so only what you edit is re-checked.

| Model | Download | Speed on a typical laptop | Best for |
|---|---|---|---|
| Fast (Qwen2.5 0.5B) | about 500 MB | quick | any PC |
| Accurate (Qwen2.5 1.5B, default) | about 1.1 GB | slower | PCs with 8 GB of RAM or more |

The models are downloaded from Hugging Face on first use; they are not bundled with the installer. Qwen2.5 is licensed under Apache-2.0 by the Qwen team.

## Quality

`harper-ai/tests/data/tricky.jsonl` holds 288 sentences: the mistakes Grammarly is known for catching, plus correct sentences that must be left alone. Every pull request runs `cargo run -p harper-ai --release --example benchmark` against both models and posts the scores as a "Benchmark results" check.

## Building

- AI checker tests: `cargo test -p harper-ai`
- Desktop unit tests: `cd harper-desktop/src-tauri && cargo test --lib`
- Windows installer: `just build-desktop-windows` (cross-compiles from Linux with `cargo-xwin`; this is what CI runs)

## License

Harper is licensed under Apache-2.0; see [LICENSE](LICENSE). This project keeps that license and credits the Harper authors for the core grammar engine. `harper-ai/src/qwen2.rs` is adapted from candle-transformers (MIT OR Apache-2.0).

---

<div id="header" align="center">
    <img src="logo.svg" width="400px" />
    <h1>Harper</h1>
</div>

[![Harper Binaries](https://github.com/automattic/harper/actions/workflows/binaries.yml/badge.svg)](https://github.com/automattic/harper/actions/workflows/binaries.yml)
[![Website](https://github.com/automattic/harper/actions/workflows/build_web.yml/badge.svg)](https://github.com/automattic/harper/actions/workflows/build_web.yml)
[![Checks](https://github.com/automattic/harper/actions/workflows/just_checks.yml/badge.svg)](https://github.com/automattic/harper/actions/workflows/just_checks.yml)
[![Crates.io](https://img.shields.io/crates/v/harper-ls)](https://crates.io/crates/harper-ls)
![NPM Version](https://img.shields.io/npm/v/harper.js)
![Downloads](https://img.shields.io/github/downloads/automattic/harper/total?label=Binary+Downloads)
![Obsidian Plugin Downloads](https://img.shields.io/github/downloads/automattic/harper-obsidian-plugin/total?label=Obsidian+Plugin+Downloads)

Harper is an English grammar checker designed to be _just right._
I created it after years of dealing with the shortcomings of the competition.

Grammarly was too expensive and too overbearing.
Its suggestions lacked context, and were often just plain _wrong_.
Not to mention: it's a privacy nightmare.
Everything you write with Grammarly is sent to their servers.
Their privacy policy claims they don't sell the data, but that doesn't mean they don't use it to train large language models and god knows what else.
Not only that, but the round-trip-time of the network request makes revising your work all the more tedious.

LanguageTool is great, if you have gigabytes of RAM to spare and are willing to download the ~16GB n-gram dataset.
Besides the memory requirements, I found LanguageTool too slow: it would take several seconds to lint even a moderate-size document.

That's why I created Harper: it is the grammar checker that fits my needs.
Not only does it take milliseconds to lint a document, take less than 1/50th of LanguageTool's memory footprint,
but it is also completely private.

Harper is even small enough to load via [WebAssembly.](https://writewithharper.com)

## Language Support

Harper currently only supports English, but the core is extensible to support other languages, so we welcome contributions that allow for other language support.

## Performance Issues

We consider long lint times bugs.
If you encounter any significant performance issues, please create an issue on the topic.

If you find a fix to any performance issue, we would appreciate the contribution.
Just please make sure to read [our contribution guidelines first.](https://writewithharper.com/docs/contributors/introduction)

## Links

- [Frequently Asked Questions](https://writewithharper.com/#faqs)
- [Obsidian Documentation](https://writewithharper.com/docs/integrations/obsidian)
- [`harper-ls` Documentation](https://writewithharper.com/docs/integrations/language-server)
- Supported Editors' Documentation
  - [Visual Studio Code](https://writewithharper.com/docs/integrations/visual-studio-code)
  - [Neovim](https://writewithharper.com/docs/integrations/neovim)
  - [Helix](https://writewithharper.com/docs/integrations/helix)
  - [Emacs](https://writewithharper.com/docs/integrations/emacs)
  - [Zed](https://writewithharper.com/docs/integrations/zed)
- [`harper.js` Documentation](https://writewithharper.com/docs/harperjs/introduction)
- [Official Discord Server](https://discord.com/invite/JBqcAaKrzQ)

## Huge Thanks

This project would not be possible without the hard work from those who [contribute](https://writewithharper.com/docs/contributors/introduction).

<a href="https://github.com/automattic/harper/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=automattic/harper" />
</a>

Harper's logo was designed by [Lukas Werner](https://lukaswerner.com/).
