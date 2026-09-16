//! Load the native CAD backend only inside a STEP render request.
use std::path::Path;

pub(super) fn load(path: &Path, budget: usize) -> anyhow::Result<crate::Scene> {
    #[cfg(windows)]
    {
        windows::load(path, budget)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, budget);
        anyhow::bail!("STEP thumbnails require the Windows CAD backend")
    }
}

#[cfg(windows)]
mod windows {
    use crate::{Scene, Triangle, Vertex};
    use anyhow::{bail, ensure, Context, Result};
    use glam::{Vec2, Vec3};
    use std::{
        ffi::{c_char, c_void, CStr},
        os::windows::ffi::OsStrExt,
        path::Path,
    };

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> i32;
    }
    struct Backend(*mut c_void);
    impl Drop for Backend {
        fn drop(&mut self) {
            unsafe {
                FreeLibrary(self.0);
            }
        }
    }
    impl Backend {
        fn open() -> Result<Self> {
            let executable = std::env::current_exe()?;
            let dll = executable
                .parent()
                .context("missing executable directory")?
                .join("step")
                .join("meshthumbs_step.dll");
            let path = dll
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            // Search dependencies beside this DLL and in system directories,
            // never in the model's folder or the process working directory.
            let handle = unsafe { LoadLibraryExW(path.as_ptr(), std::ptr::null_mut(), 0x1100) };
            ensure!(
                !handle.is_null(),
                "cannot load STEP backend {}: {}",
                dll.display(),
                std::io::Error::last_os_error()
            );
            Ok(Self(handle))
        }
        fn symbol(&self, name: &CStr) -> Result<*mut c_void> {
            let address = unsafe { GetProcAddress(self.0, name.as_ptr()) };
            ensure!(
                !address.is_null(),
                "STEP backend is missing {}",
                name.to_string_lossy()
            );
            Ok(address)
        }
    }
    type Sink = unsafe extern "C" fn(*mut c_void, *const f64) -> i32;
    type Load = unsafe extern "C" fn(*const u16, u64, Sink, *mut c_void, *mut c_char, u32) -> i32;
    type Abi = unsafe extern "C" fn() -> u32;
    struct Output {
        scene: Scene,
        budget: usize,
        error: Option<&'static str>,
    }

    unsafe extern "C" fn receive(context: *mut c_void, data: *const f64) -> i32 {
        let output = &mut *context.cast::<Output>();
        let data = std::slice::from_raw_parts(data, 18);
        if output.scene.triangles.len() >= output.budget {
            output.error = Some("STEP exceeds the triangle limit");
            return 0;
        }
        if !data
            .iter()
            .all(|v| v.is_finite() && (*v as f32).is_finite())
        {
            output.error = Some("STEP returned invalid mesh data");
            return 0;
        }
        if output.scene.triangles.try_reserve(1).is_err() {
            output.error = Some("not enough memory for STEP mesh");
            return 0;
        }
        let vertices = std::array::from_fn(|i| {
            let v = &data[i * 6..];
            Vertex {
                position: Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32),
                normal: Vec3::new(v[3] as f32, v[4] as f32, v[5] as f32),
                color: [255; 4],
                uv: Vec2::ZERO,
            }
        });
        output.scene.triangles.push(Triangle {
            vertices: super::super::fix_normals(vertices),
            color: [196, 205, 214, 255],
            texture: None,
        });
        1
    }
    pub(super) fn load(path: &Path, budget: usize) -> Result<Scene> {
        let backend = Backend::open()?;
        let abi: Abi = unsafe { std::mem::transmute(backend.symbol(c"meshthumbs_step_abi")?) };
        ensure!(unsafe { abi() } == 1, "incompatible STEP backend ABI");
        let load: Load = unsafe { std::mem::transmute(backend.symbol(c"meshthumbs_step_load")?) };
        let path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut output = Output {
            scene: Scene::new(),
            budget,
            error: None,
        };
        let mut error = [0 as c_char; 1024];
        let result = unsafe {
            load(
                path.as_ptr(),
                budget as u64,
                receive,
                (&mut output as *mut Output).cast(),
                error.as_mut_ptr(),
                error.len() as u32,
            )
        };
        if let Some(message) = output.error {
            bail!(message);
        }
        if result != 0 {
            error[1023] = 0;
            let message = unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy();
            bail!("STEP import failed: {message}");
        }
        Ok(output.scene)
    }
}
