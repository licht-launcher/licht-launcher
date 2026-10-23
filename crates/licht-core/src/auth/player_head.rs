use std::fs;
use std::io::{Cursor, ErrorKind};
use std::path::PathBuf;

use futures_util::{StreamExt, future::BoxFuture};
use reqwest::Client;

use super::endpoints::DEFAULT_SKIN_URL;
use super::parse::validate_skin_url;
use crate::{CoreError, Result, SharedCache};

const MAX_PNG_BYTES: usize = 256 * 1024;
type FetchTexture = fn(Client, String) -> BoxFuture<'static, Result<Vec<u8>>>;

/// Return an integrity-checked local head, preferring the player's skin over Steve.
pub async fn cached_player_head(
    cache: &SharedCache,
    client: &Client,
    skin_url: Option<&str>,
) -> Result<PathBuf> {
    load_head(cache, client, skin_url, fetch_texture).await
}

async fn load_head(
    cache: &SharedCache,
    client: &Client,
    skin_url: Option<&str>,
    fetch: FetchTexture,
) -> Result<PathBuf> {
    let source = skin_url
        .map(validate_skin_url)
        .transpose()
        .map_err(|_| CoreError::SkinImage)?
        .unwrap_or_else(|| DEFAULT_SKIN_URL.to_string());
    if let Some(path) = read_cached(cache, &source)? {
        return Ok(path);
    }
    let _publishing = crate::store::layout::initialization_lock(cache, "player-heads.lock").await?;
    match load_source(cache, client, &source, fetch).await {
        Ok(path) => Ok(path),
        Err(_) if source != DEFAULT_SKIN_URL => {
            tracing::warn!("Player skin head unavailable; using the default skin head");
            load_source(cache, client, DEFAULT_SKIN_URL, fetch).await
        }
        Err(error) => Err(error),
    }
}

async fn load_source(
    cache: &SharedCache,
    client: &Client,
    source: &str,
    fetch: FetchTexture,
) -> Result<PathBuf> {
    if let Some(path) = read_cached(cache, source)? {
        return Ok(path);
    }
    let texture = fetch(client.clone(), source.to_string()).await?;
    let head = render_head(&texture)?;
    let destination = head_path(cache, source);
    fs::create_dir_all(destination.parent().ok_or(CoreError::CachePath)?)?;
    let digest = crate::hashes::sha1_hex(&head);
    let temporary = destination.with_extension("png.part");
    fs::write(&temporary, head)?;
    fs::rename(&temporary, &destination)?;
    let receipt = destination.with_extension("sha1");
    let temporary_receipt = destination.with_extension("sha1.part");
    fs::write(&temporary_receipt, digest)?;
    fs::rename(temporary_receipt, receipt)?;
    Ok(destination)
}

fn head_path(cache: &SharedCache, source: &str) -> PathBuf {
    let key = crate::hashes::sha1_hex(source.as_bytes());
    cache
        .root()
        .join("cache")
        .join("player-heads")
        .join(format!("{key}.png"))
}

fn read_cached(cache: &SharedCache, source: &str) -> Result<Option<PathBuf>> {
    let path = head_path(cache, source);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.len() > MAX_PNG_BYTES as u64 {
        return Ok(None);
    }
    let receipt = path.with_extension("sha1");
    match fs::metadata(&receipt) {
        Ok(metadata) if metadata.len() != 40 => return Ok(None),
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let expected = match fs::read(receipt) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if bytes.len() <= MAX_PNG_BYTES
        && expected == crate::hashes::sha1_hex(&bytes).as_bytes()
        && decode_png(&bytes).is_ok_and(|image| image.width == 8 && image.height == 8)
    {
        return Ok(Some(path));
    }
    tracing::warn!("Cached player head failed integrity verification; rebuilding");
    Ok(None)
}

fn fetch_texture(client: Client, source: String) -> BoxFuture<'static, Result<Vec<u8>>> {
    Box::pin(async move {
        let response = client.get(&source).send().await?.error_for_status()?;
        let final_url = response.url().as_str();
        if final_url != DEFAULT_SKIN_URL && validate_skin_url(final_url).is_err() {
            return Err(CoreError::SkinImage);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_PNG_BYTES as u64)
        {
            return Err(CoreError::SkinImage);
        }
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if bytes.len().saturating_add(chunk.len()) > MAX_PNG_BYTES {
                return Err(CoreError::SkinImage);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    })
}

struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn decode_png(bytes: &[u8]) -> Result<Pixels> {
    if bytes.len() > MAX_PNG_BYTES {
        return Err(CoreError::SkinImage);
    }
    let mut decoder =
        png::Decoder::new_with_limits(Cursor::new(bytes), png::Limits { bytes: 1024 * 1024 });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| CoreError::SkinImage)?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > 64
        || info.height > 64
        || info.animation_control.is_some()
    {
        return Err(CoreError::SkinImage);
    }
    let mut output = vec![0; reader.output_buffer_size().ok_or(CoreError::SkinImage)?];
    let frame = reader
        .next_frame(&mut output)
        .map_err(|_| CoreError::SkinImage)?;
    let channels = match frame.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return Err(CoreError::SkinImage),
    };
    let mut rgba = Vec::new();
    for pixel in output[..frame.buffer_size()].chunks_exact(channels) {
        match channels {
            4 => rgba.extend_from_slice(pixel),
            3 => rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]),
            2 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]),
            _ => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255]),
        }
    }
    Ok(Pixels {
        width: frame.width,
        height: frame.height,
        rgba,
    })
}

fn render_head(bytes: &[u8]) -> Result<Vec<u8>> {
    let skin = decode_png(bytes)?;
    if skin.width != 64 || !matches!(skin.height, 32 | 64) {
        return Err(CoreError::SkinImage);
    }
    let mut pixels = vec![0; 8 * 8 * 4];
    for y in 0..8 {
        for x in 0..8 {
            let face = ((y + 8) * 64 + x + 8) * 4;
            let hat = ((y + 8) * 64 + x + 40) * 4;
            let destination = (y * 8 + x) * 4;
            let front_alpha = u32::from(skin.rgba[hat + 3]);
            let back_alpha = u32::from(skin.rgba[face + 3]);
            let alpha = front_alpha * 255 + back_alpha * (255 - front_alpha);
            for channel in 0..3 {
                let front = u32::from(skin.rgba[hat + channel]);
                let back = u32::from(skin.rgba[face + channel]);
                let color =
                    front * front_alpha * 255 + back * back_alpha * (255 - front_alpha) + alpha / 2;
                pixels[destination + channel] = color.checked_div(alpha).unwrap_or(0) as u8;
            }
            pixels[destination + 3] = ((alpha + 127) / 255) as u8;
        }
    }
    encode_head(&pixels)
}

fn encode_head(pixels: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 8, 8);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| CoreError::SkinImage)?;
        writer
            .write_image_data(pixels)
            .map_err(|_| CoreError::SkinImage)?;
        writer.finish().map_err(|_| CoreError::SkinImage)?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
