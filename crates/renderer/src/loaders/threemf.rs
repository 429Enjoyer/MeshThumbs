//! Read 3MF mesh resources directly so material/UV indices stay per corner.
//! Package parts stay in memory; archive paths are never extracted to disk.
use std::{collections::HashMap, fs::File, io::Read, path::Path, sync::Arc};

use anyhow::{bail, ensure, Context, Result};
use glam::{Mat4, Vec2, Vec3};
use roxmltree::{Document, Node};
use zip::ZipArchive;

use crate::{RenderError, Scene, Texture, Triangle, Vertex, WrapMode, MAX_MODEL_BYTES};

const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const MATERIAL: &str = "http://schemas.microsoft.com/3dmanufacturing/material/2015/02";
const PRODUCTION: &str = "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";
const DEFAULT_COLOR: [u8; 4] = [196, 205, 214, 255];

struct Package {
    zip: ZipArchive<File>,
    remaining: u64,
    triangles: usize,
    budget: usize,
}

struct Model {
    objects: HashMap<u32, Object>,
    build: Vec<Instance>,
    unit: f32,
}

struct Object {
    triangles: Vec<Triangle>,
    components: Vec<Instance>,
}

#[derive(Clone)]
struct Instance {
    part: String,
    id: u32,
    transform: Mat4,
}

enum Property {
    Colors(Vec<[u8; 4]>),
    Texture(Arc<Texture>, Vec<Vec2>),
}

pub(super) fn load(path: &Path, budget: usize) -> Result<Scene> {
    let zip = ZipArchive::new(File::open(path)?)?;
    ensure!(zip.len() <= 20_000, "too many 3MF package parts");
    let mut package = Package {
        zip,
        remaining: MAX_MODEL_BYTES,
        triangles: 0,
        budget,
    };
    let relationships = package.text("_rels/.rels")?;
    let rels = Document::parse(&relationships)?;
    let root = rels
        .descendants()
        .find(|n| {
            n.has_tag_name("Relationship")
                && n.attribute("Type")
                    == Some("http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel")
        })
        .context("3MF package has no model relationship")?;
    ensure!(
        root.attribute("TargetMode") != Some("External"),
        "external 3MF model is unsupported"
    );
    let root = part_path("", attr(root, "Target")?)?;
    let model = package.model(&root)?;
    let root_unit = model.unit;
    // 3MF is Z-up. The renderer uses Y-up; retain a right-handed coordinate system.
    let root_transform = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        * Mat4::from_scale(Vec3::splat(model.unit));
    let mut pending = model
        .build
        .iter()
        .cloned()
        .map(|i| (i, root_transform, root_unit, Vec::new()))
        .collect::<Vec<_>>();
    let mut models = HashMap::from([(root, model)]);
    let mut scene = Scene::new();
    let mut instances = 0;
    while let Some((instance, parent, parent_unit, mut ancestors)) = pending.pop() {
        instances += 1;
        ensure!(
            instances <= 100_000 && ancestors.len() < 128,
            "3MF component hierarchy is too large"
        );
        let key = (instance.part.clone(), instance.id);
        ensure!(!ancestors.contains(&key), "cyclic 3MF component reference");
        ancestors.push(key);
        if !models.contains_key(&instance.part) {
            let model = package.model(&instance.part)?;
            models.insert(instance.part.clone(), model);
        }
        let model = &models[&instance.part];
        let object = model
            .objects
            .get(&instance.id)
            .context("missing 3MF object")?;
        // Component translations use the referring model's units; mesh
        // coordinates use the referenced model's units.
        let transform =
            parent * instance.transform * Mat4::from_scale(Vec3::splat(model.unit / parent_unit));
        let total = scene
            .triangles
            .len()
            .checked_add(object.triangles.len())
            .context("3MF triangle count overflow")?;
        ensure!(
            total <= budget,
            RenderError::TooManyTriangles {
                actual: total,
                limit: budget
            }
        );
        for triangle in &object.triangles {
            let mut triangle = triangle.clone();
            for vertex in &mut triangle.vertices {
                vertex.position = transform.transform_point3(vertex.position);
                // Recompute after transforms, including nonuniform scale and reflections.
                vertex.normal = Vec3::ZERO;
            }
            scene.triangles.push(triangle);
        }
        for component in &object.components {
            pending.push((component.clone(), transform, model.unit, ancestors.clone()));
        }
    }
    Ok(scene)
}

impl Package {
    fn bytes(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut file = self
            .zip
            .by_name(path)
            .with_context(|| format!("missing 3MF part {path}"))?;
        ensure!(
            file.size() <= self.remaining,
            "3MF expanded data exceeds 300 MiB"
        );
        let mut bytes = Vec::new();
        (&mut file)
            .take(self.remaining + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= self.remaining,
            "3MF expanded data exceeds 300 MiB"
        );
        self.remaining -= bytes.len() as u64;
        Ok(bytes)
    }

