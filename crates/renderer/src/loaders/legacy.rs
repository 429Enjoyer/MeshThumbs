//! Compatibility importers. The original OBJ/FBX/glTF paths do not use Assimp.
use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{bail, Context};
use asset_importer::{
    material::{TextureInfo, TextureMapMode, TextureType},
    postprocess::PostProcessSteps,
    texture::TextureDataRef,
    Importer,
};
use glam::{Vec2, Vec3};

use super::{fix_normals, to_rgba};
use crate::{Scene, Texture, Triangle, Vertex, WrapMode};

#[cfg(all(windows, target_env = "gnu"))]
use libz_sys as _;

// Keep zlib available regardless of Rust's native static archive ordering.
#[cfg(all(windows, target_env = "gnu"))]
#[link(name = "z", kind = "static", modifiers = "+whole-archive")]
extern "C" {}

use crate::MAX_MODEL_BYTES;
const DEFAULT_COLOR: [u8; 4] = [196, 205, 214, 255];

pub(super) fn load(path: &Path, budget: usize) -> anyhow::Result<Scene> {
    if std::fs::metadata(path)?.len() > MAX_MODEL_BYTES {
        bail!("model exceeds the 300 MiB compatibility importer limit");
    }
    let imported = Importer::new()
        .read_file(path)
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::PRE_TRANSFORM_VERTICES
                | PostProcessSteps::TRANSFORM_UV_COORDS
                | PostProcessSteps::VALIDATE_DATA_STRUCTURE,
        )
        .import()
        .with_context(|| format!("failed to import {}", path.display()))?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut texture_cache = HashMap::<String, Option<Arc<Texture>>>::new();
    let materials = imported
        .materials()
        .map(|mat| {
            let color = mat
                .base_color()
                .map(|c| to_rgba(c.x, c.y, c.z, c.w))
                .or_else(|| {
                    mat.diffuse_color()
                        .map(|c| to_rgba(c.x, c.y, c.z, mat.opacity().unwrap_or(1.0)))
                })
                .unwrap_or(DEFAULT_COLOR);
            let info = mat
                .texture(TextureType::BaseColor, 0)
                .or_else(|| mat.texture(TextureType::Diffuse, 0));
            let texture = info.as_ref().and_then(|info| {
                // Include the sampler in the cache key; two materials can share an image.
                let key = format!("{}:{:?}", info.path, info.map_modes);
                texture_cache
                    .entry(key)
                    .or_insert_with(|| load_texture(&imported, parent, info).ok().map(Arc::new))
                    .clone()
            });
            (
                color,
                texture,
                info.map(|i| i.uv_index as usize).unwrap_or(0),
            )
        })
        .collect::<Vec<_>>();

    // Sampling individual faces destroys surface coverage on dense scans.
    // Check the limit before allocating Rust triangles, then keep the whole mesh.
    let total = imported
        .meshes()
        .map(|m| m.faces().filter(|f| f.indices().len() == 3).count())
        .sum::<usize>();
    if total > budget {
        return Err(crate::RenderError::TooManyTriangles {
            actual: total,
            limit: budget,
        }
        .into());
    }
    let mut scene = Scene::new();
    scene.triangles.reserve_exact(total);
    for mesh in imported.meshes() {
        let positions = mesh.vertices_raw();
        let normals = mesh.normals_raw_opt().unwrap_or(&[]);
        let colors = mesh.vertex_colors_raw_opt(0).unwrap_or(&[]);
        let material = materials.get(mesh.material_index());
        let uv_channel = material.map(|m| m.2).unwrap_or(0);
        let uvs = mesh.texture_coords_raw_opt(uv_channel).unwrap_or(&[]);
        for face in mesh.faces() {
            let indices = face.indices();
            if indices.len() != 3 {
                continue;
            }
            if indices.iter().any(|&i| i as usize >= positions.len()) {
                continue;
            }
            let vertices = std::array::from_fn(|corner| {
                let i = indices[corner] as usize;
                let p = positions[i];
                Vertex {
                    position: Vec3::new(p.x, p.y, p.z),
                    normal: normals
                        .get(i)
                        .map(|n| Vec3::new(n.x, n.y, n.z))
                        .filter(|n| n.is_finite())
                        .unwrap_or(Vec3::ZERO),
                    color: colors
                        .get(i)
                        .map(|c| to_rgba(c.r, c.g, c.b, c.a))
                        .unwrap_or([255; 4]),
                    uv: uvs
                        .get(i)
                        .map(|uv| Vec2::new(uv.x, uv.y))
                        .filter(|uv| uv.is_finite())
                        .unwrap_or(Vec2::ZERO),
                }
            });
            if vertices.iter().any(|v| !v.position.is_finite()) {
                continue;
            }
            let face_normal = (vertices[1].position - vertices[0].position)
                .cross(vertices[2].position - vertices[0].position);
            if !face_normal.is_finite() || face_normal.length_squared() == 0.0 {
                continue;
            }
            let color = if colors.is_empty() {
                material.map(|m| m.0).unwrap_or(DEFAULT_COLOR)
            } else {
                [255; 4]
            };
            scene.triangles.push(Triangle {
                vertices: fix_normals(vertices),
                color,
                texture: material.and_then(|m| m.1.clone()),
            });
        }
    }
    Ok(scene)
}

fn load_texture(
    scene: &asset_importer::Scene,
    parent: &Path,
    info: &TextureInfo,
) -> anyhow::Result<Texture> {
    let image = if let Some(embedded) = scene.embedded_texture_by_name(&info.path)? {
        match embedded.data_ref()? {
            TextureDataRef::Compressed(bytes) => image::load_from_memory(bytes)?,
            TextureDataRef::Texels(texels) => {
                let bytes = texels.iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
                image::DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(embedded.width(), embedded.height(), bytes)
                        .context("invalid embedded texture dimensions")?,
                )
            }
        }
    } else {
        let decoded = urlencoding::decode(&info.path).unwrap_or_else(|_| info.path.as_str().into());
        let normalized = decoded.replace('\\', "/");
        let texture_path = Path::new(&normalized);
        image::open(parent.join(texture_path)).or_else(|original| {
            // Exporters often leave an absolute path from another machine.
            texture_path
                .file_name()
                .map(|name| image::open(parent.join(name)))
                .unwrap_or(Err(original))
        })?
    };
    // Thumbnails do not need full-resolution texture copies in memory.
    let image = if image.width() > 1024 || image.height() > 1024 {
        image.thumbnail(1024, 1024)
    } else {
        image
    };
    Ok(Texture::from_image(image).with_wrap(wrap(&info.map_modes[0]), wrap(&info.map_modes[1])))
}

fn wrap(mode: &TextureMapMode) -> WrapMode {
    match mode {
        TextureMapMode::Clamp | TextureMapMode::Decal => WrapMode::ClampToEdge,
        TextureMapMode::Mirror => WrapMode::MirroredRepeat,
        _ => WrapMode::Repeat,
    }
}
