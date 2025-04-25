use crate::error::{CrcFailureReason, Error};

pub const READ_SERIAL_NUMBER_COMMAND: u8 = 0x89;
pub const SOFT_RESET_COMMAND: u8 = 0x94;

pub struct Unvalidated([u8; 6]);

impl Unvalidated {
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Return the data bytes from the sensor if the CRC for each pair
    /// is valid, otherwise return an error with the appropriate description
    /// of which bytes failed to validate.
    pub fn try_get_bytes<I>(
        self,
        first_byte_pair_meaning: CrcFailureReason,
        second_byte_pair_meaning: CrcFailureReason,
    ) -> Result<[u8; 4], Error<I>>
    where
        I: embedded_hal::i2c::Error,
    {
        let bytes = self.0;
        if crc8(&bytes[0..3]) != 0 {
            return Err(Error::CrcValidationFailed(first_byte_pair_meaning));
        }
        if crc8(&bytes[3..6]) != 0 {
            return Err(Error::CrcValidationFailed(second_byte_pair_meaning));
        }
        Ok([bytes[0], bytes[1], bytes[3], bytes[4]])
    }
}

/// Power applied to the sensor heater before reading.
#[derive(Clone, Copy)]
pub enum HeaterPower {
    /// 200mW
    High,
    /// 110mW
    Medium,
    /// 20mW
    Low,
}

/// Length of time to run the heater before reading.
#[derive(Clone, Copy)]
pub enum HeaterDuration {
    /// 1 second
    Long,
    /// 0.1 seconds
    Short,
}

#[derive(Clone, Copy)]
/// Level of accuracy with which to read the sensor.
///
/// ## Precision
///
/// The values given for each precision mode are three times the
/// standard deviation of multiple consecutive measurement values
/// at constant conditions and are a measure for the noise on the
/// physical sensor output. (Paraphrasing slightly from p4 of the
/// datasheet.)
///
/// ## Heater
///
/// The sensor may be pre-heated before taking a reading, to improve
/// humidity readings in high-humidity environments or when there is
/// moisture on the sensor (p13 of the datasheet). This measurement
/// is always taken with the high-repeatability mode.
///
/// The heater is designed for a maximum duty cycle of 10%, meaning the
/// total heater-on-time should not be longer than 10% of the sensor's
/// lifetime. (p13)
///
/// Note that the heater can draw 75mA in its highest power setting.
pub enum ReadingMode {
    /// High precision: 0.04°C and 0.08%RH.
    HighPrecision,
    /// Medium precision: 0.07°C and 0.15%RH.
    MediumPrecision,
    /// Low precision: 0.1°C and 0.25%RH.
    LowPrecision,
    /// Apply heat to the sensor before taking a high-precision reading.
    HighPrecisionWithHeater(HeaterPower, HeaterDuration),
}

impl ReadingMode {
    /// I2C command byte for the given reading mode.
    pub fn command_byte(&self) -> u8 {
        match self {
            ReadingMode::HighPrecision => 0xFD,
            ReadingMode::MediumPrecision => 0xF6,
            ReadingMode::LowPrecision => 0xE0,
            ReadingMode::HighPrecisionWithHeater(power, duration) => match (power, duration) {
                (HeaterPower::High, HeaterDuration::Long) => 0x39,
                (HeaterPower::High, HeaterDuration::Short) => 0x32,
                (HeaterPower::Medium, HeaterDuration::Long) => 0x2F,
                (HeaterPower::Medium, HeaterDuration::Short) => 0x24,
                (HeaterPower::Low, HeaterDuration::Long) => 0x1E,
                (HeaterPower::Low, HeaterDuration::Short) => 0x15,
            },
        }
    }
}

