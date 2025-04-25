/// Temperature value converted from the raw sensor reading using the
/// formulas at 4.6 (p12) in the SHT4x datasheet.
pub fn temperature_reading_to_celsius(reading: u16) -> f32 {
    let s_t: f32 = reading.into();
    -45.0 + 175.0 * (s_t / 65_535.0)
}

/// Temperature value converted from the raw sensor reading using the
/// formulas at 4.6 (p12) in the SHT4x datasheet.
pub fn temperature_reading_to_fahrenheit(reading: u16) -> f32 {
    let s_t: f32 = reading.into();
    -49.0 + 315.0 * (s_t / 65_535.0)
}

/// Percent relative humidity converted from the raw sensor reading using
/// the formulas at 4.6 (p12) in the SHT4x datasheet. The humidity value
/// is clamped to `[0, 100]` %RH.
pub fn humidity_reading_to_percent_rh(reading: u16) -> f32 {
    let s_rh: f32 = reading.into();
    let converted = -6.0 + 125.0 * (s_rh / 65_535.0);
    converted.clamp(0.0, 100.0)
}
