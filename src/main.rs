#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod encoder;
mod ffmpeg_manager;
mod settings;

fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 620.0])
            .with_min_inner_size([440.0, 440.0])
            .with_title("SysVoice Recorder")
            .with_icon(make_app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "SysVoice Recorder",
        options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    )
}

/// アプリアイコンをプログラムで生成（64×64 RGBA）。
/// ネイビー円背景 + 赤い録音ドット + 白い音波アーク。
/// すべてオリジナルの幾何図形で著作権フリー。
fn make_app_icon() -> egui::IconData {
    const W: usize = 64;
    const H: usize = 64;
    let mut rgba = vec![0u8; W * H * 4];
    let cx = W as f32 / 2.0;
    let cy = H as f32 / 2.0;

    for y in 0..H {
        for x in 0..W {
            let px = x as f32 + 0.5 - cx;
            let py = y as f32 + 0.5 - cy;
            let r = (px * px + py * py).sqrt();
            let i = (y * W + x) * 4;

            // --- 外側の円（ネイビー背景） ---
            if r < 31.0 {
                let aa = (31.5 - r).clamp(0.0, 1.0);
                let a = (aa * 255.0) as u8;
                rgba[i]     = 30;
                rgba[i + 1] = 58;
                rgba[i + 2] = 110;
                rgba[i + 3] = a;
            }

            // --- 内側の円（赤い録音ドット） ---
            if r < 14.0 {
                let aa = (14.5 - r).clamp(0.0, 1.0);
                let a = (aa * 255.0) as u8;
                rgba[i]     = 220;
                rgba[i + 1] = 45;
                rgba[i + 2] = 45;
                rgba[i + 3] = a;
            }

            // --- 白い音波アーク（左右対称） ---
            let ang = py.atan2(px.abs()); // 左右対称にするため px の絶対値を使用
            let half_span = std::f32::consts::PI * 0.38; // ±68.4°

            if ang.abs() < half_span {
                // 内側アーク (r ≈ 19)
                let dist1 = (r - 19.0).abs();
                if r > 15.0 && r < 24.0 && dist1 < 1.8 {
                    let edge = 1.0 - dist1 / 1.8;
                    let a = (edge * 210.0) as u8;
                    let t = a as f32 / 255.0;
                    rgba[i]     = (rgba[i]     as f32 + (255.0 - rgba[i]     as f32) * t) as u8;
                    rgba[i + 1] = (rgba[i + 1] as f32 + (255.0 - rgba[i + 1] as f32) * t) as u8;
                    rgba[i + 2] = (rgba[i + 2] as f32 + (255.0 - rgba[i + 2] as f32) * t) as u8;
                    rgba[i + 3] = rgba[i + 3].max(a);
                }

                // 外側アーク (r ≈ 24.5)
                let dist2 = (r - 24.5).abs();
                if r > 20.5 && r < 29.0 && dist2 < 1.8 {
                    let edge = 1.0 - dist2 / 1.8;
                    let a = (edge * 210.0) as u8;
                    let t = a as f32 / 255.0;
                    rgba[i]     = (rgba[i]     as f32 + (255.0 - rgba[i]     as f32) * t) as u8;
                    rgba[i + 1] = (rgba[i + 1] as f32 + (255.0 - rgba[i + 1] as f32) * t) as u8;
                    rgba[i + 2] = (rgba[i + 2] as f32 + (255.0 - rgba[i + 2] as f32) * t) as u8;
                    rgba[i + 3] = rgba[i + 3].max(a);
                }
            }
        }
    }

    egui::IconData { rgba, width: W as u32, height: H as u32 }
}
