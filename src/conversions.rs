//! Sensor readings to celsius, fahrenheit and percent relative humidity.
//!
//! These functions are used by the methods on [`Measurement`] but are
//! provided here should you need to use them. (Note that the raw sensor
//! readings are available as fields of the [`Measurement`] struct.)
//!
//! All conversion formulas can be found in section 4.6 of the [datasheet].
//!
//! [`Measurement`]: crate::common::Measurement
//! [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf

/// Convert the raw temperature reading to celsius.
pub fn temperature_reading_to_celsius(reading: u16) -> f32 {
    let s_t: f32 = reading.into();
    -45.0 + 175.0 * (s_t / 65_535.0)
}

/// Convert the raw temperature reading to fahrenheit.
pub fn temperature_reading_to_fahrenheit(reading: u16) -> f32 {
    let s_t: f32 = reading.into();
    -49.0 + 315.0 * (s_t / 65_535.0)
}

/// Convert the raw humidity reading to percent relative humidity.
///
/// The output value is clamped to the range `0.0..=100.0`, as suggested
/// in the first note in section 4.6 of the datasheet. ("Non-physical"
/// humidity values may be produced "at the measurement boundaries".)
pub fn humidity_reading_to_percent_rh(reading: u16) -> f32 {
    let s_rh: f32 = reading.into();
    let converted = -6.0 + 125.0 * (s_rh / 65_535.0);
    converted.clamp(0.0, 100.0)
}
