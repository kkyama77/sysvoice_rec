use crossbeam_channel::Sender;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DlMsg {
    Downloading,
    Extracting,
    Done(PathBuf),
    Failed(String),
}

pub fn cached_ffmpeg_path() -> PathBuf {
    #[cfg(windows)]
    let exe_name = "ffmpeg.exe";
    #[cfg(not(windows))]
    let exe_name = "ffmpeg";

    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sysvoice_rec")
        .join("ffmpeg")
        .join(exe_name)
}

pub fn is_cached() -> bool {
    cached_ffmpeg_path().exists()
}

pub fn start_download(tx: Sender<DlMsg>) {
    std::thread::spawn(move || {
        match download_inner(&tx) {
            Ok(path) => { let _ = tx.send(DlMsg::Done(path)); }
            Err(e)   => { let _ = tx.send(DlMsg::Failed(e.to_string())); }
        }
    });
}

fn download_inner(tx: &Sender<DlMsg>) -> anyhow::Result<PathBuf> {
    #[cfg(not(windows))]
    anyhow::bail!("Automatic ffmpeg download is only supported on Windows. Please install ffmpeg manually.");

    #[cfg(windows)]
    {
    let cache_ffmpeg = cached_ffmpeg_path();
    if cache_ffmpeg.exists() { return Ok(cache_ffmpeg); }

    let cache_dir   = cache_ffmpeg.parent().unwrap().to_owned();
    std::fs::create_dir_all(&cache_dir)?;

    let zip_path    = cache_dir.join("ffmpeg_dl.zip");
    let extract_dir = cache_dir.join("extracted");

    let _ = tx.send(DlMsg::Downloading);

    let url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

    let dl_cmd = format!(
        "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
        url,
        zip_path.display()
    );
    let st = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &dl_cmd])
        .status()?;
    if !st.success() { anyhow::bail!("download failed"); }

    let _ = tx.send(DlMsg::Extracting);
    std::fs::create_dir_all(&extract_dir)?;

    let ex_cmd = format!(
        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
        zip_path.display(),
        extract_dir.display()
    );
    let st = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ex_cmd])
        .status()?;
    if !st.success() { anyhow::bail!("extraction failed"); }

    // gyan.dev zip path: ffmpeg-N.N-essentials_build/bin/ffmpeg.exe
    let candidate = extract_dir
        .join("ffmpeg-release-essentials_build")
        .join("bin")
        .join("ffmpeg.exe");

    let src = if candidate.exists() {
        candidate
    } else {
        find_file_recursive(&extract_dir, "ffmpeg.exe")?
    };

    std::fs::copy(&src, &cache_ffmpeg)?;
    let _ = std::fs::remove_file(&zip_path);
    let _ = std::fs::remove_dir_all(&extract_dir);

    Ok(cache_ffmpeg)
    } // end #[cfg(windows)]
}

fn find_file_recursive(dir: &std::path::Path, name: &str) -> anyhow::Result<PathBuf> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path  = entry.path();
            if path.is_dir() {
                if let Ok(found) = find_file_recursive(&path, name) { return Ok(found); }
            } else if path.file_name().map(|n| n == name).unwrap_or(false) {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("ffmpeg.exe not found")
}
