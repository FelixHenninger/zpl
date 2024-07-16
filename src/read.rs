use std::io;
use tokio::io::AsyncReadExt;

pub struct DiagnosticString {
    #[allow(dead_code)]
    pub start: Vec<u8>,
    pub string: Vec<u8>,
}

pub async fn line_with(
    buf: &mut Vec<u8>,
    rx: &mut (impl AsyncReadExt + core::marker::Unpin),
) -> Result<DiagnosticString, io::Error> {
    let post_etx = 'brk: {
        if let Some(pos) = buf.iter().position(|c| *c == b'\x03') {
            break 'brk pos + 1;
        }

        let mut read_buf = [0; 128];

        loop {
            let n = rx.read(&mut read_buf).await?;

            if n == 0 {
                return Err(io::ErrorKind::BrokenPipe)?;
            }

            let post_fin = read_buf[..n]
                .iter()
                .position(|c| *c == b'\x03')
                .map(|x| x + 1 + buf.len());

            buf.extend_from_slice(&read_buf[..n]);

            if let Some(fin) = post_fin {
                break 'brk fin;
            }
        }
    };

    let tail = buf.split_off(post_etx);

    let mut line = core::mem::replace(buf, tail);
    let _ = line.pop();

    // if there's anything before, discard it.
    let start = line.iter().position(|c| *c == b'\x02').map_or(0, |n| n + 1);
    let string = line.split_off(start);

    return Ok(DiagnosticString {
        start: line,
        string,
    });
}
