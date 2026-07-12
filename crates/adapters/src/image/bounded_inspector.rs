use f2s_application::ports::{ImageFacts, ImageInspector};
pub struct BoundedImageInspector;
impl ImageInspector for BoundedImageInspector {
    fn inspect(&self, bytes: &[u8]) -> Result<ImageFacts, String> {
        if bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
            return png(bytes);
        }
        if bytes.starts_with(&[0xff, 0xd8]) {
            return jpeg(bytes);
        }
        if bytes.len() >= 20 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return webp(bytes);
        }
        Err("magic is not PNG, JPEG or WebP".into())
    }
}
fn png(b: &[u8]) -> Result<ImageFacts, String> {
    if b.len() < 29 || &b[12..16] != b"IHDR" {
        return Err("truncated PNG IHDR".into());
    }
    let width = u32::from_be_bytes(b[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(b[20..24].try_into().unwrap());
    let depth = b[24];
    let color = b[25];
    if !matches!(color, 0 | 2 | 3 | 4 | 6) {
        return Err("invalid PNG color type".into());
    }
    Ok(ImageFacts {
        media_type: "image/png".into(),
        width,
        height,
        bit_depth: depth,
        has_alpha: matches!(color, 4 | 6),
    })
}
fn jpeg(b: &[u8]) -> Result<ImageFacts, String> {
    let mut p = 2;
    while p + 4 <= b.len() {
        if b[p] != 0xff {
            p += 1;
            continue;
        }
        while p < b.len() && b[p] == 0xff {
            p += 1
        }
        if p >= b.len() {
            break;
        }
        let marker = b[p];
        p += 1;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if p + 2 > b.len() {
            break;
        }
        let len = u16::from_be_bytes([b[p], b[p + 1]]) as usize;
        if len < 2 || p + len > b.len() {
            return Err("malformed JPEG segment".into());
        }
        if matches!(marker, 0xc0 | 0xc1 | 0xc2) {
            if len < 8 {
                return Err("truncated JPEG SOF".into());
            }
            return Ok(ImageFacts {
                media_type: "image/jpeg".into(),
                bit_depth: b[p + 2],
                height: u16::from_be_bytes([b[p + 3], b[p + 4]]) as u32,
                width: u16::from_be_bytes([b[p + 5], b[p + 6]]) as u32,
                has_alpha: false,
            });
        }
        p += len
    }
    Err("JPEG dimensions not found".into())
}
fn le24(b: &[u8]) -> u32 {
    u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16)
}
fn webp(b: &[u8]) -> Result<ImageFacts, String> {
    let kind = &b[12..16];
    let data = &b[20..];
    let (width, height, alpha) = match kind {
        b"VP8X" => {
            if data.len() < 10 {
                return Err("truncated WebP VP8X".into());
            }
            (
                le24(&data[4..7]) + 1,
                le24(&data[7..10]) + 1,
                data[0] & 0x10 != 0,
            )
        }
        b"VP8L" => {
            if data.len() < 5 || data[0] != 0x2f {
                return Err("malformed WebP VP8L".into());
            }
            let bits = u32::from_le_bytes(data[1..5].try_into().unwrap());
            ((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1, true)
        }
        b"VP8 " => {
            if data.len() < 10 || data[3..6] != [0x9d, 0x01, 0x2a] {
                return Err("malformed WebP VP8".into());
            }
            (
                u16::from_le_bytes([data[6], data[7]]) as u32 & 0x3fff,
                u16::from_le_bytes([data[8], data[9]]) as u32 & 0x3fff,
                false,
            )
        }
        _ => return Err("unsupported WebP chunk".into()),
    };
    Ok(ImageFacts {
        media_type: "image/webp".into(),
        width,
        height,
        bit_depth: 8,
        has_alpha: alpha,
    })
}
