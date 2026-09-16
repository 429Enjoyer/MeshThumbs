use std::path::PathBuf;

use anyhow::{bail, Context};
use renderer::{render_thumbnail, RenderOptions};

fn main() -> anyhow::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    #[cfg(windows)]
    if args.first().is_some_and(|arg| arg == "--export-png") {
        if args.len() < 3 {
            bail!("usage: thumbgen --export-png <256|512|1024> <files...>");
        }
        let size = args[1]
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
            .context("invalid export size")?;
        struct Console;
        impl png_export::batch::Observer for Console {
            fn begin(&mut self, index: usize, total: usize, path: &std::path::Path) {
                println!("[{}/{}] {}", index + 1, total, path.display());
            }
            fn cancelled(&mut self) -> bool {
                false
            }
        }
        let report = png_export::batch::run(
            args[2..].iter().map(PathBuf::from).collect(),
            size,
            &mut Console,
        )?;
        println!(
            "Generated {} of {} PNG thumbnails; {} failed; {} output-name collisions.",
            report.written,
            report.total,
            report.failures.len(),
            report.collisions
        );
        for (path, error) in &report.failures {
            eprintln!("{}: {error}", path.display());
        }
        if !report.failures.is_empty() {
            bail!("some PNG exports failed");
        }
        return Ok(());
    }
    let raw = args.get(3).is_some_and(|arg| arg == "--raw-rgba");
    if raw && args.get(4).is_some_and(|arg| arg == "--work-dir") {
        let directory = args.get(5).context("missing worker directory")?;
        std::env::set_var("MESHTHUMBS_WORK_DIR", directory);
    }
    let result = render(&args);
    if raw {
        if let (Err(error), Some(output)) = (&result, args.get(1)) {
            let message = format!("{error:#}").chars().take(4096).collect::<String>();
            let _ = std::fs::write(PathBuf::from(output).with_extension("error.txt"), message);
        }
    }
    result
}

fn render(args: &[std::ffi::OsString]) -> anyhow::Result<()> {
    if args.len() < 2 {
        bail!("usage: thumbgen <model.obj|fbx|glb|gltf|stl|dae|ply|3ds|3mf|vrm|blend|x3d|off|usd|usda|usdc|usdz|wrl|vrml|step|stp|abc|igs|iges|3dm|ifc|pmx|vox|lwo|smd|md2|md3|md5mesh|ase|lxo|lws|dxf> <out.png> [size]");
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
