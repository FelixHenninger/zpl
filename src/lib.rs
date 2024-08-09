use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{bail, Context};
use core::num::NonZeroU32;
use clap::Parser;

use command::{CommandSequence, MediaType, PostPrintAction, ZplCommand};
use device::ZplPrinter;

mod command;
mod device;
mod image;
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
    #[arg(long = "mm-width", default_value = "51")]
    width: u32,
    #[arg(long = "mm-height", default_value = "51")]
    height: u32,
    #[arg(long = "dpmm", default_value = "12")]
    dpmm: u32,
    #[arg(long = "output-zpl-only", default_value = "false")]
    output_zpl_only: bool,
}

pub async fn make_label(args: Args, dpmm_override: Option<u32>) -> anyhow::Result<CommandSequence> {
    let Args {
        ip: _,
        image,
        svg,
        copies,
        width,
        height,
        dpmm: dpmm_manual,
        output_zpl_only: _,
    } = args;

    let dpmm = if let Some(v) = dpmm_override {
        v
    } else {
        dpmm_manual
    };

    let margin_x = 0;
    let margin_y = 0;
    let content_px_width = width * dpmm - 2 * margin_x;
    let content_px_height = height * dpmm - 2 * margin_y;

    // Resize image, or rasterize SVG
    let image = if let Some(image) = image {
        let img = ::image::open(image).expect("Image file not found");
        img.resize_to_fill(
            content_px_height,
            content_px_width,
            ::image::imageops::FilterType::Lanczos3,
        );
        crate::image::SerializedImage::from_image(&img)
    } else if let Some(svg) = svg {
        let svg = tokio::fs::read_to_string(svg)
            .await
            .expect("SVG file not found");

        crate::image::SerializedImage::from_svg(svg, content_px_width, content_px_height)
            .context("Could not load SVG")?
    } else {
        bail!("No image source selected");
    };

    Ok(CommandSequence(vec![
        //ZplCommand::Magic,
        ZplCommand::Start,
        ZplCommand::SetVerticalShift(12),
        ZplCommand::SetTearOffPosition(-20),
        ZplCommand::SetMediaType(MediaType::Transfer),
        ZplCommand::SetHome(0, 0),
        ZplCommand::SetHalfDensity(false),
        ZplCommand::SetSpeed { print: 4, slew: 4 },
        ZplCommand::SetDarkness(25),
        ZplCommand::PersistConfig,
        ZplCommand::SetInverted(false),
        ZplCommand::SetEncoding(0),
        ZplCommand::End,
        ZplCommand::Start,
        ZplCommand::SetPostPrintAction(PostPrintAction::Cut),
        ZplCommand::LabelSetup {
            w: width,
            h: height,
            dpmm,
        },
        ZplCommand::SetHorizontalShift(0),
        ZplCommand::MoveOrigin(margin_x, margin_y),
        ZplCommand::Image(image),
        ZplCommand::PrintQuantity {
            total: copies.get(),
            pause_and_cut_after: copies.get(),
            replicates: copies.get(),
            cut_only: true,
        },
        ZplCommand::End,
    ]))
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
