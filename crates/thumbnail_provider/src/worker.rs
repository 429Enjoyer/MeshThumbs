//! Native parsers run outside Explorer. A crashing or stuck parser cannot take
//! down the host; each request gets a five-second deadline and owned temp files.
use std::{
    ffi::OsString,
    os::windows::{ffi::OsStringExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use renderer::RgbaBitmap;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{
            GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        },
    },
};

pub(super) fn render(input: &Path, size: u32) -> anyhow::Result<RgbaBitmap> {
    let executable = worker_path()?;
    // The COM host owns converter intermediates too, so a killed worker cannot
    // strand its IFC mesh. The converter's job kills descendants on worker exit.
    let workspace = tempfile::Builder::new()
        .prefix("meshthumbs-result-")
        .tempdir()?;
    let output = workspace.path().join("result.rgba");
    let child = Command::new(&executable)
        .arg(input)
        .arg(&output)
        .arg(size.to_string())
        .arg("--raw-rgba")
        .env("MESHTHUMBS_WORK_DIR", workspace.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .context("could not start thumbnail worker")?;
    let mut worker = Worker(child, workspace);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = worker.0.try_wait()? {
            if !status.success() {
                bail!("thumbnail worker failed: {status}");
            }
            break;
        }
        if Instant::now() >= deadline {
            bail!("thumbnail worker exceeded five-second deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let expected = (size * size * 4) as u64;
    if std::fs::metadata(&output)?.len() != expected {
        bail!("invalid worker bitmap length");
    }
    let pixels = std::fs::read(&output)?;
    Ok(RgbaBitmap {
        width: size,
        height: size,
        pixels,
    })
}

struct Worker(Child, tempfile::TempDir);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
        // Job termination is asynchronous; briefly retry while the converter
        // releases its output handles. This directory was created by this call.
        for _ in 0..20 {
            if std::fs::remove_dir_all(self.1.path()).is_ok() || !self.1.path().exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

pub(super) fn worker_path() -> anyhow::Result<PathBuf> {
    let mut module = HMODULE::default();
    let mut path = vec![0u16; 32768];
    let length = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(super::DllGetClassObject as *const () as *const u16),
            &mut module,
        )?;
        GetModuleFileNameW(module, &mut path) as usize
    };
    if length == 0 || length >= path.len() {
        bail!("cannot locate thumbnail provider DLL");
    }
    let provider = PathBuf::from(OsString::from_wide(&path[..length]));
    Ok(provider.with_file_name("thumbgen.exe"))
}
