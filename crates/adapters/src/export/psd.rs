use image::{GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PsdLayer {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub visible: bool,
    pub opacity: u8,
    pub origin_x: i32,
    pub origin_y: i32,
}

pub fn psd_layer_from_png(
    name: impl Into<String>,
    png_bytes: &[u8],
    visible: bool,
) -> Result<PsdLayer, String> {
    let image = image::load_from_memory_with_format(png_bytes, ImageFormat::Png)
        .map_err(|error| format!("cannot decode approved PNG for PSD: {error}"))?;
    let (width, height) = image.dimensions();
    let rgba = image.into_rgba8().into_raw();
    Ok(PsdLayer {
        name: name.into(),
        width,
        height,
        rgba,
        visible,
        opacity: u8::MAX,
        origin_x: 0,
        origin_y: 0,
    })
}
fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}
fn bei16(v: i16) -> [u8; 2] {
    v.to_be_bytes()
}
fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}
fn bei32(v: i32) -> [u8; 4] {
    v.to_be_bytes()
}
fn pascal4(name: &str) -> Vec<u8> {
    let ascii = name
        .chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'_' })
        .take(255)
        .collect::<Vec<_>>();
    let total = (1 + ascii.len() + 3) / 4 * 4;
    let mut out = vec![0; total];
    out[0] = ascii.len() as u8;
    out[1..1 + ascii.len()].copy_from_slice(&ascii);
    out
}
fn unicode_name(name: &str) -> Vec<u8> {
    let utf16 = name.encode_utf16().collect::<Vec<_>>();
    let mut data = Vec::new();
    data.extend(be32(utf16.len() as u32));
    for unit in utf16 {
        data.extend(be16(unit))
    }
    let mut block = b"8BIMluni".to_vec();
    block.extend(be32(data.len() as u32));
    block.extend(data);
    if block.len() % 2 == 1 {
        block.push(0)
    }
    block
}
pub fn minimal_psd_bytes(width: u32, height: u32, layers: &[PsdLayer]) -> Result<Vec<u8>, String> {
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || layers.is_empty()
        || layers.len() > 256
    {
        return Err("PSD canvas/layers outside minimal profile".into());
    }
    let canvas_pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or("PSD size overflow")?;
    let total_layer_pixels = layers.iter().try_fold(0usize, |total, layer| {
        let pixels = (layer.width as usize)
            .checked_mul(layer.height as usize)
            .ok_or("PSD layer size overflow")?;
        total.checked_add(pixels).ok_or("PSD total pixel overflow")
    })?;
    if canvas_pixels > 67_108_864 || total_layer_pixels > 268_435_456 {
        return Err("PSD decoded pixel budget exceeded".into());
    }
    let mut records = Vec::new();
    let mut channel_pixels = Vec::new();
    for layer in layers {
        let pixels = (layer.width as usize)
            .checked_mul(layer.height as usize)
            .ok_or("layer size overflow")?;
        if layer.rgba.len() != pixels * 4 {
            return Err("PSD layer RGBA length mismatch".into());
        }
        records.extend(bei32(layer.origin_y));
        records.extend(bei32(layer.origin_x));
        records.extend(bei32(layer.origin_y + layer.height as i32));
        records.extend(bei32(layer.origin_x + layer.width as i32));
        records.extend(be16(4));
        let channel_len = (2 + pixels) as u32;
        for id in [0i16, 1, 2, -1] {
            records.extend(bei16(id));
            records.extend(be32(channel_len))
        }
        records.extend(b"8BIMnorm");
        records.push(layer.opacity);
        records.push(0);
        records.push(if layer.visible { 0 } else { 2 });
        records.push(0);
        let mut extra = Vec::new();
        extra.extend(be32(0));
        extra.extend(be32(0));
        extra.extend(pascal4(&layer.name));
        extra.extend(unicode_name(&layer.name));
        records.extend(be32(extra.len() as u32));
        records.extend(extra);
        for channel in [0usize, 1, 2, 3] {
            channel_pixels.extend(be16(0));
            for pixel in layer.rgba.chunks_exact(4) {
                channel_pixels.push(pixel[channel])
            }
        }
    }
    let mut layer_info = Vec::new();
    layer_info.extend(bei16(layers.len() as i16));
    layer_info.extend(records);
    layer_info.extend(channel_pixels);
    if layer_info.len() % 2 == 1 {
        layer_info.push(0)
    }
    let mut layer_mask = Vec::new();
    layer_mask.extend(be32(layer_info.len() as u32));
    layer_mask.extend(layer_info);
    layer_mask.extend(be32(0));
    let mut out = b"8BPS".to_vec();
    out.extend(be16(1));
    out.extend([0u8; 6]);
    out.extend(be16(4));
    out.extend(be32(height));
    out.extend(be32(width));
    out.extend(be16(8));
    out.extend(be16(3));
    out.extend(be32(0));
    out.extend(be32(0));
    out.extend(be32(layer_mask.len() as u32));
    out.extend(layer_mask);
    out.extend(be16(0));
    for channel in 0..4 {
        for index in 0..canvas_pixels {
            let value = layers
                .iter()
                .rev()
                .find_map(|l| {
                    let x = (index % width as usize) as i32;
                    let y = (index / width as usize) as i32;
                    if x >= l.origin_x
                        && y >= l.origin_y
                        && x < l.origin_x + l.width as i32
                        && y < l.origin_y + l.height as i32
                    {
                        let local = ((y - l.origin_y) as usize * l.width as usize
                            + (x - l.origin_x) as usize)
                            * 4;
                        Some(l.rgba[local + channel])
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            out.push(value)
        }
    }
    Ok(out)
}
