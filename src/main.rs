use tokio::io::{self, AsyncWriteExt};
use tokio::net::TcpStream;

use core::num::NonZeroU32;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use command::{MediaType, PostPrintAction, ZplCommand};
use image::render_image;
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

#[tokio::main]
async fn main() -> io::Result<()> {
    let Args {
        ip,
        image,
        svg,
        copies,
        width,
        height,
        mut dpmm,
        output_zpl_only,
        output_rendered,
    } = Args::parse();

    let homex = 32;
    let homey = 0;

    let device_config = if !output_zpl_only {
        let socket = TcpStream::connect(ip).await?;
        let config = device::discover(socket).await?;
        Some(config)
    } else {
        None
    };

    if let Some(cfg) = &device_config {
        eprintln!("Detected {cfg:?}");
        dpmm = cfg.indication.dpmm;
    }

    let pix_width = width * dpmm - 2 * homex;
    let pix_height = height * dpmm - 2 * homey;
    let image = if let Some(image) = image {
        let img = ::image::open(image).expect("Image file not found");
        img.resize_to_fill(
            pix_height,
            pix_width,
            ::image::imageops::FilterType::Lanczos3,
        )
    } else if let Some(svg) = svg {
        let svg = tokio::fs::read_to_string(svg)
            .await
            .expect("SVG file not found");

        let pix = svg::pixmap_svg(svg, pix_width, pix_height).expect("SVG file invalid");

        if let Some(output_rendered) = output_rendered {
            pix.save_with_format(output_rendered, ::image::ImageFormat::Png)
                .unwrap();
        }

        pix
    } else {
        eprintln!("No image source selected");
        std::process::exit(1);
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
            ZplCommand::MoveOrigin(homex, homex),
            render_image(&image),
            ZplCommand::PrintQuantity {
                total: copies.get(),
                pause_and_cut_after: copies.get(),
                replicates: copies.get(),
                cut_only: true,
            },
            ZplCommand::End,
        ],
    };

    if output_zpl_only {
        // Convert to ZPL
        let zpl_code = String::from(l);
        println!("{}", zpl_code);
    } else {
        // Output
        let socket = TcpStream::connect(ip).await?;
        let (mut rx, mut tx) = io::split(socket);

        // Send data to the printer
        let response_lines = l.how_many_lines_of_text();
        tokio::spawn(async move {
            for line in String::from(l).lines() {
                tx.write_all(line.as_bytes()).await?;
            }

            Ok::<_, io::Error>(())
        });

        // Wait for incoming data
        let mut buf = vec![];
        for _ in 0..response_lines {
            let line = read::line_with(&mut buf, &mut rx).await?;
            eprintln!("{}", String::from_utf8_lossy(&line.string));
        }

        if response_lines == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10_000)).await;
        }
    }

    Ok(())
}
