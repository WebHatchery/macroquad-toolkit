//! CPU-side alpha composition and tinting for Macroquad images.
//!
//! These helpers keep source layers as decoded [`Image`] values until a finished
//! composite is ready for upload. Images use straight-alpha RGBA bytes.

use macroquad::prelude::{Color, FilterMode, Image, Texture2D};

/// Composites `source` over `destination` using straight-alpha Porter-Duff over.
///
/// Both images must have the same dimensions. Fully transparent output pixels
/// are cleared to transparent black so hidden RGB cannot create edge halos.
pub fn alpha_over(destination: &mut Image, source: &Image) -> Result<(), String> {
    validate_image(destination, "destination")?;
    validate_image(source, "source")?;
    ensure_same_dimensions(destination, source)?;

    let destination_pixels = destination.bytes.as_chunks_mut::<4>().0;
    let source_pixels = source.bytes.as_chunks::<4>().0;
    for (destination_pixel, source_pixel) in destination_pixels.iter_mut().zip(source_pixels) {
        let source_alpha = f64::from(source_pixel[3]) / 255.0;
        let destination_alpha = f64::from(destination_pixel[3]) / 255.0;
        let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

        if output_alpha <= 0.0 {
            destination_pixel.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }

        for channel in 0..3 {
            let source_color = f64::from(source_pixel[channel]) / 255.0;
            let destination_color = f64::from(destination_pixel[channel]) / 255.0;
            let output_color = (source_color * source_alpha
                + destination_color * destination_alpha * (1.0 - source_alpha))
                / output_alpha;
            destination_pixel[channel] = to_byte(output_color);
        }
        destination_pixel[3] = to_byte(output_alpha);
    }

    Ok(())
}

/// Applies a tint to grayscale fill shading, clipped by a grayscale mask.
///
/// The fill's red channel supplies grayscale shading. The mask's red and alpha
/// channels both contribute coverage, as do the fill alpha and tint alpha.
/// Images must have identical dimensions.
pub fn tint_with_mask(fill: &Image, mask: &Image, tint: Color) -> Result<Image, String> {
    validate_image(fill, "fill")?;
    validate_image(mask, "mask")?;
    ensure_same_dimensions(fill, mask)?;

    let tint_channels = [
        finite_channel(tint.r),
        finite_channel(tint.g),
        finite_channel(tint.b),
    ];
    let tint_alpha = f64::from(finite_channel(tint.a)) / 255.0;
    let mut output =
        Image::gen_image_color(fill.width, fill.height, Color::new(0.0, 0.0, 0.0, 0.0));

    let output_pixels = output.bytes.as_chunks_mut::<4>().0;
    let fill_pixels = fill.bytes.as_chunks::<4>().0;
    let mask_pixels = mask.bytes.as_chunks::<4>().0;
    for ((output_pixel, fill_pixel), mask_pixel) in
        output_pixels.iter_mut().zip(fill_pixels).zip(mask_pixels)
    {
        let shade = f64::from(fill_pixel[0]) / 255.0;
        let fill_alpha = f64::from(fill_pixel[3]) / 255.0;
        let mask_coverage = f64::from(mask_pixel[0]) / 255.0 * f64::from(mask_pixel[3]) / 255.0;
        let output_alpha = fill_alpha * mask_coverage * tint_alpha;

        if output_alpha <= 0.0 {
            continue;
        }

        for channel in 0..3 {
            output_pixel[channel] = to_byte(shade * f64::from(tint_channels[channel]) / 255.0);
        }
        output_pixel[3] = to_byte(output_alpha);
    }

    Ok(output)
}

