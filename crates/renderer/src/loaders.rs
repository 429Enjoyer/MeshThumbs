use std::{path::Path, sync::Arc};

use anyhow::{bail, Context};
use glam::{Mat4, Vec2, Vec3};
use gltf::{image::Format, texture::WrappingMode};

use crate::{RenderError, Scene, Texture, Triangle, Vertex, WrapMode};

mod dxf;
mod legacy;
mod lws;
mod native_scene;
mod off;
mod pcx;
mod step;
mod threemf;
mod usd;
mod vox;
mod vrm;
mod vrml;
mod x3d;

pub(crate) fn load_scene(path: &Path, max_triangles: usize) -> Result<Scene, RenderError> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "obj" => load_obj(path).map_err(RenderError::Load),
        "glb" | "gltf" => load_gltf(path).map_err(RenderError::Load),
        "vrm" => vrm::load(path).map_err(RenderError::Load),
        "3mf" => threemf::load(path, max_triangles).map_err(RenderError::Load),
        "vox" => vox::load(path, max_triangles).map_err(RenderError::Load),
        "abc" | "3dm" => native_scene::load(path, max_triangles).map_err(RenderError::Load),
        "step" | "stp" | "igs" | "iges" => {
            step::load(path, max_triangles).map_err(RenderError::Load)
        }
        "usd" | "usda" | "usdc" | "usdz" => {
            usd::load(path, max_triangles).map_err(RenderError::Load)
        }
        "fbx" => load_fbx(path).map_err(RenderError::Load),
        "stl" | "ply" | "dae" | "3ds" | "x3d" | "off" | "wrl" | "vrml" | "ifc" | "pmx" | "lwo"
        | "smd" | "md2" | "md3" | "md5mesh" | "ase" | "lxo" | "lws" | "dxf" => {
            legacy::load(path, max_triangles).map_err(RenderError::Load)
        }
        _ => Err(RenderError::UnsupportedFormat),
    }
}

fn load_obj(path: &Path) -> anyhow::Result<Scene> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let load_options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };
    let (models, materials) = tobj::load_obj(path, &load_options)
        .with_context(|| format!("failed to load OBJ {}", path.display()))?;
    let materials = materials.unwrap_or_default();
    let textures = materials
        .iter()
        .map(|m| {
            m.diffuse_texture
                .as_deref()
                .and_then(|name| load_texture(parent.join(name)).ok())
                .map(Arc::new)
        })
        .collect::<Vec<_>>();

    let mut scene = Scene::new();
    for model in models {
        let mesh = model.mesh;
        let color = mesh
            .material_id
            .and_then(|id| materials.get(id))
            .and_then(|m| m.diffuse)
            .map(|c| to_rgba(c[0], c[1], c[2], 1.0))
            .unwrap_or([196, 205, 214, 255]);
        let texture = mesh
            .material_id
            .and_then(|id| textures.get(id))
            .cloned()
            .flatten();

        for tri in mesh.indices.chunks_exact(3) {
            let vertices = [
                obj_vertex(&mesh, tri[0] as usize),
                obj_vertex(&mesh, tri[1] as usize),
                obj_vertex(&mesh, tri[2] as usize),
            ];
            scene.triangles.push(Triangle {
                vertices: fix_normals(vertices),
                color,
                texture: texture.clone(),
            });
        }
    }

    Ok(scene)
}

fn load_gltf(path: &Path) -> anyhow::Result<Scene> {
    let source = gltf::Gltf::open(path)
        .with_context(|| format!("failed to load glTF {}", path.display()))?;
    load_gltf_source(path, source, Mat4::IDENTITY)
}

