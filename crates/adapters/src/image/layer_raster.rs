use f2s_application::layers::ApplyMaskStroke;
use f2s_domain::layers::RecompositionMetrics;
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::io::Cursor;

pub fn initial_mask(width: u32, height: u32, filled: bool) -> Result<Vec<u8>, String> {
    let length = (width as usize)
        .checked_mul(height as usize)
        .ok_or("mask size overflow")?;
    if length == 0 || length > 16_777_216 {
        return Err("mask dimensions outside V1 limits".into());
    }
    Ok(vec![if filled { 255 } else { 0 }; length])
}

pub fn apply_mask_stroke(
    mask: &[u8],
    width: u32,
    height: u32,
    stroke: &ApplyMaskStroke,
) -> Result<Vec<u8>, String> {
    stroke.validate()?;
    if mask.len()
        != (width as usize)
            .checked_mul(height as usize)
            .ok_or("mask size overflow")?
    {
        return Err("base mask dimensions mismatch".into());
    }
    let radius = ((stroke.radius_milli + 999) / 1000).max(1) as i32;
    let estimated = (stroke.points.len() as u64)
        .checked_mul(
            (radius as u64)
                .saturating_mul(radius as u64)
                .saturating_mul(4),
        )
        .ok_or("stroke cost overflow")?;
    if estimated > 50_000_000 {
        return Err("stroke operation budget exceeded".into());
    }
    let mut output = mask.to_vec();
    let value = if stroke.mode == "add" { 255 } else { 0 };
    let mut previous: Option<(i32, i32)> = None;
    for point in &stroke.points {
        let x = point.x_milli / 1000;
        let y = point.y_milli / 1000;
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return Err("stroke point outside canvas".into());
        }
        if let Some((px, py)) = previous {
            let dx = x - px;
            let dy = y - py;
            let steps = dx.abs().max(dy.abs()).max(1);
            for step in 0..=steps {
                paint_circle(
                    &mut output,
                    width,
                    height,
                    px + dx * step / steps,
                    py + dy * step / steps,
                    radius,
                    value,
                );
            }
        } else {
            paint_circle(&mut output, width, height, x, y, radius, value)
        }
        previous = Some((x, y));
    }
    Ok(output)
}
fn paint_circle(
    mask: &mut [u8],
    width: u32,
    height: u32,
    cx: i32,
    cy: i32,
    radius: i32,
    value: u8,
) {
    let r2 = radius * radius;
    for y in (cy - radius).max(0)..=(cy + radius).min(height as i32 - 1) {
        for x in (cx - radius).max(0)..=(cx + radius).min(width as i32 - 1) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r2 {
                mask[y as usize * width as usize + x as usize] = value
            }
        }
    }
}

pub fn render_masked_png(
    source_bytes: &[u8],
    mask: &[u8],
    expected_width: u32,
    expected_height: u32,
) -> Result<Vec<u8>, String> {
    let image =
        image::load_from_memory(source_bytes).map_err(|e| format!("source decode failed: {e}"))?;
    if image.width() != expected_width
        || image.height() != expected_height
        || mask.len() != expected_width as usize * expected_height as usize
    {
        return Err("source/mask dimensions mismatch".into());
    }
    let mut rgba = image.to_rgba8();
    for (pixel, mask_alpha) in rgba.pixels_mut().zip(mask) {
        pixel[3] = ((u16::from(pixel[3]) * u16::from(*mask_alpha) + 127) / 255) as u8;
    }
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(rgba)
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}

/// Normalizes a user-supplied, full-canvas transparent PNG into the exact
/// attachment/mask pair consumed by LayerSet. The alpha channel becomes the
/// authoritative layer mask; RGB bytes remain straight-alpha PNG data.
pub fn normalize_manual_layer_png(
    source_bytes: &[u8],
    expected_width: u32,
    expected_height: u32,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let format = image::guess_format(source_bytes)
        .map_err(|error| format!("manual layer format probe failed: {error}"))?;
    if format != ImageFormat::Png {
        return Err("manual layer replacement must be a transparent PNG".into());
    }
    let image = image::load_from_memory_with_format(source_bytes, ImageFormat::Png)
        .map_err(|error| format!("manual layer PNG decode failed: {error}"))?;
    if image.dimensions() != (expected_width, expected_height) {
        return Err("manual layer PNG must match the approved master canvas exactly".into());
    }
    if !image.color().has_alpha() {
        return Err("manual layer PNG requires an alpha channel".into());
    }
    let rgba = image.to_rgba8();
    let alpha = rgba.pixels().map(|pixel| pixel[3]).collect::<Vec<_>>();
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(rgba)
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok((output.into_inner(), alpha))
}

