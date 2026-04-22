# sysvoice_rec

Windows のシステム音声（WASAPI ループバック）をリアルタイムで録音し、WAV / FLAC / MP3 / AAC / Opus 形式で保存するデスクトップ GUI アプリです。

## 主な機能
- **システム音声キャプチャ** — WASAPI ループバックを使用（Windows のみ）
- **複数フォーマット対応** — WAV はそのまま保存、その他は ffmpeg 経由でエンコード
- **ラウドネス正規化** — 録音後に -14 LUFS に自動調整（loudnorm）
- **ffmpeg 自動管理** — 未インストール時に自動ダウンロード、失敗時は手動案内
- **設定永続化** — 保存先・フォーマット設定を自動保存

## 動作要件
- **OS**: Windows 10 / 11（WASAPI ループバック対応）
- **Rust**: stable 1.70 以降
- **ffmpeg**: WAV 以外のフォーマットで必要（アプリ内で自動ダウンロード可）

## ビルド & 実行
```powershell
cargo run --release
```

## 注意事項
- macOS / Linux では WASAPI が使えないため、システム音声キャプチャは動作しません（デフォルト入力デバイスにフォールバック）
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