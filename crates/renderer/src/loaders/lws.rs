//! Resolve a bounded local LWS package before Assimp imports it. Object image
//! paths are rebased before merging, so identical texture names cannot collide.
use anyhow::{ensure, Context};
use asset_importer::io::MemoryFileSystem;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(super) struct SceneFiles(MemoryFileSystem);
impl asset_importer::io::FileSystem for SceneFiles {
    fn exists(&self, path: &str) -> bool {
        self.0.exists(&path.replace('\\', "/"))
    }
    fn open(
        &self,
        path: &str,
    ) -> asset_importer::error::Result<Box<dyn asset_importer::io::FileStream>> {
        self.0.open(&path.replace('\\', "/"))
    }
    fn separator(&self) -> char {
        '/'
    }
}

fn key(path: &Path) -> anyhow::Result<String> {
    let path = std::path::absolute(path)?;
    let path = path.to_str().context("LWS path is not Unicode")?;
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        Ok(format!("//{}", unc.replace('\\', "/")))
    } else {
        Ok(path
            .strip_prefix(r"\\?\")
            .unwrap_or(path)
            .replace('\\', "/"))
    }
}

fn resolve(name: &str, base: &Path) -> PathBuf {
    let name = name.trim().trim_matches('"').replace('\\', "/");
    let path = Path::new(&name);
    for parent in base.ancestors().take(3) {
        let candidate = parent.join(path);
        if candidate.is_file() {
            return candidate;
        }
    }
    base.join(path)
}

fn token<'a>(rest: &mut &'a str) -> anyhow::Result<&'a str> {
    *rest = rest.trim_start();
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let value = &rest[..end];
    ensure!(!value.is_empty(), "missing LWS token");
    *rest = &rest[end..];
    Ok(value)
}

