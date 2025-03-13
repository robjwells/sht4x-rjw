#![no_std]

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{I2c, SevenBitAddress};

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
pub enum ReadingMode {
    HighPrecision,
    MediumPrecision,
    LowPrecision,
    /// Apply heat to the sensor before taking a high precision reading.
    HighPrecisionWithHeater(HeaterPower, HeaterDuration),
}

impl ReadingMode {
    fn command_byte(&self) -> u8 {
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
#[derive(Clone, Copy)]
pub enum ReadingDelayMode {
    Typical,
    Maximum,
}

impl ReadingDelayMode {
    /// Microsecond delay for the current delay mode and the given reading mode.
    ///
    /// Attempting to read from the sensor before its operation has completed
    /// will result in a NACK from the sensor, so this delay is used to ensure
    /// we can successfully read the measurement data over I2C.
    fn us_for_reading_mode(&self, reading_mode: ReadingMode) -> u32 {
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

#[derive(Clone, Copy, Debug)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

impl TemperatureUnit {
    fn reading_to_celsius(bytes: [u8; 2]) -> f32 {
        let reading = u16::from_be_bytes(bytes);
        let s_t: f32 = reading.into();
        -45.0 + 175.0 * (s_t / 65_535.0)
    }

    fn reading_to_fahrenheit(bytes: [u8; 2]) -> f32 {
        let reading = u16::from_be_bytes(bytes);
        let s_t: f32 = reading.into();
        -49.0 + 315.0 * (s_t / 65_535.0)
    }

    fn convert_reading(&self, bytes: [u8; 2]) -> f32 {
        match self {
            TemperatureUnit::Celsius => Self::reading_to_celsius(bytes),
            TemperatureUnit::Fahrenheit => Self::reading_to_fahrenheit(bytes),
        }
    }
}

pub struct Config {
    pub reading_mode: ReadingMode,
    pub delay_mode: ReadingDelayMode,
    pub temperature_unit: TemperatureUnit,
    pub should_validate_crc: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reading_mode: ReadingMode::HighPrecision,
            delay_mode: ReadingDelayMode::Typical,
            temperature_unit: TemperatureUnit::Celsius,
            should_validate_crc: true,
        }
    }
}

#[derive(Debug)]
pub enum Error<I2cError>
where
    I2cError: embedded_hal::i2c::Error,
{
    CrcValidationFailed(&'static str),
    I2c(I2cError),
}

impl<I2cError> From<I2cError> for Error<I2cError>
where
    I2cError: embedded_hal::i2c::Error,
{
    fn from(value: I2cError) -> Self {
        Error::I2c(value)
    }
}

impl<I2cError> core::fmt::Display for Error<I2cError>
where
    I2cError: embedded_hal::i2c::Error,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::CrcValidationFailed(location) => {
                write!(f, "CRC validation failed for {location}")
            }
            Error::I2c(e) => write!(f, "Received I2C error: {:?}", e),
        }
    }
}

impl<I> core::error::Error for Error<I> where I: embedded_hal::i2c::Error {}

#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    /// Temperature value converted from the raw sensor reading using the
    /// formulas at 4.6 (p12) in the SHT4x datasheet.
    pub temperature: f32,
    pub temperature_unit: TemperatureUnit,
    /// Percent relative humidity converted from the raw sensor reading using
    /// the formulas at 4.6 (p12) in the SHT4x datasheet. The humidity value
    /// is clamped to `[0, 100]` %RH.
    pub humidity: f32,
}

pub struct SHT40<I: I2c> {
    i2c: I,
    read_buffer: [u8; 6],
    pub address: SevenBitAddress,
    pub config: Config,
}

impl<I: I2c> SHT40<I> {
    pub fn new(i2c: I, config: Config) -> Self {
        Self {
            i2c,
            address: 0x44,
            read_buffer: [0u8; 6],
            config,
        }
    }

