use std::{
    ffi::c_void,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::PathBuf,
    ptr::null_mut,
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;
use renderer::MAX_MODEL_BYTES;

mod worker;
use windows::{
    core::{implement, Error, IUnknown, Interface, Result, GUID, HRESULT, PCWSTR},
    Win32::{
        Foundation::{
            BOOL, CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_POINTER, S_FALSE,
            S_OK,
        },
        Graphics::Gdi::{
            CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
        },
        System::Com::{
            CoTaskMemFree, IClassFactory, IClassFactory_Impl, IStream, STATFLAG_DEFAULT,
            STREAM_SEEK_SET,
        },
        UI::Shell::PropertiesSystem::{
            IInitializeWithFile, IInitializeWithFile_Impl, IInitializeWithStream,
            IInitializeWithStream_Impl,
        },
        UI::Shell::{
            IInitializeWithItem, IInitializeWithItem_Impl, IThumbnailProvider,
            IThumbnailProvider_Impl, SIGDN_FILESYSPATH, WTSAT_ARGB, WTSAT_UNKNOWN, WTS_ALPHATYPE,
        },
    },
};

const PROVIDER_VERSION: &str = env!("CARGO_PKG_VERSION");

const CLSID_OBJ_PROVIDER: GUID = GUID::from_u128(0xa9ffd4c4_3fa9_4eb7_8b47_b89a7f09d059);
const CLSID_FBX_PROVIDER: GUID = GUID::from_u128(0x4e5fd91f_c018_4850_9636_3069629c6d3d);
const CLSID_GLB_PROVIDER: GUID = GUID::from_u128(0xe859325c_5506_4419_8ac5_6a4b03f3a138);
const CLSID_GLTF_PROVIDER: GUID = GUID::from_u128(0xb7265976_0dba_44b5_9303_0b0dafd034e0);
const CLSID_STL_PROVIDER: GUID = GUID::from_u128(0xa3bafd17_52cd_4cf6_869e_a4bb020591ef);
const CLSID_DAE_PROVIDER: GUID = GUID::from_u128(0x7bf654cd_6b62_4a1c_be5f_53df447c2be6);
const CLSID_PLY_PROVIDER: GUID = GUID::from_u128(0xab2cde52_5c15_4daf_b43a_e4c9f1eaaec0);
const CLSID_3DS_PROVIDER: GUID = GUID::from_u128(0x0ad51061_9a3c_4ec3_9757_874ecb89457c);

#[derive(Clone, Copy)]
struct ProviderInfo {
    clsid: GUID,
    extension: &'static str,
}

const PROVIDERS: [ProviderInfo; 8] = [
    ProviderInfo {
        clsid: CLSID_OBJ_PROVIDER,
        extension: ".obj",
    },
    ProviderInfo {
        clsid: CLSID_FBX_PROVIDER,
        extension: ".fbx",
    },
    ProviderInfo {
        clsid: CLSID_GLB_PROVIDER,
        extension: ".glb",
    },
    ProviderInfo {
        clsid: CLSID_GLTF_PROVIDER,
        extension: ".gltf",
    },
    ProviderInfo {
        clsid: CLSID_STL_PROVIDER,
        extension: ".stl",
    },
    ProviderInfo {
        clsid: CLSID_DAE_PROVIDER,
        extension: ".dae",
    },
    ProviderInfo {
        clsid: CLSID_PLY_PROVIDER,
        extension: ".ply",
    },
    ProviderInfo {
        clsid: CLSID_3DS_PROVIDER,
        extension: ".3ds",
    },
];

static LOG_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[implement(
    IInitializeWithFile,
    IInitializeWithItem,
    IInitializeWithStream,
    IThumbnailProvider
)]
struct ThumbnailProvider {
    extension_hint: &'static str,
    path: Mutex<Option<Arc<ModelInput>>>,
}

struct ModelInput {
    path: PathBuf,
    _temporary: Option<tempfile::TempPath>,
}

impl ModelInput {
    fn file(path: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            path,
            _temporary: None,
        })
    }
}

