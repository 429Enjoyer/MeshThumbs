use super::{assets::Assets, token, value};
use crate::{Texture, WrapMode};
use anyhow::{ensure, Result};
use glam::{Mat3, Vec2, Vec3};
use openusd::{
    gf, sdf,
    usd::{Attribute, Prim, Stage, TimeCode},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone)]
pub(super) struct Material {
    pub color: [u8; 4],
    pub texture: Option<Arc<Texture>>,
    pub uv_name: String,
    pub uv_transform: Mat3,
    pub bound: bool,
}
impl Default for Material {
    fn default() -> Self {
        Self {
            color: [196, 205, 214, 255],
            texture: None,
            uv_name: "st".into(),
            uv_transform: Mat3::IDENTITY,
            bound: false,
        }
    }
}
#[derive(Default)]
pub(super) struct Materials {
    entries: HashMap<String, Material>,
    images: HashMap<String, Option<Arc<Texture>>>,
}

pub(super) fn binding(prim: &Prim) -> Result<Option<sdf::Path>> {
    for name in ["material:binding:preview", "material:binding"] {
        if let Some(target) = prim
            .relationship(name)
            .forwarded_targets()?
            .into_iter()
            .next()
        {
            return Ok(Some(target.prim_path()));
        }
    }
    Ok(None)
}

pub(super) fn get<'a>(
    stage: &Stage,
    assets: &Assets,
    path: Option<&sdf::Path>,
    time: TimeCode,
    cache: &'a mut Materials,
) -> Result<&'a Material> {
    let key = path.map(ToString::to_string).unwrap_or_default();
    if !cache.entries.contains_key(&key) {
        let mat = if let Some(path) = path {
            load(stage, assets, path, time, &mut cache.images)?
        } else {
            Material::default()
        };
        cache.entries.insert(key.clone(), mat);
    }
    Ok(&cache.entries[&key])
}

fn connected(stage: &Stage, mut attribute: Attribute) -> Result<Option<Prim>> {
    for _ in 0..32 {
        let Some(path) = attribute.connections()?.into_iter().next() else {
            return Ok(None);
        };
        let prim = stage.prim(path.prim_path())?;
        if prim.type_name()?.is_some_and(|n| n.as_str() == "Shader") {
            return Ok(Some(prim));
        }
        attribute = stage.attribute(path)?;
    }
    anyhow::bail!("USD shader connection chain exceeds limit")
}

fn load(
    stage: &Stage,
    assets: &Assets,
    path: &sdf::Path,
    time: TimeCode,
    images: &mut HashMap<String, Option<Arc<Texture>>>,
) -> Result<Material> {
    let prim = stage.prim(path.clone())?;
    let Some(shader) = connected(stage, prim.attribute("outputs:surface"))? else {
        return Ok(Material::default());
    };
    if token(&shader, "info:id", time)?.as_deref() != Some("UsdPreviewSurface") {
        return Ok(Material::default());
    }
    let opacity = value(&shader, "inputs:opacity", time)?
        .map(|v| v.cast::<f32>())
        .transpose()?
        .unwrap_or(1.0);
    let diffuse = value(&shader, "inputs:diffuseColor", time)?
        .map(|v| v.cast::<gf::Vec3f>().map(<[f32; 3]>::from))
        .transpose()?
        .unwrap_or([0.18; 3]);
    let mut mat = Material {
        color: color(diffuse, opacity),
        bound: true,
        ..Default::default()
    };
    if let Some(texture) = connected(stage, shader.attribute("inputs:diffuseColor"))? {
        if token(&texture, "info:id", time)?.as_deref() == Some("UsdUVTexture") {
            let file = texture
                .attribute("inputs:file")
                .get_at::<sdf::AssetPath>(Some(time))?;
            if let Some(file) = file {
                if let Some(path) = file.resolved_path() {
                    let image = images
                        .entry(path.into())
                        .or_insert_with(|| {
                            let bytes = assets.read(path).ok()?;
                            let image = image::ImageReader::new(std::io::Cursor::new(bytes))
                                .with_guessed_format()
                                .ok()?
                                .decode()
                                .ok()?;
                            Some(Arc::new(Texture::from_image(image)))
                        })
                        .clone();
                    if let Some(image) = image {
                        let s = wrap(token(&texture, "inputs:wrapS", time)?.as_deref());
                        let t = wrap(token(&texture, "inputs:wrapT", time)?.as_deref());
                        mat.texture = Some(Arc::new((*image).clone().with_wrap(s, t)));
                        mat.color = [
                            255,
                            255,
                            255,
                            (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
                        ];
                    }
                }
            }
            let mut input = texture.attribute("inputs:st");
            let mut visited = HashSet::new();
            loop {
                let Some(node) = connected(stage, input)? else {
                    break;
                };
                ensure!(
                    visited.len() < 16 && visited.insert(node.path().to_string()),
                    "USD UV connection cycle or depth limit"
                );
                match token(&node, "info:id", time)?.as_deref() {
                    Some("UsdPrimvarReader_float2") => {
                        let variable = if let Some(v) = value(&node, "inputs:varname", time)? {
                            Some(v)
                        } else {
                            let links = node.attribute("inputs:varname").connections()?;
                            links
                                .first()
                                .map(|p| {
                                    stage.attribute(p.clone())?.get_at::<sdf::Value>(Some(time))
                                })
                                .transpose()?
                                .flatten()
                        };
                        if let Some(v) = variable {
                            mat.uv_name = v.cast::<String>()?;
                        }
                        break;
                    }
                    Some("UsdTransform2d") => {
                        let scale = value(&node, "inputs:scale", time)?
                            .map(<[f32; 2]>::try_from)
                            .transpose()?
                            .unwrap_or([1.0; 2]);
                        let shift = value(&node, "inputs:translation", time)?
                            .map(<[f32; 2]>::try_from)
                            .transpose()?
                            .unwrap_or([0.0; 2]);
                        let angle = value(&node, "inputs:rotation", time)?
                            .map(|v| v.cast::<f32>())
                            .transpose()?
                            .unwrap_or(0.0)
                            .to_radians();
                        let scale = Vec2::from(scale);
                        let shift = Vec2::from(shift);
                        ensure!(
                            scale.is_finite() && shift.is_finite() && angle.is_finite(),
                            "invalid USD UV transform"
                        );
                        let (s, c) = angle.sin_cos();
                        mat.uv_transform *= Mat3::from_cols(
                            Vec3::new(c * scale.x, s * scale.x, 0.0),
                            Vec3::new(-s * scale.y, c * scale.y, 0.0),
                            shift.extend(1.0),
                        );
                        input = node.attribute("inputs:in");
                    }
                    _ => break,
                }
            }
        }
    }
    Ok(mat)
}

pub(super) fn color(rgb: [f32; 3], alpha: f32) -> [u8; 4] {
    let channel = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        let srgb = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round() as u8
    };
    [
        channel(rgb[0]),
        channel(rgb[1]),
        channel(rgb[2]),
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
fn wrap(token: Option<&str>) -> WrapMode {
    match token {
        Some("clamp" | "black") => WrapMode::ClampToEdge,
        Some("mirror") => WrapMode::MirroredRepeat,
        _ => WrapMode::Repeat,
    }
}
