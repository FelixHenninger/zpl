use zpl_typst::{PrinterLabel, ZplHost};

fn main() {
    const TEMPLATE_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/simple.typ");

    let host = ZplHost::new();

    let printer = PrinterLabel {
        width: 50.0,
        height: 50.0,
        margin_left: 1.0,
        margin_right: 1.0,
        margin_top: 1.0,
        margin_bottom: 1.0,
    };

    let instance = host
        .instantiate(std::fs::read_to_string(TEMPLATE_PATH).unwrap(), printer);

    let pages = instance.render_to_svg_pages().unwrap();

    for page in pages {
        println!("{}", page);
    }
}
