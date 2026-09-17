//! Optional Blender conversion is used only by explicit PNG export, never Explorer previews.
use crate::batch::Observer;
use anyhow::{bail, ensure, Context, Result};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
            Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject},
        },
    },
};

struct Slot(HANDLE, bool);
const CONVERSION_SECONDS: u64 = 120;

fn memory_budget(total: u64, available: u64) -> usize {
    (total / 4)
        .min(available / 2)
        .clamp(512 * 1024 * 1024, 8 * 1024 * 1024 * 1024) as usize
}

pub(super) fn memory_limit() -> usize {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut status) }.is_ok() {
        memory_budget(status.ullTotalPhys, status.ullAvailPhys)
    } else {
        2 * 1024 * 1024 * 1024
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        unsafe {
            if self.1 {
                let _ = ReleaseMutex(self.0);
            }
            let _ = CloseHandle(self.0);
        }
    }
}

fn version(path: &Path) -> Vec<u32> {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn discover() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("MESHTHUMBS_BLENDER") {
        let path = PathBuf::from(path);
        ensure!(
            path.is_absolute() && path.is_file(),
            "MESHTHUMBS_BLENDER must name an existing absolute Blender executable"
        );
        return Ok(path);
    }
    let mut candidates = Vec::new();
    for root in [
        std::env::var_os("ProgramW6432"),
        std::env::var_os("ProgramFiles"),
    ]
    .into_iter()
    .flatten()
    {
        candidates.push(PathBuf::from(root).join("Blender Foundation"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Programs/Blender Foundation"));
    }
    let mut installs = Vec::new();
    for root in candidates {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let folder = entry.path();
                if folder.join("blender.exe").is_file() {
                    installs.push(folder);
                }
            }
        }
    }
    installs.sort_by_key(|p| version(p));
    if let Some(folder) = installs.pop() {
        return Ok(folder.join("blender.exe"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
            let exe = dir.join("blender.exe");
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    bail!("Blender was not found")
}

/// None means cancellation. Errors trigger the stored-preview fallback in batch.
pub(super) fn convert(
    input: &Path,
    work: &Path,
    observer: &mut dyn Observer,
) -> Result<Option<PathBuf>> {
    let executable = discover()?;
    let mut slot = Slot(
        unsafe { CreateMutexW(None, false, w!("Local\\MeshThumbs.BlenderExport"))? },
        false,
    );
    let wait_until = Instant::now() + Duration::from_secs(CONVERSION_SECONDS);
    loop {
        if observer.cancelled() {
            return Ok(None);
        }
        match unsafe { WaitForSingleObject(slot.0, 20) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => {
                slot.1 = true;
                break;
            }
            WAIT_TIMEOUT => ensure!(Instant::now() < wait_until, "Blender export is busy"),
            _ => return Err(std::io::Error::last_os_error().into()),
        }
    }
    let script = work.join("export_blend.py");
    let output = work.join("scene.glb");
    std::fs::write(&script, include_str!("export_blend.py"))?;
    let args: Vec<OsString> = vec![
        "--background".into(),
        "--factory-startup".into(),
        "--disable-autoexec".into(),
        "--threads".into(),
        "2".into(),
        "--python-exit-code".into(),
        "1".into(),
        "--python".into(),
        script.into_os_string(),
        "--".into(),
        input.into(),
        output.clone().into_os_string(),
    ];
    let memory = memory_limit();
    let mut child = renderer::process::Process::spawn(&executable, &args, work, memory)
        .context("Could not start Blender")?;
    let deadline = Instant::now() + Duration::from_secs(CONVERSION_SECONDS);
    loop {
        if observer.cancelled() {
            return Ok(None);
        }
        if let Some(code) = child.poll()? {
            if code != 0 {
                let mut detail = String::new();
                if let Ok(file) = std::fs::File::open(output.with_extension("error.txt")) {
                    let _ = file.take(8192).read_to_string(&mut detail);
                }
                if !detail.trim().is_empty() {
                    bail!("Blender: {}", detail.trim());
                }
                if let Ok(file) = std::fs::File::open(output.with_extension("phase.txt")) {
                    let _ = file.take(256).read_to_string(&mut detail);
                }
                bail!(
                    "Blender stopped while {} (exit {code}; memory budget {} MiB)",
                    if detail.is_empty() {
                        "converting the scene"
                    } else {
                        detail.trim()
                    },
                    memory / (1024 * 1024)
                );
            }
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "Blender conversion exceeded {CONVERSION_SECONDS} seconds"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    ensure!(
        std::fs::metadata(&output)
            .context("Blender produced no geometry")?
            .len()
            <= renderer::MAX_BLEND_EXPORT_BYTES,
        "Converted scene exceeds 1 GiB"
    );
    Ok(Some(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn install_versions_sort_numerically() {
        assert!(version(Path::new("Blender 4.10")) > version(Path::new("Blender 4.9")));
        assert!(version(Path::new("Blender 5.0")) > version(Path::new("Blender 4.10")));
    }
    #[test]
    fn memory_budget_reserves_capacity_for_other_apps() {
        let gib = 1024 * 1024 * 1024;
        assert_eq!(memory_budget(64 * gib, 32 * gib), 8 * gib as usize);
        assert_eq!(memory_budget(16 * gib, 4 * gib), 2 * gib as usize);
        assert_eq!(memory_budget(8 * gib, gib), gib as usize / 2);
    }
}
