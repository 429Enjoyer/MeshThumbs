//! Static composed USD mesh previews. The backend reads USDA, USDC, and USDZ;
//! rendering uses authored topology at the start time, without subdivision or
//! simulation. All assets are read locally and stay inside the worker process.
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{RenderError, Scene, Triangle, Vertex};
use anyhow::{bail, ensure, Context, Result};
use glam::{DMat4, DQuat, DVec3, Vec2, Vec3};
use openusd::{
    gf, sdf,
    tf::Token,
    usd::{Prim, Stage, TimeCode},
};

mod assets;
mod geometry;
mod material;
use assets::Assets;
use material::Materials;

pub(super) fn load(path: &Path, budget: usize) -> Result<Scene> {
    let path = path.canonicalize()?;
    let assets = Assets::new(path.parent().unwrap_or(Path::new(".")));
    let stage = Stage::builder()
        .resolver(assets.clone())
        .open(path.to_str().context("USD path must be UTF-8")?)?;
    let axis = stage
        .stage_metadata("upAxis")?
        .map(|v| v.cast::<String>())
        .transpose()?
        .unwrap_or_else(|| "Y".into());
    let root = match axis.as_str() {
        "Y" => DMat4::IDENTITY,
        "Z" => DMat4::from_rotation_x(-std::f64::consts::FRAC_PI_2),
        _ => bail!("unsupported USD upAxis"),
    };
    let time = TimeCode::new(
        stage
            .stage_metadata("startTimeCode")?
            .map(|v| v.cast::<f64>())
            .transpose()?
            .unwrap_or(0.0),
    );
    ensure!(time.value().is_finite(), "USD start time must be finite");
    let mut stack = stage
        .prim("/")?
        .children()?
        .into_iter()
        .map(|p| (p, root, None, "default".to_owned(), 0usize))
        .collect::<Vec<_>>();
    let mut scene = Scene::new();
    let mut visited = 0usize;
    let mut materials = Materials::default();
    while let Some((prim, parent, inherited, inherited_purpose, depth)) = stack.pop() {
        visited += 1;
        ensure!(
            visited <= 100_000 && depth < 256,
            "USD scene hierarchy exceeds limits"
        );
        if !prim.is_active()? || !prim.is_defined()? || prim.is_abstract()? {
            continue;
        }
        if token(&prim, "visibility", time)?.as_deref() == Some("invisible") {
            continue;
        }
        let purpose = token(&prim, "purpose", time)?.unwrap_or(inherited_purpose);
        let transform = transform(&prim, parent, root, time)?;
        let binding = material::binding(&prim)?.or(inherited);
        let kind = prim.type_name()?.map(|n| n.to_string()).unwrap_or_default();
        ensure!(
            kind != "PointInstancer",
            "USD PointInstancer geometry is unsupported"
        );
        if matches!(
            kind.as_str(),
            "Mesh" | "Cube" | "Sphere" | "Cylinder" | "Cone"
        ) && !matches!(purpose.as_str(), "guide" | "proxy")
        {
            mesh(
                &stage,
                &assets,
                &prim,
                transform,
                binding.as_ref(),
                time,
                budget,
                &mut materials,
                &mut scene,
            )?;
        }
        for child in prim.children()? {
            stack.push((
                child,
                transform,
                binding.clone(),
                purpose.clone(),
                depth + 1,
            ));
        }
    }
    ensure!(
        stage.composition_errors().is_empty(),
        "USD stage contains unresolved or invalid composition references"
    );
    Ok(scene)
}

fn value(prim: &Prim, name: &str, time: TimeCode) -> Result<Option<sdf::Value>> {
    Ok(prim.attribute(name).get_at::<sdf::Value>(Some(time))?)
}
fn token(prim: &Prim, name: &str, time: TimeCode) -> Result<Option<String>> {
    value(prim, name, time)?
        .map(|v| v.cast::<String>().map_err(Into::into))
        .transpose()
}
fn numbers3(prim: &Prim, name: &str, time: TimeCode) -> Result<Vec<[f32; 3]>> {
    value(prim, name, time)?
        .map(Vec::<[f32; 3]>::try_from)
        .transpose()
        .map(|v| v.unwrap_or_default())
        .map_err(Into::into)
}
fn integers(prim: &Prim, name: &str, time: TimeCode) -> Result<Vec<i32>> {
    match value(prim, name, time)? {
        None => Ok(Vec::new()),
        Some(sdf::Value::IntVec(values)) => Ok(values),
        _ => bail!("USD {name} must be an int array"),
    }
}

