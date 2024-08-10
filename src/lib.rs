use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{bail, Context};
use clap::Parser;
use core::num::NonZeroU32;

use command::{
    BackfeedSequence, CommandSequence, MediaTracking, MediaType,
    PostPrintAction, ZplCommand,
};
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

pub fn make_preamble() -> CommandSequence {
    CommandSequence(vec![
        ZplCommand::SetDelimiter(','),
        ZplCommand::SetControlCommandPrefix('~'),
        ZplCommand::SetFormatCommandPrefix('^'),
        ZplCommand::SetEncoding(0),
        ZplCommand::StartLabel,
        ZplCommand::SetTearOffPosition(0),
        ZplCommand::SetVerticalShift(0),
        ZplCommand::SetMediaType(MediaType::Transfer),
        ZplCommand::SetMediaTracking(MediaTracking::NonContinuousWebSensing),
        ZplCommand::SetBackfeedSequence(BackfeedSequence::Default),
        ZplCommand::SetHome(0, 0),
        ZplCommand::SetDarkness(25),
        ZplCommand::SetHalfDensity(false),
        ZplCommand::SetSpeed { print: 4, slew: 4 },
        ZplCommand::PersistConfiguration,
        ZplCommand::SetInverted(false),
        // Adjustments
        ZplCommand::SetVerticalShift(12),
        ZplCommand::SetTearOffPosition(-20),
        ZplCommand::EndLabel,
    ])
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

        crate::image::SerializedImage::from_svg(
            svg,
            content_px_width,
            content_px_height,
        )
        .context("Could not load SVG")?
    } else {
        bail!("No image source selected");
    };

    let mut label = make_preamble();
    label.append(CommandSequence(vec![
        ZplCommand::StartLabel,
        ZplCommand::SetPostPrintAction(PostPrintAction::Cut),
        ZplCommand::SetPrintWidth(width * dpmm),
        ZplCommand::SetLabelLength(height * dpmm),
        ZplCommand::SetHorizontalShift(0),
        ZplCommand::MoveOrigin(margin_x, margin_y),
        ZplCommand::RenderImage(image),
        ZplCommand::PrintQuantity {
            total: copies.get(),
            pause_and_cut_after: copies.get(),
            replicates_per_serial: copies.get(),
            cut_only: true,
        },
        ZplCommand::EndLabel,
    ]));

    Ok(label)
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
