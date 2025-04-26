/// Calculate the CRC8 for the given bytes.
///
/// The three bytes passed should be two data bytes with the following CRC
/// byte, as read from the sensor. The result of the function will be 0
/// if the CRC byte is correct for the preceding two data bytes.
///
/// # Example usage
///
/// ```rust,ignore
/// // Example taken from the datasheet.
/// assert_eq!(crc8([0xBE, 0xEF, 0x92]), 0);
/// ````
///
/// # CRC details
///
/// This is pre-set with the polynomial 0x31 and the initial value of 0xFF,
/// with no reflection or final XOR, as specified in section 4.4 of the
/// [datasheet].
///
/// This CRC appears to be also known as the "NRSC-5" CRC8, but this is not
/// a term Sensirion use in their documentation.
///
/// [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
#[must_use]
fn crc8(bytes: [u8; 3]) -> u8 {
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

/// A wrapper around [`crc8`] that relieves the caller of having
/// to compare the CRC result to 0.
pub(crate) fn validate_crc(bytes: [u8; 3]) -> Result<(), u8> {
    match crc8(bytes) {
        0 => Ok(()),
        x => Err(x),
    }
}

#[cfg(test)]
mod test {
    use super::crc8;

    #[test]
    fn crc_0000() {
        assert_eq!(crc8([0x00, 0x00, 0x81]), 0x00);
    }

    #[test]
    #[allow(non_snake_case)]
    fn crc_BEEF() {
        assert_eq!(crc8([0xBE, 0xEF, 0x92]), 0x00);
    }
}
