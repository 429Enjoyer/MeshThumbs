//! Read the saved file preview, without loading scene data or executing Blender.
//! File layouts: Blender's BLO_core_blend_header.hh / BLO_core_bhead.hh.
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::Path,
};

use anyhow::{bail, ensure, Context, Result};
use image::{imageops, DynamicImage, RgbaImage};

use crate::RgbaBitmap;

// A thumbnail lives near the start, before GLOB and scene data. Bound both
// decompression and allocations even if a malformed file advertises huge sizes.
const MAX_PREFIX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PREVIEW_PIXELS: u64 = 4 * 1024 * 1024;

pub(crate) fn render(path: &Path, size: u32) -> Result<RgbaBitmap> {
    let mut file = BufReader::new(File::open(path)?);
    let prefix = file.fill_buf()?;
    let reader: Box<dyn Read> = if prefix.starts_with(&[0x1f, 0x8b]) {
        Box::new(flate2::read::MultiGzDecoder::new(file))
    } else if prefix.len() >= 4 && {
        let magic = u32::from_le_bytes(prefix[..4].try_into().unwrap());
        magic == 0xfd2fb528 || magic & 0xfffffff0 == 0x184d2a50
    } {
        let mut decoder = zstd::stream::read::Decoder::with_buffer(file)?;
        decoder.window_log_max(27)?;
        Box::new(decoder)
    } else {
        Box::new(file)
    };
    let preview = extract(&mut reader.take(MAX_PREFIX_BYTES))?;
    let preview = DynamicImage::ImageRgba8(preview)
        .resize(size, size, imageops::FilterType::Lanczos3)
        .into_rgba8();
    let mut canvas = RgbaImage::new(size, size);
    imageops::replace(
        &mut canvas,
        &preview,
        i64::from((size - preview.width()) / 2),
        i64::from((size - preview.height()) / 2),
    );
    Ok(RgbaBitmap {
        width: size,
        height: size,
        pixels: canvas.into_raw(),
    })
}

fn extract(reader: &mut impl Read) -> Result<RgbaImage> {
    let mut header = [0u8; 17];
    reader
        .read_exact(&mut header[..12])
        .context("truncated Blender file header")?;
    ensure!(&header[..7] == b"BLENDER", "invalid Blender file header");

    // Legacy blocks have a 32-bit length and either 32- or 64-bit pointers.
    // The modern format uses 64-bit lengths at a different offset.
    let (block_size, length_offset, wide_length, version_digits) = match header[7] {
        b'_' | b'-' => {
            ensure!(header[8] == b'v', "unsupported Blender byte order");
            (
                if header[7] == b'_' { 20 } else { 24 },
                4,
                false,
                &header[9..12],
            )
        }
        _ => {
            reader
                .read_exact(&mut header[12..])
                .context("truncated Blender file header")?;
            ensure!(
                &header[7..13] == b"17-01v",
                "unsupported Blender file header format"
            );
            (32, 16, true, &header[13..17])
        }
    };
    ensure!(
        version_digits.iter().all(u8::is_ascii_digit),
        "invalid Blender version"
    );
    let version = version_digits
        .iter()
        .fold(0u32, |value, digit| value * 10 + u32::from(digit - b'0'));
    ensure!(
        version >= 250,
        "Blender file predates embedded preview support"
    );

    let mut block = [0u8; 32];
    for _ in 0..4096 {
        reader
            .read_exact(&mut block[..4])
            .context("truncated Blender block header")?;
        // Only the early preview/render-info blocks are relevant. In particular,
        // do not traverse scene blocks searching for arbitrary bytes named TEST.
        if &block[..4] != b"REND" && &block[..4] != b"TEST" {
            bail!("Blender file has no embedded preview");
        }
        reader
            .read_exact(&mut block[4..block_size])
            .context("truncated Blender block header")?;
        let length = if wide_length {
            i64::from_le_bytes(block[length_offset..length_offset + 8].try_into().unwrap())
        } else {
            i64::from(i32::from_le_bytes(
                block[length_offset..length_offset + 4].try_into().unwrap(),
            ))
        };
        ensure!(
            length >= 0 && length as u64 <= MAX_PREFIX_BYTES,
            "invalid Blender block length"
        );
        if &block[..4] == b"REND" {
            let read = io::copy(&mut (&mut *reader).take(length as u64), &mut io::sink())?;
            ensure!(read == length as u64, "truncated Blender render-info block");
            continue;
        }
        ensure!(length >= 8, "invalid Blender preview block");
        let mut dimensions = [0u8; 8];
        reader
            .read_exact(&mut dimensions)
            .context("truncated Blender preview dimensions")?;
        let width = u32::from_le_bytes(dimensions[..4].try_into().unwrap());
        let height = u32::from_le_bytes(dimensions[4..].try_into().unwrap());
        let pixels = u64::from(width) * u64::from(height);
        ensure!(
            width > 0
                && height > 0
                && width <= 4096
                && height <= 4096
                && pixels <= MAX_PREVIEW_PIXELS,
            "invalid or oversized Blender preview dimensions"
        );
        ensure!(
            length as u64 == 8 + pixels * 4,
            "Blender preview length does not match its dimensions"
        );
        let mut rgba = vec![0; pixels as usize * 4];
        reader
            .read_exact(&mut rgba)
            .context("truncated Blender preview pixels")?;
        let mut preview =
            RgbaImage::from_raw(width, height, rgba).context("invalid Blender preview")?;
        // Saved preview rows run from the bottom up; the worker uses top-down RGBA.
        imageops::flip_vertical_in_place(&mut preview);
        return Ok(preview);
    }
    bail!("too many Blender preview header blocks")
}
