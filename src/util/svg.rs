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
    render: &crate::label::RenderOptions,
) -> Result<::image::DynamicImage, Error> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let options = Options {
        fontdb: Arc::new(db),
        ..Default::default()
    };

    let rtree = Tree::from_str(&svg_data, &options).ok().unwrap();
    render_svg_tree(rtree, canvas_px_width, canvas_px_height, render)
}

pub fn render_svg_tree(
    rtree: Tree,
    canvas_px_width: u32,
    canvas_px_height: u32,
    options: &crate::label::RenderOptions,
) -> Result<::image::DynamicImage, Error> {
    let is_wh_swapped;

    let rtree_size = if options.auto_rotate
        && should_rotate_svg(canvas_px_width, canvas_px_height, &rtree)
    {
        let rtree_size = rtree.size();
        is_wh_swapped = true;
        tiny_skia::Size::from_wh(rtree_size.height(), rtree_size.width())
            .unwrap()
    } else {
        is_wh_swapped = false;
        rtree.size()
    };

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
            .post_rotate(if is_wh_swapped { 90.0 } else { 0.0 })
            .post_translate(
                if is_wh_swapped { image_px_width } else { 0.0 },
                0.0,
            )
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

pub fn should_rotate_svg(w: u32, h: u32, tree: &Tree) -> bool {
    let aspect_label = f64::from(w) / f64::from(h);
    let aspect_image =
        f64::from(tree.size().width()) / f64::from(tree.size().height());

    (aspect_label - aspect_image).abs()
        > (aspect_label - 1.0 / aspect_image).abs()
}