fn load_gltf_source(
    path: &Path,
    source: gltf::Gltf,
    root_transform: Mat4,
) -> anyhow::Result<Scene> {
    let document = source.document;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let buffers = gltf::import_buffers(&document, Some(base), source.blob)?;
    let used_images = document
        .materials()
        .filter_map(|material| {
            material
                .pbr_metallic_roughness()
                .base_color_texture()
                .map(|info| info.texture().source().index())
        })
        .collect::<std::collections::HashSet<_>>();
    // A missing or corrupt optional image must not discard valid geometry.
    let textures = document
        .images()
        .map(|image| {
            if !used_images.contains(&image.index()) {
                return None;
            }
            if let gltf::image::Source::View { view, .. } = image.source() {
                let buffer = buffers.get(view.buffer().index())?;
                buffer.get(view.offset()..view.offset().checked_add(view.length())?)?;
            }
            gltf::image::Data::from_source(image.source(), Some(base), &buffers)
                .ok()
                .and_then(gltf_image_to_texture)
                .map(Arc::new)
        })
        .collect::<Vec<_>>();

    let mut scene = Scene::new();
    if let Some(default_scene) = document
        .default_scene()
        .or_else(|| document.scenes().next())
    {
        for node in default_scene.nodes() {
            load_gltf_node(node, root_transform, &buffers, &textures, &mut scene);
        }
    }

    Ok(scene)
}

fn load_gltf_node(
    node: gltf::Node<'_>,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    textures: &[Option<Arc<Texture>>],
    scene: &mut Scene,
) {
    let mut stack = vec![(node, parent_transform)];
    let mut visited = std::collections::HashSet::new();
    while let Some((node, parent)) = stack.pop() {
        if !visited.insert(node.index()) {
            continue;
        }
        let transform = parent * gltf_transform(node.transform());
        if let Some(mesh) = node.mesh() {
            load_gltf_mesh(mesh, transform, buffers, textures, scene);
        }
        stack.extend(node.children().map(|child| (child, transform)));
    }
}

