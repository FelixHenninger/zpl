use anyhow::{bail, Context};
use device::ZplPrinter;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::TcpStream;

use core::num::NonZeroU32;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use command::{MediaType, PostPrintAction, ZplCommand};
use label::Label;

mod command;
mod device;
mod image;
mod label;
mod read;
mod svg;

#[derive(Parser)]
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
    #[arg(long = "output-rendered")]
    output_rendered: Option<PathBuf>,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let Args {
        ip,
        image,
        svg,
        copies,
        width,
        height,
        dpmm,
        output_zpl_only,
        output_rendered,
    } = args;

    let margin_x = 0;
    let margin_y = 0;

    let mut device = ZplPrinter::with_address(ip).await?;
    let config = device.discover_device_info().await?;

    let pix_width = width * config.indication.dpmm - 2 * margin_x;
    let pix_height = height * config.indication.dpmm - 2 * margin_y;
    let image = if let Some(image) = image {
        let img = ::image::open(image).expect("Image file not found");
        img.resize_to_fill(
            pix_height,
            pix_width,
            ::image::imageops::FilterType::Lanczos3,
        );
        crate::image::SerializedImage::from_image(&img)
    } else if let Some(svg) = svg {
        let svg = tokio::fs::read_to_string(svg)
            .await
            .expect("SVG file not found");

        crate::image::SerializedImage::from_svg(svg, pix_width, pix_height)
            .context("Could not load SVG")?
    } else {
        bail!("No image source selected");
    };

    let l = Label {
        commands: vec![
            //ZplCommand::Magic,
            ZplCommand::Start,
            ZplCommand::SetVerticalShift(12),
            ZplCommand::SetTearOffPosition(-20),
            ZplCommand::SetMediaType(MediaType::Direct),
            ZplCommand::SetHome(0, 0),
            ZplCommand::SetHalfDensity(false),
            ZplCommand::SetSpeed { print: 4, slew: 4 },
            ZplCommand::SetDarkness(15),
            ZplCommand::PersistConfig,
            ZplCommand::SetInverted(false),
            ZplCommand::SetEncoding(0),
            ZplCommand::End,
            ZplCommand::Start,
            ZplCommand::SetPostPrintAction(PostPrintAction::Cut),
            ZplCommand::LabelSetup {
                w: width,
                h: height,
                dpmm: config.indication.dpmm,
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
        ],
    };

    Ok(device.print(l).await?)
}

pub async fn run_output_zpl_only(args: Args) -> anyhow::Result<()> {
    let Args {
        ip,
        image,
        svg,
        copies,
        width,
        height,
        dpmm,
        output_zpl_only,
        output_rendered,
    } = args;

    let margin_x = 0;
    let margin_y = 0;

    let pix_width = width * dpmm - 2 * margin_x;
    let pix_height = height * dpmm - 2 * margin_y;
    let image = if let Some(image) = image {
        let img = ::image::open(image).expect("Image file not found");
        img.resize_to_fill(
            pix_height,
            pix_width,
            ::image::imageops::FilterType::Lanczos3,
        );
        crate::image::SerializedImage::from_image(&img)
    } else if let Some(svg) = svg {
        let svg = tokio::fs::read_to_string(svg)
            .await
            .expect("SVG file not found");

        crate::image::SerializedImage::from_svg(svg, pix_width, pix_height)
            .context("Could not load SVG")?
    } else {
        bail!("No image source selected");
    };

    let l = Label {
        commands: vec![
            //ZplCommand::Magic,
            ZplCommand::Start,
            ZplCommand::SetVerticalShift(12),
            ZplCommand::SetTearOffPosition(-20),
            ZplCommand::SetMediaType(MediaType::Direct),
            ZplCommand::SetHome(0, 0),
            ZplCommand::SetHalfDensity(false),
            ZplCommand::SetSpeed { print: 4, slew: 4 },
            ZplCommand::SetDarkness(15),
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
        ],
    };

    println!("{}", l);

    Ok(())
}
