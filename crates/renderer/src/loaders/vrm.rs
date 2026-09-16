//! VRM 0.x/1.0 static previews. MToon uses its base color/texture fallback;
//! outlines, spring bones, expressions, and animation are not evaluated.
use std::{fs::File, io::BufReader, path::Path};

use anyhow::{ensure, Context};
use glam::Mat4;
use serde_json::{json, Value};

use crate::Scene;

pub(super) fn load(path: &Path) -> anyhow::Result<Scene> {
    let glb = gltf::binary::Glb::from_reader(BufReader::new(File::open(path)?))
        .context("VRM must be a binary glTF 2.0 file")?;
    let mut root: Value = serde_json::from_slice(&glb.json)?;
    let vrm0 = root.pointer("/extensions/VRM").is_some();
    let vrm1 = root.pointer("/extensions/VRMC_vrm").is_some();
    ensure!(vrm0 || vrm1, "file has no VRM 0.x or VRM 1.0 metadata");

    if vrm0 {
        // VRM 0 exporters can put the visible material exclusively in this
        // extension. Map it to glTF instead of showing an untextured avatar.
        let materials = root
            .pointer("/extensions/VRM/materialProperties")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(targets) = root.get_mut("materials").and_then(Value::as_array_mut) {
            for (source, target) in materials.iter().zip(targets) {
                let shader = source.get("shader").and_then(Value::as_str).unwrap_or("");
                if !shader.starts_with("VRM/") {
                    continue;
                }
                let target = target.as_object_mut().context("invalid VRM material")?;
                let pbr = target
                    .entry("pbrMetallicRoughness")
                    .or_insert_with(|| json!({}));
                let pbr = pbr.as_object_mut().context("invalid VRM PBR material")?;
                if let Some(color) = source.pointer("/vectorProperties/_Color") {
                    pbr.insert("baseColorFactor".into(), color.clone());
                }
                if let Some(index) = source
                    .pointer("/textureProperties/_MainTex")
                    .and_then(Value::as_u64)
                {
                    let mut texture = json!({"index": index, "texCoord": 0});
                    if let Some(st) = source
                        .pointer("/vectorProperties/_MainTex")
                        .and_then(Value::as_array)
                    {
                        ensure!(st.len() == 4, "invalid VRM texture transform");
                        let offset_x = st[0].as_f64().context("invalid VRM texture offset")?;
                        let offset_y = st[1].as_f64().context("invalid VRM texture offset")?;
                        let scale_x = st[2].as_f64().context("invalid VRM texture scale")?;
                        let scale_y = st[3].as_f64().context("invalid VRM texture scale")?;
                        // Unity UVs originate at the bottom; glTF's originate at the top.
                        texture["extensions"] = json!({"KHR_texture_transform": {
                            "offset": [offset_x, 1.0 - scale_y - offset_y], "scale": [scale_x, scale_y]
                        }});
                    }
                    pbr.insert("baseColorTexture".into(), texture);
                }
            }
        }
    }

    // These avatar/appearance extensions can be approximated in a static
    // thumbnail. Keep all other requirements, notably compression, validated.
    if let Some(required) = root
        .get_mut("extensionsRequired")
        .and_then(Value::as_array_mut)
    {
        required.retain(|name| {
            !matches!(
                name.as_str(),
                Some(
                    "VRM"
                        | "VRMC_vrm"
                        | "VRMC_materials_mtoon"
                        | "VRMC_springBone"
                        | "VRMC_node_constraint"
                )
            )
        });
    }
    let document = gltf::Document::from_json(serde_json::from_value(root)?)?;
    let source = gltf::Gltf {
        document,
        blob: glb.bin.map(|b| b.into_owned()),
    };
    // VRM 0 faces -Z, whereas VRM 1 faces +Z, towards our thumbnail camera.
    let transform = if vrm0 && !vrm1 {
        Mat4::from_rotation_y(std::f32::consts::PI)
    } else {
        Mat4::IDENTITY
    };
    super::load_gltf_source(path, source, transform)
}
