//! Private bounded UTF-16 handoff: filenames never pass through a command shell.
use anyhow::{ensure, Result};
use std::{
    ffi::OsString,
    io::{Read, Write},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

const MAGIC: &[u8; 8] = b"MTPNG001";
const LIMIT: u64 = 16 * 1024 * 1024;

pub fn write(mut out: impl Write, size: u32, paths: &[PathBuf]) -> Result<()> {
    ensure!(super::SIZES.contains(&size), "Invalid export size");
    ensure!(
        !paths.is_empty() && paths.len() <= super::MAX_FILES,
        "Invalid selection count"
    );
    let mut data = MAGIC.to_vec();
    data.extend(size.to_le_bytes());
    data.extend((paths.len() as u32).to_le_bytes());
    for path in paths {
        ensure!(
            path.is_absolute() && super::supported(path),
            "Invalid selected path"
        );
        let words = path.as_os_str().encode_wide().collect::<Vec<_>>();
        ensure!(
            !words.is_empty() && words.len() <= 32767 && !words.contains(&0),
            "Invalid path length"
        );
        data.extend((words.len() as u32).to_le_bytes());
        for word in words {
            data.extend(word.to_le_bytes());
        }
        ensure!(data.len() as u64 <= LIMIT, "Selection is too large");
    }
    out.write_all(&data)?;
    Ok(())
}

pub fn read(input: impl Read) -> Result<(u32, Vec<PathBuf>)> {
    let mut data = Vec::new();
    input.take(LIMIT + 1).read_to_end(&mut data)?;
    ensure!(
        data.len() as u64 <= LIMIT && data.starts_with(MAGIC),
        "Invalid export request"
    );
    let mut rest = &data[MAGIC.len()..];
    fn number(rest: &mut &[u8]) -> Result<u32> {
        ensure!(rest.len() >= 4, "Truncated export request");
        let result = u32::from_le_bytes(rest[..4].try_into()?);
        *rest = &rest[4..];
        Ok(result)
    }
    let size = number(&mut rest)?;
    let count = number(&mut rest)? as usize;
    ensure!(
        super::SIZES.contains(&size) && (1..=super::MAX_FILES).contains(&count),
        "Invalid export size or count"
    );
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        let len = number(&mut rest)? as usize;
        ensure!(
            (1..=32767).contains(&len) && rest.len() >= len * 2,
            "Invalid export path length"
        );
        let words = rest[..len * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>();
        ensure!(!words.contains(&0), "NUL in export path");
        rest = &rest[len * 2..];
        let path = PathBuf::from(OsString::from_wide(&words));
        ensure!(
            path.is_absolute() && super::supported(&path),
            "Invalid export path"
        );
        files.push(path);
    }
    ensure!(rest.is_empty(), "Unexpected export request data");
    Ok((size, super::selection(files)?))
}

pub fn consume(path: &Path) -> Result<(u32, Vec<PathBuf>)> {
    let result = read(std::fs::File::open(path)?)?;
    // Delete only a valid handoff in the menu's own temporary-file namespace.
    if path.parent() == Some(std::env::temp_dir().as_path())
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("meshthumbs-export-") && n.ends_with(".job"))
    {
        std::fs::remove_file(path)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_paths_roundtrip_and_malformed_jobs_fail() {
        let files = vec![PathBuf::from("C:\\モデル space\\file & $x.obj")];
        let mut data = Vec::new();
        write(&mut data, 512, &files).unwrap();
        assert_eq!(read(data.as_slice()).unwrap(), (512, files));
        for cut in 0..data.len() {
            assert!(read(&data[..cut]).is_err());
        }
        data.push(0);
        assert!(read(data.as_slice()).is_err());
        assert!(write(Vec::new(), 2048, &[PathBuf::from("C:\\a.obj")]).is_err());
    }
}