impl ThumbnailProvider {
    fn new(extension_hint: &'static str) -> Self {
        Self {
            extension_hint,
            path: Mutex::new(None),
        }
    }
}

#[allow(non_snake_case)]
impl ThumbnailProvider {
    fn initialize_file(&self, pszfilepath: &PCWSTR, _grfmode: u32) -> Result<()> {
        let path = unsafe { pszfilepath.to_string() }.map_err(|_| Error::from(E_FAIL))?;
        log_line(&format!("v{PROVIDER_VERSION} Initialize {path}"));
        *self.path.lock().map_err(|_| Error::from(E_FAIL))? =
            Some(ModelInput::file(PathBuf::from(path)));
        Ok(())
    }
}

#[allow(non_snake_case)]
impl ThumbnailProvider {
    fn initialize_item(
        &self,
        psi: Option<&windows::Win32::UI::Shell::IShellItem>,
        _grfmode: u32,
    ) -> Result<()> {
        let item = psi.ok_or_else(|| Error::from(E_POINTER))?;
        let display_name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
        let converted = unsafe { display_name.to_string() };
        unsafe { CoTaskMemFree(Some(display_name.0 as *const c_void)) };
        let path = converted.map_err(|_| Error::from(E_FAIL))?;
        log_line(&format!("v{PROVIDER_VERSION} InitializeWithItem {path}"));
        *self.path.lock().map_err(|_| Error::from(E_FAIL))? =
            Some(ModelInput::file(PathBuf::from(path)));
        Ok(())
    }
}

#[allow(non_snake_case)]
impl IInitializeWithStream_Impl for ThumbnailProvider_Impl {
    fn Initialize(&self, pstream: Option<&IStream>, _grfmode: u32) -> Result<()> {
        let stream = pstream.ok_or_else(|| Error::from(E_POINTER))?;
        let temp_path = write_stream_to_temp_model(stream, self.extension_hint)?;
        log_line(&format!(
            "v{PROVIDER_VERSION} InitializeWithStream {}",
            temp_path.path.display()
        ));
        *self.path.lock().map_err(|_| Error::from(E_FAIL))? = Some(Arc::new(temp_path));
        Ok(())
    }
}

#[allow(non_snake_case)]
impl ThumbnailProvider {
    fn get_thumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> Result<()> {
        if phbmp.is_null() || pdwalpha.is_null() {
            return Err(Error::from(E_POINTER));
        }

        unsafe {
            *phbmp = HBITMAP::default();
            *pdwalpha = WTSAT_UNKNOWN;
        }
        let input = self
            .path
            .lock()
            .map_err(|_| Error::from(E_FAIL))?
            .clone()
            .ok_or_else(|| Error::from(E_FAIL))?;

        let path = &input.path;
        let size = cx.clamp(32, 512);
        let rendered = std::panic::catch_unwind(|| worker::render(path, size));
        let rendered = rendered.map_err(|_| {
            log_line("Renderer panic caught at COM boundary");
            Error::from(E_FAIL)
        })?;
        match rendered {
            Ok(bitmap) => unsafe {
                let hbmp = create_hbitmap(&bitmap.pixels, bitmap.width, bitmap.height)?;
                *phbmp = hbmp;
                *pdwalpha = WTSAT_ARGB;
                log_line(&format!(
                    "v{PROVIDER_VERSION} OK {} {}px",
                    path.display(),
                    size
                ));
                Ok(())
            },
            Err(error) => {
                log_line(&format!(
                    "v{PROVIDER_VERSION} ERR {}: {error}",
                    path.display()
                ));
                Err(Error::from(E_FAIL))
            }
        }
    }
}

// Sidecar-based formats require the original path. Do not advertise an
// anonymous stream interface that cannot resolve external geometry/textures.
#[implement(IInitializeWithFile, IInitializeWithItem, IThumbnailProvider)]
struct FileThumbnailProvider {
    inner: ThumbnailProvider,
}

