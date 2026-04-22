# sysvoice_rec

Windows / Linux で音声をリアルタイム録音し、WAV / FLAC / MP3 / AAC / Opus 形式で保存するデスクトップ GUI アプリです。

## 主な機能
- **音声キャプチャ**
  - Windows: WASAPI ループバック（システム出力音声）
  - Linux: デフォルト入力デバイス（マイク等）
- **複数フォーマット対応** — WAV はそのまま保存、その他は ffmpeg 経由でエンコード
- **ラウドネス正規化** — 録音後に -14 LUFS に自動調整（loudnorm）
- **ffmpeg 管理** — Windows は未インストール時に自動ダウンロード、Linux は手動インストール案内
- **設定永続化** — 保存先・フォーマット設定を自動保存

## 動作要件
- **OS**: Windows 10 / 11 または Linux
- **Rust**: stable 1.70 以降
- **ffmpeg**: WAV 以外のフォーマットで必要（アプリ内で自動ダウンロード可）

## ビルド & 実行
```powershell
cargo run --release
```

## 注意事項
- Linux ではシステム音声ループバックではなく、デフォルト入力デバイス（マイク等）を録音します
- ffmpeg 自動ダウンロードは Windows のみ対応です
- 録音中にアプリを強制終了した場合、`%TEMP%\sysvoice_*.wav` が残ることがありますが、次回起動時に自動削除されます

---

# sysvoice_rec — Rust 開発環境セットアップガイド (日本語)

このリポジトリで Rust 開発を始めるための手順をまとめています。Windows 環境（WSL 利用も可）を想定していますが、Linux / macOS でもほとんど同様に適用できます。

## 目次
- 概要
- 前提条件
- Rust のインストール (Windows)
- 共通ツールのインストール
- VS Code の推奨設定
- 新しいプロジェクトの作成と基本コマンド
- フォーマット・静的解析
- デバッグ
- CI (GitHub Actions) の例
- トラブルシューティング / 参考リンク

---

## 概要
この README は、ローカル環境で Rust 開発を快適に行うための最低限の手順と推奨ツールを記載しています。プロジェクト固有のルール（フォーマット、Lint 等）があれば別途追加してください。

---

## 前提条件
- インターネット接続
- 管理者権限（Windows の場合、一部インストールで必要）
- ターミナル（PowerShell / cmd / WSL / Windows Terminal など）

Windows 10/11 をお使いの場合、WSL2 + Ubuntu を利用すると Linux 環境でのビルドが容易になります。WSL を利用する場合は、WSL 内で以下の手順を実行してください。

---

## Rust のインストール (Windows)
公式推奨のツールチェイン管理ツールは `rustup` です。Windows ではインストーラを利用するのが簡単です。

1. 公式インストーラ（GUI）を使用する:
   - https://rust-lang.org にアクセスし「Install」から Windows Installer をダウンロードして実行します。

2. コマンドラインでインストールする（PowerShell の例）:
   - PowerShell（管理者でなくても可）を開き、以下を実行します。
     - `Invoke-WebRequest -Uri https://sh.rustup.rs -OutFile rustup-init.sh`
     - `sh rustup-init.sh` として手順に従います（WSL / Git Bash / MSYS2 の利用を推奨）。

3. インストール確認:
   - `rustc --version`
   - `cargo --version`
   - `rustup --version`

インストール後、デフォルトで `stable` が設定されます。必要に応じてツールチェインを切り替えてください:
- `rustup default stable`
- `rustup toolchain install nightly`
- `rustup default nightly`（必要な場合）

---

## 共通ツールのインストール
- rustfmt（コードフォーマッタ）
  - `rustup component add rustfmt`
- clippy（Lint ツール）
  - `rustup component add clippy`
- その他（必要に応じて）
  - `rustup component add rls`（古いツール: 現在は rust-analyzer を推奨）

Windows でネイティブライブラリを使う場合は、Visual Studio Build Tools（C++ ビルドツール）などが必要になることがあります。MSVC toolchain を使う場合は Visual Studio の「C++ ビルドツール」をインストールしてください。GNU toolchain を使う場合は MinGW や MSYS2 を検討してください。

---

## VS Code の推奨設定
- 拡張機能:
  - `rust-lang.rust-analyzer`（推奨）
  - `matklad.rust-analyzer`（古い ID。現在は上記）
  - `vadimcn.vscode-lldb`（CodeLLDB デバッガ）
  - `ms-vscode.cpptools`（ネイティブ拡張が必要な場合）

- 推奨設定（ワークスペース `.vscode/settings.json` に追加例）:
    "rust-analyzer.cargo.runBuildScripts": true  
    "rust-analyzer.procMacro.enable": true  
    "editor.formatOnSave": true  
    "editor.defaultFormatter": "rust-lang.rust-analyzer"

（上記は JSON 形式のキー: 値として `.vscode/settings.json` に記述してください）

---

