//! Bounded PCX v5 reader for indexed Quake II skins and 24-bit RGB images.
//! Implements the ZSoft PCX header, scanline RLE, and trailing VGA palette.
use std::{io::Read, path::Path};

use anyhow::{ensure, Context};

pub(super) fn load(path: &Path) -> anyhow::Result<image::DynamicImage> {
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() <= 64 * 1024 * 1024, "PCX exceeds 64 MiB");
    decode(&data)
}

fn decode(data: &[u8]) -> anyhow::Result<image::DynamicImage> {
    ensure!(data.len() >= 128, "truncated PCX header");
    ensure!(
        data[0] == 10 && data[1] == 5 && data[2] <= 1 && data[3] == 8,
        "unsupported PCX encoding (expected v5, 8 bits per plane)"
    );
    let word = |i| u16::from_le_bytes([data[i], data[i + 1]]) as usize;
    let (xmin, ymin, xmax, ymax) = (word(4), word(6), word(8), word(10));
    ensure!(xmax >= xmin && ymax >= ymin, "invalid PCX bounds");
    let (width, height) = (xmax - xmin + 1, ymax - ymin + 1);
    let planes = data[65] as usize;
    let stride = word(66);
    ensure!(
        matches!(planes, 1 | 3) && stride >= width && stride % 2 == 0,
        "unsupported PCX planes or row stride"
    );
    ensure!(
        width <= 8192 && height <= 8192 && width * height <= 16_777_216,
        "PCX exceeds pixel limit"
    );
    ensure!(
        stride * planes * height <= 64 * 1024 * 1024,
        "PCX exceeds decoded size limit"
    );
    let (encoded, palette) = if planes == 1 {
        ensure!(data.len() >= 128 + 769, "missing PCX palette");
        let start = data.len() - 769;
        ensure!(data[start] == 12, "invalid PCX palette marker");
        (&data[128..start], Some(&data[start + 1..]))
    } else {
        (&data[128..], None)
    };
    let mut cursor = 0;
    let mut scanline = vec![0; stride * planes];
    let mut rgb = Vec::with_capacity(width * height * 3);
    for _ in 0..height {
        let mut filled = 0;
        while filled < scanline.len() {
            let byte = *encoded.get(cursor).context("truncated PCX pixels")?;
            cursor += 1;
            let (count, value) = if data[2] == 1 && byte & 0xc0 == 0xc0 {
                let value = *encoded.get(cursor).context("truncated PCX run")?;
                cursor += 1;
                ((byte & 63) as usize, value)
            } else {
                (1, byte)
            };
            ensure!(
                count > 0 && count <= scanline.len() - filled,
                "invalid PCX scanline run"
            );
            scanline[filled..filled + count].fill(value);
            filled += count;
        }
        for x in 0..width {
            if let Some(palette) = palette {
                let i = scanline[x] as usize * 3;
                rgb.extend_from_slice(&palette[i..i + 3]);
            } else {
                rgb.extend_from_slice(&[
                    scanline[x],
                    scanline[stride + x],
                    scanline[2 * stride + x],
                ]);
            }
        }
    }
    ensure!(cursor == encoded.len(), "unexpected PCX pixel data");
    Ok(image::RgbImage::from_raw(width as u32, height as u32, rgb)
        .context("invalid PCX dimensions")?
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(width: u16, height: u16, planes: u8, stride: u16) -> Vec<u8> {
        let mut data = vec![0; 128];
        data[..4].copy_from_slice(&[10, 5, 1, 8]);
        data[8..10].copy_from_slice(&(width - 1).to_le_bytes());
        data[10..12].copy_from_slice(&(height - 1).to_le_bytes());
        data[65] = planes;
        data[66..68].copy_from_slice(&stride.to_le_bytes());
        data
    }
    fn indexed(pixels: &[u8]) -> Vec<u8> {
        let mut data = header(3, 1, 1, 4);
        data.extend(pixels);
        data.push(12);
        for i in 0..256 {
            data.extend([i as u8, 0, 255 - i as u8]);
        }
        data
    }
    #[test]
    fn decodes_palette_runs_and_padding() {
        let im = decode(&indexed(&[0xc2, 192, 4, 0])).unwrap().to_rgb8();
        assert_eq!(im.dimensions(), (3, 1));
        assert_eq!(im.as_raw(), &[192, 0, 63, 192, 0, 63, 4, 0, 251]);
    }
    #[test]
    fn decodes_rgb_planes_and_uncompressed_bytes() {
        let mut data = header(1, 1, 3, 2);
        data[2] = 0;
        data.extend([250, 0, 170, 0, 10, 0]);
        assert_eq!(decode(&data).unwrap().to_rgb8().as_raw(), &[250, 170, 10]);
    }
    #[test]
    fn rejects_truncated_bad_runs_and_oversized_images() {
        let valid = indexed(&[1, 2, 3, 0]);
        for n in 0..valid.len() {
            assert!(decode(&valid[..n]).is_err());
        }
        for pixels in [&[0xc0, 1][..], &[0xc5, 1], &[0xc4], &[1, 2, 3, 0, 1]] {
            assert!(decode(&indexed(pixels)).is_err());
        }
        assert!(decode(&header(8193, 8193, 3, 8194)).is_err());
        assert!(decode(&header(4, 1, 1, 2)).is_err());
        let mut bad_palette = valid;
        let i = bad_palette.len() - 769;
        bad_palette[i] = 0;
        assert!(decode(&bad_palette).is_err());
    }
}
