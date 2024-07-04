use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
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

#[derive(Parser)]
pub struct Args {
    #[arg(default_value = "192.168.1.39:9100")]
    ip: SocketAddr,
    #[arg(long = "image", default_value = "picture.png")]
    image: PathBuf,
    #[arg(long = "repeat", default_value = "1")]
    repeat_stuff_repeat_stuff: NonZeroU32,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let Args {
        ip,
        image,
        repeat_stuff_repeat_stuff,
    } = Args::parse();

    let l = Label {
        commands: vec![
            //ZplCommand::Magic,
            ZplCommand::Start,
            ZplCommand::SetVerticalShift(12),
            ZplCommand::SetTearOffPosition(50),
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
                w: 51,
                h: 51,
                dots: 12,
            },
            ZplCommand::SetHorizontalShift(0),
            ZplCommand::MoveOrigin(32, 32),
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
    tokio::spawn(async move {
        for line in String::from(l).lines() {
            tx.write_all(line.as_bytes()).await?;
        }

        Ok::<_, io::Error>(())
    });

    // Wait for incoming data
    let mut buf = vec![0; 128];
    loop {
        let n = rx.read(&mut buf).await?;

        if n == 0 {
            break;
        }

        println!("Received: {:?}", &buf[..n]);
    }

    Ok(())
}
