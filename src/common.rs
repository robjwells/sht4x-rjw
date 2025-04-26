use crate::error::{CrcFailureReason, Error};

pub(crate) const READ_SERIAL_NUMBER_COMMAND: u8 = 0x89;
pub(crate) const SOFT_RESET_COMMAND: u8 = 0x94;

/// Internal wrapper around the 6 bytes read from the sensor, so that the
/// 4 data bytes may only be accessed after passing CRC verification.
pub(crate) struct Unvalidated([u8; 6]);

impl Unvalidated {
    pub(crate) fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Return the data bytes from the sensor if the CRC for each pair
    /// is valid, otherwise return an error with the appropriate description
    /// of which bytes failed to validate.
    ///
    /// If we name the bytes read from the sensor `s0` through `s5`, the
    /// bytes returned from this method are `[s0, s1, s3, s4]`, as bytes
    /// `s2` and `s5` are CRC values for the preceding two bytes.
    ///
    /// See sections 4.3 and 4.5 of the [datasheet] for a detailed description
    /// of the six bytes returned for each I2C command.
    ///
    /// [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
    pub(crate) fn try_get_bytes<I>(
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

/// Power applied to the sensor heater before measuring.
///
/// See section 4.9 of the [datasheet] for general information on use of the
/// sensor heater, and section 3.1 for heater current data. Current figures
/// for each variant are drawn from those sections, where the "typical" values
/// are at 3.3V supply and 25°C ambient temperature, and maximum values are
/// valid across the full −40°C to 125°C temperature range.
///
/// [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
#[derive(Clone, Copy)]
pub enum HeaterPower {
    /// 200mW nominal
    ///
    /// Typically this is 60mA of current, up to a maximum of 100mA.
    /// Note that, in section 4.9 of the datasheet, Sensirion list the
    /// highest heater power mode as drawing "~75 mA".
    High,
    /// 110mW nominal
    ///
    /// Typically 33mA, up to a maximum of 55mA.
    Medium,
    /// 20mW nominal
    ///
    /// Typically 6mA, up to a maximum of 10mA.
    Low,
}

/// Length of time to run the heater before measuring.
///
/// See section 3.2 of the [datasheet] for timing details. In short, the actual
/// heater pulse duration may be ±10% of the listed duration. The heater is
/// automatically shut off after the heating pulse.
///
/// Note that Sensirion state "the heater is designed for a maximum duty cycle
/// of 10%, meaning the total heater-on-time should not be longer than 10% of
/// the sensor’s lifetime". See section 4.9 of the datasheet for further
/// information.
///
/// [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
#[derive(Clone, Copy)]
pub enum HeaterDuration {
    /// 1 second
    Long,
    /// 0.1 seconds
    Short,
}

#[derive(Clone, Copy)]
/// Level of precision with which to read the sensor.
///
/// "Precision" or "accuracy" here refer to the repeatability of the measurement,
/// i.e. consecutive readings at lower precision will have a wider distribution
/// than those taken at higher precision.
///
/// Note 2 in section 2 of the [datasheet] states:
///
/// > The stated repeatability is three times the standard deviation (3σ) of
/// > multiple consecutive measurement values at constant conditions and is a
/// > measure for the noise on the physical sensor output.
///
/// These repeatability figures are stated below for each reading mode.
///
/// Note that lower precision readings complete faster than higher precision
/// readings (see [ReadingDelayMode] and section 3.2 of the datasheet).
/// As well, "low precision" does not mean "inaccurate" and the acceptable
/// level of repeatability will depend on your own use case.
///
/// # Heater
///
/// The sensor may be pre-heated before taking a reading, to improve humidity
/// readings in high-humidity environments or when there is moisture on the
/// sensor. This measurement is always taken with the high-precision mode.
/// See section 4.9 of the [datasheet] for information about the use of the
/// heater, as well as section 3 for electrical and timing information.
///
/// [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
pub enum ReadingMode {
    /// High repeatability: 3σ of 0.04°C and 0.08%RH.
    HighPrecision,
    /// Medium repeatability: 3σ of 0.07°C and 0.15%RH.
    MediumPrecision,
    /// Low repeatability: 3σ of 0.1°C and 0.25%RH.
    LowPrecision,
    /// Apply heat to the sensor before taking a high-repeatability reading.
    HighPrecisionWithHeater(HeaterPower, HeaterDuration),
}

impl ReadingMode {
    /// I2C command byte for the given reading mode.
    pub(crate) fn command_byte(&self) -> u8 {
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

/// Length of delay before attempting to read from the sensor.
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
    pub(crate) fn us_for_reading_mode(&self, reading_mode: ReadingMode) -> u32 {
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
    pub(crate) fn from_read_bytes<I>(sensor_data: Unvalidated) -> Result<Self, Error<I>>
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

pub(crate) fn serial_number_from_read_bytes<I>(sensor_data: Unvalidated) -> Result<u32, Error<I>>
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
pub(crate) fn crc8(bytes: &[u8]) -> u8 {
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
