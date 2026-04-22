fn main() {
    // Windows 以外では何もしない
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    // ICO ファイルを OUT_DIR に生成
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let ico_path = std::path::PathBuf::from(&out_dir).join("app_icon.ico");
    generate_ico(&ico_path);

    // winres でリソースとして EXE に埋め込む
    let mut res = winres::WindowsResource::new();
    res.set_icon(&ico_path.to_string_lossy());
    res.compile().expect("Failed to compile Windows resources");
}

/// main.rs の make_app_icon() と同じアルゴリズムで
/// 複数サイズの ICO を生成して保存する。
fn generate_ico(path: &std::path::Path) {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for &size in &[16u32, 32u32, 48u32, 64u32] {
        let rgba = render_icon(size);
        let img = ico::IconImage::from_rgba_data(size, size, rgba);
        let entry = ico::IconDirEntry::encode(&img).expect("ico encode failed");
        icon_dir.add_entry(entry);
    }

    let file = std::fs::File::create(path).expect("Cannot create ico file");
    icon_dir.write(file).expect("ico write failed");
}

/// size × size の RGBA ピクセル列を返す。
/// ネイビー円背景 + 赤い録音ドット + 白い音波アーク（左右対称）。
fn render_icon(size: u32) -> Vec<u8> {
    let w = size as usize;
    let h = size as usize;
    let mut rgba = vec![0u8; w * h * 4];
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let s = size as f32 / 64.0; // スケール係数（64px 基準）

    for y in 0..h {
        for x in 0..w {
            let px = x as f32 + 0.5 - cx;
            let py = y as f32 + 0.5 - cy;
            let r = (px * px + py * py).sqrt();
            let i = (y * w + x) * 4;

            // 外側円（ネイビー）
            let outer = 31.0 * s;
            if r < outer {
                let aa = (outer + 0.5 * s - r).clamp(0.0, 1.0);
                let a = (aa * 255.0) as u8;
                rgba[i] = 30;
                rgba[i + 1] = 58;
                rgba[i + 2] = 110;
                rgba[i + 3] = a;
            }

            // 内側ドット（赤）
            let inner = 14.0 * s;
            if r < inner {
                let aa = (inner + 0.5 * s - r).clamp(0.0, 1.0);
                let a = (aa * 255.0) as u8;
                rgba[i] = 220;
                rgba[i + 1] = 45;
                rgba[i + 2] = 45;
                rgba[i + 3] = a;
            }

            // 白い音波アーク（左右対称）
            let ang = py.atan2(px.abs());
            let half_span = std::f32::consts::PI * 0.38;

            if ang.abs() < half_span {
                // 内側アーク
                let r1 = 19.0 * s;
                let thick = 1.8 * s;
                let d1 = (r - r1).abs();
                if r > 15.0 * s && r < 24.0 * s && d1 < thick {
                    let edge = 1.0 - d1 / thick;
                    let a = (edge * 210.0) as u8;
                    let t = a as f32 / 255.0;
                    rgba[i] = (rgba[i] as f32 + (255.0 - rgba[i] as f32) * t) as u8;
                    rgba[i + 1] = (rgba[i + 1] as f32 + (255.0 - rgba[i + 1] as f32) * t) as u8;
                    rgba[i + 2] = (rgba[i + 2] as f32 + (255.0 - rgba[i + 2] as f32) * t) as u8;
                    rgba[i + 3] = rgba[i + 3].max(a);
                }

                // 外側アーク
                let r2 = 24.5 * s;
                let d2 = (r - r2).abs();
                if r > 20.5 * s && r < 29.0 * s && d2 < thick {
                    let edge = 1.0 - d2 / thick;
                    let a = (edge * 210.0) as u8;
                    let t = a as f32 / 255.0;
                    rgba[i] = (rgba[i] as f32 + (255.0 - rgba[i] as f32) * t) as u8;
                    rgba[i + 1] = (rgba[i + 1] as f32 + (255.0 - rgba[i + 1] as f32) * t) as u8;
                    rgba[i + 2] = (rgba[i + 2] as f32 + (255.0 - rgba[i + 2] as f32) * t) as u8;
                    rgba[i + 3] = rgba[i + 3].max(a);
                }
            }
        }
    }

    rgba
}
