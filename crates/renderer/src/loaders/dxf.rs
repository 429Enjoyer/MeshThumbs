//! Restrict the Assimp DXF reader to geometry it can position faithfully.
use anyhow::{bail, ensure};
use std::path::Path;

pub(super) fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    let bytes = std::fs::read(path)?;
    validate(&bytes)?;
    Ok(bytes)
}

fn validate(bytes: &[u8]) -> anyhow::Result<()> {
    ensure!(
        bytes.len() <= crate::MAX_MODEL_BYTES as usize,
        "DXF exceeds file size limit"
    );
    ensure!(
        !bytes.starts_with(b"AutoCAD Binary DXF"),
        "binary DXF is unsupported"
    );
    let mut lines = bytes.split(|&b| b == b'\n').peekable();
    let mut groups = 0;
    let mut eof = false;
    while let Some(code) = lines.next() {
        if code.trim_ascii().is_empty() && lines.peek().is_none() {
            break;
        }
        groups += 1;
        ensure!(groups <= 10_000_000, "too many DXF groups");
        let code: i32 = std::str::from_utf8(code.trim_ascii())?.parse()?;
        let value = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("truncated DXF group"))?
            .trim_ascii();
        if code != 0 {
            continue;
        }
        if value.eq_ignore_ascii_case(b"EOF") {
            eof = true;
            break;
        }
        // Assimp 6.0.5 ignores INSERT rotations/nested blocks. Reject these
        // instead of placing instances incorrectly or dropping solid bodies.
        for kind in [
            b"INSERT".as_slice(),
            b"3DSOLID",
            b"BODY",
            b"REGION",
            b"SURFACE",
            b"MESH",
            b"SOLID",
            b"3DLINE",
            b"HATCH",
        ] {
            if value.eq_ignore_ascii_case(kind) {
                bail!(
                    "unsupported DXF entity: {} (use exploded 3DFACE/polyface geometry)",
                    String::from_utf8_lossy(value)
                );
            }
        }
    }
    ensure!(eof, "DXF EOF record is missing");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_binary_truncated_and_misplaced_geometry() {
        for data in [
            b"AutoCAD Binary DXF\r\n".as_slice(),
            b"0\n3DFACE\n10\n",
            b"0\nINSERT\n0\nEOF\n",
            b"0\n3DSOLID\n0\nEOF\n",
        ] {
            assert!(validate(data).is_err());
        }
        assert!(validate(
            b"0\r\nSECTION\r\n2\r\nENTITIES\r\n0\r\n3DFACE\r\n0\r\nENDSEC\r\n0\r\nEOF\r\n"
        )
        .is_ok());
    }
}
