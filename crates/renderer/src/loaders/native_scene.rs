use std::path::Path;

pub(super) fn load(path: &Path, budget: usize) -> anyhow::Result<crate::Scene> {
    #[cfg(windows)]
    {
        windows::load(path, budget)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, budget);
        anyhow::bail!("Alembic/3DM require the Windows scene backend")
    }
}

#[cfg(windows)]
mod windows {
    use super::super::{fix_normals, step::windows::Backend, to_rgba, usd::triangulate};
    use crate::{Scene, Triangle, Vertex};
    use anyhow::{bail, ensure, Result};
    use glam::{Vec2, Vec3};
    use std::{
        ffi::{c_char, c_void, CStr},
        os::windows::ffi::OsStrExt,
        path::Path,
    };
    type Sink = unsafe extern "C" fn(*mut c_void, *const f64, u32) -> i32;
    type Load = unsafe extern "C" fn(*const u16, u64, Sink, *mut c_void, *mut c_char, u32) -> i32;
    struct Output {
        scene: Scene,
        budget: usize,
        error: Option<String>,
    }
    fn append(output: &mut Output, data: &[f64], count: usize) -> Result<()> {
        ensure!((3..=4096).contains(&count), "invalid native polygon size");
        ensure!(
            output.scene.triangles.len() + count - 2 <= output.budget,
            "scene exceeds triangle limit"
        );
        ensure!(
            data.iter()
                .all(|v| v.is_finite() && (*v as f32).is_finite()),
            "non-finite native geometry"
        );
        let vertices = data
            .as_chunks::<10>()
            .0
            .iter()
            .map(|v| Vertex {
                position: Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32),
                normal: Vec3::new(v[3] as f32, v[4] as f32, v[5] as f32),
                color: to_rgba(v[6] as f32, v[7] as f32, v[8] as f32, v[9] as f32),
                uv: Vec2::ZERO,
            })
            .collect::<Vec<_>>();
        let points = vertices.iter().map(|v| v.position).collect::<Vec<_>>();
        let triangles = triangulate(&(0..count).collect::<Vec<_>>(), &points)?;
        output.scene.triangles.try_reserve(triangles.len())?;
        for indices in triangles {
            output.scene.triangles.push(Triangle {
                vertices: fix_normals(indices.map(|i| vertices[i])),
                color: [255; 4],
                texture: None,
            });
        }
        Ok(())
    }
    unsafe extern "C" fn receive(context: *mut c_void, data: *const f64, count: u32) -> i32 {
        let output = &mut *context.cast::<Output>();
        // No Rust panic is allowed to unwind through a C++ stack.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ensure!(
                !data.is_null() && (3..=4096).contains(&count),
                "invalid native polygon"
            );
            append(
                output,
                std::slice::from_raw_parts(data, count as usize * 10),
                count as usize,
            )
        }));
        match result {
            Ok(Ok(())) => 1,
            Ok(Err(e)) => {
                output.error = Some(e.to_string());
                0
            }
            Err(_) => {
                output.error = Some("native polygon conversion panicked".into());
                0
            }
        }
    }
    pub(super) fn load(path: &Path, budget: usize) -> Result<Scene> {
        let backend = Backend::open_module("scene", "meshthumbs_scene.dll")?;
        let abi: unsafe extern "C" fn() -> u32 =
            unsafe { std::mem::transmute(backend.symbol(c"meshthumbs_scene_abi")?) };
        ensure!(unsafe { abi() } == 1, "incompatible scene backend ABI");
        let symbol = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("abc"))
        {
            c"meshthumbs_abc_load"
        } else {
            c"meshthumbs_3dm_load"
        };
        let load: Load = unsafe { std::mem::transmute(backend.symbol(symbol)?) };
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
                1024,
            )
        };
        if let Some(message) = output.error {
            bail!(message);
        }
        if result != 0 {
            error[1023] = 0;
            bail!(
                "scene import failed: {}",
                unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy()
            );
        }
        Ok(output.scene)
    }
}
