//! Compatibility importers. The original OBJ/FBX/glTF paths do not use Assimp.
use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{bail, Context};
use asset_importer::{
    material::{Material, TextureMapMode, TextureType},
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
    let importer = Importer::new();
    let normalized = match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => Some((super::off::to_ply(path, budget)?, "ply")),
        "x3d" => Some((super::x3d::normalize(path)?, "x3d")),
        "wrl" | "vrml" => Some((super::vrml::normalize(path)?, "x3d")),
        "md5mesh" => Some((normalize_md5(path)?, "md5mesh")),
        "ase" => Some((normalize_ase(&std::fs::read(path)?)?, "ase")),
        _ => None,
    };
    let request = match &normalized {
        Some((data, hint)) => importer.read_from_memory(data).with_memory_hint(*hint),
        None => importer.read_file(path),
    };
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let game_model = matches!(extension.as_str(), "smd" | "md2" | "md3" | "md5mesh");
    let request = if extension == "lws" {
        let (name, files) = super::lws::files(path)?;
        importer
            .read_file(name)
            .with_file_system(files)
            .with_property_int("IMPORT_LWS_ANIM_START", 0)
            .with_property_int("IMPORT_LWS_ANIM_END", 0)
            .with_property_bool("IMPORT_NO_SKELETON_MESHES", true)
    } else if extension == "ase" {
        request.with_property_bool("IMPORT_NO_SKELETON_MESHES", true)
    } else {
        request
    };
    let request = if game_model {
        // Preview one mesh in its reference pose / first vertex frame. Do not
        // auto-assemble adjacent MD3 parts or load animation/shader scripts.
        request
            .with_property_int("IMPORT_MD2_KEYFRAME", 0)
            .with_property_int("IMPORT_MD3_KEYFRAME", 0)
            .with_property_bool("IMPORT_MD3_HANDLE_MULTIPART", false)
            .with_property_bool("IMPORT_MD3_LOAD_SHADERS", false)
            .with_property_bool("IMPORT_MD5_NO_ANIM_AUTOLOAD", true)
            .with_property_bool("IMPORT_SMD_LOAD_ANIMATION_LIST", false)
            .with_property_bool("IMPORT_NO_SKELETON_MESHES", true)
    } else {
        request
    };
    let generated_uvs = if path.extension().is_some_and(|e| {
        ["x3d", "wrl", "vrml"]
            .iter()
            .any(|ext| e.eq_ignore_ascii_case(ext))
    }) {
        PostProcessSteps::GEN_UV_COORDS
    } else {
        PostProcessSteps::empty()
    };
    let imported = request
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::PRE_TRANSFORM_VERTICES
                | PostProcessSteps::TRANSFORM_UV_COORDS
                | PostProcessSteps::VALIDATE_DATA_STRUCTURE
                | generated_uvs,
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
            let info = texture_binding(&mat, TextureType::BaseColor)
                .or_else(|| texture_binding(&mat, TextureType::Diffuse));
            let texture = info.as_ref().and_then(|info| {
                // Include the sampler in the cache key; two materials can share an image.
                let key = format!("{}:{:?}", info.path, info.map_modes);
                texture_cache
                    .entry(key)
                    .or_insert_with(|| {
                        load_texture(&imported, parent, info, game_model)
                            .ok()
                            .map(Arc::new)
                    })
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
            let mut vertices = std::array::from_fn(|corner| {
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
            if extension == "smd" {
                // SMD positions are model-space and Z-up; unlike the Quake
                // importers, Assimp's SMD reader leaves this basis unchanged.
                for vertex in &mut vertices {
                    let p = vertex.position;
                    let n = vertex.normal;
                    vertex.position = Vec3::new(p.x, p.z, -p.y);
                    vertex.normal = Vec3::new(n.x, n.z, -n.y);
                }
            }
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

struct TextureBinding {
    path: String,
    uv_index: u32,
    map_modes: [TextureMapMode; 2],
}

fn texture_binding(mat: &Material, kind: TextureType) -> Option<TextureBinding> {
    // asset-importer 0.8 texture() assumes optional Assimp outputs are always
    // initialized. Missing $tex.uvwsrc (common in MD2) can yield random indices.
    // Read the properties directly with explicit defaults, avoiding that API.
    let mut binding = TextureBinding {
        path: String::new(),
        uv_index: 0,
        map_modes: [TextureMapMode::Wrap; 2],
    };
    for property in mat
        .properties()
        .filter(|p| p.semantic() == Some(kind) && p.index() == 0)
    {
        match property.key_str().as_ref() {
            "$tex.file" => binding.path = property.string_ref()?.as_str().into_owned(),
            "$tex.uvwsrc" => binding.uv_index = property.as_u32().unwrap_or(0),
            "$tex.mapmodeu" | "$tex.mapmodev" => {
                let i = usize::from(property.key_str() == "$tex.mapmodev");
                binding.map_modes[i] = match property.as_i32().unwrap_or(0) {
                    1 => TextureMapMode::Clamp,
                    2 => TextureMapMode::Mirror,
                    3 => TextureMapMode::Decal,
                    _ => TextureMapMode::Wrap,
                };
            }
            _ => {}
        }
    }
    (!binding.path.is_empty()).then_some(binding)
}

fn load_texture(
    scene: &asset_importer::Scene,
    parent: &Path,
    info: &TextureBinding,
    game_model: bool,
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
        open_image(&parent.join(texture_path), game_model).or_else(|original| {
            // Exporters often leave an absolute path from another machine.
            texture_path
                .file_name()
                .map(|name| open_image(&parent.join(name), game_model))
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

fn open_image(path: &Path, game_model: bool) -> anyhow::Result<image::DynamicImage> {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pcx"))
    {
        return super::pcx::load(path);
    }
    if game_model && path.extension().is_none() {
        // Game materials commonly store an image name without its suffix.
        // Resolve only beside the declared path; do not search game installs.
        for extension in ["tga", "png", "jpg", "jpeg", "dds", "bmp", "pcx"] {
            if let Ok(image) = open_image(&path.with_extension(extension), false) {
                return Ok(image);
            }
        }
    }
    Ok(image::open(path)?)
}

fn normalize_md5(path: &Path) -> anyhow::Result<Vec<u8>> {
    // Assimp 6.0.5 can consume the next global declaration when a value is
    // immediately followed by a lone LF. Normalize newlines without changing
    // names, comments, geometry, or texture paths; the reader needs no sidecars.
    let bytes = std::fs::read(path)?;
    md5_line_endings(&bytes)
}

fn normalize_ase(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let result = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let (pairs, remainder) = bytes[2..].as_chunks::<2>();
        if !remainder.is_empty() {
            bail!("truncated UTF-16 ASE text");
        }
        let words = pairs
            .iter()
            .map(|p| {
                if bytes[0] == 0xff {
                    u16::from_le_bytes(*p)
                } else {
                    u16::from_be_bytes(*p)
                }
            })
            .collect::<Vec<_>>();
        String::from_utf16(&words)?.into_bytes()
    } else {
        bytes
            .strip_prefix(&[0xef, 0xbb, 0xbf])
            .unwrap_or(bytes)
            .to_vec()
    };
    if result.len() > MAX_MODEL_BYTES as usize {
        bail!("normalized ASE exceeds 300 MiB");
    }
    Ok(result)
}

fn md5_line_endings(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let extra = bytes
        .iter()
        .enumerate()
        .filter(|&(i, &b)| b == b'\n' && (i == 0 || bytes[i - 1] != b'\r'))
        .count();
    if bytes.len() + extra > MAX_MODEL_BYTES as usize {
        bail!("normalized MD5 mesh exceeds the 300 MiB limit");
    }
    let mut result = Vec::with_capacity(bytes.len() + extra);
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' && (i == 0 || bytes[i - 1] != b'\r') {
            result.push(b'\r');
        }
        result.push(b);
    }
    Ok(result)
}

fn wrap(mode: &TextureMapMode) -> WrapMode {
    match mode {
        TextureMapMode::Clamp | TextureMapMode::Decal => WrapMode::ClampToEdge,
        TextureMapMode::Mirror => WrapMode::MirroredRepeat,
        _ => WrapMode::Repeat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ase_bom_encodings_match_plain_text() {
        let text = "*3DSMAX_ASCIIEXPORT 200\n*COMMENT \"Mesh 日本語\"\n";
        for little in [true, false] {
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for word in text.encode_utf16() {
                bytes.extend(if little {
                    word.to_le_bytes()
                } else {
                    word.to_be_bytes()
                });
            }
            assert_eq!(normalize_ase(&bytes).unwrap(), text.as_bytes());
        }
        assert!(normalize_ase(&[0xff, 0xfe, 1]).is_err());
    }

    #[test]
    fn missing_texture_uv_property_defaults_to_channel_zero() {
        let imported = Importer::new()
            .read_from_memory(include_bytes!("../../tests/fixtures/Crate.md2"))
            .with_memory_hint("md2")
            .import()
            .unwrap();
        let material = imported.materials().next().unwrap();
        assert!(material
            .get_property_raw_ref(c"$tex.uvwsrc", Some(TextureType::Diffuse), 0)
            .is_none());
        let binding = texture_binding(&material, TextureType::Diffuse).unwrap();
        assert_eq!(binding.uv_index, 0);
        assert_eq!(binding.path, "assets/Crate.pcx");
        assert!(matches!(
            binding.map_modes,
            [TextureMapMode::Wrap, TextureMapMode::Wrap]
        ));
    }

    #[test]
    fn md5_normalization_preserves_existing_crlf_and_text() {
        let input = b"numJoints 2\nnumMeshes 1\r\nshader \"assets/Mech\"\n";
        let out = md5_line_endings(input).unwrap();
        assert_eq!(
            out,
            b"numJoints 2\r\nnumMeshes 1\r\nshader \"assets/Mech\"\r\n"
        );
        assert_eq!(md5_line_endings(&out).unwrap(), out);
    }

    #[test]
    fn md5_lf_mesh_retains_weighted_bind_pose() {
        let lf = include_bytes!("../../../../examples/Mech.md5mesh")
            .iter()
            .copied()
            .filter(|&b| b != b'\r')
            .collect::<Vec<_>>();
        let data = md5_line_endings(&lf).unwrap();
        let imported = Importer::new()
            .read_from_memory(&data)
            .with_memory_hint("md5mesh")
            .with_property_bool("IMPORT_MD5_NO_ANIM_AUTOLOAD", true)
            .import()
            .unwrap();
        let mesh = imported.meshes().next().unwrap();
        let vertices = mesh.vertices_raw();
        // Multiple weighted joints must reconstruct the authored mesh rather
        // than collapsing every vertex to the origin on LF-only input.
        assert!(vertices.iter().any(|v| v.z > 3.4));
        assert!(vertices.iter().any(|v| v.x > 1.0));
        assert!(vertices.iter().any(|v| v.x < -1.0));
        assert_eq!(mesh.faces().count(), 504);
    }
}