## 新しいプロジェクトの作成と基本コマンド
1. 新規プロジェクト作成（バイナリプロジェクト）
   - `cargo new my_app --bin`
2. ライブラリプロジェクト
   - `cargo new my_lib --lib`
3. ビルド
   - `cargo build`（デバッグビルド）
   - `cargo build --release`（最適化ビルド）
4. 実行
   - `cargo run`
   - `cargo run --release`
5. テスト
   - `cargo test`
6. ドキュメント生成
   - `cargo doc --open`
7. 依存関係追加
   - `cargo add <crate>`（`cargo-edit` を事前にインストールする必要あり）
   - 例: `cargo install cargo-edit` → `cargo add anyhow`

---

## フォーマット・静的解析
- コード整形:
  - `cargo fmt`（ファイル単位なら `cargo fmt -- <path>`）
- Lint:
  - `cargo clippy`
  - よく使うオプション: `cargo clippy -- -D warnings`（警告をエラー扱いにする）

CI では `cargo fmt -- --check` と `cargo clippy -- -D warnings` を実行してスタイル・品質を強制することが多いです。

---

## デバッグ
- VS Code と CodeLLDB を使うのが一般的です。
- launch 設定例（`.vscode/launch.json`）: デバッグ設定はプロジェクトによって異なります。Rust + CodeLLDB の基本設定を作成してください。
- Windows で MSVC toolchain を使う場合はシンボル情報が必要になるため、`cargo build` 時にデバッグ情報が含まれます（デフォルトでデバッグは有効）。

---

## CI (GitHub Actions) の例
以下は簡易的なワークフローの流れの例です（GitHub Actions）。ワークフローファイルは `.github/workflows/ci.yml` に配置します。

インデントしてプレーンテキストで YAML を記述できます。主要なステップ:
- Checkout
- Install Rust toolchain（`actions-rs/toolchain` 等を使用）
- `cargo build --verbose`
- `cargo test --verbose`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`

GitHub Actions の公式アクションや `actions-rs` を使うと簡単にセットアップできます。

---

## トラブルシューティング
- `rustc` / `cargo` が見つからない:
  - PATH に rustup の cargo bin ディレクトリが追加されているか確認（例: `%USERPROFILE%\.cargo\bin`）
  - ターミナルを再起動して反映されます。
- ネイティブライブラリのリンクエラー:
  - Visual Studio Build Tools（MSVC）, または MSYS2 / MinGW のセットアップを確認。
- rust-analyzer が動作しない:
  - VS Code を再起動、拡張機能を再インストール、`rust-analyzer` の設定を確認。

---

## 参考リンク
- Rust 公式: https://www.rust-lang.org/ja
- rustup: https://rust-lang.github.io/rustup/
- rust-analyzer: https://rust-analyzer.github.io/
- Cargo Book: https://doc.rust-lang.org/cargo/

---

必要なら、この README にプロジェクト固有のセットアップ手順（外部サービスの API キー、DB のセットアップ手順、ビルドのカスタムフラグなど）を追記します。どのような開発フローにしたいか教えてください（例: Windows ネイティブビルド重視 / WSL 利用 / cross-compilation など）。

---

# sysvoice_rec (English)

A desktop GUI application for Windows / Linux that records audio in real time and saves it as WAV / FLAC / MP3 / AAC / Opus.

## Main Features
- **Audio capture**
  - Windows: WASAPI loopback (system output audio)
  - Linux: default input device (microphone, etc.)
- **Multiple output formats** - WAV is saved directly; other formats are encoded via ffmpeg
- **Loudness normalization** - automatically normalizes to -14 LUFS after recording (loudnorm)
- **ffmpeg management** - auto-download on Windows when missing; manual install guidance on Linux
- **Persistent settings** - save directory and output format are stored automatically

## Requirements
- **OS**: Windows 10 / 11 or Linux
- **Rust**: stable 1.70+
- **ffmpeg**: required for non-WAV formats (auto-download supported in-app)

## Build & Run
```powershell
cargo run --release
```

## Notes
- On Linux, this app records from the default input device (microphone, etc.), not system loopback audio
- Automatic ffmpeg download is supported only on Windows
- If the app is force-closed during recording, `%TEMP%\sysvoice_*.wav` may remain, but it will be cleaned up automatically on next startup

---

# sysvoice_rec - Rust Development Environment Setup Guide (English)

This section summarizes steps to start Rust development in this repository. It assumes a Windows environment (WSL optional), but almost all steps also apply to Linux / macOS.

## Table of Contents
- Overview
- Prerequisites
- Install Rust (Windows)
- Install common tools
- Recommended VS Code settings
- Create a new project and basic commands
- Formatting and static analysis
- Debugging
- CI (GitHub Actions) example
- Troubleshooting / Reference links

---

## Overview
This README includes the minimum recommended steps and tools for comfortable local Rust development. Add project-specific rules (formatting, lint rules, etc.) as needed.

---

## Prerequisites
- Internet connection
- Administrator privileges (may be required for some Windows installations)
- Terminal (PowerShell / cmd / WSL / Windows Terminal, etc.)

If you use Windows 10/11, using WSL2 + Ubuntu makes Linux builds easier. If you use WSL, run the following steps inside WSL.

---

## Install Rust (Windows)
The officially recommended toolchain manager is `rustup`. On Windows, the installer is the easiest path.

1. Use the official installer (GUI):
   - Go to https://rust-lang.org, open "Install", download the Windows Installer, and run it.

2. Install via command line (PowerShell example):
   - Open PowerShell (admin not required) and run:
     - `Invoke-WebRequest -Uri https://sh.rustup.rs -OutFile rustup-init.sh`
     - Run `sh rustup-init.sh` and follow the steps (WSL / Git Bash / MSYS2 recommended).

