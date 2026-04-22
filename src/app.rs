use crate::audio::AudioCapture;
use crate::encoder::OutputFormat;
use crate::ffmpeg_manager::{self, DlMsg};
use crate::settings::{Lang, Settings};
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

enum WorkerMsg {
    Peak(f32),
    WavDone,
    WavError(String),
}

#[derive(PartialEq)]
enum Phase {
    Idle,
    Recording,
    Processing,
    Done,
    Error,
}

/// 言語に応じた文字列を返す。CJK フォント非利用時は常に en が返る。
#[inline]
fn t(lang: Lang, en: &'static str, ja: &'static str) -> &'static str {
    if lang == Lang::Ja {
        ja
    } else {
        en
    }
}

/// Windows システムフォントから CJK フォントの読み込みを試みる。
/// 成功した場合は egui コンテキストにフォントを設定して true を返す。
fn try_load_cjk_font(ctx: &egui::Context) -> bool {
    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Windows\Fonts\meiryo.ttc",
            r"C:\Windows\Fonts\YuGothM.ttc",
            r"C:\Windows\Fonts\msgothic.ttc",
            r"C:\Windows\Fonts\meiryob.ttc",
        ];
        for path in candidates {
            if let Ok(data) = std::fs::read(path) {
                let mut fonts = egui::FontDefinitions::default();
                fonts
                    .font_data
                    .insert("cjk".to_owned(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(1, "cjk".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("cjk".to_owned());
                ctx.set_fonts(fonts);
                log::info!("CJK font loaded: {path}");
                return true;
            }
        }
        log::warn!("No CJK font found; Japanese UI disabled.");
    }

    #[cfg(not(windows))]
    let _ = ctx;

    false
}

pub struct App {
    settings: Settings,
    phase: Phase,
    ffmpeg_path: Option<PathBuf>,
    capture: Option<AudioCapture>,
    stop_flag: Option<Arc<AtomicBool>>,
    peak_rx: Option<Receiver<WorkerMsg>>,
    fin_rx: Option<Receiver<String>>,
    temp_wav: Option<PathBuf>,
    record_start: Option<Instant>,
    result_rx: Option<Receiver<Result<PathBuf, String>>>,
    dl_rx: Option<Receiver<DlMsg>>,
    dl_failed: bool,
    dl_error: Option<String>,
    peak_level: f32,
    peak_hold: f32,
    last_output: Option<PathBuf>,
    last_error: Option<String>,
    status_msg: String,
    /// CJK フォントが正常に読み込まれた場合のみ true。
    cjk_available: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let cjk_available = try_load_cjk_font(&cc.egui_ctx);
        let mut settings = Settings::load();
        // CJK フォントが使えない場合は強制的に英語に戻す
        if !cjk_available {
            settings.lang = Lang::En;
        }
        let ffmpeg_path = settings.find_ffmpeg();

        // clean up any leftover temp WAVs from previous crashes
        if let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "wav").unwrap_or(false)
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("sysvoice_"))
                        .unwrap_or(false)
                {
                    let _ = std::fs::remove_file(p);
                }
            }
        }

        // Auto-download ffmpeg only on supported platforms (Windows).
        let (dl_rx, status_msg) = if ffmpeg_path.is_none() {
            if ffmpeg_manager::supports_auto_download() {
                let (tx, rx) = crossbeam_channel::bounded::<DlMsg>(8);
                ffmpeg_manager::start_download(tx);
                (Some(rx), String::from("Fetching ffmpeg..."))
            } else {
                (
                    None,
                    String::from(
                        "ffmpeg not found - install it manually to enable MP3/FLAC/AAC/Opus",
                    ),
                )
            }
        } else {
            (None, String::from("ffmpeg found - all formats available"))
        };

        Self {
            settings,
            phase: Phase::Idle,
            ffmpeg_path,
            capture: None,
            stop_flag: None,
            peak_rx: None,
            fin_rx: None,
            temp_wav: None,
            record_start: None,
            result_rx: None,
            dl_rx,
            dl_failed: false,
            dl_error: None,
            peak_level: 0.0,
            peak_hold: 0.0,
            last_output: None,
            last_error: None,
            status_msg,
            cjk_available,
        }
    }

    #[cfg(windows)]
    fn retry_download(&mut self) {
        if !ffmpeg_manager::supports_auto_download() {
            self.dl_failed = true;
            self.dl_error = Some(String::from(
                "Automatic ffmpeg download is unsupported on this OS.",
            ));
            self.status_msg = String::from("Install ffmpeg manually and restart the app.");
            return;
        }
        let (tx, rx) = crossbeam_channel::bounded::<DlMsg>(8);
        ffmpeg_manager::start_download(tx);
        self.dl_rx = Some(rx);
        self.dl_failed = false;
        self.dl_error = None;
        self.status_msg = String::from("Retrying ffmpeg download...");
    }

    fn start_recording(&mut self) {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let temp_wav = std::env::temp_dir().join(format!("sysvoice_{epoch}.wav"));
        let (samples_tx, samples_rx) = crossbeam_channel::bounded::<Vec<f32>>(512);
        let (worker_tx, worker_rx) = crossbeam_channel::bounded::<WorkerMsg>(256);
        let (fin_tx, fin_rx_inner) = crossbeam_channel::bounded::<String>(1);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag2 = Arc::clone(&stop_flag);
        let capture = match AudioCapture::start(samples_tx) {
            Ok(c) => c,
            Err(e) => {
                self.phase = Phase::Error;
                self.last_error = Some(format!("Failed to start device: {e}"));
                return;
            }
        };
        let sample_rate = capture.sample_rate;
        let channels = capture.channels;
        let wav_path = temp_wav.clone();
        std::thread::spawn(move || {
            let spec = hound::WavSpec {
                channels,
                sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let mut writer = match hound::WavWriter::create(&wav_path, spec) {
                Ok(w) => w,
                Err(e) => {
                    let _ = worker_tx.send(WorkerMsg::WavError(e.to_string()));
                    let _ = fin_tx.send(e.to_string());
                    return;
                }
            };
            let mut peak_buf = 0.0f32;
            let mut count = 0usize;
            const UPDATE_EVERY: usize = 2048;
            loop {
                if stop_flag2.load(Ordering::Relaxed) {
                    while let Ok(samples) = samples_rx.try_recv() {
                        for s in samples {
                            let _ = writer.write_sample(s);
                        }
                    }
                    break;
                }
                match samples_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(samples) => {
                        for &s in &samples {
                            let _ = writer.write_sample(s);
                            if s.abs() > peak_buf {
                                peak_buf = s.abs();
                            }
                            count += 1;
                        }
                        if count >= UPDATE_EVERY {
                            let _ = worker_tx.try_send(WorkerMsg::Peak(peak_buf));
                            peak_buf = 0.0;
                            count = 0;
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                }
            }
            match writer.finalize() {
                Ok(_) => {
                    let _ = worker_tx.send(WorkerMsg::WavDone);
                    let _ = fin_tx.send(String::new());
                }
                Err(e) => {
                    let m = e.to_string();
                    let _ = worker_tx.send(WorkerMsg::WavError(m.clone()));
                    let _ = fin_tx.send(m);
                }
            }
        });
        self.capture = Some(capture);
        self.stop_flag = Some(stop_flag);
        self.peak_rx = Some(worker_rx);
        self.fin_rx = Some(fin_rx_inner);
        self.temp_wav = Some(temp_wav);
        self.record_start = Some(Instant::now());
        self.phase = Phase::Recording;
        self.last_error = None;
        self.last_output = None;
        self.status_msg = String::from("Recording...");
    }

    fn stop_recording(&mut self) {
        if let Some(flag) = self.stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        self.capture = None;
        self.peak_rx = None;
        self.record_start = None;
        let temp_wav = match self.temp_wav.take() {
            Some(p) => p,
            None => return,
        };
        let fin_rx = match self.fin_rx.take() {
            Some(r) => r,
            None => return,
        };
        let format = self.settings.output_format;
        let save_dir = self.settings.save_directory.clone();
        let ffmpeg = self.ffmpeg_path.clone();
        let volume_pct = self.settings.volume_pct;
        let (res_tx, res_rx) = crossbeam_channel::bounded::<Result<PathBuf, String>>(1);
        self.result_rx = Some(res_rx);
        self.phase = Phase::Processing;
        self.status_msg = String::from("Converting & saving...");
        std::thread::spawn(move || {
            let status = match fin_rx.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(s) => s,
                Err(_) => {
                    let _ = res_tx.send(Err(String::from("WAV write timeout")));
                    return;
                }
            };
            if !status.is_empty() {
                let _ = res_tx.send(Err(format!("WAV error: {status}")));
                return;
            }
            let epoch2 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let filename = format!("recording_{epoch2}.{}", format.extension());
            let output_path = save_dir.join(&filename);
            let result = if format.requires_ffmpeg() {
                match ffmpeg {
                    Some(ff) => format
                        .encode_from_wav(&temp_wav, &output_path, &ff, volume_pct)
                        .map(|_| output_path.clone())
                        .map_err(|e| e.to_string()),
                    None => Err(String::from(
                        "ffmpeg is downloading. Please wait and try again.",
                    )),
                }
            } else {
                std::fs::copy(&temp_wav, &output_path)
                    .map(|_| output_path.clone())
                    .map_err(|e| e.to_string())
            };
            let _ = std::fs::remove_file(&temp_wav);
            let _ = res_tx.send(result);
        });
    }

    fn poll_state(&mut self) {
        let dl_msgs: Vec<DlMsg> = if let Some(rx) = &self.dl_rx {
            let mut v = Vec::new();
            while let Ok(m) = rx.try_recv() {
                v.push(m);
            }
            v
        } else {
            vec![]
        };
        for msg in dl_msgs {
            match msg {
                DlMsg::Downloading => {
                    self.status_msg = String::from("Downloading ffmpeg...");
                }
                DlMsg::Extracting => {
                    self.status_msg = String::from("Extracting ffmpeg...");
                }
                DlMsg::Done(path) => {
                    self.ffmpeg_path = Some(path);
                    self.dl_rx = None;
                    self.status_msg = String::from("All formats available");
                    self.settings.save();
                }
                DlMsg::Failed(e) => {
                    self.dl_rx = None;
                    self.dl_failed = true;
                    self.dl_error = Some(e.clone());
                    self.status_msg = format!("ffmpeg download failed: {e}");
                }
            }
        }
        if let Some(rx) = &self.peak_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WorkerMsg::Peak(p) => {
                        if p > self.peak_level {
                            self.peak_level = p;
                        }
                    }
                    WorkerMsg::WavError(e) => {
                        self.last_error = Some(e);
                        self.phase = Phase::Error;
                    }
                    WorkerMsg::WavDone => {}
                }
            }
        }
        if let Some(rx) = &self.result_rx {
            if let Ok(result) = rx.try_recv() {
                self.result_rx = None;
                match result {
                    Ok(path) => {
                        self.last_output = Some(path);
                        self.phase = Phase::Done;
                        self.status_msg = String::from("Saved!");
                        self.settings.save();
                    }
                    Err(e) => {
                        self.last_error = Some(e);
                        self.phase = Phase::Error;
                        self.status_msg = String::from("Error");
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_state();
        if self.peak_level > self.peak_hold {
            self.peak_hold = self.peak_level;
        } else {
            self.peak_hold *= 0.97;
        }
        self.peak_level *= 0.75;
        let downloading = self.dl_rx.is_some();
        if matches!(self.phase, Phase::Recording | Phase::Processing) || downloading {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            // --- 言語トグル（CJK フォント利用時のみ表示） ---
            if self.cjk_available {
                let idle = matches!(self.phase, Phase::Idle | Phase::Done | Phase::Error);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled_ui(idle, |ui| {
                            let ja_active = self.settings.lang == Lang::Ja;
                            let en_btn = egui::Button::new("EN")
                                .fill(if !ja_active { egui::Color32::from_rgb(60, 120, 200) }
                                      else { egui::Color32::from_gray(55) });
                            let ja_btn = egui::Button::new("JA")
                                .fill(if ja_active  { egui::Color32::from_rgb(60, 120, 200) }
                                      else { egui::Color32::from_gray(55) });
                            if ui.add(ja_btn).clicked() {
                                self.settings.lang = Lang::Ja;
                                self.settings.save();
                            }
                            if ui.add(en_btn).clicked() {
                                self.settings.lang = Lang::En;
                                self.settings.save();
                            }
                        });
                    });
                });
            }
            let lang = self.settings.lang;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t(lang, "Format:", "形式:")).strong());
                let idle = matches!(self.phase, Phase::Idle | Phase::Done | Phase::Error);
                for &fmt in OutputFormat::all() {
                    let av  = !fmt.requires_ffmpeg() || self.ffmpeg_path.is_some();
                    let btn = egui::RadioButton::new(self.settings.output_format == fmt, fmt.display_name());
                    if ui.add_enabled(av && idle, btn).clicked() {
                        self.settings.output_format = fmt;
                    }
                }
            });

            // -- 音量スライダー（圧縮形式のみ有効、WAV時はグレーアウト） --
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t(lang, "Volume:", "音量:")).strong());
                let fmt_compressed = self.settings.output_format.requires_ffmpeg();
                let idle = matches!(self.phase, Phase::Idle | Phase::Done | Phase::Error);
                ui.add_enabled_ui(fmt_compressed && idle, |ui| {
                    let mut vol = self.settings.volume_pct as i32;
                    let slider = egui::Slider::new(&mut vol, 0..=200)
                        .text("%")
                        .clamp_to_range(true);
                    if ui.add(slider).changed() {
                        self.settings.volume_pct = vol as u32;
                    }
                    if ui.small_button(t(lang, "Reset", "リセット"))
                        .on_hover_text(t(lang, "Reset volume to 100%", "音量をデフォルト(100%)に戻す"))
                        .clicked() {
                        self.settings.volume_pct = 100;
                    }
                });
                if !fmt_compressed {
                    ui.label(
                        egui::RichText::new(t(lang,
                            "▶ Active for MP3/FLAC/AAC/Opus",
                            "▶ MP3・FLAC・AAC・Opus選択時に有効"))
                            .color(egui::Color32::GRAY)
                            .small()
                    );
                }
            });
            ui.label(
                egui::RichText::new(t(lang,
                    "  100% volume corresponds to -9 LUFS",
                    "  100% \u{306e}\u{97f3}\u{91cf}\u{306f} -9 LUFS \u{3067}\u{3059}"))
                    .color(egui::Color32::GRAY)
                    .small()
            );
            if self.settings.volume_pct > 150 {
                ui.label(
                    egui::RichText::new(t(lang,
                        "  \u{26a0} High volume (>150%) may cause clipping or distortion.",
                        "  \u{26a0} \u{9ad8}\u{97f3}\u{91cf}\u{8a2d}\u{5b9a} (150%\u{8d85}) \u{3067}\u{306f}\u{3001}\u{97f3}\u{5272}\u{308c}\u{30fb}\u{6b6a}\u{307f}\u{30fb}\u{30af}\u{30ea}\u{30c3}\u{30d7}\u{304c}\u{751f}\u{3058}\u{308b}\u{53ef}\u{80fd}\u{6027}\u{304c}\u{3042}\u{308a}\u{307e}\u{3059}\u{3002}"))
                        .color(egui::Color32::from_rgb(220, 150, 40))
                        .small()
                );
            }

            // -- ffmpeg help panel --
            if self.ffmpeg_path.is_none() {
                ui.add_space(4.0);
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new("MP3 / AAC / FLAC / Opus require ffmpeg")
                        .color(egui::Color32::from_rgb(255, 200, 50)).strong());
                    if self.dl_failed {
                        if let Some(err) = &self.dl_error {
                            ui.label(egui::RichText::new(
                                format!("Auto-download failed: {err}"))
                                .color(egui::Color32::from_rgb(220, 80, 80)).small());
                        }
                    }
                    #[cfg(windows)]
                    ui.horizontal(|ui| {
                        if self.dl_rx.is_some() {
                            ui.spinner();
                            ui.label(t(lang, "Downloading...", "ダウンロード中..."));
                        } else if ui.button(t(lang, "Retry Auto-Download", "再試行")).clicked() {
                            self.retry_download();
                        }
                    });
                    ui.separator();
                    ui.label(egui::RichText::new(t(lang, "Manual setup:", "手動セットアップ:")).strong());
                    #[cfg(windows)]
                    {
                    ui.label(t(lang,
                        "1. Download ffmpeg from:",
                        "1. 下記から ffmpeg をダウンロード:"));
                    ui.hyperlink_to(
                        "  -> https://www.gyan.dev/ffmpeg/builds/#release-builds",
                        "https://www.gyan.dev/ffmpeg/builds/#release-builds",
                    );
                    ui.label(t(lang,
                        "   Select \"ffmpeg-release-essentials.zip\"",
                        "   \u{300c}ffmpeg-release-essentials.zip\u{300d}\u{3092}\u{9078}\u{629e}"));
                    ui.label(t(lang,
                        "2. Extract the ZIP and place ffmpeg.exe:",
                        "2. ZIP を解凍し、中の ffmpeg.exe を:"));
                    ui.label(t(lang,
                        "   [Recommended] Same folder as sysvoice_rec.exe",
                        "   【推奨】sysvoice_rec.exe と同じフォルダに配置"));
                    ui.label(t(lang,
                        "   [Alternative] Or place it here:",
                        "   【代替】または以下のフォルダに配置:"));
                    let cache = ffmpeg_manager::cached_ffmpeg_path();
                    ui.label(
                        egui::RichText::new(format!("   {}", cache.display()))
                            .monospace().small()
                    );
                    ui.label(t(lang, "3. Restart the app.", "3. アプリを再起動してください。"));
                    }
                    #[cfg(not(windows))]
                    {
                        ui.label(t(lang,
                            "1. Install ffmpeg via your package manager:",
                            "1. パッケージマネージャで ffmpeg をインストール:"));
                        ui.label(egui::RichText::new("   Debian/Ubuntu: sudo apt install ffmpeg").monospace().small());
                        ui.label(egui::RichText::new("   Fedora:        sudo dnf install ffmpeg").monospace().small());
                        ui.label(egui::RichText::new("   Arch:          sudo pacman -S ffmpeg").monospace().small());
                        ui.label(t(lang,
                            "2. Or place the ffmpeg binary here:",
                            "2. または ffmpeg バイナリを以下に配置:"));
                        let cache = ffmpeg_manager::cached_ffmpeg_path();
                        ui.label(
                            egui::RichText::new(format!("   {}", cache.display()))
                                .monospace().small()
                        );
                        ui.label(t(lang,
                            "3. Restart the app after installation.",
                            "3. インストール後にアプリを再起動してください。"));
                    }
                });
            }

            ui.add_space(4.0); ui.separator(); ui.add_space(4.0);
            {
                let bw = ui.available_width() - 4.0;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(bw, 22.0), egui::Sense::hover());
                if ui.is_rect_visible(rect) {
                    let p = ui.painter();
                    p.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 20));
                    let fill = self.peak_hold.min(1.0);
                    let bc = if fill > 0.9 { egui::Color32::from_rgb(220, 50, 50) }
                        else if fill > 0.7 { egui::Color32::from_rgb(220, 180, 40) }
                        else { egui::Color32::from_rgb(55, 190, 80) };
                    if fill > 0.0 {
                        p.rect_filled(
                            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fill, rect.height())),
                            4.0, bc);
                    }
                    let db = if self.peak_hold > 1e-7 { 20.0_f32 * self.peak_hold.log10() }
                             else { f32::NEG_INFINITY };
                    let dt = if db.is_finite() { format!("{db:+.1} dBFS") }
                             else { String::from("-inf dBFS") };
                    p.text(rect.center(), egui::Align2::CENTER_CENTER, &dt,
                        egui::FontId::monospace(11.0), egui::Color32::WHITE);
                }
            }
            ui.add_space(10.0);
            let elapsed = self.record_start.map(|s| s.elapsed()).unwrap_or_default();
            let hh = elapsed.as_secs() / 3600;
            let mm = (elapsed.as_secs() % 3600) / 60;
            let ss = elapsed.as_secs() % 60;
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(format!("{hh:02}:{mm:02}:{ss:02}")).size(36.0).monospace());
            });
            ui.add_space(8.0);
            let (lbl, col): (&str, egui::Color32) = match self.phase {
                Phase::Idle | Phase::Done | Phase::Error => ("REC", egui::Color32::from_rgb(180, 45, 45)),
                Phase::Recording  => ("STOP", egui::Color32::from_rgb(50, 90, 180)),
                Phase::Processing => ("Converting...", egui::Color32::DARK_GRAY),
            };
            let en = !matches!(self.phase, Phase::Processing);
            ui.vertical_centered(|ui| {
                let btn = egui::Button::new(egui::RichText::new(lbl).size(22.0).color(egui::Color32::WHITE))
                    .fill(col).min_size(egui::vec2(240.0, 56.0));
                if ui.add_enabled(en, btn).clicked() {
                    match self.phase {
                        Phase::Idle | Phase::Done | Phase::Error => self.start_recording(),
                        Phase::Recording  => self.stop_recording(),
                        Phase::Processing => {}
                    }
                }
            });
            ui.add_space(10.0); ui.separator(); ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t(lang, "Save to:", "保存先:")).strong());
                let mut ds = self.settings.save_directory.to_string_lossy().to_string();
                ui.add(egui::TextEdit::singleline(&mut ds).desired_width(ui.available_width() - 90.0).interactive(false));
                let idle = matches!(self.phase, Phase::Idle | Phase::Done | Phase::Error);
                if ui.add_enabled(idle, egui::Button::new(t(lang, "Browse", "参照"))).clicked() {
                    if let Some(path) = rfd::FileDialog::new().set_directory(&self.settings.save_directory).pick_folder() {
                        self.settings.save_directory = path;
                        self.settings.save();
                    }
                }
            });
            if let Some(out) = &self.last_output {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(t(lang, "Saved:", "保存済:")).strong());
                    ui.monospace(out.to_string_lossy().as_ref());
                });
            }
            if matches!(self.phase, Phase::Error) {
                if let Some(err) = &self.last_error {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), format!("Error: {err}"));
                }
            }
            ui.separator();
            let sc = if downloading {
                egui::Color32::from_rgb(100, 160, 220)
            } else {
                match &self.phase {
                    Phase::Error => egui::Color32::from_rgb(220, 80, 80),
                    Phase::Done  => egui::Color32::from_rgb(80, 200, 80),
                    _            => if self.ffmpeg_path.is_some() { egui::Color32::from_rgb(100, 200, 100) }
                                    else { egui::Color32::from_rgb(220, 180, 40) },
                }
            };
            ui.colored_label(sc, &self.status_msg);
        });
    }
}