fn transform(prim: &Prim, parent: DMat4, root: DMat4, time: TimeCode) -> Result<DMat4> {
    let order = prim
        .attribute("xformOpOrder")
        .get::<Vec<Token>>()?
        .unwrap_or_default();
    let mut world = parent;
    for entry in order {
        let name = entry.as_str();
        if name == "!resetXformStack!" {
            world = root;
            continue;
        }
        let (invert, name) = name
            .strip_prefix("!invert!")
            .map(|s| (true, s))
            .unwrap_or((false, name));
        // An unauthored transform op contributes identity.
        let Some(v) = value(prim, name, time)? else {
            continue;
        };
        let kind = name
            .strip_prefix("xformOp:")
            .context("invalid USD transform op")?
            .split(':')
            .next()
            .unwrap();
        let mut op = match kind {
            "transform" => {
                let m = gf::Matrix4d::try_from(v)?;
                DMat4::from_cols_array(&m.0)
            }
            "translate" => DMat4::from_translation(DVec3::from(v.cast::<[f64; 3]>()?)),
            "scale" => DMat4::from_scale(DVec3::from(v.cast::<[f64; 3]>()?)),
            "orient" => {
                let [w, x, y, z] = v.cast::<[f64; 4]>()?;
                let q = DQuat::from_xyzw(x, y, z, w);
                ensure!(
                    q.is_finite() && q.length_squared() > 0.0,
                    "invalid USD orientation"
                );
                DMat4::from_quat(q.normalize())
            }
            "rotateX" => DMat4::from_rotation_x(v.cast::<f64>()?.to_radians()),
            "rotateY" => DMat4::from_rotation_y(v.cast::<f64>()?.to_radians()),
            "rotateZ" => DMat4::from_rotation_z(v.cast::<f64>()?.to_radians()),
            "rotateXYZ" | "rotateXZY" | "rotateYXZ" | "rotateYZX" | "rotateZXY" | "rotateZYX" => {
                let a = v.cast::<[f64; 3]>()?.map(f64::to_radians);
                let mut r = DMat4::IDENTITY;
                for axis in kind[6..].chars() {
                    r = match axis {
                        'X' => DMat4::from_rotation_x(a[0]),
                        'Y' => DMat4::from_rotation_y(a[1]),
                        _ => DMat4::from_rotation_z(a[2]),
                    } * r;
                }
                r
            }
            _ => bail!("unsupported USD transform op {kind}"),
        };
        if invert {
            ensure!(
                op.determinant().abs() > 1e-20,
                "singular inverted USD transform"
            );
            op = op.inverse();
        }
        ensure!(op.is_finite(), "nonfinite USD transform");
        world *= op;
    }
    Ok(world)
}

struct Primvar<T> {
    values: Vec<T>,
    indices: Vec<i32>,
    interpolation: String,
}
impl<T: Copy> Primvar<T> {
    fn at(&self, vertex: usize, corner: usize, face: usize) -> Result<Option<T>> {
        if self.values.is_empty() {
            return Ok(None);
        }
        let index = match self.interpolation.as_str() {
            "constant" => 0,
            "uniform" => face,
            "vertex" | "varying" => vertex,
            "faceVarying" => corner,
            _ => bail!("unsupported USD primvar interpolation"),
        };
        let index = if self.indices.is_empty() {
            index
        } else {
            usize::try_from(
                *self
                    .indices
                    .get(index)
                    .context("USD primvar index array too short")?,
            )?
        };
        Ok(Some(
            *self
                .values
                .get(index)
                .context("USD primvar value index out of range")?,
        ))
    }
}
fn interpolation(prim: &Prim, name: &str, default: &str) -> Result<String> {
    Ok(prim
        .attribute(name)
        .get_metadata::<Token>("interpolation")?
        .map(|v| v.to_string())
        .unwrap_or_else(|| default.into()))
}

