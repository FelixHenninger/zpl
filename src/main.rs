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
    #[arg(long = "repeat", default_value = "1")]
    repeat_stuff_repeat_stuff: NonZeroU32,
    #[arg(long = "mm-width", default_value = "51")]
    width: u32,
    #[arg(long = "mm-height", default_value = "51")]
    height: u32,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let Args {
        ip,
        image,
        svg,
        repeat_stuff_repeat_stuff,
        width,
        height,
    } = Args::parse();

    let ppi = 4;
    let homex = 32;
    let homey = 0;

    let pix_width = width * ppi - 2 * homex;
    let pix_height = height * ppi - 2 * homey;
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

        svg::pixmap_svg(svg, pix_width).expect("SVG file invalid")
    } else {
        eprintln!("No image source selected");
        std::process::exit(1);
    };

    let l = Label {
        commands: vec![ZplCommand::HostStatusReturn, ZplCommand::HostIndication],
    };

    let l2 = Label {
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
                dots: ppi,
            },
            ZplCommand::SetHorizontalShift(0),
            ZplCommand::MoveOrigin(homex, homex),
            render_image(&image),
            ZplCommand::PrintQuantity {
                total: repeat_stuff_repeat_stuff.get(),
                pause_and_cut_after: repeat_stuff_repeat_stuff.get(),
                replicates: repeat_stuff_repeat_stuff.get(),
                cut_only: true,
            },
            ZplCommand::End,
        ],
    };

    let socket = TcpStream::connect(ip).await?;
    let (mut rx, mut tx) = io::split(socket);

    // Send data to the printer
    let response = l.how_many_lines_of_text();
    tokio::spawn(async move {
        for line in String::from(l).lines() {
            tx.write_all(line.as_bytes()).await?;
        }

        Ok::<_, io::Error>(())
    });

    // Wait for incoming data
    let mut buf = vec![];
    for _ in 0..response {
        let line = read::line_with(&mut buf, &mut rx).await?;
        eprintln!("{}", String::from_utf8_lossy(&line.string));
    }

    Ok(())
}
