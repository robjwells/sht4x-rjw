/// Calculate the CRC8/NRSC5 for the given bytes.
///
/// This is pre-set with the polynomial 0x31 and the initial value of 0xFF,
/// with no reflection or final XOR, as specified at 4.4 (p11) in the SHT4x
/// datasheet.
pub fn crc8(bytes: &[u8]) -> u8 {
    const fn top_bit_set(b: u8) -> bool {
        b & 0x80 == 0x80
    }

    const POLYNOMIAL: u8 = 0x31;
    const INITIAL: u8 = 0xFF;

    let mut crc: u8 = INITIAL;
    for byte in bytes {
        crc ^= byte; // "XOR-in" the next byte.
        for _ in 0..8 {
            if top_bit_set(crc) {
                // CRC polynomials have their n+1 bit (here, 9th bit, x^8)
                // implicitly set, so we test the top bit of the current CRC
                // byte, then shift it left before applying the polynomial.
                crc <<= 1;
                crc ^= POLYNOMIAL;
            } else {
                // If the top bit is not set, just keep shifting until it is.
                crc <<= 1;
            }
        }
    }

    crc
}

pub fn reading_to_humidity(bytes: [u8; 2]) -> f32 {
    let reading = u16::from_be_bytes(bytes);
    let s_rh: f32 = reading.into();
    let converted = -6.0 + 125.0 * (s_rh / 65_535.0);
    converted.clamp(0.0, 100.0)
}

#[cfg(test)]
mod test {
    use super::crc8;

    #[test]
    fn crc_0000() {
        assert_eq!(crc8(&[0x00, 0x00]), 0x81);
        assert_eq!(crc8(&[0x00, 0x00, 0x81]), 0x00);
    }

    #[test]
    #[allow(non_snake_case)]
    fn crc_BEEF() {
        assert_eq!(crc8(&[0xBE, 0xEF]), 0x92);
        assert_eq!(crc8(&[0xBE, 0xEF, 0x92]), 0x00);
    }
}
