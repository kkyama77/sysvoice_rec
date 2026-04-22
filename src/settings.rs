use crate::encoder::OutputFormat;
use crate::ffmpeg_manager;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// UI 表示言語。
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum Lang { En, Ja }

fn default_volume_pct() -> u32 { 100 }
fn default_lang() -> Lang { Lang::En }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub output_format: OutputFormat,
    pub save_directory: PathBuf,
    pub ffmpeg_path: Option<PathBuf>,
    /// 出力音量 (0–200%)。100 = -14 LUFS（配信サービスで一般的な基準）。圧縮形式のみ有効。
    #[serde(default = "default_volume_pct")]
    pub volume_pct: u32,
    /// UI 表示言語。CJK フォントが利用可能な場合のみ Ja に切り替え可。
    #[serde(default = "default_lang")]
    pub lang: Lang,
}

impl Default for Settings {
    fn default() -> Self {
        let save_dir = dirs::audio_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            output_format: OutputFormat::Wav,
            save_directory: save_dir,
            ffmpeg_path: None,
            volume_pct: 100,
            lang: Lang::En,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str::<Settings>(&json) { return s; }
            }
        }
        Default::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(json) = serde_json::to_string_pretty(self) { let _ = std::fs::write(&path, json); }
    }

    /// Find ffmpeg binary.
    /// 1. Stored path in settings
    /// 2. App cache (auto-downloaded)
    /// 3. Same folder as the executable
    /// 4. System PATH
    pub fn find_ffmpeg(&self) -> Option<PathBuf> {
        if let Some(p) = &self.ffmpeg_path {
            if p.exists() { return Some(p.clone()); }
        }
        let cached = ffmpeg_manager::cached_ffmpeg_path();
        if cached.exists() { return Some(cached); }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                #[cfg(windows)] let name = "ffmpeg.exe";
                #[cfg(not(windows))] let name = "ffmpeg";
                let c = dir.join(name);
                if c.exists() { return Some(c); }
            }
        }
        which::which("ffmpeg").ok()
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sysvoice_rec")
            .join("config.json")
    }
}