#[allow(clippy::too_many_arguments)]
fn mesh(
    stage: &Stage,
    assets: &Assets,
    prim: &Prim,
    world: DMat4,
    binding: Option<&sdf::Path>,
    time: TimeCode,
    budget: usize,
    cache: &mut Materials,
    scene: &mut Scene,
) -> Result<()> {
    let kind = prim.type_name()?.map(|n| n.to_string()).unwrap_or_default();
    let implicit = kind != "Mesh";
    let geometry::Geometry {
        points,
        counts,
        indices,
        normals: generated_normals,
        uvs: generated_uvs,
    } = geometry::read(prim, &kind, time)?;
    if points.is_empty() {
        return Ok(());
    }
    ensure!(points.iter().all(|p| p.is_finite()), "nonfinite USD point");
    let holes = integers(prim, "holeIndices", time)?
        .into_iter()
        .collect::<HashSet<_>>();
    ensure!(
        holes.iter().all(|&i| i >= 0 && (i as usize) < counts.len()),
        "USD hole face index out of range"
    );
    let needed = counts.iter().try_fold(0usize, |sum, &n| {
        ensure!((3..=4096).contains(&n), "unsupported USD polygon size");
        sum.checked_add((n - 2) as usize)
            .context("USD triangle count overflow")
    })?;
    ensure!(
        scene.triangles.len().saturating_add(needed) <= budget,
        RenderError::TooManyTriangles {
            actual: scene.triangles.len().saturating_add(needed),
            limit: budget
        }
    );
    let normals = Primvar {
        values: if implicit {
            generated_normals
        } else {
            numbers3(prim, "normals", time)?
        },
        indices: Vec::new(),
        interpolation: interpolation(prim, "normals", "vertex")?,
    };
    let color_name = "primvars:displayColor";
    let colors = Primvar {
        values: numbers3(prim, color_name, time)?,
        indices: integers(prim, "primvars:displayColor:indices", time)?,
        interpolation: interpolation(prim, color_name, "constant")?,
    };
    let opacity = Primvar {
        values: prim
            .attribute("primvars:displayOpacity")
            .get_at::<Vec<f32>>(Some(time))?
            .unwrap_or_default(),
        indices: integers(prim, "primvars:displayOpacity:indices", time)?,
        interpolation: interpolation(prim, "primvars:displayOpacity", "constant")?,
    };
    let mut subsets = HashMap::new();
    for child in prim.children()? {
        if child
            .type_name()?
            .is_some_and(|n| n.as_str() == "GeomSubset")
            && token(&child, "elementType", time)?
                .as_deref()
                .unwrap_or("face")
                == "face"
        {
            if let Some(target) = material::binding(&child)? {
                for index in integers(&child, "indices", time)? {
                    ensure!(
                        index >= 0 && (index as usize) < counts.len(),
                        "USD subset face index out of range"
                    );
                    subsets.insert(index as usize, target.clone());
                }
            }
        }
    }
    let mut uv_sets = HashMap::new();
    let normal_world = world.inverse().transpose();
    let left_handed = (token(prim, "orientation", time)?.as_deref() == Some("leftHanded"))
        ^ (world.determinant() < 0.0);
    let mut cursor = 0usize;
    for (face, &count) in counts.iter().enumerate() {
        let count = count as usize;
        let end = cursor
            .checked_add(count)
            .context("USD corner count overflow")?;
        let ids = indices
            .get(cursor..end)
            .context("USD face index array too short")?
            .iter()
            .map(|&i| {
                let i = usize::try_from(i)?;
                ensure!(i < points.len(), "USD face index out of range");
                Ok(i)
            })
            .collect::<Result<Vec<_>>>()?;
        if holes.contains(&(face as i32)) {
            cursor = end;
            continue;
        }
        let mat = material::get(stage, assets, subsets.get(&face).or(binding), time, cache)?;
        let uv_name = format!("primvars:{}", mat.uv_name);
        if !uv_sets.contains_key(&uv_name) {
            let values = prim
                .attribute(uv_name.as_str())
                .get_at::<Vec<[f32; 2]>>(Some(time))?
                .unwrap_or_else(|| generated_uvs.clone());
            let uv = Primvar {
                values,
                indices: integers(prim, &format!("{uv_name}:indices"), time)?,
                interpolation: interpolation(
                    prim,
                    &uv_name,
                    if implicit { "vertex" } else { "constant" },
                )?,
            };
            uv_sets.insert(uv_name.clone(), uv);
        }
        let uv = &uv_sets[&uv_name];
        let mut vertices = Vec::with_capacity(count);
        for (corner, &id) in ids.iter().enumerate() {
            let p = world.transform_point3(points[id].as_dvec3()).as_vec3();
            ensure!(p.is_finite(), "USD transformed point overflow");
            let normal = normals
                .at(id, cursor + corner, face)?
                .map(Vec3::from)
                .unwrap_or(Vec3::ZERO);
            let normal = normal_world
                .transform_vector3(normal.as_dvec3())
                .as_vec3()
                .normalize_or_zero();
            let rgba = if mat.bound {
                [255; 4]
            } else {
                material::color(
                    colors.at(id, cursor + corner, face)?.unwrap_or([1.0; 3]),
                    opacity.at(id, cursor + corner, face)?.unwrap_or(1.0),
                )
            };
            vertices.push(Vertex {
                position: p,
                normal,
                uv: mat.uv_transform.transform_point2(Vec2::from(
                    uv.at(id, cursor + corner, face)?.unwrap_or([0.0; 2]),
                )),
                color: rgba,
            });
        }
        for mut corners in triangulate(&ids, &points)? {
            if left_handed {
                corners.swap(1, 2);
            }
            let vertex = corners.map(|i| vertices[i]);
            scene.triangles.push(Triangle {
                vertices: super::fix_normals(vertex),
                color: if !mat.bound && !colors.values.is_empty() {
                    [255; 4]
                } else {
                    mat.color
                },
                texture: mat.texture.clone(),
            });
        }
        cursor = end;
    }
    ensure!(cursor == indices.len(), "unexpected extra USD face indices");
    Ok(())
}