    pub fn destroy(self) -> I {
        self.i2c
    }

    pub fn serial_number(&mut self) -> Result<u32, Error<I::Error>> {
        self.serial_number_with_settings(self.config.should_validate_crc)
    }

    pub fn serial_number_with_settings(
        &mut self,
        should_validate_crc: bool,
    ) -> Result<u32, Error<I::Error>> {
        const READ_SERIAL_NUMBER_COMMAND: u8 = 0x89;

        self.i2c
            .write(self.address, &[READ_SERIAL_NUMBER_COMMAND])?;
        self.i2c.read(self.address, &mut self.read_buffer)?;

        if should_validate_crc {
            self.validate_crc(
                "first two bytes of serial number",
                "second two bytes of serial number",
            )?;
        }

        Ok(u32::from_be_bytes([
            self.read_buffer[0],
            self.read_buffer[1],
            self.read_buffer[3],
            self.read_buffer[4],
        ]))
    }

    pub fn soft_reset(&mut self, mut delay: impl DelayNs) -> Result<(), Error<I::Error>> {
        const SOFT_RESET_COMMAND: u8 = 0x94;

        self.i2c.write(self.address, &[SOFT_RESET_COMMAND])?;
        delay.delay_ms(1);
        Ok(())
    }

    /// Measure temperature and humidity with the settings provided upon
    /// construction of the sensor struct.
    pub fn measure(&mut self, delay: impl DelayNs) -> Result<Measurement, Error<I::Error>> {
        self.measure_with_settings(
            delay,
            self.config.reading_mode,
            self.config.delay_mode,
            self.config.temperature_unit,
            self.config.should_validate_crc,
        )
    }

    /// Measure temperature and humidity with the given settings.
    pub fn measure_with_settings(
        &mut self,
        mut delay: impl DelayNs,
        reading_mode: ReadingMode,
        delay_mode: ReadingDelayMode,
        temperature_unit: TemperatureUnit,
        should_validate_crc: bool,
    ) -> Result<Measurement, Error<I::Error>> {
        let command = reading_mode.command_byte();
        let us = delay_mode.us_for_reading_mode(reading_mode);

        self.i2c.write(self.address, &[command])?;
        delay.delay_us(us);
        self.i2c.read(self.address, &mut self.read_buffer)?;

        if should_validate_crc {
            self.validate_crc("temperature bytes", "humidity bytes")?;
        }

        let [t0, t1, _, h0, h1, _] = self.read_buffer;
        let temperature = temperature_unit.convert_reading([t0, t1]);
        let humidity = reading_to_humidity([h0, h1]);

        Ok(Measurement {
            temperature,
            temperature_unit,
            humidity,
        })
    }

    /// Validate the CRC for each half of the read buffer, returning the
    /// `first_failure` message if the first three bytes fail to validate,
    /// and `second_failure` if the last three bytes fail to validate.
    fn validate_crc(
        &self,
        first_failure: &'static str,
        second_failure: &'static str,
    ) -> Result<(), Error<I::Error>> {
        if crc8(&self.read_buffer[0..3]) != 0 {
            return Err(Error::CrcValidationFailed(first_failure));
        }
        if crc8(&self.read_buffer[3..6]) != 0 {
            return Err(Error::CrcValidationFailed(second_failure));
        }
        Ok(())
    }
}

/// Calculate the CRC8/NRSC5 for the given bytes.
///
/// This is pre-set with the polynomial 0x31 and the initial value of 0xFF,
/// with no reflection or final XOR, as specified at 4.4 (p11) in the SHT4x
/// datasheet.
fn crc8(bytes: &[u8]) -> u8 {
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

fn reading_to_humidity(bytes: [u8; 2]) -> f32 {
    let reading = u16::from_be_bytes(bytes);
    let s_rh: f32 = reading.into();
    let converted = -6.0 + 125.0 * (s_rh / 65_535.0);
    converted.clamp(0.0, 100.0)
}
