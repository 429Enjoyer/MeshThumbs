#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui {
    use anyhow::{ensure, Result};
    use png_export::batch::{Observer, Report};
    use std::{os::windows::ffi::OsStrExt, path::Path};
    use windows::{
        core::{w, GUID, PCWSTR},
        Win32::{
            Foundation::HWND,
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            UI::{
                Shell::IProgressDialog,
                WindowsAndMessaging::{
                    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK,
                },
            },
        },
    };
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    struct Progress {
        dialog: IProgressDialog,
        size: u32,
    }
    impl Drop for Progress {
        fn drop(&mut self) {
            unsafe {
                let _ = self.dialog.StopProgressDialog();
            }
        }
    }
    impl Observer for Progress {
        fn begin(&mut self, index: usize, total: usize, path: &Path) {
            let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let status = wide(&format!(
                "{} of {} — {} × {} pixels",
                index + 1,
                total,
                self.size,
                self.size
            ));
            unsafe {
                let _ = self.dialog.SetLine(1, PCWSTR(path.as_ptr()), true, None);
                let _ = self.dialog.SetLine(2, PCWSTR(status.as_ptr()), false, None);
                let _ = self.dialog.SetProgress64(index as u64, total as u64);
            }
        }
        fn cancelled(&mut self) -> bool {
            unsafe { self.dialog.HasUserCancelled().as_bool() }
        }
    }
    fn summary(report: Report, size: u32) {
        let mut text = format!(
            "{}Generated {} of {} PNG thumbnails at {} × {} pixels.",
            if report.cancelled {
                "Cancelled.\n\n"
            } else {
                ""
            },
            report.written,
            report.total,
            size,
            size
        );
        if report.collisions > 0 {
            text += "\nSome models share an output name; the last successful export is kept.";
        }
        if !report.failures.is_empty() {
            text += &format!("\n\n{} file(s) failed:", report.failures.len());
            for (path, error) in report.failures.iter().take(8) {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                text += &format!(
                    "\n\n{}\n{}",
                    name,
                    error.chars().take(220).collect::<String>()
                );
            }
            if report.failures.len() > 8 {
                text += "\n\nAdditional failures omitted.";
            }
        }
        let text = wide(&text);
        unsafe {
            MessageBoxW(
                HWND::default(),
                PCWSTR(text.as_ptr()),
                w!("MeshThumbs"),
                MB_OK
                    | if report.failures.is_empty() {
                        MB_ICONINFORMATION
                    } else {
                        MB_ICONWARNING
                    },
            );
        }
    }
    pub fn run() -> Result<()> {
        let args = std::env::args_os().skip(1).collect::<Vec<_>>();
        ensure!(
            args.len() == 2 && args[0] == "--job",
            "Use the MeshThumbs menu in Explorer, or thumbgen --export-png <size> <files...>"
        );
        let (size, files) = png_export::job::consume(Path::new(&args[1]))?;
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        }
        let _apartment = Apartment;
        let dialog: IProgressDialog = unsafe {
            CoCreateInstance(
                &GUID::from_u128(0xf8383852_fcd3_11d1_a6b9_006097df5bd4),
                None,
                CLSCTX_INPROC_SERVER,
            )?
        };
        unsafe {
            dialog.SetTitle(w!("MeshThumbs — Export PNG"))?;
            dialog.SetCancelMsg(w!("Cancelling the current render…"), None)?;
            dialog.StartProgressDialog(HWND::default(), None, 8, None)?; // PROGDLG_NOMINIMIZE
        }
        let mut progress = Progress { dialog, size };
        let report = png_export::batch::run(files, size, &mut progress);
        drop(progress);
        summary(report?, size);
        Ok(())
    }
    pub fn error(error: anyhow::Error) {
        let message = wide(&format!("{error:#}"));
        unsafe {
            MessageBoxW(
                HWND::default(),
                PCWSTR(message.as_ptr()),
                w!("MeshThumbs"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

fn main() {
    #[cfg(windows)]
    if let Err(error) = ui::run() {
        ui::error(error);
    }
    #[cfg(not(windows))]
    eprintln!("The Explorer PNG exporter requires Windows.");
}
