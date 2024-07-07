use image::{self, imageops};
use itertools::Itertools;

use crate::command::ZplCommand;

pub fn render_image(img: &image::DynamicImage) -> ZplCommand {
    let mut img = img.grayscale().into_luma8();

    imageops::dither(&mut img, &imageops::BiLevel);

    let data = img
        .pixels()
        .chunks(img.width() as usize)
        .into_iter()
        .map(|row| {
            // Take groups of 4 pixels, turning them into a single byte value using the lower 4
            // bits of each such byte turn it into hex value.
            let output = row
                .chunks(4)
                .into_iter()
                .map(|quad| {
                    quad.zip([8, 4, 2, 1])
                        .map(|(luma, b)| (luma.0[0] < 128) as i32 * b)
                        .sum()
                })
                .map(|p: i32| format!("{:x}", p))
                .collect::<Vec<String>>()
                .concat();
            // Append another 0 texel group for somewhat unknown reasons
            format!("{output}{}", if output.len() % 2 == 0 { "" } else { "0" })
        })
        .collect::<Vec<String>>()
        // ... missing grouping ...
        .concat();

    let bytes_per_row = (img.width() + 7) / 8;
    let total_field_count = bytes_per_row * img.height();
    let byte_count = total_field_count * 2;

    let output = format!("^GFA,{byte_count},{total_field_count},{bytes_per_row},{data}^FS");

    ZplCommand::Raw(output)
}
