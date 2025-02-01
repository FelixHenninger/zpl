use crc::{Crc, CRC_16_XMODEM};

pub fn checksum(data: &[u8]) -> u16 {
    let crc = Crc::<u16>::new(&CRC_16_XMODEM);
    crc.checksum(data)
}
