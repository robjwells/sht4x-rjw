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

    pub fn convert_reading(&self, bytes: [u8; 2]) -> f32 {
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