pub(super) fn files(path: &Path) -> anyhow::Result<(String, SceneFiles)> {
    let bytes = std::fs::read(path)?;
    ensure!(
        bytes.len() <= crate::MAX_MODEL_BYTES as usize,
        "LWS exceeds file size limit"
    );
    let text = std::str::from_utf8(&bytes)?.trim_start_matches('\u{feff}');
    let mut header = text.lines().filter(|s| !s.trim().is_empty());
    ensure!(
        header.next().map(str::trim) == Some("LWSC"),
        "invalid LWS signature"
    );
    let version: u32 = header
        .next()
        .context("missing LWS version")?
        .trim()
        .parse()?;
    ensure!(
        (3..=5).contains(&version),
        "LWS versions 3 through 5 are supported"
    );
    let base = path.parent().unwrap_or(Path::new("."));
    let mut fs = MemoryFileSystem::new();
    let mut output = String::new();
    let mut total = bytes.len();
    let mut object_count = 0;
    let mut nodes = HashMap::<u32, Option<u32>>::new();
    let mut next_ids = [0u32; 4];
    let mut current = None;
    let mut depth = 0usize;
    let mut plugin = false;
    for (line_number, line) in text.lines().enumerate() {
        ensure!(line_number < 100_000, "too many LWS lines");
        let trimmed = line.trim();
        if trimmed.starts_with("Plugin ") {
            plugin = true;
        }
        if plugin {
            if trimmed == "EndPlugin" {
                plugin = false;
            }
            // Plugin data is never executed and can have arbitrary braces.
            continue;
        }
        if trimmed.starts_with('{') {
            depth += 1;
            ensure!(depth <= 64, "LWS hierarchy is too deep");
        }
        if trimmed.starts_with('}') {
            depth = depth.checked_sub(1).context("unbalanced LWS braces")?;
        }
        let mut rest = trimmed;
        let directive = if rest.is_empty() {
            ""
        } else {
            token(&mut rest)?
        };
        let kind = match directive {
            "LoadObject" | "LoadObjectLayer" | "AddNullObject" => 1,
            "AddLight" => 2,
            "AddCamera" => 3,
            _ => 0,
        };
        if kind != 0 {
            let layer = if directive == "LoadObjectLayer" {
                Some(token(&mut rest)?.parse::<u32>()?)
            } else {
                None
            };
            let id = if version >= 4 {
                u32::from_str_radix(token(&mut rest)?, 16)?
            } else {
                let id = (kind << 28) | next_ids[kind as usize];
                next_ids[kind as usize] += 1;
                id
            };
            ensure!(id >> 28 == kind, "invalid LWS item type in node id");
            ensure!(
                nodes.insert(id, None).is_none() && nodes.len() <= 4096,
                "duplicate or excessive LWS nodes"
            );
            current = Some(id);
            if directive == "LoadObject" || directive == "LoadObjectLayer" {
                object_count += 1;
                ensure!(object_count <= 1024, "too many referenced LWS objects");
                let source = resolve(rest, base);
                ensure!(
                    source
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("lwo")),
                    "LWS references must be local LWO files"
                );
                let name = key(&source)?;
                if !asset_importer::io::FileSystem::exists(&fs, &name) {
                    let metadata = std::fs::metadata(&source)
                        .with_context(|| format!("missing LWS object {}", source.display()))?;
                    ensure!(
                        metadata.len() <= crate::MAX_MODEL_BYTES - total as u64,
                        "LWS package exceeds 300 MiB"
                    );
                    let object =
                        rebase_object(&std::fs::read(&source)?, source.parent().unwrap_or(base))?;
                    total += object.len();
                    ensure!(
                        total <= crate::MAX_MODEL_BYTES as usize,
                        "LWS package exceeds 300 MiB"
                    );
                    fs.add_file(name.clone(), object);
                }
                output.push_str(directive);
                output.push(' ');
                if let Some(layer) = layer {
                    output.push_str(&format!("{layer} "));
                }
                if version >= 4 {
                    output.push_str(&format!("{id:08x} "));
                }
                output.push_str(&name);
                output.push('\n');
                continue;
            }
        } else if directive == "ParentItem" {
            let id = current.context("LWS parent without a node")?;
            let parent = u32::from_str_radix(rest.trim(), 16)?;
            if parent != 0 {
                nodes.insert(id, Some(parent));
            }
        } else if matches!(directive, "FirstFrame" | "LastFrame") {
            output.push_str(directive);
            output.push_str(" 1\n");
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    ensure!(depth == 0 && !plugin, "truncated LWS block");
    for &id in nodes.keys() {
        let mut seen = Vec::new();
        let mut cursor = Some(id);
        while let Some(node) = cursor {
            ensure!(
                !seen.contains(&node) && seen.len() < 64,
                "cyclic or excessive LWS parenting"
            );
            seen.push(node);
            cursor = *nodes.get(&node).context("missing LWS parent node")?;
        }
    }
    let name = key(path)?;
    ensure!(
        total + output.len() <= crate::MAX_MODEL_BYTES as usize,
        "expanded LWS package exceeds 300 MiB"
    );
    fs.add_file(name.clone(), output.into_bytes());
    Ok((name, SceneFiles(fs)))
}

fn be32(data: &[u8]) -> anyhow::Result<usize> {
    Ok(u32::from_be_bytes(data.get(..4).context("truncated IFF length")?.try_into()?) as usize)
}

fn subchunks(data: &[u8], base: &Path, image_tag: &[u8; 4]) -> anyhow::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let header = data.get(at..at + 6).context("truncated LWO subchunk")?;
        let n = u16::from_be_bytes([header[4], header[5]]) as usize;
        let payload = data
            .get(at + 6..at + 6 + n)
            .context("invalid LWO subchunk size")?;
        let mut replacement = payload.to_vec();
        if &header[..4] == image_tag {
            let end = payload
                .iter()
                .position(|&b| b == 0)
                .context("unterminated LWO texture path")?;
            let name = std::str::from_utf8(&payload[..end])?;
            if !name.is_empty() {
                let texture = resolve(name, base);
                replacement = key(&texture)?.into_bytes();
                replacement.push(0);
                if replacement.len() % 2 != 0 {
                    replacement.push(0);
                }
            }
        }
        ensure!(
            replacement.len() <= u16::MAX as usize,
            "LWO texture path is too long"
        );
        output.extend_from_slice(&header[..4]);
        output.extend_from_slice(&(replacement.len() as u16).to_be_bytes());
        output.extend_from_slice(&replacement);
        if replacement.len() % 2 != 0 {
            output.push(0);
        }
        at += 6 + n + n % 2;
        ensure!(at <= data.len(), "missing LWO subchunk padding");
    }
    Ok(output)
}

