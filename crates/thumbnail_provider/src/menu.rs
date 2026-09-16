//! The menu only gathers paths and launches the separate GUI exporter.
use std::{
    ffi::{c_void, OsString},
    io::Write,
    os::windows::{ffi::OsStringExt, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    sync::Mutex,
};
use windows::Win32::System::SystemServices::{SFGAO_FILESYSTEM, SFGAO_FOLDER};
use windows::{
    core::{implement, Error, IUnknown, Interface, GUID, HRESULT, PWSTR},
    Win32::{
        Foundation::{
            BOOL, CLASS_E_NOAGGREGATION, E_FAIL, E_NOTIMPL, E_OUTOFMEMORY, E_POINTER, S_FALSE, S_OK,
        },
        System::Com::{CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl},
        UI::Shell::{
            IEnumExplorerCommand, IEnumExplorerCommand_Impl, IExplorerCommand,
            IExplorerCommand_Impl, IShellItemArray, ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED,
            ECS_HIDDEN, SIGDN_FILESYSPATH,
        },
    },
};
const E_PENDING: HRESULT = HRESULT(0x8000000Au32 as i32);

const ID: u128 = 0x2c7a8d3e_72ca_4b24_9d8c_426ddc3a1515;
pub(super) const CLSID: GUID = GUID::from_u128(ID);
type Result<T> = windows::core::Result<T>;

fn allocated(text: &str) -> Result<PWSTR> {
    let words = text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe {
        let memory = CoTaskMemAlloc(words.len() * 2).cast::<u16>();
        if memory.is_null() {
            return Err(Error::from_hresult(E_OUTOFMEMORY));
        }
        std::ptr::copy_nonoverlapping(words.as_ptr(), memory, words.len());
        Ok(PWSTR(memory))
    }
}

fn files(items: Option<&IShellItemArray>) -> Result<Vec<PathBuf>> {
    let items = items.ok_or_else(|| Error::from_hresult(E_POINTER))?;
    let count = unsafe { items.GetCount()? };
    if count == 0 || count as usize > png_export::MAX_FILES {
        return Err(Error::from_hresult(E_FAIL));
    }
    let mut paths = Vec::with_capacity(count as usize);
    for i in 0..count {
        unsafe {
            let item = items.GetItemAt(i)?;
            let flags = item.GetAttributes(SFGAO_FILESYSTEM | SFGAO_FOLDER)?;
            if (flags & SFGAO_FOLDER).0 != 0 || (flags & SFGAO_FILESYSTEM).0 == 0 {
                return Err(Error::from_hresult(E_FAIL));
            }
            let name = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let path = PathBuf::from(OsString::from_wide(name.as_wide()));
            CoTaskMemFree(Some(name.0.cast()));
            if !path.is_absolute() || !png_export::supported(&path) {
                return Err(Error::from_hresult(E_FAIL));
            }
            paths.push(path);
        }
    }
    Ok(paths)
}

fn launch(paths: Vec<PathBuf>, size: u32) -> anyhow::Result<()> {
    let executable = super::worker::worker_path()?.with_file_name("meshthumbs-export.exe");
    anyhow::ensure!(executable.is_file(), "MeshThumbs PNG exporter is missing");
    let mut job = tempfile::Builder::new()
        .prefix("meshthumbs-export-")
        .suffix(".job")
        .tempfile()?;
    png_export::job::write(&mut job, size, &paths)?;
    job.flush()?;
    let (file, path) = job.keep().map_err(|e| e.error)?;
    drop(file);
    let result = Command::new(executable)
        .arg("--job")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn();
    if let Err(error) = result {
        let _ = std::fs::remove_file(path);
        return Err(error.into());
    }
    Ok(())
}

#[implement(IExplorerCommand)]
struct ExportCommand {
    size: Option<u32>,
    _module: super::ModuleObject,
}
fn command(size: Option<u32>) -> IExplorerCommand {
    ExportCommand {
        size,
        _module: super::ModuleObject::new(),
    }
    .into()
}
#[allow(non_snake_case)]
impl IExplorerCommand_Impl for ExportCommand_Impl {
    fn GetTitle(&self, _: Option<&IShellItemArray>) -> Result<PWSTR> {
        allocated(&self.size.map_or_else(
            || "MeshThumbs".to_owned(),
            |n| format!("PNG {n} × {n}"),
        ))
    }
    fn GetIcon(&self, _: Option<&IShellItemArray>) -> Result<PWSTR> {
        Err(Error::from_hresult(E_NOTIMPL))
    }
    fn GetToolTip(&self, _: Option<&IShellItemArray>) -> Result<PWSTR> {
        allocated("Save PNG beside each model. Existing matching PNG files are replaced.")
    }
    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(GUID::from_u128(ID + self.size.unwrap_or(0) as u128))
    }
    fn GetState(&self, items: Option<&IShellItemArray>, slow: BOOL) -> Result<u32> {
        if let Some(items) = items {
            if !slow.as_bool() && unsafe { items.GetCount()? } > 64 {
                return Err(Error::from_hresult(E_PENDING));
            }
        }
        Ok(if files(items).is_ok() {
            ECS_ENABLED.0 as u32
        } else {
            ECS_HIDDEN.0 as u32
        })
    }
    fn Invoke(&self, items: Option<&IShellItemArray>, _: Option<&IBindCtx>) -> Result<()> {
        let size = self.size.ok_or_else(|| Error::from_hresult(E_NOTIMPL))?;
        launch(files(items)?, size).map_err(|e| Error::new(E_FAIL, format!("{e:#}")))
    }
    fn GetFlags(&self) -> Result<u32> {
        Ok(if self.size.is_none() {
            ECF_HASSUBCOMMANDS.0 as u32
        } else {
            ECF_DEFAULT.0 as u32
        })
    }
    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        if self.size.is_some() {
            return Err(Error::from_hresult(E_NOTIMPL));
        }
        Ok(enumerator(0))
    }
}

