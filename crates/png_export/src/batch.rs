use anyhow::{ensure, Context, Result};
use std::{
    collections::HashSet,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub trait Observer {
    fn begin(&mut self, index: usize, total: usize, path: &Path);
    fn cancelled(&mut self) -> bool;
}
#[derive(Default, Debug)]
pub struct Report {
    pub total: usize,
    pub written: usize,
    pub failures: Vec<(PathBuf, String)>,
    pub collisions: usize,
    pub cancelled: bool,
}

pub fn run(files: Vec<PathBuf>, size: u32, observer: &mut dyn Observer) -> Result<Report> {
    ensure!(
        super::SIZES.contains(&size),
        "Choose 256, 512, or 1024 pixels"
    );
    let files = super::selection(files)?;
    let executable = std::env::current_exe()?.with_file_name("thumbgen.exe");
    ensure!(
        executable.is_file(),
        "thumbgen.exe is missing beside the exporter"
    );
    let mut report = Report {
        total: files.len(),
        ..Default::default()
    };
    let mut destinations = HashSet::new();
    for (index, input) in files.iter().enumerate() {
        if observer.cancelled() {
            report.cancelled = true;
            break;
        }
        let destination = input.with_extension("png");
        if !destinations.insert(destination.to_string_lossy().to_lowercase()) {
            report.collisions += 1;
        }
        observer.begin(index, files.len(), input);
        match one(&executable, input, &destination, size, observer) {
            Ok(true) => report.written += 1,
            Ok(false) => {
                report.cancelled = true;
                break;
            }
            Err(e) => report.failures.push((input.clone(), format!("{e:#}"))),
        }
    }
    Ok(report)
}

struct Workspace(tempfile::TempDir);
impl Drop for Workspace {
    fn drop(&mut self) {
        for _ in 0..20 {
            if std::fs::remove_dir_all(self.0.path()).is_ok() || !self.0.path().exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn one(
    executable: &Path,
    input: &Path,
    destination: &Path,
    size: u32,
    observer: &mut dyn Observer,
) -> Result<bool> {
    ensure!(input.is_file(), "The selected file no longer exists");
    let workspace = Workspace(
        tempfile::Builder::new()
            .prefix("meshthumbs-png-")
            .tempdir()?,
    );
    let raw = workspace.0.path().join("result.rgba");
    let args: Vec<OsString> = vec![
        input.into(),
        raw.as_os_str().into(),
        size.to_string().into(),
        "--raw-rgba".into(),
        "--work-dir".into(),
        workspace.0.path().as_os_str().into(),
    ];
    let mut worker = renderer::process::Process::spawn(
        executable,
        &args,
        workspace.0.path(),
        2 * 1024 * 1024 * 1024,
    )?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if observer.cancelled() {
            return Ok(false);
        }
        if let Some(code) = worker.poll()? {
            if code != 0 {
                let mut detail = String::new();
                if let Ok(file) = std::fs::File::open(raw.with_extension("error.txt")) {
                    let _ = file.take(8192).read_to_string(&mut detail);
                }
                anyhow::bail!(
                    "{}",
                    if detail.is_empty() {
                        format!("Renderer exited with code {code}")
                    } else {
                        detail
                    }
                );
            }
            break;
        }
        ensure!(Instant::now() < deadline, "Rendering exceeded 30 seconds");
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(worker);
    let expected = size as usize * size as usize * 4;
    ensure!(
        std::fs::metadata(&raw)?.len() == expected as u64,
        "Invalid rendered bitmap size"
    );
    let pixels = std::fs::read(&raw)?;
    publish(destination, &pixels, size, observer)
}

fn publish(
    destination: &Path,
    pixels: &[u8],
    size: u32,
    observer: &mut dyn Observer,
) -> Result<bool> {
    ensure!(
        pixels.len() == size as usize * size as usize * 4,
        "Invalid rendered pixel buffer"
    );
    let folder = destination
        .parent()
        .context("Output has no parent folder")?;
    let mut png = tempfile::Builder::new()
        .prefix(".meshthumbs-")
        .suffix(".tmp")
        .tempfile_in(folder)
        .context("Cannot write PNG in the model folder")?;
    image::write_buffer_with_format(
        png.as_file_mut(),
        pixels,
        size,
        size,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;
    png.as_file().sync_all()?;
    if observer.cancelled() {
        return Ok(false);
    }
    // Same-directory atomic replacement: a failed render/cancel never truncates
    // the previous PNG. This replaces the directory entry, not a symlink target.
    png.persist(destination)
        .map_err(|e| e.error)
        .context("Cannot replace the destination PNG")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Observe(bool);
    impl Observer for Observe {
        fn begin(&mut self, _: usize, _: usize, _: &Path) {}
        fn cancelled(&mut self) -> bool {
            self.0
        }
    }
    #[test]
    fn png_replacement_is_complete_and_cancel_or_bad_data_preserves_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("model.png");
        std::fs::write(&out, b"previous").unwrap();
        assert!(publish(&out, &[0; 3], 1, &mut Observe(false)).is_err());
        assert_eq!(std::fs::read(&out).unwrap(), b"previous");
        assert!(!publish(&out, &[10, 20, 30, 255], 1, &mut Observe(true)).unwrap());
        assert_eq!(std::fs::read(&out).unwrap(), b"previous");
        assert!(publish(&out, &[10, 20, 30, 255], 1, &mut Observe(false)).unwrap());
        let image = image::open(&out).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (1, 1));
        assert_eq!(image.as_raw(), &[10, 20, 30, 255]);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
