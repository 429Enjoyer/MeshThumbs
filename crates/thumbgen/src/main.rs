use std::path::PathBuf;

use anyhow::{bail, Context};
use renderer::{render_thumbnail, RenderOptions};

fn main() -> anyhow::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        bail!("usage: thumbgen <model.obj|fbx|glb|gltf|stl|dae|ply|3ds|3mf|vrm|blend|x3d|off|usd|usda|usdc|usdz|wrl|vrml|step|stp|abc|igs|iges|3dm|ifc|pmx|vox|lwo|smd|md2|md3|md5mesh> <out.png> [size]");
    }

    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let size = args
        .get(2)
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(256);

    let bitmap = render_thumbnail(
        &input,
        &RenderOptions {
            size,
            ..Default::default()
        },
    )
    .with_context(|| format!("failed to render {}", input.display()))?;

    // Private worker transport: fixed dimensions are validated by the COM host.
    if args.get(3).is_some_and(|arg| arg == "--raw-rgba") {
        std::fs::write(&output, &bitmap.pixels)?;
        return Ok(());
    }

    image::save_buffer(
        &output,
        &bitmap.pixels,
        bitmap.width,
        bitmap.height,
        image::ColorType::Rgba8,
    )
    .with_context(|| format!("failed to write {}", output.display()))?;

    Ok(())
}
