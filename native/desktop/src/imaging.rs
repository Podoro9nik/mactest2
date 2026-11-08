use crate::types::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{
    DynamicImage, GenericImageView, ImageDecoder, ImageFormat, ImageReader, Rgb, RgbImage,
};
use std::io::Cursor;

pub const MAX_BYTES: usize = 35 * 1024 * 1024;
// Elyum rejects JSON request bodies over 4 MiB. Leave room for base64's
// 4/3 expansion and the JSON-RPC envelope, not just the compressed file.
pub const MAX_ELYUM_IMAGE_BYTES: usize = 2_900_000;

pub fn decode(bytes: &[u8]) -> AppResult<DynamicImage> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("image_invalid".into());
    }
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "image_invalid")?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    ) {
        return Err("image_invalid".into());
    }
    let mut decoder = reader.into_decoder().map_err(|_| "image_invalid")?;
    let (w, h) = decoder.dimensions();
    if (w as u64) * (h as u64) > 40_000_000 || w == 0 || h == 0 {
        return Err("image_too_large".into());
    }
    let orientation = decoder.orientation().map_err(|_| "image_invalid")?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| "image_invalid")?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn data_bytes(value: &str) -> AppResult<Vec<u8>> {
    let (header, encoded) = value.split_once(',').ok_or("image_invalid")?;
    if ![
        "data:image/png;base64",
        "data:image/jpeg;base64",
        "data:image/webp;base64",
    ]
    .contains(&header)
        || encoded.len() > (MAX_BYTES * 4 / 3 + 8)
    {
        return Err("image_invalid".into());
    }
    STANDARD.decode(encoded).map_err(|_| "image_invalid".into())
}

pub fn rgb_on_white(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    let mut rgb = RgbImage::new(rgba.width(), rgba.height());
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = u32::from(p[3]);
        rgb.put_pixel(
            x,
            y,
            Rgb(std::array::from_fn(|i| {
                ((u32::from(p[i]) * a + 255 * (255 - a) + 127) / 255) as u8
            })),
        );
    }
    rgb
}

pub fn png_bytes(image: &DynamicImage) -> AppResult<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|_| "image_invalid")?;
    Ok(bytes.into_inner())
}

pub fn data_url(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}

pub fn prepare(value: &str) -> AppResult<String> {
    let bytes = data_bytes(value)?;
    prepare_bytes(&bytes)
}

pub fn prepare_bytes(bytes: &[u8]) -> AppResult<String> {
    let image = normalized_image(bytes)?;
    let bytes = png_bytes(&image)?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("image_too_large".into());
    }
    Ok(data_url(&bytes))
}

pub fn prepare_elyum(value: &str) -> AppResult<String> {
    prepare_elyum_bytes(&data_bytes(value)?)
}

pub fn prepare_elyum_bytes(bytes: &[u8]) -> AppResult<String> {
    // Cap again after the minimum-size upscale for exceptionally narrow inputs.
    let mut image = normalized_image(bytes)?.thumbnail(2048, 2048);
    let lossless = png_bytes(&image)?;
    if lossless.len() <= MAX_ELYUM_IMAGE_BYTES {
        return Ok(data_url(&lossless));
    }
    loop {
        for quality in [92, 85] {
            let mut encoded = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, quality)
                .encode_image(&image)
                .map_err(|_| "image_invalid")?;
            if encoded.len() <= MAX_ELYUM_IMAGE_BYTES {
                return Ok(format!(
                    "data:image/jpeg;base64,{}",
                    STANDARD.encode(encoded)
                ));
            }
        }
        let (w, h) = image.dimensions();
        let (next_w, next_h) = (w * 3 / 4, h * 3 / 4);
        if next_w.min(next_h) < 64 {
            return Err("image_too_large".into());
        }
        image = image.resize(next_w, next_h, image::imageops::FilterType::Lanczos3);
    }
}

fn normalized_image(bytes: &[u8]) -> AppResult<DynamicImage> {
    let image = decode(bytes)?;
    let mut image = DynamicImage::ImageRgb8(rgb_on_white(&image)).thumbnail(2048, 2048);
    let (w, h) = image.dimensions();
    if w.min(h) < 64 {
        return Err("image_too_small".into());
    }
    if w.min(h) < 256 {
        let scale = 256.0 / f64::from(w.min(h));
        image = image.resize(
            (f64::from(w) * scale) as u32,
            (f64::from(h) * scale) as u32,
            image::imageops::FilterType::Lanczos3,
        );
    }
    Ok(image)
}