/// How long to delay before attempting to read from the sensor.
///
/// The sensor will reject (with NACK) attempts to read before the measurement
/// is ready, so using the maximum delay mode _may_ allow for more reliable
/// first-time reads.
///
/// The **increase** from typical to maximum delay for each mode are:
///
/// - Low: 0.3ms
/// - Medium: 0.8ms
/// - High: 1.4ms
/// - Heater, short: 10ms
/// - Heater, long: 100ms
///
/// Refer to p10 of the datasheet for full timing details.
#[derive(Clone, Copy)]
pub enum ReadingDelayMode {
    /// Use the typical delay times before attempting to read.
    ///
    /// - Low: 1.3ms
    /// - Medium: 3.7ms
    /// - High: 6.9ms
    /// - Heater, short: 100ms
    /// - Heater, long: 1,000ms
    Typical,
    /// Use the maximum delay times before attempting to read.
    ///
    /// - Low: 1.6ms
    /// - Medium: 4.5ms
    /// - High: 8.3ms
    /// - Heater, short: 110ms
    /// - Heater, long: 1,100ms
    Maximum,
}

impl ReadingDelayMode {
    /// Microsecond delay for the current delay mode and the given reading mode.
    ///
    /// Attempting to read from the sensor before its operation has completed
    /// will result in a NACK from the sensor, so this delay is used to ensure
    /// we can successfully read the measurement data over I2C.
    pub fn us_for_reading_mode(&self, reading_mode: ReadingMode) -> u32 {
        use ReadingDelayMode::{Maximum, Typical};
        use ReadingMode::{HighPrecision, HighPrecisionWithHeater, LowPrecision, MediumPrecision};

        match (reading_mode, self) {
            (HighPrecision, Typical) => 6_900,
            (HighPrecision, Maximum) => 8_300,
            (MediumPrecision, Typical) => 3_700,
            (MediumPrecision, Maximum) => 4_500,
            (LowPrecision, Typical) => 1_300,
            (LowPrecision, Maximum) => 1_600,
            (HighPrecisionWithHeater(_, HeaterDuration::Long), Typical) => 1_000_000,
            (HighPrecisionWithHeater(_, HeaterDuration::Long), Maximum) => 1_100_000,
            (HighPrecisionWithHeater(_, HeaterDuration::Short), Typical) => 100_000,
            (HighPrecisionWithHeater(_, HeaterDuration::Short), Maximum) => 110_000,
        }
    }
}

pub struct Config {
    pub reading_mode: ReadingMode,
    pub delay_mode: ReadingDelayMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reading_mode: ReadingMode::HighPrecision,
            delay_mode: ReadingDelayMode::Typical,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    pub raw_temperature_reading: u16,
    pub raw_humidity_reading: u16,
}

impl Measurement {
    pub fn from_read_bytes<I>(sensor_data: Unvalidated) -> Result<Self, Error<I>>
    where
        I: embedded_hal::i2c::Error,
    {
        let [t0, t1, h0, h1] = sensor_data.try_get_bytes(
            CrcFailureReason::TemperatureBytes,
            CrcFailureReason::HumidityBytes,
        )?;
        Ok(Measurement {
            raw_temperature_reading: u16::from_be_bytes([t0, t1]),
            raw_humidity_reading: u16::from_be_bytes([h0, h1]),
        })
    }

    pub fn celsius(&self) -> f32 {
        crate::conversions::temperature_reading_to_celsius(self.raw_temperature_reading)
    }

    pub fn fahrenheit(&self) -> f32 {
        crate::conversions::temperature_reading_to_fahrenheit(self.raw_temperature_reading)
    }

    pub fn humidity(&self) -> f32 {
        crate::conversions::humidity_reading_to_percent_rh(self.raw_humidity_reading)
    }
}

pub fn serial_number_from_read_bytes<I>(sensor_data: Unvalidated) -> Result<u32, Error<I>>
where
    I: embedded_hal::i2c::Error,
{
    let bytes = sensor_data.try_get_bytes(
        CrcFailureReason::SerialNumberFirstPair,
        CrcFailureReason::SerialNumberSecondPair,
    )?;
    Ok(u32::from_be_bytes(bytes))
}

/// Calculate the CRC8/NRSC5 for the given bytes.
///
/// This is pre-set with the polynomial 0x31 and the initial value of 0xFF,
/// with no reflection or final XOR, as specified at 4.4 (p11) in the SHT4x
/// datasheet.
#[must_use]
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
