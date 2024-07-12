use std::sync::Arc;

use resvg::tiny_skia::{self, Pixmap};
use resvg::usvg::{Options, Tree, fontdb};

use quick_error::quick_error;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        /// The SVG did not parse.
        Svg(err: resvg::usvg::Error) {
            from()
        }
    }
}

pub fn pixmap_svg(svg_data: String, pix_width: u32) -> Result<::image::DynamicImage, Error> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut options = Options::default();
    options.fontdb = Arc::new(db);
    let rtree = Tree::from_str(&svg_data, &options).ok().unwrap();
    let pixmap_size = rtree.size();

    let mut pixmap = Pixmap::new(pix_width, pix_width).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(255, 255, 255, 255));

    let scale =
        (pix_width as f32 / pixmap_size.width()).min(pix_width as f32 / pixmap_size.height());

    resvg::render(
        &rtree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // Unwrapping here since this must succeed.
    let png = pixmap.encode_png().unwrap();

    let image = image::io::Reader::with_format(std::io::Cursor::new(png), image::ImageFormat::Png)
        .decode()
        .unwrap();

    Ok(image)
}
