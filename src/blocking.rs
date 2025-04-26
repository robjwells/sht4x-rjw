//! Blocking driver for SHT40
use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::{I2c, SevenBitAddress};

use crate::common::{Config, Measurement, ReadingDelayMode, ReadingMode, Unvalidated};
use crate::common::{
    READ_SERIAL_NUMBER_COMMAND, SOFT_RESET_COMMAND, serial_number_from_read_bytes,
};
use crate::error::Error;

/// Blocking SHT40 sensor interface
///
/// With this you can read the temperature and humidity from the SHT40,
/// read its 4-byte serial number, and perform a soft reset of the sensor.
///
/// Note that the driver must be declared as `mut` as I2C reads and writes
/// mutate the I2C interface struct.
///
/// ## Example usage
///
/// ```rust
/// # use embedded_hal_mock::eh1::i2c::{Mock, Transaction};
/// # fn main() -> anyhow::Result<()> {
/// #   let mut delay = embedded_hal_mock::eh1::delay::NoopDelay::new();
/// #   let expectations = [
/// #     // Request the sensor serial number.
/// #     Transaction::write(0x44, vec![0x89]),
/// #     // Receive a fake sensor serial number.
/// #     Transaction::read(0x44, vec![0x01, 0x02, 0x17, 0x3, 0x4, 0x68]),
/// #     // Request a high-precision read.
/// #     Transaction::write(0x44, vec![0xFD]),
/// #     // Receive a fake temp & humidity measurement.
/// #     Transaction::read(0x44, vec![0x12, 0x34, 0x37, 0x56, 0x78, 0x7D])
/// #   ];
/// #   let i2c = Mock::new(&expectations);
/// use sht40_rjw::blocking::SHT40;
/// let mut sensor = SHT40::new(i2c, Default::default());
/// let serial_number = sensor.serial_number()?;
/// let measurement = sensor.measure(&mut delay)?;
///
/// defmt::info!(
///     "SHT40 sensor with serial {}, currently: {}°C, {}%RH",
///     serial_number,
///     measurement.celsius(),
///     measurement.humidity()
/// );
/// #   sensor.destroy().done();    // Call done on the I2C mock.
/// #   Ok(())
/// # }
/// ```
pub struct SHT40<I: I2c> {
    i2c: I,

    /// Internal buffer to hold the response from the SHT40, which
    /// is always 6 bytes: `[data, data, CRC, data, data, CRC]`
    ///
    /// This buffer is reused for each read from the sensor.
    read_buffer: [u8; 6],

    /// I2C address of your SHT40 sensor.
    ///
    /// If your sensor is not at all the default address (`0x44`), write to
    /// this field after instantiation. The new address will affect all
    /// subsequent I2C interactions.
    pub address: SevenBitAddress,

    /// Default reading and delay modes used by [`SHT40::measure()`].
    pub config: Config,
}

impl<I: I2c> SHT40<I> {
    /// Create a new sensor with the default address of `0x44`.
    ///
    /// Example usage of configuring the driver to use the heater on
    /// highest power, longest pulse, and maximum delay:
    ///
    /// ```rust
    /// # use embedded_hal_mock::eh1::i2c::{Mock, Transaction};
    /// # let i2c = Mock::new(&[]);
    /// use sht40_rjw::blocking::SHT40;
    /// use sht40_rjw::common::*;
    /// let sensor = SHT40::new(i2c, Config {
    ///     reading_mode: ReadingMode::HighPrecisionWithHeater(
    ///         HeaterPower::High,
    ///         HeaterDuration::Long,
    ///     ),
    ///     delay_mode: ReadingDelayMode::Maximum,
    /// });
    /// # sensor.destroy().done();
    /// ```
    pub fn new(i2c: I, config: Config) -> Self {
        Self {
            i2c,
            address: 0x44,
            read_buffer: [0u8; 6],
            config,
        }
    }

    /// Drop the sensor struct and return its I2C interface.
    pub fn destroy(self) -> I {
        self.i2c
    }

    /// Read the 4-byte serial number from the sensor.
    ///
    /// # Errors
    ///
    /// An error may be returned if the serial number data bytes fail
    /// to pass CRC validation, or if a problem occurs with the I2C
    /// interface.
    pub fn serial_number(&mut self) -> Result<u32, Error<I::Error>> {
        // Note that the SHT4x I2C interface requires a STOP condition after
        // the write, so we cannot use self.i2c.write_read(), which issues
        // a REPEATED-START between writing the command and attempting to
        // read from the sensor.
        //
        // This is the case even here, where no delay is needed for the
        // sensor to make the data available for reading.
        self.i2c
            .write(self.address, &[READ_SERIAL_NUMBER_COMMAND])?;
        self.i2c.read(self.address, &mut self.read_buffer)?;
        serial_number_from_read_bytes(Unvalidated::new(self.read_buffer))
    }

    /// Reset the sensor and wait for it to return to its idle state.
    ///
    /// # Errors
    ///
    /// An error may be returned if there is a problem with the I2C interface.
    pub fn soft_reset(&mut self, mut delay: impl DelayNs) -> Result<(), Error<I::Error>> {
        self.i2c.write(self.address, &[SOFT_RESET_COMMAND])?;
        delay.delay_ms(1);
        Ok(())
    }

    /// Measure temperature and humidity with the settings provided upon
    /// construction of the sensor struct.
    ///
    /// This method is a convenience wrapper around [`SHT40::measure_with_settings()`]
    /// so that it is not necessary to specify the reading and delay mode
    /// each time you wish to obtain a measurement from the sensor.
    pub fn measure(&mut self, delay: impl DelayNs) -> Result<Measurement, Error<I::Error>> {
        self.measure_with_settings(delay, self.config.reading_mode, self.config.delay_mode)
    }

    /// Measure temperature and humidity with the given settings.
    ///
    /// # Errors
    ///
    /// An error may be returned if either the temperature or humidity
    /// data bytes fail to pass CRC validation, or if a problem occurs
    /// with the I2C interface.
    ///
    /// # Timing
    ///
    /// A delay is required between requesting the measurement and being able
    /// to read in the data. This varies depending on your reading and delay
    /// modes. Refer to the [ReadingDelayMode] documentation for the length
    /// of the delay.
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