/// Applies an authoritative mask transition without replacing hand-authored
/// attachment pixels with pixels from the approved master.
///
/// Pixels covered by `old_mask` keep the current attachment's straight-alpha
/// RGB. Pixels newly covered outside `old_mask` use the master as a fallback.
/// Subtracted pixels retain their attachment RGB with zero alpha, so encoding
/// the transition never premultiplies or destroys hand-authored color bytes.
pub fn render_updated_layer_attachment_png(
    current_attachment_png: &[u8],
    master_source_bytes: &[u8],
    old_mask: &[u8],
    new_mask: &[u8],
    expected_width: u32,
    expected_height: u32,
) -> Result<Vec<u8>, String> {
    let pixel_count = checked_canvas_pixels(expected_width, expected_height)?;
    if old_mask.len() != pixel_count || new_mask.len() != pixel_count {
        return Err("attachment mask transition dimensions mismatch".into());
    }

    let attachment_format = image::guess_format(current_attachment_png)
        .map_err(|error| format!("current layer attachment format probe failed: {error}"))?;
    if attachment_format != ImageFormat::Png {
        return Err("current layer attachment must be a PNG".into());
    }
    let attachment = image::load_from_memory_with_format(current_attachment_png, ImageFormat::Png)
        .map_err(|error| format!("current layer attachment decode failed: {error}"))?;
    if attachment.dimensions() != (expected_width, expected_height) {
        return Err("current layer attachment dimensions mismatch".into());
    }
    if !attachment.color().has_alpha() {
        return Err("current layer attachment requires an alpha channel".into());
    }
    let attachment = attachment.to_rgba8();

    let master = image::load_from_memory(master_source_bytes)
        .map_err(|error| format!("master source decode failed: {error}"))?;
    if master.dimensions() != (expected_width, expected_height) {
        return Err("master source dimensions mismatch".into());
    }
    let master = master.to_rgba8();

    let mut output = attachment.clone();
    for (index, output_pixel) in output.pixels_mut().enumerate() {
        let current_pixel = attachment.get_pixel(
            (index % expected_width as usize) as u32,
            (index / expected_width as usize) as u32,
        );
        let old_alpha = old_mask[index];
        let new_alpha = new_mask[index];
        if current_pixel[3] > old_alpha {
            return Err("current layer attachment alpha exceeds its old mask".into());
        }

        if old_alpha == 0 && new_alpha != 0 {
            let master_pixel = master.get_pixel(
                (index % expected_width as usize) as u32,
                (index / expected_width as usize) as u32,
            );
            output_pixel.0 = [
                master_pixel[0],
                master_pixel[1],
                master_pixel[2],
                multiply_alpha(master_pixel[3], new_alpha),
            ];
        } else {
            // PNG stores unassociated (straight) alpha. Preserve RGB byte-for-byte
            // and only transform alpha when the mask itself changed.
            output_pixel[3] = if old_alpha == new_alpha {
                current_pixel[3]
            } else if new_alpha == 0 || current_pixel[3] == 0 {
                0
            } else {
                let unmasked_alpha = ((u32::from(current_pixel[3]) * 255
                    + u32::from(old_alpha) / 2)
                    / u32::from(old_alpha))
                .min(255) as u8;
                multiply_alpha(unmasked_alpha, new_alpha)
            };
        }
    }

    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(output)
        .write_to(&mut encoded, ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(encoded.into_inner())
}

fn checked_canvas_pixels(width: u32, height: u32) -> Result<usize, String> {
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or("attachment canvas size overflow")?;
    if pixels == 0 || pixels > 16_777_216 {
        return Err("attachment canvas dimensions outside V1 limits".into());
    }
    Ok(pixels)
}

fn multiply_alpha(content_alpha: u8, mask_alpha: u8) -> u8 {
    ((u16::from(content_alpha) * u16::from(mask_alpha) + 127) / 255) as u8
}

pub fn render_safe_preview_png(source_bytes: &[u8], max_side: u32) -> Result<Vec<u8>, String> {
    if !(64..=320).contains(&max_side) {
        return Err("preview dimension outside policy".into());
    }
    let image = image::load_from_memory(source_bytes)
        .map_err(|error| format!("preview source decode failed: {error}"))?;
    let pixels = u64::from(image.width())
        .checked_mul(u64::from(image.height()))
        .ok_or("preview source dimensions overflow")?;
    if pixels == 0 || pixels > 16_777_216 {
        return Err("preview source exceeds decoded policy".into());
    }
    let preview = image.thumbnail(max_side, max_side);
    let mut output = Cursor::new(Vec::new());
    preview
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    let bytes = output.into_inner();
    if bytes.len() > 512 * 1024 {
        return Err("encoded preview exceeds IPC policy".into());
    }
    Ok(bytes)
}

pub fn recomposition_metrics(
    source_bytes: &[u8],
    masks: &[Vec<u8>],
    width: u32,
    height: u32,
) -> Result<RecompositionMetrics, String> {
    let source = image::load_from_memory(source_bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    if source.dimensions() != (width, height)
        || masks
            .iter()
            .any(|v| v.len() != width as usize * height as usize)
    {
        return Err("recomposition inputs mismatch".into());
    }
    let mut missing = 0;
    let mut overlap = 0;
    let mut alpha_error = 0;
    let mut layer_has_visible_pixels = vec![false; masks.len()];
    for (index, pixel) in source.pixels().enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        for (layer_index, mask) in masks.iter().enumerate() {
            if mask[index] != 0 {
                layer_has_visible_pixels[layer_index] = true;
            }
        }
        let sum: u32 = masks.iter().map(|v| u32::from(v[index])).sum();
        if sum == 0 {
            missing += 1
        } else if sum > 255 {
            overlap += 1
        }
        if sum != 255 {
            alpha_error += 1
        }
    }
    Ok(RecompositionMetrics {
        missing_pixels: missing,
        overlap_pixels: overlap,
        changed_visible_pixels: 0,
        alpha_error_pixels: alpha_error,
        empty_layer_masks: layer_has_visible_pixels
            .iter()
            .filter(|has_pixels| !**has_pixels)
            .count() as u64,
    })
}

pub fn changed_attachment_pixels(
    source_bytes: &[u8],
    masks: &[Vec<u8>],
    attachment_bytes: &[Vec<u8>],
    width: u32,
    height: u32,
) -> Result<u64, String> {
    if masks.len() != attachment_bytes.len() {
        return Err("attachment count does not match mask count".into());
    }
    let source = image::load_from_memory(source_bytes)
        .map_err(|error| error.to_string())?
        .to_rgba8();
    if source.dimensions() != (width, height) {
        return Err("attachment QA source dimensions mismatch".into());
    }
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or("attachment QA size overflow")?;
    let mut changed = vec![false; pixel_count];
    for (mask, bytes) in masks.iter().zip(attachment_bytes) {
        if mask.len() != pixel_count {
            return Err("attachment QA mask dimensions mismatch".into());
        }
        let attachment = image::load_from_memory(bytes)
            .map_err(|error| format!("layer attachment decode failed: {error}"))?
            .to_rgba8();
        if attachment.dimensions() != (width, height) {
            return Err("layer attachment dimensions mismatch".into());
        }
        for (index, (source_pixel, actual_pixel)) in
            source.pixels().zip(attachment.pixels()).enumerate()
        {
            let expected_alpha =
                ((u16::from(source_pixel[3]) * u16::from(mask[index]) + 127) / 255) as u8;
            if actual_pixel[0] != source_pixel[0]
                || actual_pixel[1] != source_pixel[1]
                || actual_pixel[2] != source_pixel[2]
                || actual_pixel[3] != expected_alpha
            {
                changed[index] = true;
            }
        }
    }
    Ok(changed.iter().filter(|value| **value).count() as u64)
}
