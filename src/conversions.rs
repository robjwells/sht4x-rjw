//! Sensor readings to celsius, fahrenheit and percent relative humidity.
//!
//! These functions are used by the methods on [`Measurement`] but are
//! provided here should you need to use them. (Note that the raw sensor
//! readings are available as fields of the [`Measurement`] struct.)
//!
//! All conversion formulas can be found in section 4.6 of the [datasheet].
//!
//! The conversions in the root of this module work with and return `f32`s.
//! If you prefer to work with fixed- rather than floating-point numbers,
//! use the `fixed` feature, the [`fixed_point`] submodule, and the
//! corresponding `*_fixed_point` methods on [`Measurement`].
//!
//! [`Measurement`]: crate::common::Measurement
//! [datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf

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

/// Fixed-point numeric conversions from sensor readings.
///
/// The functions in this module are the same as those in the parent
/// `conversions` module, except that they operate with fixed-point numbers
/// rather than floating-point numbers.
///
/// Part of the conversion formula involves converting the reading (a `u16`)
/// into a percentage of `u16::MAX` (between 0 and 1). By using a numeric
/// type with 16 bits for the fractional portion, we can avoid any potential
/// precision loss in the converted reading that may result from working
/// with `f32`s.
#[cfg(feature = "fixed")]
pub mod fixed_point {
    use fixed::types::{I16F16, U16F16};

    /// Convert the raw humidity reading to percent relative humidity.
    ///
    /// The output value is clamped to the range `0.0..=100.0`, as suggested
    /// in the first note in section 4.6 of the datasheet. ("Non-physical"
    /// humidity values may be produced "at the measurement boundaries".)
    pub fn humidity_reading_to_percent_rh(reading: u16) -> I16F16 {
        // Convert u16 reading into a fraction 0..=1
        let fraction: U16F16 = U16F16::from_num(reading) / U16F16::from_num(u16::MAX);
        let converted: I16F16 =
            I16F16::from_num(-6) + I16F16::from_num(125) * I16F16::from_num(fraction);
        converted.clamp(I16F16::ZERO, I16F16::from_num(100))
    }

    /// Convert the raw temperature reading to celsius.
    pub fn temperature_reading_to_celsius(reading: u16) -> I16F16 {
        // Convert u16 reading into a fraction 0..=1
        let fraction: U16F16 = U16F16::from_num(reading) / U16F16::from_num(u16::MAX);
        I16F16::from_num(-45) + I16F16::from_num(175) * I16F16::from_num(fraction)
    }

    /// Convert the raw temperature reading to fahrenheit.
    pub fn temperature_reading_to_fahrenheit(reading: u16) -> I16F16 {
        // Convert u16 reading into a fraction 0..=1
        let fraction: U16F16 = U16F16::from_num(reading) / U16F16::from_num(u16::MAX);
        I16F16::from_num(-49) + I16F16::from_num(315) * I16F16::from_num(fraction)
    }
}
