use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use command::ZplCommand;
use label::Label;

mod command;
mod label;

#[tokio::main]
async fn main() -> io::Result<()> {
    let l = Label {
        commands: vec![
            ZplCommand::Raw("CT~~CD,~CC^~CT~".to_string()),
            ZplCommand::Raw("^XA~TA000~JSN^LT0^MNW^MTD^PON^PMN^LH0,0^JMA^PR4,4~SD15^JUS^LRN^CI0^XZ".to_string()),
            ZplCommand::Raw("^XA".to_string()),
            // Begin ZPL-II code
            // Print mode -> C: Cut after printing
            ZplCommand::Raw("^MMC".to_string()),
            // Print width in dots
            ZplCommand::Raw("^PW684".to_string()),
            // Label length
            ZplCommand::Raw("^LL0384".to_string()),
            // Label shift (to the left)
            ZplCommand::Raw("^LS0".to_string()),
            // Field origin
            ZplCommand::Raw("^FO32,32^GFA,20480,20480,00064,:Z64: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:40FE".to_string()),
            ZplCommand::Raw("^PQ1,1,1,Y^XZ".to_string()),
        ],
    };

    let socket = TcpStream::connect("192.168.1.39:9100").await?;
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
