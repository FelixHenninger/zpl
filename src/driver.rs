pub async fn make_label(
    args: Args,
    id: String,
    dpmm: u32,
) -> anyhow::Result<CommandSequence> {
    let Args {
        ip: _,
        copies,
        output_zpl_only: _,
    } = args;

    let width: f32 = 100.0;
    let height: f32 = 50.0;

    let margin_x: f32 = 3.0; //32.0 / 12.0;
    let margin_y: f32 = 3.0; //32.0 / 12.0;
    let content_width = width - 2.0 * margin_x;
    let content_height = height - 2.0 * margin_y;
    let offset_y = 0.0;

    let mut label = Label::new(
        (width.floor() as i32).try_into().unwrap(),
        (height.floor() as i32).try_into().unwrap(),
        dpmm,
    );

    let logo = tokio::fs::read_to_string("logo-cert.svg")
        .await
        .expect("SVG file not found");

    let text_code = tokio::fs::read_to_string("code.svg")
        .await
        .expect("SVG file not found");
    let text_code = str::replace(&text_code, "b1234", &id);

    use qrcode::render::svg;
    use qrcode::{EcLevel, QrCode};

    let qr_contents = format!("https://urn.ccc.de/cert:{}", id).to_uppercase();
    info!("Content: {:?}", qr_contents);
    info!("Content length: {:?}", qr_contents.len());

    let qr = QrCode::with_error_correction_level(
        qr_contents, //
        EcLevel::Q,
    )
    .unwrap();

    let qr_svg = qr
        .render()
        .min_dimensions(200, 200)
        .quiet_zone(false)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    let block_margin = 1.5;
    label.content.push(LabelContent::Svg {
        code: logo,
        x: Unit::Millimetres(margin_x + content_height + 6.0 + block_margin),
        y: Unit::Millimetres(margin_y + offset_y),
        w: Unit::Millimetres(35.0 - 2.0 * block_margin),
        h: Unit::Millimetres(content_height),
    });
    let block_margin = 2.0;
    label.content.push(LabelContent::Svg {
        code: text_code,
        x: Unit::Millimetres(
            margin_x + content_height + 35.0 + 2.0 + block_margin,
        ),
        y: Unit::Millimetres(margin_y + offset_y),
        w: Unit::Millimetres(
            content_width - content_height - 35.0 - 2.0 * block_margin,
        ),
        h: Unit::Millimetres(content_height),
    });
    label.content.push(LabelContent::Svg {
        code: qr_svg,
        x: Unit::Millimetres(margin_x + 0.0),
        y: Unit::Millimetres(margin_y + offset_y),
        w: Unit::Millimetres(content_height),
        h: Unit::Millimetres(content_height),
    });

    let commands = label.print(2).await?;

    Ok(commands)
}
