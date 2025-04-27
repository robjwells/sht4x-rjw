/// Error wrapper for all driver methods that interact with the sensor.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<I2cError>
where
    I2cError: embedded_hal::i2c::Error,
{
    /// A byte pair had an incorrect CRC.
    CrcValidationFailed {
        reason: CrcFailureReason,
        received_bytes: [u8; 3],
        calculated_crc: u8,
    },

    /// An error was returned from the underlying I2C interface.
    I2c(I2cError),
}

/// Describes which byte pair had an incorrect CRC.
pub enum CrcFailureReason {
    /// The first two bytes of the four-byte serial number.
    SerialNumberFirstPair,
    /// The second two bytes of the four-byte serial number.
    SerialNumberSecondPair,
    /// The temperature reading bytes.
    TemperatureBytes,
    /// The humidity reading bytes.
    HumidityBytes,
}

impl core::fmt::Debug for CrcFailureReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SerialNumberFirstPair => write!(f, "first two bytes of serial number"),
            Self::SerialNumberSecondPair => write!(f, "second two bytes of serial number"),
            Self::TemperatureBytes => write!(f, "temperature bytes"),
            Self::HumidityBytes => write!(f, "humidity bytes"),
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for CrcFailureReason {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Self::SerialNumberFirstPair => defmt::write!(fmt, "first two bytes of serial number"),
            Self::SerialNumberSecondPair => defmt::write!(fmt, "second two bytes of serial number"),
            Self::TemperatureBytes => defmt::write!(fmt, "temperature bytes"),
            Self::HumidityBytes => defmt::write!(fmt, "humidity bytes"),
        }
    }
}

impl core::fmt::Display for CrcFailureReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <CrcFailureReason as core::fmt::Debug>::fmt(self, f)
    }
}

/// Enable `?` to convert embedded-hal I2C errors into our `Error`.
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
            Error::CrcValidationFailed {
                reason,
                received_bytes,
                calculated_crc,
            } => {
                write!(
                    f,
                    "CRC validation failed for {reason} (received bytes {:02X?}, expected CRC to be 0, calculated {:02X})",
                    received_bytes, calculated_crc
                )
            }
            Error::I2c(e) => write!(f, "Received I2C error: {:?}", e),
        }
    }
}

impl<I> core::error::Error for Error<I> where I: embedded_hal::i2c::Error {}
