use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{I2c, SevenBitAddress};

use crate::error::Error;
use crate::types::*;
use crate::utils::*;

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
        self.i2c
            .write(self.address, &[READ_SERIAL_NUMBER_COMMAND])?;
        self.i2c.read(self.address, &mut self.read_buffer)?;

        validate_crc(
            &self.read_buffer,
            "first two bytes of serial number",
            "second two bytes of serial number",
        )?;

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
        )
    }

    /// Measure temperature and humidity with the given settings.
    pub fn measure_with_settings(
        &mut self,
        mut delay: impl DelayNs,
        reading_mode: ReadingMode,
        delay_mode: ReadingDelayMode,
        temperature_unit: TemperatureUnit,
    ) -> Result<Measurement, Error<I::Error>> {
        let command = reading_mode.command_byte();
        let us = delay_mode.us_for_reading_mode(reading_mode);

        self.i2c.write(self.address, &[command])?;
        delay.delay_us(us);
        self.i2c.read(self.address, &mut self.read_buffer)?;

        validate_crc(&self.read_buffer, "temperature bytes", "humidity bytes")?;

        let [t0, t1, _, h0, h1, _] = self.read_buffer;
        let temperature = temperature_unit.convert_reading([t0, t1]);
        let humidity = reading_to_humidity([h0, h1]);

        Ok(Measurement {
            temperature,
            temperature_unit,
            humidity,
        })
    }
}