/// Edge-aware median and an undithered palette flatten low-contrast speckle.
/// All dark ink and high-contrast edge locations remain unchanged.
pub fn clean_flat_art(image: &DynamicImage) -> DynamicImage {
    let source = rgb_on_white(image);
    let (w, h) = source.dimensions();
    let mut filtered = source.clone();
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let p = source.get_pixel(x, y);
            if luminance(p) < 150 {
                continue;
            }
            let mut channels = [[0u8; 9]; 3];
            let mut idx = 0;
            for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    let q = source.get_pixel(xx, yy);
                    for c in 0..3 {
                        channels[c][idx] = q[c];
                    }
                    idx += 1;
                }
            }
            let mut low_contrast = true;
            for c in &mut channels {
                c.sort_unstable();
                if c[8] - c[0] >= 38 {
                    low_contrast = false;
                }
            }
            if low_contrast {
                filtered.put_pixel(x, y, Rgb(std::array::from_fn(|i| channels[i][4])));
            }
        }
    }
    let rgba = DynamicImage::ImageRgb8(filtered).to_rgba8();
    let quantizer = color_quant::NeuQuant::new(10, 64, rgba.as_raw());
    let palette = quantizer.color_map_rgba();
    let mut counts = vec![0usize; 64];
    for p in rgba.pixels() {
        counts[quantizer.index_of(&p.0)] += 1;
    }
    let mut indices: Vec<usize> = (0..64).collect();
    indices.sort_by_key(|&i| std::cmp::Reverse(counts[i]));
    let mut mapped = vec![[0u8; 3]; 64];
    let mut anchors: Vec<[u8; 3]> = Vec::new();
    for i in indices {
        let color = [palette[i * 4], palette[i * 4 + 1], palette[i * 4 + 2]];
        let anchor = anchors
            .iter()
            .find(|a| {
                let d: [u16; 3] = std::array::from_fn(|c| u16::from(color[c].abs_diff(a[c])));
                *d.iter().max().unwrap() <= 24 && d.iter().sum::<u16>() <= 54
            })
            .copied();
        mapped[i] = anchor.unwrap_or_else(|| {
            anchors.push(color);
            color
        });
    }
    let mut output = source.clone();
    for (x, y, p) in rgba.enumerate_pixels() {
        if luminance(source.get_pixel(x, y)) >= 150 {
            output.put_pixel(x, y, Rgb(mapped[quantizer.index_of(&p.0)]));
        }
    }
    DynamicImage::ImageRgb8(output)
}

fn luminance(p: &Rgb<u8>) -> u32 {
    (u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_fills_are_white_and_png_valid() {
        let image = DynamicImage::new_rgba8(256, 256);
        let url = prepare_bytes(&png_bytes(&image).unwrap()).unwrap();
        let decoded = decode(&data_bytes(&url).unwrap()).unwrap().to_rgb8();
        assert_eq!(decoded.get_pixel(0, 0).0, [255, 255, 255]);
    }
    #[test]
    fn rejects_non_raster_and_malformed_data() {
        assert!(data_bytes("file:///secret").is_err());
        assert!(decode(b"<svg></svg>").is_err());
        assert!(data_bytes("data:image/svg+xml;base64,AA==").is_err());
    }
    #[test]
    fn elyum_upload_fits_json_body_limit_for_a_noisy_photo() {
        // A high-entropy photograph is the worst case for the old PNG path.
        let mut noise = 0x9e3779b9u32;
        let image = RgbImage::from_fn(2048, 2048, |_, _| {
            Rgb(std::array::from_fn(|_| {
                noise ^= noise << 13;
                noise ^= noise >> 17;
                noise ^= noise << 5;
                noise as u8
            }))
        });
        let source = png_bytes(&DynamicImage::ImageRgb8(image)).unwrap();
        assert!(source.len() > 4 * 1024 * 1024);
        let prepared = prepare_elyum_bytes(&source).unwrap();
        assert!(prepared.starts_with("data:image/jpeg;base64,"));
        let bytes = data_bytes(&prepared).unwrap();
        assert!(bytes.len() <= MAX_ELYUM_IMAGE_BYTES);
        assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Jpeg);
        let decoded = decode(&bytes).unwrap();
        assert!(decoded.width() <= 2048 && decoded.height() <= 2048);
        assert_eq!(decoded.width(), decoded.height());
        let body = serde_json::json!({"jsonrpc":"2.0","id":u64::MAX,"method":"tools/call","params":{"name":"elyum_upload","arguments":{"base64":prepared.split_once(',').unwrap().1,"mimeType":"image/jpeg","filename":"reference-10.jpg"}}});
        assert!(serde_json::to_vec(&body).unwrap().len() < 4 * 1024 * 1024);
        // Codex keeps its lossless path and is not constrained by Elyum's body cap.
        let codex = prepare_bytes(&source).unwrap();
        assert!(codex.starts_with("data:image/png;base64,"));
        assert!(data_bytes(&codex).unwrap().len() > MAX_ELYUM_IMAGE_BYTES);
    }
    #[test]
    fn elyum_small_art_stays_lossless_and_transparency_is_white() {
        let source = png_bytes(&DynamicImage::new_rgba8(256, 256)).unwrap();
        let prepared = prepare_elyum_bytes(&source).unwrap();
        assert!(prepared.starts_with("data:image/png;base64,"));
        let image = decode(&data_bytes(&prepared).unwrap()).unwrap().to_rgb8();
        assert_eq!(image.get_pixel(0, 0).0, [255, 255, 255]);
    }
    #[test]
    fn clean_art_preserves_ink_and_reduces_speckles() {
        let mut im = RgbImage::from_pixel(40, 40, Rgb([244, 238, 241]));
        for y in 12..28 {
            im.put_pixel(20, y, Rgb([20, 18, 19]));
        }
        im.put_pixel(8, 8, Rgb([232, 229, 233]));
        let clean = clean_flat_art(&DynamicImage::ImageRgb8(im.clone())).to_rgb8();
        for y in 12..28 {
            assert_eq!(clean.get_pixel(20, y), im.get_pixel(20, y));
        }
        assert_eq!(clean.get_pixel(8, 8), clean.get_pixel(8, 9));
    }
}
