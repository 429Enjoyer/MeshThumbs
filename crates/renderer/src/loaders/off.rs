//! Normalize common ASCII OFF variants into an in-memory PLY stream.
//! This avoids the native OFF importer's RGB/byte-color and polygon-size limits;
//! Assimp still triangulates polygons and supplies the regular scene pipeline.
use std::path::Path;

use anyhow::{ensure, Context, Result};

use crate::{RenderError, MAX_MODEL_BYTES};

pub(super) fn to_ply(path: &Path, budget: usize) -> Result<Vec<u8>> {
    let text = std::fs::read_to_string(path).context("OFF requires an ASCII text file")?;
    let mut records = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty());
    let first = records.next().context("empty OFF file")?;
    let mut header = first.split_whitespace();
    let format = header.next().context("missing OFF header")?;
    let (normals, colors) = match format {
        "OFF" => (false, false),
        "COFF" => (false, true),
        "NOFF" => (true, false),
        "CNOFF" => (true, true),
        _ => anyhow::bail!("unsupported OFF variant; expected OFF, COFF, NOFF, or CNOFF"),
    };
    let rest = header.collect::<Vec<_>>();
    let count_line;
    let counts = if rest.is_empty() {
        count_line = records.next().context("missing OFF counts")?;
        count_line.split_whitespace().collect::<Vec<_>>()
    } else {
        rest
    };
    ensure!(
        counts.len() == 3,
        "invalid OFF counts or unsupported binary OFF"
    );
    let vertex_count: usize = counts[0].parse().context("invalid OFF vertex count")?;
    let face_count: usize = counts[1].parse().context("invalid OFF face count")?;
    let _: usize = counts[2].parse().context("invalid OFF edge count")?;
    ensure!(
        vertex_count > 0 && face_count > 0,
        "OFF has no renderable faces"
    );
    let required = vertex_count
        .checked_add(face_count)
        .context("OFF counts overflow")?;
    ensure!(
        required <= records.clone().count(),
        "OFF counts exceed available records"
    );
    ensure!(vertex_count <= u32::MAX as usize, "too many OFF vertices");
    ensure!(
        face_count <= budget,
        RenderError::TooManyTriangles {
            actual: face_count,
            limit: budget
        }
    );

    let mut header = format!("ply\nformat binary_little_endian 1.0\nelement vertex {vertex_count}\nproperty float x\nproperty float y\nproperty float z\n");
    if normals {
        header.push_str("property float nx\nproperty float ny\nproperty float nz\n");
    }
    if colors {
        header.push_str(
            "property uchar red\nproperty uchar green\nproperty uchar blue\nproperty uchar alpha\n",
        );
    }
    header.push_str(&format!(
        "element face {face_count}\nproperty list uint uint vertex_indices\nend_header\n"
    ));
    let mut ply = header.into_bytes();
    let coordinate_count = if normals { 6 } else { 3 };
    let stride = coordinate_count * 4 + if colors { 4 } else { 0 };
    ensure!(
        (vertex_count as u64) * stride as u64 <= MAX_MODEL_BYTES,
        "normalized OFF exceeds 300 MiB"
    );
    for _ in 0..vertex_count {
        let line = records.next().context("missing OFF vertex")?;
        let fields = line.split_whitespace().take(11).collect::<Vec<_>>();
        ensure!(
            if colors {
                fields.len() == coordinate_count + 3 || fields.len() == coordinate_count + 4
            } else {
                fields.len() == coordinate_count
            },
            "invalid OFF vertex record"
        );
        for value in &fields[..coordinate_count] {
            let value: f32 = value.parse().context("invalid OFF coordinate or normal")?;
            ensure!(value.is_finite(), "nonfinite OFF coordinate or normal");
            ply.extend_from_slice(&value.to_le_bytes());
        }
        if colors {
            let channels = fields[coordinate_count..]
                .iter()
                .map(|v| v.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()
                .context("invalid OFF color")?;
            ensure!(
                channels
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=255.0).contains(v)),
                "invalid OFF color range"
            );
            let bytes = channels.iter().any(|&c| c > 1.0);
            for i in 0..4 {
                let value = channels
                    .get(i)
                    .copied()
                    .unwrap_or(if bytes { 255.0 } else { 1.0 });
                ply.push((if bytes { value } else { value * 255.0 }).round() as u8);
            }
        }
    }
    let mut triangles = 0usize;
    for _ in 0..face_count {
        let mut fields = records
            .next()
            .context("missing OFF face")?
            .split_whitespace();
        let count: usize = fields.next().context("missing OFF face size")?.parse()?;
        ensure!(
            count >= 3 && count <= u32::MAX as usize,
            "OFF face must contain at least three vertices"
        );
        ensure!(
            fields.clone().count() == count,
            "invalid OFF face record; per-face colors are unsupported"
        );
        triangles = triangles
            .checked_add(count - 2)
            .context("OFF triangle count overflow")?;
        ensure!(
            triangles <= budget,
            RenderError::TooManyTriangles {
                actual: triangles,
                limit: budget
            }
        );
        ensure!(
            ply.len() as u64 + 4 + count as u64 * 4 <= MAX_MODEL_BYTES,
            "normalized OFF exceeds 300 MiB"
        );
        ply.extend_from_slice(&(count as u32).to_le_bytes());
        for field in fields {
            let index: u32 = field.parse().context("invalid OFF face index")?;
            ensure!(
                (index as usize) < vertex_count,
                "OFF face index is out of range"
            );
            ply.extend_from_slice(&index.to_le_bytes());
        }
    }
    ensure!(records.next().is_none(), "unexpected extra OFF records");
    Ok(ply)
}