3. Verify installation:
   - `rustc --version`
   - `cargo --version`
   - `rustup --version`

After installation, `stable` is set by default. Switch toolchains if needed:
- `rustup default stable`
- `rustup toolchain install nightly`
- `rustup default nightly` (if needed)

---

## Install Common Tools
- rustfmt (code formatter)
  - `rustup component add rustfmt`
- clippy (lint tool)
  - `rustup component add clippy`
- Others (optional)
  - `rustup component add rls` (legacy tool; rust-analyzer is recommended now)

If you use native libraries on Windows, you may need Visual Studio Build Tools (C++ build tools). For the MSVC toolchain, install Visual Studio "C++ Build Tools". For GNU toolchain, consider MinGW or MSYS2.

---

## Recommended VS Code Settings
- Extensions:
  - `rust-lang.rust-analyzer` (recommended)
  - `matklad.rust-analyzer` (old ID; use the one above now)
  - `vadimcn.vscode-lldb` (CodeLLDB debugger)
  - `ms-vscode.cpptools` (if native extension support is needed)

- Recommended settings (example entries for workspace `.vscode/settings.json`):
    "rust-analyzer.cargo.runBuildScripts": true  
    "rust-analyzer.procMacro.enable": true  
    "editor.formatOnSave": true  
    "editor.defaultFormatter": "rust-lang.rust-analyzer"

(Write the above key-value pairs in proper JSON format in `.vscode/settings.json`.)

---

## Create a New Project and Basic Commands
1. Create a new binary project
   - `cargo new my_app --bin`
2. Create a library project
   - `cargo new my_lib --lib`
3. Build
   - `cargo build` (debug build)
   - `cargo build --release` (optimized build)
4. Run
   - `cargo run`
   - `cargo run --release`
5. Test
   - `cargo test`
6. Generate docs
   - `cargo doc --open`
7. Add dependencies
   - `cargo add <crate>` (requires `cargo-edit` installed first)
   - Example: `cargo install cargo-edit` -> `cargo add anyhow`

---

## Formatting and Static Analysis
- Format code:
  - `cargo fmt` (file-scoped: `cargo fmt -- <path>`)
- Lint:
  - `cargo clippy`
  - Common option: `cargo clippy -- -D warnings` (treat warnings as errors)

In CI, `cargo fmt -- --check` and `cargo clippy -- -D warnings` are commonly used to enforce style and quality.

---

## Debugging
- VS Code + CodeLLDB is a common setup.
- Launch config example (`.vscode/launch.json`): debug settings vary by project. Create a basic Rust + CodeLLDB configuration.
- When using the MSVC toolchain on Windows, symbol information is required; debug info is included by default in `cargo build`.

---

## CI (GitHub Actions) Example
Below is a simple workflow outline for GitHub Actions. Place the workflow file at `.github/workflows/ci.yml`.

You can write YAML as plain text with indentation. Typical steps:
- Checkout
- Install Rust toolchain (e.g., using `actions-rs/toolchain`)
- `cargo build --verbose`
- `cargo test --verbose`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`

Using official GitHub Actions and/or `actions-rs` simplifies setup.

---

## Troubleshooting
- `rustc` / `cargo` not found:
  - Check whether rustup cargo bin is in PATH (e.g., `%USERPROFILE%\.cargo\bin`)
  - Restart your terminal to apply changes.
- Native library link errors:
  - Check Visual Studio Build Tools (MSVC), or MSYS2 / MinGW setup.
- rust-analyzer not working:
  - Restart VS Code, reinstall extension, and verify rust-analyzer settings.

---

## Reference Links
- Rust official: https://www.rust-lang.org/
- rustup: https://rust-lang.github.io/rustup/
- rust-analyzer: https://rust-analyzer.github.io/
- Cargo Book: https://doc.rust-lang.org/cargo/

---

If needed, we can add project-specific setup steps to this README (external API keys, DB setup, custom build flags, etc.). Let us know your preferred workflow (e.g., Windows-native focus / WSL / cross-compilation).