use f2s_adapters::{
    image::{
        BoundedImageInspector, apply_mask_stroke, changed_attachment_pixels, decode_image_bounded,
        initial_mask, normalize_manual_layer_png, recomposition_metrics, render_masked_png,
        render_updated_layer_attachment_png,
    },
    storage::FsCas,
};
use f2s_application::layers::{ApplyMaskStroke, StrokePoint};
use f2s_application::{
    import::{inspect_bounded, promote_image},
    ports::ImageInspector,
};
use f2s_domain::import::ImportLimits;
use image::{DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use std::fs;
use std::io::Cursor;
#[test]
fn png_fixture_preflights_and_promotes() {
    let bytes = fs::read("../../fixtures/m00/synthetic-character/master.png").unwrap();
    let facts = inspect_bounded(&BoundedImageInspector, &ImportLimits::default(), &bytes).unwrap();
    assert_eq!((facts.width, facts.height, facts.bit_depth), (512, 512, 8));
    let root = std::env::temp_dir().join("f2s-image-test");
    let _ = fs::remove_dir_all(&root);
    let artifact =
        promote_image(&FsCas::new(&root), &bytes, &facts, "user-local-selection").unwrap();
    assert_eq!(artifact.approval_state, "UNAPPROVED");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn extension_spoof_and_non_eight_bit_are_rejected() {
    assert!(BoundedImageInspector.inspect(b"not a png").is_err());
    let mut png = fs::read("../../fixtures/m00/synthetic-character/master.png").unwrap();
    png[24] = 16;
    let facts = BoundedImageInspector.inspect(&png).unwrap();
    assert!(
        f2s_application::ports::validate_preflight(
            &ImportLimits::default(),
            png.len() as u64,
            &facts
        )
        .is_err()
    );
}
#[test]
fn limits_fail_at_plus_one() {
    let limits = ImportLimits {
        max_file_bytes: 10,
        max_pixels: 100,
        max_compression_ratio: 200,
        absolute_file_bytes: 20,
        absolute_pixels: 200,
        absolute_compression_ratio: 500,
    };
    assert!(limits.validate(11, 2, 2).is_err());
    assert!(limits.validate(10, 10, 11).is_err());
}

#[test]
fn full_decoder_validates_the_same_bytes_before_preview_or_cas() {
    let bytes = fs::read("../../fixtures/m00/synthetic-character/master.png").unwrap();
    let report = decode_image_bounded(&bytes, &ImportLimits::default()).unwrap();
    assert!(report.complete_decode);
    assert_eq!(report.bit_depth, 8);
    assert!(decode_image_bounded(&bytes[..bytes.len() / 3], &ImportLimits::default()).is_err());
}

fn opaque_test_png(width: u32, height: u32) -> Vec<u8> {
    let image = RgbaImage::from_pixel(width, height, Rgba([20, 40, 60, 255]));
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut output, ImageFormat::Png)
        .unwrap();
    output.into_inner()
}

#[test]
fn layer_masks_are_replayed_and_qa_is_recomputed_from_pixels() {
    let source = opaque_test_png(4, 4);
    let full = initial_mask(4, 4, true).unwrap();
    let empty = initial_mask(4, 4, false).unwrap();
    let incomplete = recomposition_metrics(&source, &[full.clone(), empty], 4, 4).unwrap();
    assert_eq!(incomplete.empty_layer_masks, 1);
    assert!(!incomplete.passes());

    let mut left = vec![0; 16];
    let mut right = vec![0; 16];
    for y in 0..4 {
        for x in 0..4 {
            if x < 2 {
                left[y * 4 + x] = 255;
            } else {
                right[y * 4 + x] = 255;
            }
        }
    }
    let exact = recomposition_metrics(&source, &[left.clone(), right.clone()], 4, 4).unwrap();
    assert!(exact.passes());
    let png = render_masked_png(&source, &left, 4, 4).unwrap();
    let rendered = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(rendered.get_pixel(0, 0)[3], 255);
    assert_eq!(rendered.get_pixel(3, 0)[3], 0);
    let right_png = render_masked_png(&source, &right, 4, 4).unwrap();
    assert_eq!(
        changed_attachment_pixels(
            &source,
            &[left.clone(), right.clone()],
            &[png.clone(), right_png],
            4,
            4
        )
        .unwrap(),
        0
    );
    let tampered = opaque_test_png(4, 4);
    assert!(changed_attachment_pixels(&source, &[left.clone()], &[tampered], 4, 4).unwrap() > 0);

    right[0] = 255;
    let overlap = recomposition_metrics(&source, &[left, right], 4, 4).unwrap();
    assert_eq!(overlap.overlap_pixels, 1);
    assert!(!overlap.passes());
}

#[test]
fn mask_stroke_enforces_budget_coordinates_and_base_shape() {
    let mask = initial_mask(8, 8, false).unwrap();
    let stroke = ApplyMaskStroke {
        layer_id: "layer-1".into(),
        base_mask_sha256: "a".repeat(64),
        radius_milli: 1_000,
        mode: "add".into(),
        points: vec![StrokePoint {
            x_milli: 4_000,
            y_milli: 4_000,
            pressure_milli: 1_000,
            tick: 0,
        }],
    };
    let painted = apply_mask_stroke(&mask, 8, 8, &stroke).unwrap();
    assert!(painted.iter().any(|value| *value == 255));
    assert!(apply_mask_stroke(&mask[..63], 8, 8, &stroke).is_err());
    let mut outside = stroke;
    outside.points[0].x_milli = 9_000;
    assert!(apply_mask_stroke(&mask, 8, 8, &outside).is_err());
}

#[test]
fn manual_layer_png_normalizes_pixels_and_derives_authoritative_alpha_mask() {
    let mut image = RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 0]));
    image.put_pixel(1, 2, Rgba([90, 80, 70, 192]));
    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut encoded, ImageFormat::Png)
        .unwrap();

    let (normalized, mask) = normalize_manual_layer_png(&encoded.into_inner(), 4, 4).unwrap();
    assert_eq!(mask.len(), 16);
    assert_eq!(mask[2 * 4 + 1], 192);
    assert_eq!(mask.iter().filter(|alpha| **alpha != 0).count(), 1);
    let reopened = image::load_from_memory(&normalized).unwrap().to_rgba8();
    assert_eq!(reopened.dimensions(), (4, 4));
    assert_eq!(reopened.get_pixel(1, 2), &Rgba([90, 80, 70, 192]));

    let wrong_size = opaque_test_png(2, 2);
    assert!(normalize_manual_layer_png(&wrong_size, 4, 4).is_err());
}

