use crate::types::*;
use crate::utils::crc8;
use crate::utils::reading_to_humidity;

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{I2c, SevenBitAddress};


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