#[allow(non_snake_case)]
impl IInitializeWithFile_Impl for ThumbnailProvider_Impl {
    fn Initialize(&self, path: &PCWSTR, mode: u32) -> Result<()> {
        self.initialize_file(path, mode)
    }
}
#[allow(non_snake_case)]
impl IInitializeWithItem_Impl for ThumbnailProvider_Impl {
    fn Initialize(
        &self,
        item: Option<&windows::Win32::UI::Shell::IShellItem>,
        mode: u32,
    ) -> Result<()> {
        self.initialize_item(item, mode)
    }
}
#[allow(non_snake_case)]
impl IThumbnailProvider_Impl for ThumbnailProvider_Impl {
    fn GetThumbnail(
        &self,
        size: u32,
        bitmap: *mut HBITMAP,
        alpha: *mut WTS_ALPHATYPE,
    ) -> Result<()> {
        self.get_thumbnail(size, bitmap, alpha)
    }
}
#[allow(non_snake_case)]
impl IInitializeWithFile_Impl for FileThumbnailProvider_Impl {
    fn Initialize(&self, path: &PCWSTR, mode: u32) -> Result<()> {
        self.inner.initialize_file(path, mode)
    }
}
#[allow(non_snake_case)]
impl IInitializeWithItem_Impl for FileThumbnailProvider_Impl {
    fn Initialize(
        &self,
        item: Option<&windows::Win32::UI::Shell::IShellItem>,
        mode: u32,
    ) -> Result<()> {
        self.inner.initialize_item(item, mode)
    }
}
#[allow(non_snake_case)]
impl IThumbnailProvider_Impl for FileThumbnailProvider_Impl {
    fn GetThumbnail(
        &self,
        size: u32,
        bitmap: *mut HBITMAP,
        alpha: *mut WTS_ALPHATYPE,
    ) -> Result<()> {
        self.inner.get_thumbnail(size, bitmap, alpha)
    }
}

#[implement(IClassFactory)]
struct ClassFactory {
    provider: ProviderInfo,
}

#[allow(non_snake_case)]
impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if ppvobject.is_null() || riid.is_null() {
            return Err(Error::from(E_POINTER));
        }
        unsafe { *ppvobject = null_mut() };

        if punkouter.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }

        let state = ThumbnailProvider::new(self.provider.extension);
        let unknown: IUnknown = if matches!(
            self.provider.extension,
            ".obj" | ".fbx" | ".gltf" | ".dae" | ".3ds"
        ) {
            FileThumbnailProvider { inner: state }.into()
        } else {
            state.into()
        };
        let hr = unsafe { unknown.query(riid, ppvobject) };
        if hr.is_ok() {
            Ok(())
        } else {
            Err(Error::from(hr))
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        log_line("DllGetClassObject E_POINTER");
        return E_POINTER;
    }
    unsafe { *ppv = null_mut() };

    let Some(provider) = PROVIDERS
        .iter()
        .copied()
        .find(|provider| provider.clsid == unsafe { *rclsid })
    else {
        log_line("DllGetClassObject CLASS_E_CLASSNOTAVAILABLE");
        return CLASS_E_CLASSNOTAVAILABLE;
    };

    log_line(&format!(
        "v{PROVIDER_VERSION} DllGetClassObject OK {}",
        provider.extension
    ));
    let factory: IClassFactory = ClassFactory { provider }.into();
    let hr = unsafe { factory.query(riid, ppv) };
    if hr.is_ok() {
        S_OK
    } else {
        hr
    }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

unsafe fn create_hbitmap(pixels: &[u8], width: u32, height: u32) -> Result<HBITMAP> {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut c_void = null_mut();
    let hbmp = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
    if hbmp.0.is_null() || bits.is_null() {
        return Err(Error::from(E_FAIL));
    }

    let out = std::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);
    for (src, dst) in pixels.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        dst[0] = (src[2] as u16 * src[3] as u16 / 255) as u8;
        dst[1] = (src[1] as u16 * src[3] as u16 / 255) as u8;
        dst[2] = (src[0] as u16 * src[3] as u16 / 255) as u8;
        dst[3] = src[3];
    }

    Ok(hbmp)
}