fn encode_rgba(image: RgbaImage) -> Vec<u8> {
    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut encoded, ImageFormat::Png)
        .unwrap();
    encoded.into_inner()
}

#[test]
fn manual_attachment_pixels_survive_subtraction_and_only_new_regions_use_master() {
    let mut master = RgbaImage::new(4, 1);
    master.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
    master.put_pixel(1, 0, Rgba([40, 50, 60, 255]));
    master.put_pixel(2, 0, Rgba([70, 80, 90, 192]));
    master.put_pixel(3, 0, Rgba([100, 110, 120, 255]));
    let mut attachment = RgbaImage::new(4, 1);
    attachment.put_pixel(0, 0, Rgba([210, 11, 22, 255]));
    attachment.put_pixel(1, 0, Rgba([12, 220, 34, 64]));
    attachment.put_pixel(2, 0, Rgba([1, 2, 3, 0]));
    attachment.put_pixel(3, 0, Rgba([150, 160, 170, 77]));

    let rendered = render_updated_layer_attachment_png(
        &encode_rgba(attachment),
        &encode_rgba(master),
        &[255, 128, 0, 128],
        &[0, 255, 255, 128],
        4,
        1,
    )
    .unwrap();
    assert_eq!(image::guess_format(&rendered).unwrap(), ImageFormat::Png);
    let decoded = image::load_from_memory(&rendered).unwrap();
    assert!(decoded.color().has_alpha());
    let decoded = decoded.to_rgba8();

    // Subtraction changes only alpha; the hand-authored straight RGB survives.
    assert_eq!(decoded.get_pixel(0, 0), &Rgba([210, 11, 22, 0]));
    // Expanding within the old mask keeps manual RGB and recovers content alpha.
    assert_eq!(decoded.get_pixel(1, 0), &Rgba([12, 220, 34, 128]));
    // The master is used only where the new mask extends beyond the old mask.
    assert_eq!(decoded.get_pixel(2, 0), &Rgba([70, 80, 90, 192]));
    // An unchanged mask is byte-stable, including a non-opaque straight alpha.
    assert_eq!(decoded.get_pixel(3, 0), &Rgba([150, 160, 170, 77]));
}

#[test]
fn attachment_mask_transition_rejects_bad_dimensions_masks_and_alpha_contracts() {
    let attachment = encode_rgba(RgbaImage::from_pixel(2, 1, Rgba([90, 80, 70, 128])));
    let master = encode_rgba(RgbaImage::from_pixel(2, 1, Rgba([10, 20, 30, 255])));

    assert!(
        render_updated_layer_attachment_png(&attachment, &master, &[128], &[128, 128], 2, 1)
            .is_err()
    );
    assert!(
        render_updated_layer_attachment_png(&attachment, &master, &[128, 128], &[128, 128], 1, 2)
            .is_err()
    );
    let wrong_master = encode_rgba(RgbaImage::from_pixel(1, 2, Rgba([10, 20, 30, 255])));
    assert!(
        render_updated_layer_attachment_png(
            &attachment,
            &wrong_master,
            &[128, 128],
            &[128, 128],
            2,
            1
        )
        .is_err()
    );

    let mut rgb_png = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 1, Rgb([1, 2, 3])))
        .write_to(&mut rgb_png, ImageFormat::Png)
        .unwrap();
    assert!(
        render_updated_layer_attachment_png(
            &rgb_png.into_inner(),
            &master,
            &[255, 255],
            &[255, 255],
            2,
            1
        )
        .is_err()
    );

    // A visible attachment pixel outside the authoritative old mask is invalid.
    assert!(
        render_updated_layer_attachment_png(&attachment, &master, &[0, 128], &[255, 128], 2, 1)
            .is_err()
    );
    assert!(
        render_updated_layer_attachment_png(&attachment, &master, &[128, 128], &[128, 128], 0, 1)
            .is_err()
    );
}
