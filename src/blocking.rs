use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{I2c, SevenBitAddress};

use crate::common::{
    Config, Measurement, ReadingDelayMode, ReadingMode, Unvalidated,
};
use crate::common::{
    READ_SERIAL_NUMBER_COMMAND, SOFT_RESET_COMMAND, serial_number_from_read_bytes,
};
use crate::error::Error;

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
        serial_number_from_read_bytes(Unvalidated::new(self.read_buffer))
    }

    pub fn soft_reset(&mut self, mut delay: impl DelayNs) -> Result<(), Error<I::Error>> {
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
        )
    }

    /// Measure temperature and humidity with the given settings.
    pub fn measure_with_settings(
        &mut self,
        mut delay: impl DelayNs,
        reading_mode: ReadingMode,
        delay_mode: ReadingDelayMode,
    ) -> Result<Measurement, Error<I::Error>> {
        let command = reading_mode.command_byte();
        let us = delay_mode.us_for_reading_mode(reading_mode);

        self.i2c.write(self.address, &[command])?;
        delay.delay_us(us);
        self.i2c.read(self.address, &mut self.read_buffer)?;

        Measurement::from_read_bytes(Unvalidated::new(self.read_buffer))
    }
}