    fn text(&mut self, path: &str) -> Result<String> {
        Ok(String::from_utf8(self.bytes(path)?)?)
    }

    fn model(&mut self, path: &str) -> Result<Model> {
        let xml = self.text(path)?;
        let doc = Document::parse(&xml)?;
        let root = doc.root_element();
        ensure!(root.has_tag_name((CORE, "model")), "invalid 3MF model root");
        for prefix in root
            .attribute("requiredextensions")
            .unwrap_or("")
            .split_whitespace()
        {
            let ns = root.lookup_namespace_uri(Some(prefix));
            ensure!(
                matches!(ns, Some(CORE | MATERIAL | PRODUCTION)),
                "unsupported required 3MF extension: {prefix}"
            );
        }
        let unit = match root.attribute("unit").unwrap_or("millimeter") {
            "micron" => 0.001,
            "millimeter" => 1.0,
            "centimeter" => 10.0,
            "inch" => 25.4,
            "foot" => 304.8,
            "meter" => 1000.0,
            _ => bail!("unsupported 3MF unit"),
        };
        let resources = child(root, "resources")?;
        let mut textures = HashMap::new();
        for node in resources
            .children()
            .filter(|n| n.has_tag_name((MATERIAL, "texture2d")))
        {
            let id = number(node, "id")?;
            let texture_path = part_path(path, attr(node, "path")?)?;
            let bytes = self.bytes(&texture_path)?;
            let image = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()?
                .decode()?;
            let texture = Texture::from_image(image).with_wrap(
                texture_wrap(node.attribute("tilestyleu")),
                texture_wrap(node.attribute("tilestylev")),
            );
            textures.insert(id, Arc::new(texture));
        }
        let mut properties = HashMap::new();
        for node in resources.children().filter(Node::is_element) {
            let property = match node.tag_name().name() {
                "basematerials" => Property::Colors(
                    node.children()
                        .filter(|n| n.has_tag_name((CORE, "base")))
                        .map(|n| parse_color(attr(n, "displaycolor")?))
                        .collect::<Result<_>>()?,
                ),
                "colorgroup" => Property::Colors(
                    node.children()
                        .filter(|n| n.has_tag_name((MATERIAL, "color")))
                        .map(|n| parse_color(attr(n, "color")?))
                        .collect::<Result<_>>()?,
                ),
                "texture2dgroup" => {
                    let texture = textures
                        .get(&number(node, "texid")?)
                        .cloned()
                        .context("missing 3MF texture")?;
                    let coords = node
                        .children()
                        .filter(|n| n.has_tag_name((MATERIAL, "tex2coord")))
                        .map(|n| Ok(Vec2::new(float(n, "u")?, float(n, "v")?)))
                        .collect::<Result<_>>()?;
                    Property::Texture(texture, coords)
                }
                _ => continue,
            };
            properties.insert(number(node, "id")?, property);
        }
        let mut objects = HashMap::new();
        for node in resources
            .children()
            .filter(|n| n.has_tag_name((CORE, "object")))
        {
            let mut object = Object {
                triangles: Vec::new(),
                components: Vec::new(),
            };
            if let Some(mesh) = node.children().find(|n| n.has_tag_name((CORE, "mesh"))) {
                let vertices = child(mesh, "vertices")?
                    .children()
                    .filter(|n| n.has_tag_name((CORE, "vertex")))
                    .map(|n| Ok(Vec3::new(float(n, "x")?, float(n, "y")?, float(n, "z")?)))
                    .collect::<Result<Vec<_>>>()?;
                let default_pid = optional_number(node, "pid")?;
                let default_index = optional_number(node, "pindex")?;
                for face in child(mesh, "triangles")?
                    .children()
                    .filter(|n| n.has_tag_name((CORE, "triangle")))
                {
                    self.triangles += 1;
                    ensure!(
                        self.triangles <= self.budget,
                        RenderError::TooManyTriangles {
                            actual: self.triangles,
                            limit: self.budget
                        }
                    );
                    let indices = [
                        number(face, "v1")?,
                        number(face, "v2")?,
                        number(face, "v3")?,
                    ];
                    let mut corners = [Vertex {
                        position: Vec3::ZERO,
                        normal: Vec3::ZERO,
                        uv: Vec2::ZERO,
                        color: [255; 4],
                    }; 3];
                    for (corner, index) in corners.iter_mut().zip(indices) {
                        corner.position = *vertices
                            .get(index as usize)
                            .context("invalid 3MF vertex index")?;
                    }
                    let mut texture = None;
                    let pid = optional_number(face, "pid")?.or(default_pid);
                    let color = if let Some(pid) = pid {
                        let p1 = optional_number(face, "p1")?
                            .or(default_index)
                            .context("missing 3MF property index")?;
                        let indices = [
                            p1,
                            optional_number(face, "p2")?.unwrap_or(p1),
                            optional_number(face, "p3")?.unwrap_or(p1),
                        ];
                        match properties
                            .get(&pid)
                            .context("unsupported or missing 3MF property group")?
                        {
                            Property::Colors(colors) => {
                                for (corner, index) in corners.iter_mut().zip(indices) {
                                    corner.color = *colors
                                        .get(index as usize)
                                        .context("invalid 3MF color index")?;
                                }
                            }
                            Property::Texture(image, coords) => {
                                texture = Some(image.clone());
                                for (corner, index) in corners.iter_mut().zip(indices) {
                                    corner.uv = *coords
                                        .get(index as usize)
                                        .context("invalid 3MF UV index")?;
                                }
                            }
                        }
                        [255; 4]
                    } else {
                        DEFAULT_COLOR
                    };
                    object.triangles.push(Triangle {
                        vertices: corners,
                        color,
                        texture,
                    });
                }
            }
            if let Some(components) = node
                .children()
                .find(|n| n.has_tag_name((CORE, "components")))
            {
                object.components = components
                    .children()
                    .filter(|n| n.has_tag_name((CORE, "component")))
                    .map(|n| instance(n, path))
                    .collect::<Result<_>>()?;
            }
            ensure!(
                objects.insert(number(node, "id")?, object).is_none(),
                "duplicate 3MF object id"
            );
        }
        let build = root
            .children()
            .find(|n| n.has_tag_name((CORE, "build")))
            .map(|build| {
                build
                    .children()
                    .filter(|n| n.has_tag_name((CORE, "item")))
                    .map(|n| instance(n, path))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Model {
            objects,
            build,
            unit,
        })
    }
}

fn instance(node: Node<'_, '_>, current: &str) -> Result<Instance> {
    let part = node
        .attribute((PRODUCTION, "path"))
        .map(|p| part_path(current, p))
        .transpose()?
        .unwrap_or_else(|| current.to_owned());
    let transform = if let Some(value) = node.attribute("transform") {
        let values = value
            .split_whitespace()
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()?;
        ensure!(
            values.len() == 12 && values.iter().all(|v| v.is_finite()),
            "invalid 3MF transform"
        );
        Mat4::from_cols_array(&[
            values[0], values[1], values[2], 0.0, values[3], values[4], values[5], 0.0, values[6],
            values[7], values[8], 0.0, values[9], values[10], values[11], 1.0,
        ])
    } else {
        Mat4::IDENTITY
    };
    Ok(Instance {
        part,
        id: number(node, "objectid")?,
        transform,
    })
}

fn part_path(current: &str, target: &str) -> Result<String> {
    let decoded = urlencoding::decode(target)?;
    ensure!(
        !decoded.contains(['\\', ':', '\0', '?', '#']),
        "invalid 3MF part URI"
    );
    let base = if decoded.starts_with('/') {
        ""
    } else {
        current.rsplit_once('/').map(|p| p.0).unwrap_or("")
    };
    let joined = format!("{base}/{decoded}");
    let mut parts = Vec::new();
    for segment in joined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                ensure!(parts.pop().is_some(), "3MF URI escapes package");
            }
            _ => parts.push(segment),
        }
    }
    ensure!(!parts.is_empty(), "empty 3MF part URI");
    Ok(parts.join("/"))
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Node<'a, 'input>> {
    node.children()
        .find(|n| n.has_tag_name((CORE, name)))
        .with_context(|| format!("missing 3MF {name}"))
}
fn attr<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str> {
    node.attribute(name)
        .with_context(|| format!("missing 3MF attribute {name}"))
}
fn number(node: Node<'_, '_>, name: &str) -> Result<u32> {
    Ok(attr(node, name)?.parse()?)
}
fn optional_number(node: Node<'_, '_>, name: &str) -> Result<Option<u32>> {
    Ok(node.attribute(name).map(str::parse).transpose()?)
}
fn float(node: Node<'_, '_>, name: &str) -> Result<f32> {
    let value: f32 = attr(node, name)?.parse()?;
    ensure!(value.is_finite(), "nonfinite 3MF coordinate");
    Ok(value)
}
fn parse_color(value: &str) -> Result<[u8; 4]> {
    ensure!(
        value.is_ascii() && value.starts_with('#') && matches!(value.len(), 7 | 9),
        "invalid 3MF color"
    );
    let mut color = [255; 4];
    for (i, channel) in color.iter_mut().enumerate().take((value.len() - 1) / 2) {
        *channel = u8::from_str_radix(&value[1 + i * 2..3 + i * 2], 16)?;
    }
    Ok(color)
}
fn texture_wrap(value: Option<&str>) -> WrapMode {
    match value {
        None | Some("wrap") => WrapMode::Repeat,
        Some("mirror") => WrapMode::MirroredRepeat,
        _ => WrapMode::ClampToEdge,
    }
}
