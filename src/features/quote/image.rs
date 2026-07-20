use ab_glyph::{FontRef, PxScale};
use image::{Rgba, RgbaImage, imageops::FilterType};
use imageproc::drawing::{draw_text_mut, text_size};
use std::error::Error;

const FONT_BYTES: &[u8] = include_bytes!("./space-grotesk-semibold.ttf");

#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scale_channel(value: u8, factor: f32) -> u8 {
    (f32::from(value) * factor).clamp(0.0, 255.0) as u8
}

// its FINE.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]
pub async fn generate_quote_image(
    profile_picture_url: &str,
    quote_text: &str,
    username: &str,
    handle: &str,
) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let width = 1200;
    let height = 630;
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));

    let resp = reqwest::get(profile_picture_url).await?.bytes().await?;
    let img = image::load_from_memory(&resp)?;

    let left_img_width = 500;
    let left_img = img
        .resize_to_fill(left_img_width, height, FilterType::Lanczos3)
        .grayscale()
        .to_rgba8();

    let fade_start = 50u32;
    let fade_end = left_img_width;

    #[allow(clippy::cast_precision_loss)]
    let fade_range = (fade_end - fade_start) as f32;
    let inv_fade_range = 1.0 / fade_range;

    for y in 0..height {
        for x in 0..left_img_width {
            let mut pixel = *left_img.get_pixel(x, y);

            if x > fade_start {
                #[allow(clippy::cast_precision_loss)]
                let t = (x - fade_start) as f32 * inv_fade_range;
                let factor = (1.0 - t).clamp(0.0, 1.0);

                let [red, green, blue, alpha] = pixel.0;

                pixel.0 = [
                    scale_channel(red, factor),
                    scale_channel(green, factor),
                    scale_channel(blue, factor),
                    alpha,
                ];
            }

            canvas.put_pixel(x, y, pixel);
        }
    }

    let font = FontRef::try_from_slice(FONT_BYTES)?;

    let text_scale_val = 65.0;
    let max_text_width = 650;
    let scale = PxScale::from(text_scale_val);

    let mut wrapped_lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for word in quote_text.split_whitespace() {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{current_line} {word}")
        };

        let (w, _) = text_size(scale, &font, &test_line);
        if w > max_text_width && !current_line.is_empty() {
            wrapped_lines.push(current_line);
            current_line = word.to_string();
        } else {
            current_line = test_line;
        }
    }
    if !current_line.is_empty() {
        wrapped_lines.push(current_line);
    }

    let username_scale_val = text_scale_val * 0.55;
    let username_scale = PxScale::from(username_scale_val);
    let (user_w, user_h) = text_size(username_scale, &font, username);

    let handle_scale_val = text_scale_val * 0.40;
    let handle_scale = PxScale::from(handle_scale_val);
    let (handle_w, handle_h) = text_size(handle_scale, &font, handle);

    let center_x = 825i32;

    let (_, single_line_h) = text_size(scale, &font, "A");
    let line_gap = 10i32;
    let text_block_h = (wrapped_lines.len() as i32 * single_line_h as i32)
        + ((wrapped_lines.len() as i32 - 1) * line_gap);

    let gap1 = 30i32;
    let gap2 = 10i32;
    let total_h = text_block_h + gap1 + user_h as i32 + gap2 + handle_h as i32;

    let mut current_y = (height as i32 / 2) - (total_h / 2);

    let text_color = Rgba([255, 255, 255, 255]);
    let username_color = Rgba([200, 200, 200, 255]);
    let handle_color = Rgba([150, 150, 150, 255]);

    for line in &wrapped_lines {
        let (line_w, _) = text_size(scale, &font, line);
        let line_x = center_x - (line_w as i32 / 2);
        draw_text_mut(
            &mut canvas,
            text_color,
            line_x,
            current_y,
            scale,
            &font,
            line,
        );
        current_y += single_line_h as i32 + line_gap;
    }

    current_y += gap1 - line_gap;
    let user_x = center_x - (user_w as i32 / 2);
    draw_text_mut(
        &mut canvas,
        username_color,
        user_x,
        current_y,
        username_scale,
        &font,
        username,
    );

    current_y += user_h as i32 + gap2;
    let handle_x = center_x - (handle_w as i32 / 2);
    draw_text_mut(
        &mut canvas,
        handle_color,
        handle_x,
        current_y,
        handle_scale,
        &font,
        handle,
    );

    Ok(canvas)
}
