use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Output format
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Wav,
    Flac,
    Mp3,
    Aac,
    Opus,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav  => "wav",
            Self::Flac => "flac",
            Self::Mp3  => "mp3",
            Self::Aac  => "m4a",
            Self::Opus => "opus",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Wav  => "WAV",
            Self::Flac => "FLAC",
            Self::Mp3  => "MP3",
            Self::Aac  => "AAC",
            Self::Opus => "Opus",
        }
    }

    /// Returns true if ffmpeg is required
    pub fn requires_ffmpeg(self) -> bool {
        !matches!(self, Self::Wav)
    }

    pub fn all() -> &'static [OutputFormat] {
        &[
            OutputFormat::Wav,
            OutputFormat::Flac,
            OutputFormat::Mp3,
            OutputFormat::Aac,
            OutputFormat::Opus,
        ]
    }

    /// Convert WAV to target format using 2-pass loudnorm normalization.
    ///
    /// Pass 1: analyse loudness statistics (no output file).
    /// Pass 2: encode with measured values for accurate -14 LUFS normalisation.
    /// `volume_pct`: 100 = -14 LUFS 基準（配信サービスで一般的）, 0–200% の範囲で調整可。
    pub fn encode_from_wav(self, wav_src: &Path, output_path: &Path, ffmpeg: &Path, volume_pct: u32) -> Result<()> {
        if matches!(self, Self::Wav) {
            std::fs::copy(wav_src, output_path)?;
            return Ok(());
        }

        let codec_args: &[&str] = match self {
            Self::Flac => &["-c:a", "flac", "-compression_level", "8"],
            Self::Mp3  => &["-c:a", "libmp3lame", "-q:a", "2"],
            Self::Aac  => &["-c:a", "aac", "-b:a", "256k"],
            Self::Opus => &["-c:a", "libopus", "-b:a", "192k"],
            Self::Wav  => unreachable!(),
        };

        // ---- Pass 1: measure loudness ----
        let pass1_filter = "loudnorm=I=-14:TP=-1:LRA=11:print_format=json";
        let mut pass1_cmd = Command::new(ffmpeg);
        pass1_cmd.args(["-y", "-i"])
            .arg(wav_src)
            .args(["-af", pass1_filter, "-f", "null", "-"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            pass1_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let pass1 = pass1_cmd.output()?;

        if !pass1.status.success() {
            let err = String::from_utf8_lossy(&pass1.stderr);
            anyhow::bail!("loudnorm pass 1 failed:\n{}", err);
        }

        // ffmpeg prints the loudnorm JSON to stderr
        let stderr = String::from_utf8_lossy(&pass1.stderr);
        log::debug!("loudnorm pass1 stderr:\n{}", stderr);
        let stats = parse_loudnorm_json(&stderr)?;

        // ---- Pass 2: encode with measured values ----
        // If pass1 returned -inf (e.g. silence), fall back to single-pass loudnorm
        let pass2_filter = if stats.input_i.contains("inf") || stats.input_tp.contains("inf") {
            log::warn!("loudnorm pass1 returned -inf, falling back to single-pass");
            "loudnorm=I=-14:TP=-1:LRA=11".to_owned()
        } else {
            format!(
                "loudnorm=I=-14:TP=-1:LRA=11\
                 :measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}\
                 :offset={}:linear=true",
                stats.input_i,
                stats.input_tp,
                stats.input_lra,
                stats.input_thresh,
                stats.target_offset,
            )
        };

        // 音量調整フィルタを追加（100% の場合は変化なし）
        let volume_gain = volume_pct as f32 / 100.0;
        let final_filter = format!("{},volume={:.4}", pass2_filter, volume_gain);

        let mut cmd = Command::new(ffmpeg);
        cmd.arg("-y").arg("-i").arg(wav_src).arg("-af").arg(&final_filter);
        for arg in codec_args {
            cmd.arg(arg);
        }
        cmd.arg(output_path);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr2 = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ffmpeg エラー:\n{}", stderr2);
        }
        Ok(())
    }
}

/// Measured loudness values returned by loudnorm pass 1.
struct LoudnormStats {
    input_i:      String,
    input_tp:     String,
    input_lra:    String,
    input_thresh: String,
    target_offset: String,
}

/// Extract the loudnorm JSON block from ffmpeg stderr and parse the fields we need.
fn parse_loudnorm_json(stderr: &str) -> Result<LoudnormStats> {
    // Find the JSON object that loudnorm prints
    let start = stderr
        .rfind('{')
        .ok_or_else(|| anyhow::anyhow!("loudnorm JSON not found in ffmpeg output"))?;
    let end = stderr[start..]
        .find('}')
        .ok_or_else(|| anyhow::anyhow!("loudnorm JSON not terminated"))?;
    let json = &stderr[start..=start + end];

    fn extract(json: &str, key: &str) -> Result<String> {
        // Match: "key" : "value"
        let needle = format!("\"{}\"", key);
        let pos = json
            .find(&needle)
            .ok_or_else(|| anyhow::anyhow!("loudnorm key '{}' not found", key))?;
        let after = &json[pos + needle.len()..];
        let val_start = after
            .find('"')
            .ok_or_else(|| anyhow::anyhow!("loudnorm value for '{}' not found", key))?;
        let inner = &after[val_start + 1..];
        let val_end = inner
            .find('"')
            .ok_or_else(|| anyhow::anyhow!("loudnorm value for '{}' not terminated", key))?;
        Ok(inner[..val_end].to_owned())
    }

    Ok(LoudnormStats {
        input_i:       extract(json, "input_i")?,
        input_tp:      extract(json, "input_tp")?,
        input_lra:     extract(json, "input_lra")?,
        input_thresh:  extract(json, "input_thresh")?,
        target_offset: extract(json, "target_offset")?,
    })
}