fn load_gltf_mesh(
    mesh: gltf::Mesh<'_>,
    transform: Mat4,
    buffers: &[gltf::buffer::Data],
    textures: &[Option<Arc<Texture>>],
    scene: &mut Scene,
) {
    let normal_transform = transform.inverse().transpose();

    for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let Some(positions) = reader.read_positions() else {
            continue;
        };
        let positions = positions.map(Vec3::from).collect::<Vec<_>>();
        let normals = reader
            .read_normals()
            .map(|n| n.map(Vec3::from).collect::<Vec<_>>())
            .unwrap_or_default();
        let indices = reader
            .read_indices()
            .map(|i| i.into_u32().collect::<Vec<_>>())
            .unwrap_or_else(|| (0..positions.len() as u32).collect());

        let vertex_colors = reader
            .read_colors(0)
            .map(|c| c.into_rgba_f32().collect::<Vec<_>>())
            .unwrap_or_default();
        let mat = primitive.material();
        let pbr = mat.pbr_metallic_roughness();
        let base = pbr.base_color_factor();
        let color = to_rgba(base[0], base[1], base[2], base[3]);
        let base_color_texture = pbr.base_color_texture();
        let texcoord_set = base_color_texture
            .as_ref()
            .and_then(|info| {
                info.texture_transform()
                    .and_then(|transform| transform.tex_coord())
            })
            .or_else(|| base_color_texture.as_ref().map(|info| info.tex_coord()))
            .unwrap_or(0);
        let texture_transform = base_color_texture
            .as_ref()
            .and_then(|info| info.texture_transform())
            .map(|transform| TextureTransformData {
                offset: Vec2::from(transform.offset()),
                scale: Vec2::from(transform.scale()),
                rotation: transform.rotation(),
            })
            .unwrap_or_default();
        let texcoords = reader
            .read_tex_coords(texcoord_set)
            .map(|t| {
                t.into_f32()
                    .map(Vec2::from)
                    .map(|uv| texture_transform.apply(uv))
                    .collect::<Vec<_>>()
            })
            .or_else(|| {
                if texcoord_set != 0 {
                    reader
                        .read_tex_coords(0)
                        .map(|t| t.into_f32().map(Vec2::from).collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let texture = base_color_texture.as_ref().and_then(|info| {
            textures
                .get(info.texture().source().index())
                .cloned()
                .flatten()
                .map(|texture| {
                    let sampler = info.texture().sampler();
                    Arc::new(
                        (*texture)
                            .clone()
                            .with_wrap(wrap_mode(sampler.wrap_s()), wrap_mode(sampler.wrap_t())),
                    )
                })
        });

        for tri in topology_indices(&indices, primitive.mode()) {
            if tri.iter().any(|&i| i as usize >= positions.len()) {
                continue;
            }
            let vertices = tri.map(|i| {
                let mut v = gltf_vertex(
                    &positions,
                    &normals,
                    &texcoords,
                    i as usize,
                    transform,
                    normal_transform,
                );
                if let Some(c) = vertex_colors.get(i as usize) {
                    v.color = to_rgba(c[0], c[1], c[2], c[3]);
                }
                v
            });
            scene.triangles.push(Triangle {
                vertices: fix_normals(vertices),
                color,
                texture: texture.clone(),
            });
        }
    }
}

fn load_fbx(path: &Path) -> anyhow::Result<Scene> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let path_utf8 = path.to_str().context("FBX path must be UTF-8")?;
    let source = ufbx::load_file(path_utf8, ufbx::LoadOpts::default())
        .map_err(|error| anyhow::anyhow!("failed to load FBX: {:?}", error))?;
    let mut scene = Scene::new();
    for node in &source.nodes {
        let Some(mesh) = node.mesh.as_ref() else {
            continue;
        };
        let transform = fbx_transform(&node.geometry_to_world);
        let normal_transform = transform.inverse().transpose();
        let materials = (&node.materials)
            .into_iter()
            .map(|material| fbx_material_data(material, parent))
            .collect::<Vec<_>>();
        let mut indices = Vec::new();
        for (face_index, face) in (&mesh.faces).into_iter().enumerate() {
            let count = ufbx::triangulate_face_vec(&mut indices, mesh, *face);
            if count == 0 {
                continue;
            }
            let material = mesh
                .face_material
                .get(face_index)
                .and_then(|i| materials.get(*i as usize))
                .cloned()
                .unwrap_or_else(default_material_data);
            for tri in indices.chunks_exact(3) {
                let vertices = [
                    fbx_vertex(mesh, tri[0] as usize),
                    fbx_vertex(mesh, tri[1] as usize),
                    fbx_vertex(mesh, tri[2] as usize),
                ];
                let vertices = vertices.map(|mut v| {
                    v.position = transform.transform_point3(v.position);
                    v.normal = normal_transform.transform_vector3(v.normal);
                    v
                });
                scene.triangles.push(Triangle {
                    vertices: fix_normals(vertices),
                    color: material.0,
                    texture: material.1.clone(),
                });
            }
        }
    }

    Ok(scene)
}

fn load_texture(path: impl AsRef<Path>) -> anyhow::Result<Texture> {
    Ok(Texture::from_image(image::open(path)?))
}

fn gltf_image_to_texture(image: gltf::image::Data) -> Option<Texture> {
    let mut rgba = Vec::with_capacity((image.width * image.height * 4) as usize);

    match image.format {
        Format::R8 => {
            for r in image.pixels {
                rgba.extend_from_slice(&[r, r, r, 255]);
            }
        }
        Format::R8G8 => {
            for px in image.pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        Format::R8G8B8 => {
            for px in image.pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        Format::R8G8B8A8 => rgba = image.pixels,
        Format::R16 => {
            for px in image.pixels.chunks_exact(2) {
                let r = px[1];
                rgba.extend_from_slice(&[r, r, r, 255]);
            }
        }
        Format::R16G16 => {
            for px in image.pixels.chunks_exact(4) {
                rgba.extend_from_slice(&[px[1], px[1], px[1], px[3]]);
            }
        }
        Format::R16G16B16 => {
            for px in image.pixels.chunks_exact(6) {
                rgba.extend_from_slice(&[px[1], px[3], px[5], 255]);
            }
        }
        Format::R16G16B16A16 => {
            for px in image.pixels.chunks_exact(8) {
                rgba.extend_from_slice(&[px[1], px[3], px[5], px[7]]);
            }
        }
        Format::R32G32B32FLOAT | Format::R32G32B32A32FLOAT => return None,
    }

    Texture::from_gltf_image(image.width, image.height, rgba)
}

fn wrap_mode(mode: WrappingMode) -> WrapMode {
    match mode {
        WrappingMode::ClampToEdge => WrapMode::ClampToEdge,
        WrappingMode::MirroredRepeat => WrapMode::MirroredRepeat,
        WrappingMode::Repeat => WrapMode::Repeat,
    }
}

type MaterialData = ([u8; 4], Option<Arc<Texture>>);

#[derive(Clone, Copy)]
struct TextureTransformData {
    offset: Vec2,
    scale: Vec2,
    rotation: f32,
}

impl Default for TextureTransformData {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }
}

impl TextureTransformData {
    fn apply(self, uv: Vec2) -> Vec2 {
        let scaled = uv * self.scale;
        let (sin, cos) = self.rotation.sin_cos();
        Vec2::new(
            scaled.x * cos - scaled.y * sin,
            scaled.x * sin + scaled.y * cos,
        ) + self.offset
    }
}

fn default_material_data() -> MaterialData {
    ([198, 202, 210, 255], None)
}

fn fbx_material_data(material: &ufbx::Material, parent: &Path) -> MaterialData {
    let pbr = &material.pbr.base_color;
    let fbx = &material.fbx.diffuse_color;
    let color = if pbr.has_value {
        vec4_to_rgba(pbr.value_vec4)
    } else if fbx.has_value {
        vec4_to_rgba(fbx.value_vec4)
    } else {
        default_material_data().0
    };

    let texture = pbr
        .texture
        .as_ref()
        .or_else(|| fbx.texture.as_ref())
        .and_then(|texture| load_fbx_texture(texture, parent).ok())
        .map(Arc::new);

    (color, texture)
}

fn load_fbx_texture(texture: &ufbx::Texture, parent: &Path) -> anyhow::Result<Texture> {
    if texture.content.size > 0 {
        return Ok(Texture::from_image(image::load_from_memory(
            &texture.content,
        )?));
    }

    for name in [
        texture.absolute_filename.as_ref(),
        texture.relative_filename.as_ref(),
        texture.filename.as_ref(),
    ] {
        if name.is_empty() {
            continue;
        }
        let path = Path::new(name);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            parent.join(path)
        };
        if let Ok(texture) = load_texture(&resolved) {
            return Ok(texture);
        }
    }

    bail!("FBX texture file not found");
}

fn obj_vertex(mesh: &tobj::Mesh, i: usize) -> Vertex {
    Vertex {
        position: read_vec3(&mesh.positions, i).unwrap_or(Vec3::ZERO),
        normal: read_vec3(&mesh.normals, i).unwrap_or(Vec3::ZERO),
        uv: read_vec2(&mesh.texcoords, i).unwrap_or(Vec2::ZERO),
        color: read_vec3(&mesh.vertex_color, i)
            .map(|c| to_rgba(c.x, c.y, c.z, 1.0))
            .unwrap_or([255; 4]),
    }
}

fn gltf_vertex(
    positions: &[Vec3],
    normals: &[Vec3],
    texcoords: &[Vec2],
    i: usize,
    transform: Mat4,
    normal_transform: Mat4,
) -> Vertex {
    Vertex {
        position: transform.transform_point3(positions.get(i).copied().unwrap_or(Vec3::ZERO)),
        normal: normal_transform.transform_vector3(normals.get(i).copied().unwrap_or(Vec3::ZERO)),
        uv: texcoords.get(i).copied().unwrap_or(Vec2::ZERO),
        color: [255; 4],
    }
}

fn gltf_transform(transform: gltf::scene::Transform) -> Mat4 {
    Mat4::from_cols_array_2d(&transform.matrix())
}

fn topology_indices(
    indices: &[u32],
    mode: gltf::mesh::Mode,
) -> impl Iterator<Item = [u32; 3]> + '_ {
    use gltf::mesh::Mode;
    let count = match mode {
        Mode::Triangles => indices.len() / 3,
        Mode::TriangleStrip | Mode::TriangleFan => indices.len().saturating_sub(2),
        _ => 0, // Lines and points must not be mistaken for triangles.
    };
    (0..count).map(move |i| match mode {
        Mode::TriangleStrip if i % 2 == 1 => [indices[i + 1], indices[i], indices[i + 2]],
        Mode::TriangleStrip => [indices[i], indices[i + 1], indices[i + 2]],
        Mode::TriangleFan => [indices[0], indices[i + 1], indices[i + 2]],
        _ => [indices[i * 3], indices[i * 3 + 1], indices[i * 3 + 2]],
    })
}

fn fbx_transform(m: &ufbx::Matrix) -> Mat4 {
    Mat4::from_cols_array(&[
        m.m00 as f32,
        m.m10 as f32,
        m.m20 as f32,
        0.0,
        m.m01 as f32,
        m.m11 as f32,
        m.m21 as f32,
        0.0,
        m.m02 as f32,
        m.m12 as f32,
        m.m22 as f32,
        0.0,
        m.m03 as f32,
        m.m13 as f32,
        m.m23 as f32,
        1.0,
    ])
}

fn fbx_vertex(mesh: &ufbx::Mesh, i: usize) -> Vertex {
    let p = mesh.vertex_position[i];
    let n = if mesh.vertex_normal.exists {
        mesh.vertex_normal[i]
    } else {
        ufbx::Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    };
    let uv = if mesh.vertex_uv.exists {
        mesh.vertex_uv[i]
    } else {
        ufbx::Vec2 { x: 0.0, y: 0.0 }
    };
    Vertex {
        position: Vec3::new(p.x as f32, p.y as f32, p.z as f32),
        normal: Vec3::new(n.x as f32, n.y as f32, n.z as f32),
        uv: Vec2::new(uv.x as f32, uv.y as f32),
        color: if mesh.vertex_color.exists {
            vec4_to_rgba(mesh.vertex_color[i])
        } else {
            [255; 4]
        },
    }
}

fn read_vec3(data: &[f32], index: usize) -> Option<Vec3> {
    let i = index * 3;
    Some(Vec3::new(
        *data.get(i)?,
        *data.get(i + 1)?,
        *data.get(i + 2)?,
    ))
}

fn read_vec2(data: &[f32], index: usize) -> Option<Vec2> {
    let i = index * 2;
    Some(Vec2::new(*data.get(i)?, *data.get(i + 1)?))
}

pub(crate) fn fix_normals(mut vertices: [Vertex; 3]) -> [Vertex; 3] {
    let face = (vertices[1].position - vertices[0].position)
        .cross(vertices[2].position - vertices[0].position)
        .normalize_or_zero();
    for vertex in &mut vertices {
        if !vertex.normal.is_finite() || vertex.normal.length_squared() < 0.01 {
            vertex.normal = face;
        }
    }
    vertices
}

fn to_rgba(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
    [
        (r.clamp(0.0, 1.0) * 255.0) as u8,
        (g.clamp(0.0, 1.0) * 255.0) as u8,
        (b.clamp(0.0, 1.0) * 255.0) as u8,
        (a.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

fn vec4_to_rgba(v: ufbx::Vec4) -> [u8; 4] {
    to_rgba(v.x as f32, v.y as f32, v.z as f32, v.w as f32)
}