#[implement(IEnumExplorerCommand)]
struct Commands {
    next: Mutex<usize>,
    _module: super::ModuleObject,
}
fn enumerator(next: usize) -> IEnumExplorerCommand {
    Commands {
        next: Mutex::new(next),
        _module: super::ModuleObject::new(),
    }
    .into()
}
#[allow(non_snake_case)]
impl IEnumExplorerCommand_Impl for Commands_Impl {
    fn Next(
        &self,
        count: u32,
        commands: *mut Option<IExplorerCommand>,
        fetched: *mut u32,
    ) -> HRESULT {
        if !fetched.is_null() {
            unsafe {
                fetched.write(0);
            }
        }
        if count == 0 {
            return S_OK;
        }
        if commands.is_null() || (count != 1 && fetched.is_null()) {
            return E_POINTER;
        }
        let Ok(mut next) = self.next.lock() else {
            return E_FAIL;
        };
        let n = (count as usize).min(png_export::SIZES.len() - *next);
        for i in 0..n {
            unsafe {
                commands
                    .add(i)
                    .write(Some(command(Some(png_export::SIZES[*next + i]))));
            }
        }
        *next += n;
        if !fetched.is_null() {
            unsafe {
                fetched.write(n as u32);
            }
        }
        if n == count as usize {
            S_OK
        } else {
            S_FALSE
        }
    }
    fn Skip(&self, count: u32) -> Result<()> {
        let mut next = self.next.lock().map_err(|_| Error::from_hresult(E_FAIL))?;
        let n = (count as usize).min(png_export::SIZES.len() - *next);
        *next += n;
        if n == count as usize {
            Ok(())
        } else {
            Err(Error::from_hresult(S_FALSE))
        }
    }
    fn Reset(&self) -> Result<()> {
        *self.next.lock().map_err(|_| Error::from_hresult(E_FAIL))? = 0;
        Ok(())
    }
    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        Ok(enumerator(
            *self.next.lock().map_err(|_| Error::from_hresult(E_FAIL))?,
        ))
    }
}

#[implement(IClassFactory)]
struct Factory {
    _module: super::ModuleObject,
}
#[allow(non_snake_case)]
impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Option<&IUnknown>,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> Result<()> {
        if out.is_null() || iid.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        unsafe {
            out.write(std::ptr::null_mut());
        }
        if outer.is_some() {
            return Err(Error::from_hresult(CLASS_E_NOAGGREGATION));
        }
        unsafe { command(None).query(iid, out).ok() }
    }
    fn LockServer(&self, lock: BOOL) -> Result<()> {
        super::lock_server(lock)
    }
}

pub(super) unsafe fn factory(iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    let factory: IClassFactory = Factory {
        _module: super::ModuleObject::new(),
    }
    .into();
    factory.query(iid, out)
}
