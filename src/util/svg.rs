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

pub fn render_svg(
    svg_data: String,
    canvas_px_width: u32,
    canvas_px_height: u32,
) -> Result<::image::DynamicImage, Error> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let options = Options {
        fontdb: Arc::new(db),
        ..Default::default()
    };

    let rtree = Tree::from_str(&svg_data, &options).ok().unwrap();
    render_svg_tree(rtree, canvas_px_width, canvas_px_height)
}

pub fn render_svg_tree(
    rtree: Tree,
    canvas_px_width: u32,
    canvas_px_height: u32,
) -> Result<::image::DynamicImage, Error> {
    let rtree_size = rtree.size();

    let mut pixmap = Pixmap::new(canvas_px_width, canvas_px_height).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(255, 255, 255, 255));

    let scale = (canvas_px_width as f32 / rtree_size.width())
        .min(canvas_px_height as f32 / rtree_size.height());

    let image_px_width = rtree_size.width() * scale;
    let image_px_height = rtree_size.height() * scale;

    let offset_x = (canvas_px_width as f32 - image_px_width) / 2.0;
    let offset_y = (canvas_px_height as f32 - image_px_height) / 2.0;

    if offset_x < 0.0 {
        log::warn!("SVG Rendering Offset X non-positive: {offset_x:?}");
    }

    if offset_y < 0.0 {
        log::warn!("SVG Rendering Offset Y non-positive: {offset_y:?}");
    }

    resvg::render(
        &rtree,
        tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(offset_x, offset_y),
        &mut pixmap.as_mut(),
    );

    // Unwrapping here since this must succeed, if resvg is correct.
    let png = pixmap.encode_png().unwrap();

    let image = image::ImageReader::with_format(
        std::io::Cursor::new(png),
        image::ImageFormat::Png,
    )
    .decode()
    .unwrap();

    debug_assert_eq!(image.width(), canvas_px_width);
    debug_assert_eq!(image.height(), canvas_px_height);

    Ok(image)
}
