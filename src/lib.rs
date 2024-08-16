use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use core::num::NonZeroU32;
use label::{Label, LabelContent, Unit};

use command::CommandSequence;
use device::ZplPrinter;

pub mod command;
pub mod device;
mod image;
pub mod label;
mod read;
mod svg;

#[derive(Parser, Clone)]
pub struct Args {
    #[arg(default_value = "192.168.1.39:9100")]
    ip: SocketAddr,

    #[arg(long = "image")]
    image: Option<PathBuf>,

    #[arg(long = "svg")]
    svg: Option<PathBuf>,

    #[arg(long = "copies", default_value = "1")]
    copies: NonZeroU32,
    
    #[arg(long = "margin", default_value = "5", help = "xy margin in mm")]
    margin: u32,
    
    #[arg(long = "width", default_value = "51", help = "label width in mm")]
    width: u32,

    #[arg(long = "height", default_value = "51", help = "label height in mm")]
    height: u32,

    #[arg(
        long = "dpmm",
        default_value = "None",
        help = "print resolution in dots per mm (overrides printer autodetection)"
    )]
    dpmm: Option<u32>,

    #[arg(long = "output-zpl-only", default_value = "false")]
    output_zpl_only: bool,
}

pub async fn make_label(
    args: Args,
    dpmm_autodetect: Option<u32>,
) -> anyhow::Result<CommandSequence> {
    let Args {
        ip: _,
        image,
        svg,
        copies,
        margin,
        width,
        height,
        dpmm: dpmm_override,
        output_zpl_only: _,
    } = args;

    let dpmm = if let Some(v) = dpmm_override {
        v
    } else if let Some(v) = dpmm_autodetect {
        v
    } else {
        bail!("Can't ascertain resolution, please supply dpmm");
    };

    let margin_x = margin as f32;
    let margin_y = margin as f32;
    let content_width = width as f32 - 2.0 * margin_x;
    let content_height = height as f32 - 2.0 * margin_y;

    let mut label = Label::new(width, height, dpmm);
    // Resize image, or rasterize SVG
    if let Some(image) = image {
        let img = ::image::open(image).expect("Image file not found");

        label.content.push(LabelContent::Image {
            img,
            x: Unit::Millimetres(margin_x),
            y: Unit::Millimetres(margin_y),
            w: Unit::Millimetres(content_width),
            h: Unit::Millimetres(content_height),
        });
    } else if let Some(path) = svg {
        let code = tokio::fs::read_to_string(path)
            .await
            .expect("SVG file not found");

        label.content.push(LabelContent::Svg {
            code,
            x: Unit::Millimetres(margin_x),
            y: Unit::Millimetres(margin_y),
            w: Unit::Millimetres(content_width),
            h: Unit::Millimetres(content_height),
        });
    } else {
        bail!("No image/vector source selected");
    };

    let commands = label.print(copies.get()).await?;

    Ok(commands)
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let mut device = ZplPrinter::with_address(args.ip).await?;
    let config = device.request_device_status().await?;
    let dpmm = config.identification.dpmm;

    let label = make_label(args, Some(dpmm)).await?;

    Ok(device.send(label).await?)
}

pub async fn run_output_zpl_only(args: Args) -> anyhow::Result<()> {
    let label = make_label(args, None).await?;
    println!("{}", label);

    Ok(())
}
