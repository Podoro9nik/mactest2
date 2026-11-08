use crate::types::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{
    DynamicImage, GenericImageView, ImageDecoder, ImageFormat, ImageReader, Rgb, RgbImage,
};
use std::io::Cursor;

pub const MAX_BYTES: usize = 35 * 1024 * 1024;

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
    let bytes = png_bytes(&image)?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("image_too_large".into());
    }
    Ok(data_url(&bytes))
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
