use std::sync::Arc;

use base64::engine::{general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::{self, imageops};
use itertools::Itertools;
use std::io::prelude::*;

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
    /// Base64 data.
    B64,
    /// flate compressed and then bas64.
    Z64,
}

impl SerializedImage {
    pub fn from_image(img: &image::DynamicImage) -> Self {
        let mut img = img.grayscale().into_luma8();
        imageops::dither(&mut img, &imageops::BiLevel);

        let data = bit_encode(&img);

        let bytes_per_row = img.width().div_ceil(8);
        let total_field_count = bytes_per_row * img.height();
        let byte_count = total_field_count * 2;

        //format!("^GFA,{byte_count},{total_field_count},{bytes_per_row},{data}^FS")
        SerializedImage::AsciiHex {
            byte_count,
            total_field_count,
            bytes_per_row,
            data: hex::encode(data).into(),
        }
    }

    pub fn from_compressed(img: &image::DynamicImage) -> Self {
        let mut img = img.grayscale().into_luma8();
        imageops::dither(&mut img, &imageops::BiLevel);

        let bytes_per_row = img.width().div_ceil(8);
        let total_field_count = bytes_per_row * img.height();
        let byte_count = total_field_count;

        let data = bit_encode(&img);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).unwrap();
        let compressed = encoder.finish().unwrap();

        let image_base64 = STANDARD.encode(compressed);

        SerializedImage::Compressed {
            byte_count,
            total_field_count,
            bytes_per_row,
            crc: crc::checksum(image_base64.as_bytes()),
            data: image_base64.into(),
            id: CompressedId::Z64,
        }
    }

    pub fn from_base64(img: &image::DynamicImage) -> Self {
        let mut img = img.grayscale().into_luma8();
        //imageops::dither(&mut img, &imageops::BiLevel);

        let data = bit_encode(&img);
        let image_base64 = STANDARD.encode(data);

        // Byte count includes prefix and CRC
        let byte_count = (image_base64.chars().count() + 10) as u32;
        let bytes_per_row = img.width().div_ceil(8);
        let total_field_count = bytes_per_row * img.height();

        SerializedImage::Compressed {
            byte_count,
            total_field_count,
            bytes_per_row,
            crc: crc::checksum(image_base64.as_bytes()),
            data: image_base64.into(),
            id: CompressedId::B64,
        }
    }

    pub fn from_svg(
        svg: String,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<Self, svg::Error> {
        let img = svg::render_svg(svg, pix_width, pix_height)?;
        Ok(Self::from_compressed(&img))
    }

    pub fn from_svg_tree(
        svg: resvg::usvg::Tree,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<Self, svg::Error> {
        let img = svg::render_svg_tree(svg, pix_width, pix_height)?;
        Ok(Self::from_compressed(&img))
    }
}

/// Encode a *linear grayscale* image to the bit-packed vector.
pub fn bit_encode(image: &image::GrayImage) -> Vec<u8> {
    use bitvec::prelude::*;

    let pixels = image
        .pixels()
        .map(|luma| (luma.0[0] < 127) as bool) // I really think this should be *<*
        .collect::<Vec<bool>>();

    let pixels = pixels
        .iter()
        .chunks(image.width() as usize)
        .into_iter()
        .map(|row| row.collect::<BitVec<u8, Msb0>>().as_raw_slice().to_vec())
        .concat();

    return pixels;
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
