use std::sync::Arc;

use resvg::tiny_skia::{self, Pixmap};
use resvg::usvg::{fontdb, Options, Tree};

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

pub fn pixmap_svg(
    svg_data: String,
    pix_width: u32,
    pix_height: u32,
) -> Result<::image::DynamicImage, Error> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut options = Options::default();
    options.fontdb = Arc::new(db);
    let rtree = Tree::from_str(&svg_data, &options).ok().unwrap();
    let rtree_size = rtree.size();

    let mut pixmap = Pixmap::new(pix_width, pix_height).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(255, 255, 255, 255));

    let scale =
        (pix_width as f32 / rtree_size.width()).min(pix_height as f32 / rtree_size.height());

    let xsize = rtree_size.width() * scale;
    let ysize = rtree_size.height() * scale;

    let x_off = (pix_width as f32 - xsize) / 2.0;
    let y_off = (pix_height as f32 - ysize) / 2.0;
    assert!(x_off >= 0.0);
    assert!(y_off >= 0.0);

    resvg::render(
        &rtree,
        // FIXME: multiply by 4.0 makes no sense.
        tiny_skia::Transform::from_scale(scale, scale).post_translate(x_off, 4.0 * y_off),
        &mut pixmap.as_mut(),
    );

    // Unwrapping here since this must succeed.
    let png = pixmap.encode_png().unwrap();

    let image = image::io::Reader::with_format(std::io::Cursor::new(png), image::ImageFormat::Png)
        .decode()
        .unwrap();

    Ok(image)
}
