use std::sync::Arc;

use base64::engine::{general_purpose::STANDARD, Engine as _};
use image::{self, imageops};
use itertools::Itertools;
use std::io::prelude::*;

use crate::util::{crc, svg};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Compression {
    /// Base64-encoded bits
    B64,
    /// Zlib-compressed, base64-encoded bits
    Z64,
}

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
        id: Compression,
        crc: u16,
    },
}

impl SerializedImage {
    pub fn new_ascii(img: &image::DynamicImage) -> Self {
        let mut img = img.grayscale().into_luma8();
        imageops::dither(&mut img, &imageops::BiLevel);

        let data = bit_encode(&img);

        let bytes_per_row = img.width().div_ceil(8);
        let total_field_count = bytes_per_row * img.height();
        let byte_count = total_field_count * 2;

        SerializedImage::AsciiHex {
            byte_count,
            total_field_count,
            bytes_per_row,
            data: hex::encode(data).into(),
        }
    }

    pub fn new_z64(img: &image::DynamicImage) -> Self {
        let mut img = img.grayscale().into_luma8();
        imageops::dither(&mut img, &imageops::BiLevel);

        let data = bit_encode(&img);

        // Compress via zlib
        use flate2::{write::ZlibEncoder, Compression as ZlibCompression};
        let mut encoder =
            ZlibEncoder::new(Vec::new(), ZlibCompression::default());
        encoder.write_all(&data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Encode as base64
        let image_base64 = STANDARD.encode(compressed);

        let byte_count = (image_base64.chars().count() + 10) as u32;
        let bytes_per_row = img.width().div_ceil(8);
        let total_field_count = bytes_per_row * img.height();

        SerializedImage::Compressed {
            byte_count,
            total_field_count,
            bytes_per_row,
            crc: crc::checksum(image_base64.as_bytes()),
            data: image_base64.into(),
            id: Compression::Z64,
        }
    }

    pub fn new_b64(img: &image::DynamicImage) -> Self {
        let mut img = img.grayscale().into_luma8();
        imageops::dither(&mut img, &imageops::BiLevel);

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
            id: Compression::B64,
        }
    }

    pub fn from_svg(
        svg: String,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<Self, svg::Error> {
        let img = svg::render_svg(svg, pix_width, pix_height)?;
        Ok(Self::new_z64(&img))
    }

    pub fn from_svg_tree(
        svg: resvg::usvg::Tree,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<Self, svg::Error> {
        let img = svg::render_svg_tree(svg, pix_width, pix_height)?;
        Ok(Self::new_z64(&img))
    }
}

/// Encode a *linear grayscale* image to the bit-packed vector.
pub fn bit_encode(image: &image::GrayImage) -> Vec<u8> {
    use bitvec::prelude::*;

    let pixels = image
        .pixels()
        .map(|luma| luma.0[0] < 127)
        .collect::<Vec<bool>>();

    pixels
        .iter()
        .chunks(image.width() as usize)
        .into_iter()
        .map(|row| row.collect::<BitVec<u8, Msb0>>().as_raw_slice().to_vec())
        .concat()
}

impl core::fmt::Display for Compression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Compression::B64 => "B64",
                Compression::Z64 => "Z64",
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