fn log_line(message: &str) {
    let _guard = LOG_LOCK.lock();
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let timestamp = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown-time".to_string());

    if let Some(path) = log_path() {
        let _ = create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{timestamp} {message}");
        }
    }
}

fn log_path() -> Option<PathBuf> {
    std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .map(|p| p.join("MeshThumbs").join("meshthumbs.log"))
        .or_else(|| {
            directories::ProjectDirs::from("dev", "MeshThumbs", "MeshThumbs")
                .map(|d| d.data_local_dir().join("meshthumbs.log"))
        })
}

fn write_stream_to_temp_model(stream: &IStream, extension_hint: &str) -> Result<ModelInput> {
    let mut stat = unsafe { std::mem::zeroed() };
    unsafe {
        stream.Stat(&mut stat, STATFLAG_DEFAULT)?;
    }
    let named_path = if !stat.pwcsName.0.is_null() {
        let name = unsafe { stat.pwcsName.to_string() };
        unsafe { CoTaskMemFree(Some(stat.pwcsName.0.cast())) };
        name.ok().map(PathBuf::from)
    } else {
        None
    };
    if stat.cbSize == 0 || stat.cbSize > MAX_MODEL_BYTES {
        return Err(Error::from(E_FAIL));
    }
    // File-backed Shell streams can preserve sidecar buffers and textures.
    // Anonymous/cloud streams still use a bounded, owned temporary copy below.
    if let Some(path) = named_path.filter(|p| p.is_absolute() && p.is_file()) {
        let extension_matches = path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case(extension_hint.trim_start_matches('.')));
        if extension_matches && std::fs::metadata(&path).is_ok_and(|m| m.len() == stat.cbSize) {
            return Ok(ModelInput {
                path,
                _temporary: None,
            });
        }
    }
    unsafe {
        stream.Seek(0, STREAM_SEEK_SET, None)?;
    }
    let mut buffer = vec![0u8; 64 * 1024];
    let mut prefix_len = 0;
    let requested = stat.cbSize.min(128) as u32;
    unsafe {
        stream
            .Read(buffer.as_mut_ptr().cast(), requested, Some(&mut prefix_len))
            .ok()?;
    }
    if prefix_len == 0 || prefix_len > requested {
        return Err(Error::from(E_FAIL));
    }
    let extension = extension_hint
        .strip_prefix('.')
        .filter(|ext| renderer::SUPPORTED_EXTENSIONS.contains(ext))
        .ok_or_else(|| Error::from(E_FAIL))?;
    let dir = std::env::temp_dir().join("MeshThumbs");
    create_dir_all(&dir).map_err(|_| Error::from(E_FAIL))?;
    let mut file = tempfile::Builder::new()
        .prefix("stream-")
        .suffix(&format!(".{extension}"))
        .tempfile_in(dir)
        .map_err(|_| Error::from(E_FAIL))?;
    file.write_all(&buffer[..prefix_len as usize])
        .map_err(|_| Error::from(E_FAIL))?;
    let mut remaining = stat.cbSize - prefix_len as u64;
    while remaining > 0 {
        let requested = remaining.min(buffer.len() as u64) as u32;
        let mut read = 0;
        unsafe {
            stream
                .Read(buffer.as_mut_ptr().cast(), requested, Some(&mut read))
                .ok()?;
        }
        if read == 0 || read > requested {
            return Err(Error::from(E_FAIL));
        }
        file.write_all(&buffer[..read as usize])
            .map_err(|_| Error::from(E_FAIL))?;
        remaining -= read as u64;
    }
    // TempPath owns only this generated file; it is deleted on release, replacement,
    // or error, after any concurrent GetThumbnail reader releases its Arc.
    let temporary = file.into_temp_path();
    Ok(ModelInput {
        path: temporary.to_path_buf(),
        _temporary: Some(temporary),
    })
}
