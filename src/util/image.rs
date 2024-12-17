use std::sync::Arc;

use base64::engine::{general_purpose::STANDARD, Engine as _};
use flate2::{bufread::GzEncoder, Compression};
use image::{self, imageops};
use itertools::Itertools;

use crate::util::{crc, svg};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SerializedImage {
    AsciiHex {
        byte_count: u32,
        total_field_count: u32,
        bytes_per_row: u32,
        data: Arc<str>,
    },
    Compressed {
        byte_count: u32,
        total_field_count: u32,
        bytes_per_row: u32,
        data: Arc<str>,
        id: CompressedId,
        crc: u16,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompressedId {
    #[allow(unused)]
    /// Base64 data.
    B64,
    /// flate compressed and then bas64.
    Z64,
}

impl SerializedImage {
    pub fn from_image(img: &image::DynamicImage) -> Self {
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
                format!(
                    "{output}{}",
                    if output.len() % 2 == 0 { "" } else { "0" }
                )
            })
            .collect::<Vec<String>>()
            // ... missing grouping ...
            .concat();

        let bytes_per_row = (img.width() + 7) / 8;
        let total_field_count = bytes_per_row * img.height();
        let byte_count = total_field_count * 2;

        //format!("^GFA,{byte_count},{total_field_count},{bytes_per_row},{data}^FS")
        SerializedImage::AsciiHex {
            byte_count,
            total_field_count,
            bytes_per_row,
            data: data.into(),
        }
    }

    pub fn from_compressed(img: &image::DynamicImage) -> Self {
        let bytes_per_row = (img.width() + 7) / 8;
        let total_field_count = bytes_per_row * img.height();
        let byte_count = total_field_count * 2;

        let img = img.to_luma8();
        let bytes = std::io::Cursor::new(bit_encode(&img));

        let mut encode = GzEncoder::new(bytes, Compression::best());
        let mut encoded = vec![];
        std::io::Read::read_to_end(&mut encode, &mut encoded).unwrap();

        let data = STANDARD.encode(encoded);

        SerializedImage::Compressed {
            byte_count,
            total_field_count,
            bytes_per_row,
            crc: crc::checksum(data.as_bytes()),
            data: data.into(),
            id: CompressedId::Z64,
        }
    }

    pub fn from_svg(
        svg: String,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<Self, svg::Error> {
        let img = svg::render_svg(svg, pix_width, pix_height)?;
        Ok(Self::from_image(&img))
    }

    pub fn from_svg_tree(
        svg: resvg::usvg::Tree,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<Self, svg::Error> {
        let img = svg::render_svg_tree(svg, pix_width, pix_height)?;
        Ok(Self::from_image(&img))
    }
}

/// Encode a *linear grayscale* image to the bit-packed vector.
pub fn bit_encode(image: &image::GrayImage) -> Vec<u8> {
    use image_canvas::{
        layout::{Block, CanvasLayout, SampleBits, SampleParts, Texel},
        Canvas,
    };

    let texel_in = Texel {
        block: Block::Pixel,
        parts: SampleParts::Luma,
        bits: SampleBits::UInt8,
    };

    let texel_out = Texel {
        block: Block::Pack1x8,
        parts: SampleParts::Luma,
        bits: SampleBits::UInt1x8,
    };

    let (w, h) = image.dimensions();
    let mut from =
        Canvas::new(CanvasLayout::with_texel(&texel_in, w, h).unwrap());

    let mut into =
        Canvas::new(CanvasLayout::with_texel(&texel_out, w, h).unwrap());

    from.as_bytes_mut().copy_from_slice(image);
    from.convert(&mut into);

    into.into_bytes()
}

impl core::fmt::Display for CompressedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CompressedId::B64 => "B64",
                CompressedId::Z64 => "Z64",
            }
        )
    }
}

#[test]
fn convert_luma_to_bits() {
    assert_eq!(bit_encode(&image::GrayImage::new(0, 0)).len(), 0);
    assert_eq!(bit_encode(&image::GrayImage::new(4, 4)).len(), 4);
    assert_eq!(bit_encode(&image::GrayImage::new(8, 1)).len(), 1);
    assert_eq!(bit_encode(&image::GrayImage::new(7, 1)).len(), 1);
    assert_eq!(bit_encode(&image::GrayImage::new(7, 2)).len(), 2);

    assert_eq!(bit_encode(&image::GrayImage::new(224, 64)).len(), 28 * 64);

    assert_eq!(
        bit_encode(&image::GrayImage::from_fn(1, 1, |_, _| image::Luma(
            [127; 1]
        ))),
        vec![0]
    );

    assert_eq!(
        bit_encode(&image::GrayImage::from_fn(1, 1, |_, _| image::Luma(
            [128; 1]
        ))),
        vec![0x80]
    );

    assert_eq!(
        bit_encode(&image::GrayImage::from_fn(8, 2, |_, y| image::Luma(
            [128 >> y; 1]
        ))),
        vec![0xff, 0x0]
    );

    assert_eq!(
        bit_encode(&image::GrayImage::from_fn(8, 2, |x, y| image::Luma(
            [((x + y) as u8 * 31u8); 1]
        ))),
        vec![0x07, 0x0f]
    );
}