/// Reduces an image with an area filter in premultiplied-alpha space.
///
/// Target dimensions must be nonzero and cannot exceed the source dimensions.
/// The result is returned in straight-alpha form.
pub fn downsample_alpha_aware(
    source: &Image,
    target_width: u16,
    target_height: u16,
) -> Result<Image, String> {
    validate_image(source, "source")?;
    if target_width == 0 || target_height == 0 {
        return Err("Downsample target dimensions must be nonzero".to_string());
    }
    if target_width > source.width || target_height > source.height {
        return Err("Downsample target dimensions cannot exceed the source".to_string());
    }
    if target_width == source.width && target_height == source.height {
        return Ok(source.clone());
    }

    let mut output =
        Image::gen_image_color(target_width, target_height, Color::new(0.0, 0.0, 0.0, 0.0));
    let scale_x = f64::from(source.width) / f64::from(target_width);
    let scale_y = f64::from(source.height) / f64::from(target_height);

    for target_y in 0..target_height {
        for target_x in 0..target_width {
            let left = f64::from(target_x) * scale_x;
            let right = f64::from(target_x + 1) * scale_x;
            let top = f64::from(target_y) * scale_y;
            let bottom = f64::from(target_y + 1) * scale_y;
            let rgba = sample_area(source, left, top, right, bottom);
            let index = ((u32::from(target_y) * u32::from(target_width) + u32::from(target_x)) * 4)
                as usize;
            output.bytes[index..index + 4].copy_from_slice(&rgba);
        }
    }

    Ok(output)
}

/// Uploads a completed CPU image once and applies its sampling filter.
pub fn upload_texture(image: &Image, filter: FilterMode) -> Texture2D {
    let texture = Texture2D::from_image(image);
    texture.set_filter(filter);
    texture
}

fn sample_area(source: &Image, left: f64, top: f64, right: f64, bottom: f64) -> [u8; 4] {
    let mut premultiplied = [0.0_f64; 3];
    let mut alpha_sum = 0.0_f64;
    let mut weight_sum = 0.0_f64;
    let first_x = left.floor() as u32;
    let last_x = right.ceil() as u32;
    let first_y = top.floor() as u32;
    let last_y = bottom.ceil() as u32;

    for source_y in first_y..last_y.min(u32::from(source.height)) {
        let vertical =
            (bottom.min(f64::from(source_y + 1)) - top.max(f64::from(source_y))).max(0.0);
        for source_x in first_x..last_x.min(u32::from(source.width)) {
            let horizontal =
                (right.min(f64::from(source_x + 1)) - left.max(f64::from(source_x))).max(0.0);
            let weight = horizontal * vertical;
            let source_index = ((source_y * u32::from(source.width) + source_x) * 4) as usize;
            let source_pixel = &source.bytes[source_index..source_index + 4];
            let alpha = f64::from(source_pixel[3]) / 255.0;

            for channel in 0..3 {
                premultiplied[channel] += f64::from(source_pixel[channel]) / 255.0 * alpha * weight;
            }
            alpha_sum += alpha * weight;
            weight_sum += weight;
        }
    }

    if alpha_sum <= 0.0 || weight_sum <= 0.0 {
        return [0, 0, 0, 0];
    }

    let mut rgba = [0; 4];
    for channel in 0..3 {
        rgba[channel] = to_byte(premultiplied[channel] / alpha_sum);
    }
    rgba[3] = to_byte(alpha_sum / weight_sum);
    rgba
}

fn ensure_same_dimensions(first: &Image, second: &Image) -> Result<(), String> {
    if first.width != second.width || first.height != second.height {
        return Err(format!(
            "Image dimensions differ: {}x{} and {}x{}",
            first.width, first.height, second.width, second.height
        ));
    }
    Ok(())
}

fn validate_image(image: &Image, label: &str) -> Result<(), String> {
    let expected = usize::from(image.width)
        .checked_mul(usize::from(image.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("{} image dimensions overflow", label))?;
    if image.bytes.len() != expected {
        return Err(format!(
            "{} image has {} bytes; expected {} for {}x{} RGBA",
            label,
            image.bytes.len(),
            expected,
            image.width,
            image.height
        ));
    }
    if image.width == 0 || image.height == 0 {
        return Err(format!("{} image dimensions must be nonzero", label));
    }
    Ok(())
}

fn finite_channel(channel: f32) -> u8 {
    if channel.is_finite() {
        (channel.clamp(0.0, 1.0) * 255.0).round() as u8
    } else {
        0
    }
}

fn to_byte(channel: f64) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}
