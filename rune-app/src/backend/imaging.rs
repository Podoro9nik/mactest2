use super::types::AppResult;
use base64::{Engine, engine::general_purpose::STANDARD};
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

#[cfg(test)]
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

pub fn prepare_bytes(bytes: &[u8]) -> AppResult<String> {
    // Cap again after the minimum-size upscale for exceptionally narrow inputs.
    let mut image = normalized_image(bytes)?;
    if image.width() > 2048 || image.height() > 2048 {
        image = image.thumbnail(2048, 2048);
    }
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
    let mut image = DynamicImage::ImageRgb8(rgb_on_white(&image));
    if image.width() > 2048 || image.height() > 2048 {
        image = image.thumbnail(2048, 2048);
    }
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

pub fn prepare_path(path: &std::path::Path) -> AppResult<String> {
    prepare_bytes(&read_path(path)?)
}

pub fn thumbnail(path: &std::path::Path) -> AppResult<Vec<u8>> {
    let mut image = decode(&read_path(path)?)?;
    if image.width() > 240 || image.height() > 240 {
        image = image.thumbnail(240, 240);
    }
    png_bytes(&image)
}

fn read_path(path: &std::path::Path) -> AppResult<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|_| "image_invalid")?;
    if file.metadata().map_err(|_| "image_invalid")?.len() > MAX_BYTES as u64 {
        return Err("image_too_large".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "image_invalid")?;
    if bytes.len() > MAX_BYTES {
        return Err("image_too_large".into());
    }
    Ok(bytes)
}