fn rebase_object(data: &[u8], base: &Path) -> anyhow::Result<Vec<u8>> {
    ensure!(
        data.get(..4) == Some(b"FORM") && data.len() >= 12,
        "invalid LWO object header"
    );
    let kind = &data[8..12];
    ensure!(
        matches!(kind, b"LWO2" | b"LWOB" | b"LXOB"),
        "unsupported LWS object encoding"
    );
    let end = be32(&data[4..])? + 8;
    ensure!(end <= data.len() && end >= 12, "invalid LWO object length");
    let mut output = data[..12].to_vec();
    let mut at = 12;
    while at < end {
        let header = data.get(at..at + 8).context("truncated LWO chunk")?;
        let n = be32(&header[4..])?;
        ensure!(at + 8 + n <= end, "invalid LWO chunk size");
        let payload = &data[at + 8..at + 8 + n];
        let replacement = if &header[..4] == b"CLIP" && kind != b"LWOB" {
            ensure!(n >= 4, "invalid LWO clip");
            [
                payload[..4].to_vec(),
                subchunks(&payload[4..], base, b"STIL")?,
            ]
            .concat()
        } else if &header[..4] == b"SURF" && kind == b"LWOB" {
            let len = payload
                .iter()
                .position(|&b| b == 0)
                .context("invalid LWOB surface name")?
                + 1;
            let len = len + len % 2;
            ensure!(len <= n, "invalid LWOB surface padding");
            [
                payload[..len].to_vec(),
                subchunks(&payload[len..], base, b"TIMG")?,
            ]
            .concat()
        } else {
            payload.to_vec()
        };
        output.extend_from_slice(&header[..4]);
        output.extend_from_slice(&(replacement.len() as u32).to_be_bytes());
        output.extend_from_slice(&replacement);
        if replacement.len() % 2 != 0 {
            output.push(0);
        }
        at += 8 + n + n % 2;
        ensure!(at <= end, "missing LWO chunk padding");
    }
    let size = (output.len() - 8) as u32;
    output[4..8].copy_from_slice(&size.to_be_bytes());
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_importer::io::FileSystem;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    #[test]
    #[cfg(windows)]
    fn preserves_extended_local_and_unc_roots() {
        assert_eq!(
            key(Path::new(r"\\?\C:\models\scene.lws")).unwrap(),
            "C:/models/scene.lws"
        );
        assert_eq!(
            key(Path::new(r"\\?\UNC\server\share\scene.lws")).unwrap(),
            "//server/share/scene.lws"
        );
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "meshthumbs-lws-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_quoted_objects_and_normalizes_io_separators() {
        let dir = Fixture::new();
        let name = "Satellite 日本語.lwo";
        std::fs::write(
            dir.0.join(name),
            include_bytes!("../../tests/fixtures/Satellite.lwo"),
        )
        .unwrap();
        let path = dir.0.join("scene.lws");
        std::fs::write(
            &path,
            format!("LWSC\n3\nFirstFrame 1\nLastFrame 600\nLoadObject \"{name}\"\n"),
        )
        .unwrap();
        let (scene, fs) = files(&path).unwrap();
        assert!(fs.exists(&scene));
        assert!(fs.exists(&key(&dir.0.join(name)).unwrap().replace('/', "\\")));
        let mut stream = fs.open(&scene).unwrap();
        let mut data = vec![0; stream.size().unwrap() as usize];
        stream.read(&mut data).unwrap();
        assert!(String::from_utf8(data).unwrap().contains("LastFrame 1\n"));
    }

    #[test]
    fn refuses_missing_objects_cycles_and_old_scenes() {
        let dir = Fixture::new();
        let path = dir.0.join("scene.lws");
        for scene in [
            "LWSC\n3\nLoadObject missing.lwo\n",
            "LWSC\n3\nLoadObject scene.lws\n",
            "LWSC\n2\n",
            "LWSC\n3\nAddNullObject a\nParentItem 10000001\nAddNullObject b\nParentItem 10000000\n",
            "LWSC\n3\nAddNullObject a\nParentItem 10000009\n",
            "LWSC\n4\nAddNullObject 00000000 wrong-type\n",
            "LWSC\n3\n{ Envelope\n",
        ] {
            std::fs::write(&path, scene).unwrap();
            assert!(files(&path).is_err(), "{scene}");
        }
    }

    #[test]
    fn image_paths_keep_each_objects_directory() {
        let dir = Fixture::new();
        let name = b"diffuse.png\0";
        let mut clip = 1u32.to_be_bytes().to_vec();
        clip.extend(b"STIL");
        clip.extend((name.len() as u16).to_be_bytes());
        clip.extend(name);
        let mut data = b"FORM".to_vec();
        data.extend((12 + clip.len() as u32).to_be_bytes());
        data.extend(b"LWO2CLIP");
        data.extend((clip.len() as u32).to_be_bytes());
        data.extend(clip);
        let a = rebase_object(&data, &dir.0.join("first")).unwrap();
        let b = rebase_object(&data, &dir.0.join("second")).unwrap();
        assert!(String::from_utf8_lossy(&a).contains("first/diffuse.png"));
        assert!(String::from_utf8_lossy(&b).contains("second/diffuse.png"));
        assert_ne!(a, b);
        assert_eq!(be32(&a[4..]).unwrap() + 8, a.len());
        for n in [0, 4, 8, 11, data.len() - 1] {
            assert!(rebase_object(&data[..n], &dir.0).is_err());
        }
    }
}