pub(super) fn triangulate(ids: &[usize], points: &[Vec3]) -> Result<Vec<[usize; 3]>> {
    if ids.len() == 3 {
        return Ok(vec![[0, 1, 2]]);
    }
    let mut normal = Vec3::ZERO;
    for i in 0..ids.len() {
        let a = points[ids[i]];
        let b = points[ids[(i + 1) % ids.len()]];
        normal += Vec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    let n = normal.abs();
    let axis = if n.x >= n.y && n.x >= n.z {
        0
    } else if n.y >= n.z {
        1
    } else {
        2
    };
    let points = ids
        .iter()
        .map(|&i| {
            let p = points[i];
            match axis {
                0 => glam::DVec2::new(p.y as f64, p.z as f64),
                1 => glam::DVec2::new(p.z as f64, p.x as f64),
                _ => glam::DVec2::new(p.x as f64, p.y as f64),
            }
        })
        .collect::<Vec<_>>();
    let sign = points
        .iter()
        .enumerate()
        .map(|(i, p)| p.perp_dot(points[(i + 1) % points.len()]))
        .sum::<f64>()
        .signum();
    if sign == 0.0 {
        return Ok(Vec::new());
    }
    let mut remaining = (0..ids.len()).collect::<Vec<_>>();
    let mut triangles = Vec::new();
    while remaining.len() > 3 {
        let mut ear = None;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            if (points[b] - points[a]).perp_dot(points[c] - points[a]) * sign <= 0.0 {
                continue;
            }
            let inside = remaining.iter().any(|&p| {
                p != a
                    && p != b
                    && p != c
                    && (points[b] - points[a]).perp_dot(points[p] - points[a]) * sign >= 0.0
                    && (points[c] - points[b]).perp_dot(points[p] - points[b]) * sign >= 0.0
                    && (points[a] - points[c]).perp_dot(points[p] - points[c]) * sign >= 0.0
            });
            if !inside {
                ear = Some((i, [a, b, c]));
                break;
            }
        }
        let (index, triangle) = ear.context("polygon cannot be triangulated")?;
        triangles.push(triangle);
        remaining.remove(index);
    }
    triangles.push([remaining[0], remaining[1], remaining[2]]);
    Ok(triangles)
}
