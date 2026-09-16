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
        // Exporters may represent a triangle as a quad with a repeated corner.
        // Zero-length edges block ear clipping; retain original corner indices
        // so removing them does not detach colors/normals from their vertices.
        let mut corners = Vec::with_capacity(count);
        for i in 0..count {
            if corners.last().is_none_or(|&j| points[j] != points[i]) {
                corners.push(i);
            }
        }
        if corners.len() > 1 && points[corners[0]] == points[*corners.last().unwrap()] {
            corners.pop();
        }
        if corners.len() < 3 {
            return Ok(());
        }
        let triangles = triangulate(&corners, &points)?;
        output.scene.triangles.try_reserve(triangles.len())?;
        for indices in triangles {
            output.scene.triangles.push(Triangle {
                vertices: fix_normals(indices.map(|i| vertices[corners[i]])),
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

    #[cfg(test)]
    mod tests {
        use super::*;

        fn output(budget: usize) -> Output {
            Output {
                scene: Scene::new(),
                budget,
                error: None,
            }
        }

        fn polygon(points: &[[f64; 3]]) -> Vec<f64> {
            points
                .iter()
                .enumerate()
                .flat_map(|(i, p)| [p[0], p[1], p[2], 0., 0., 1., i as f64 / 16., 0.5, 1., 1.])
                .collect()
        }

        #[test]
        fn repeated_classroom_corner_keeps_the_triangle() {
            // Reduced from the CC0 Classroom Alembic export (Christophe Seux;
            // Gaffer example export), credited in examples/README.md.
            let points = [
                [0.011283021, -0.03381071, -0.34303266],
                [0.011395096, -0.033801265, -0.3429906],
                [0.011382262, -0.03424911, -0.3430281],
                [0.011382262, -0.03424911, -0.3430281],
            ];
            let mut out = output(2);
            append(&mut out, &polygon(&points), 4).unwrap();
            assert_eq!(out.scene.triangles.len(), 1);
            for (i, v) in out.scene.triangles[0].vertices.iter().enumerate() {
                assert_eq!(v.position, Vec3::from_array(points[i].map(|v| v as f32)));
            }
        }

        #[test]
        fn concave_polygon_preserves_winding_area_and_corner_attributes() {
            for reverse in [false, true] {
                let mut points = vec![
                    [0., 0., 0.],
                    [2., 0., 0.],
                    [2., 0., 0.],
                    [2., 2., 0.],
                    [1., 1., 0.],
                    [0., 2., 0.],
                    [0., 0., 0.],
                ];
                if reverse {
                    points.reverse();
                }
                let data = polygon(&points);
                let mut out = output(5);
                append(&mut out, &data, points.len()).unwrap();
                assert_eq!(out.scene.triangles.len(), 3);
                let mut area = 0.;
                for t in &out.scene.triangles {
                    let [a, b, c] = t.vertices.map(|v| v.position);
                    let signed = (b - a).cross(c - a).z / 2.;
                    assert!(if reverse { signed < 0. } else { signed > 0. });
                    area += signed.abs();
                    for v in t.vertices {
                        let i = points
                            .iter()
                            .position(|p| Vec3::from_array(p.map(|v| v as f32)) == v.position)
                            .unwrap();
                        assert_eq!(v.color, to_rgba(i as f32 / 16., 0.5, 1., 1.));
                        assert_eq!(v.normal, Vec3::Z);
                    }
                }
                assert!((area - 3.).abs() < 1e-6);
            }
        }

        #[test]
        fn collapsed_corners_do_not_emit_invalid_geometry() {
            let mut out = output(8);
            append(&mut out, &polygon(&[[0., 0., 0.]; 4]), 4).unwrap();
            append(
                &mut out,
                &polygon(&[[0., 0., 0.], [1., 0., 0.], [0., 0., 0.]]),
                3,
            )
            .unwrap();
            assert!(out.scene.triangles.is_empty());
            append(
                &mut out,
                &polygon(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]),
                3,
            )
            .unwrap();
            assert_eq!(out.scene.triangles.len(), 1);
        }

        #[test]
        fn duplicate_cleanup_keeps_limits_and_nonfinite_checks() {
            let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 1., 0.]];
            assert!(append(&mut output(1), &polygon(&points), 4).is_err());
            let mut data = polygon(&points);
            data[30] = f64::NAN;
            assert!(append(&mut output(2), &data, 4).is_err());
        }
    }
}
