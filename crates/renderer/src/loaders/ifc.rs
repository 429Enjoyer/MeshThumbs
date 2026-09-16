//! Keep the IFC2x3 importer; convert IFC4 with the bundled, isolated IfcConvert.
use anyhow::{bail, ensure, Context, Result};
use std::{io::Read, path::Path};

pub(super) fn load(path: &Path, budget: usize) -> Result<crate::Scene> {
    ensure!(
        std::fs::metadata(path)?.len() <= crate::MAX_MODEL_BYTES,
        "IFC exceeds the 300 MiB limit"
    );
    let mut header = Vec::new();
    std::fs::File::open(path)?
        .take(1024 * 1024)
        .read_to_end(&mut header)?;
    match schema(&header)?.as_str() {
        "IFC2X3" => super::legacy::load(path, budget),
        "IFC4" => convert(path, budget),
        name => bail!("unsupported IFC schema {name}; expected IFC2X3 or IFC4"),
    }
}

// Read STEP header tokens, not substrings inside comments or FILE_DESCRIPTION.
// The bounded header scan also works with the anonymous .ifc stream spool.
fn schema(data: &[u8]) -> Result<String> {
    let mut i = usize::from(data.starts_with(&[0xef, 0xbb, 0xbf])) * 3;
    let mut in_header = false;
    while i < data.len() {
        if data[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if data[i..].starts_with(b"/*") {
            i += 2;
            let end = data[i..]
                .windows(2)
                .position(|w| w == b"*/")
                .context("unterminated IFC comment")?;
            i += end + 2;
            continue;
        }
        if data[i] == b'\'' {
            string(data, &mut i)?;
            continue;
        }
        if data[i].is_ascii_alphabetic() {
            let start = i;
            while i < data.len() && (data[i].is_ascii_alphanumeric() || data[i] == b'_') {
                i += 1;
            }
            let word = &data[start..i];
            if word.eq_ignore_ascii_case(b"HEADER") {
                in_header = true;
            }
            if in_header && word.eq_ignore_ascii_case(b"ENDSEC") {
                break;
            }
            if in_header && word.eq_ignore_ascii_case(b"FILE_SCHEMA") {
                let mut names = Vec::new();
                // FILE_SCHEMA contains a parenthesized list of schema strings.
                while i < data.len() && data[i] != b';' {
                    if data[i..].starts_with(b"/*") {
                        i += 2;
                        let n = data[i..]
                            .windows(2)
                            .position(|w| w == b"*/")
                            .context("unterminated IFC schema comment")?;
                        i += n + 2;
                    } else if data[i] == b'\'' {
                        names.push(string(data, &mut i)?);
                    } else {
                        ensure!(
                            data[i].is_ascii_whitespace() || matches!(data[i], b'(' | b')'),
                            "invalid IFC schema declaration"
                        );
                        i += 1;
                    }
                }
                ensure!(
                    i < data.len() && names.len() == 1,
                    "expected one IFC schema in the first MiB of the header"
                );
                return Ok(names.remove(0).to_ascii_uppercase());
            }
        } else {
            i += 1;
        }
    }
    bail!("missing IFC FILE_SCHEMA in the first MiB of the header")
}

fn string(data: &[u8], i: &mut usize) -> Result<String> {
    *i += 1;
    let mut out = Vec::new();
    while *i < data.len() {
        let ch = data[*i];
        *i += 1;
        if ch == b'\'' {
            if data.get(*i) == Some(&b'\'') {
                out.push(ch);
                *i += 1;
            } else {
                return Ok(String::from_utf8_lossy(&out).into_owned());
            }
        } else {
            out.push(ch);
        }
    }
    bail!("unterminated IFC header string")
}

#[cfg(not(windows))]
fn convert(_path: &Path, _budget: usize) -> Result<crate::Scene> {
    bail!("IFC4 previews require the bundled Windows IfcConvert backend")
}

#[cfg(windows)]
fn convert(path: &Path, budget: usize) -> Result<crate::Scene> {
    use std::time::{Duration, Instant};
    let executable = std::env::current_exe()?
        .parent()
        .context("missing executable directory")?
        .join("ifc/IfcConvert.exe");
    ensure!(
        executable.is_file(),
        "IFC4 backend missing: run scripts/prepare-ifc.ps1"
    );
    let mut builder = tempfile::Builder::new();
    builder.prefix("meshthumbs-ifc-");
    let workspace = match std::env::var_os("MESHTHUMBS_WORK_DIR") {
        Some(parent) => builder.tempdir_in(parent)?,
        None => builder.tempdir()?,
    };
    let output = workspace.path().join("model.glb");
    // The MinGW converter uses narrow CRT paths. IFC is self-contained here:
    // copy to ASCII leaf names and use a Unicode Windows working directory,
    // so non-ASCII input paths and user-profile names still work correctly.
    let copied = std::io::copy(
        &mut std::fs::File::open(path)?.take(crate::MAX_MODEL_BYTES + 1),
        &mut std::fs::File::create(workspace.path().join("input.ifc"))?,
    )?;
    ensure!(
        copied <= crate::MAX_MODEL_BYTES,
        "IFC grew beyond the 300 MiB limit"
    );
    let args = vec![
        "input.ifc".into(),
        "model.glb".into(),
        "-y".into(),
        "-q".into(),
        "--no-progress".into(),
        "--center-model-geometry".into(),
        "--kernel".into(),
        "opencascade".into(),
        "--exclude".into(),
        "entities".into(),
        "IfcSpace".into(),
        "IfcOpeningElement".into(),
    ];
    let mut process = isolated::Process::spawn(&executable, &args, workspace.path())?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Ok(metadata) = std::fs::metadata(&output) {
            ensure!(
                metadata.len() <= crate::MAX_MODEL_BYTES,
                "IFC converted mesh exceeds 300 MiB"
            );
        }
        if let Some(code) = process.poll()? {
            ensure!(code == 0, "IFC4 conversion failed (exit {code})");
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "IFC4 conversion exceeded four-second limit"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // Finish/close the helper before reading its output or deleting its workspace.
    drop(process);
    let size = std::fs::metadata(&output)
        .context("IFC4 produced no mesh")?
        .len();
    ensure!(
        size > 0 && size <= crate::MAX_MODEL_BYTES,
        "invalid IFC converted mesh size"
    );
    let source = gltf::Gltf::open(&output)?;
    // The generated file must remain self-contained; never follow a model-supplied URI.
    ensure!(
        source
            .buffers()
            .all(|b| matches!(b.source(), gltf::buffer::Source::Bin)),
        "external IFC mesh buffer"
    );
    ensure!(
        source
            .images()
            .all(|i| matches!(i.source(), gltf::image::Source::View { .. })),
        "external IFC mesh image"
    );
    let indices: usize = source
        .meshes()
        .flat_map(|m| m.primitives())
        .map(|p| p.indices().map_or(0, |i| i.count()))
        .try_fold(0usize, |n, x| n.checked_add(x))
        .context("IFC index count overflow")?;
    ensure!(indices / 3 <= budget, "IFC exceeds triangle limit");
    let scene = super::load_gltf_source(&output, source, glam::Mat4::IDENTITY)?;
    ensure!(
        scene.triangles.len() <= budget,
        "IFC exceeds triangle limit"
    );
    Ok(scene)
}

#[cfg(windows)]
mod isolated;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_real_header_ignoring_comments_and_quoted_schema_names() {
        let header = b"\xef\xbb\xbfISO-10303-21; HEADER; /* FILE_SCHEMA(('IFC4')); */ FILE_DESCRIPTION(('IFC4 and ''quoted'' text'),'2;1'); FILE_SCHEMA /* test */ (('IFC2X3')); ENDSEC;";
        assert_eq!(schema(header).unwrap(), "IFC2X3");
        assert_eq!(
            schema(b"header; file_schema(('ifc4'));endsec;").unwrap(),
            "IFC4"
        );
    }

    #[test]
    fn refuses_absent_truncated_or_ambiguous_headers() {
        for bad in [
            "HEADER; ENDSEC; DATA;FILE_SCHEMA(('IFC4'));",
            "HEADER;FILE_SCHEMA(('IFC4'",
            "HEADER;/*missing",
            "HEADER;FILE_SCHEMA(('IFC4','IFC2X3'));",
            "HEADER;FILE_SCHEMA(());",
        ] {
            assert!(schema(bad.as_bytes()).is_err(), "{bad}");
        }
    }
}
