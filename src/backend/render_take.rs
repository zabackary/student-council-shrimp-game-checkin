use image::GenericImage;

pub fn render_take(photos: Vec<image::RgbaImage>) -> image::RgbaImage {
    let mut strip = image::load_from_memory(include_bytes!("../../assets/template.png"))
        .expect("Failed to load strip image")
        .to_rgba8();

    // All frames are 2000x1333
    // First frame
    // 134, 134
    // 134, 1600
    // 134, 3066
    // 134, 4532

    assert!(photos.len() == 4, "Expected 4 photos");

    for (i, photo) in photos.iter().enumerate() {
        let x = 134;
        let y = 134 + (i as u32 * 1466);
        let resized_photo =
            image::imageops::resize(photo, 2000, 1333, image::imageops::FilterType::Lanczos3);
        strip.copy_from(&resized_photo, x, y).unwrap();
    }

    // Resize the strip to 1/3 of the original size
    let strip = image::imageops::resize(
        &strip,
        (strip.width() / 3) as u32,
        (strip.height() / 3) as u32,
        image::imageops::FilterType::Lanczos3,
    );

    strip
}
