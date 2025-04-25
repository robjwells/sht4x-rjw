#[derive(Debug)]
pub enum Error<I2cError>
where
    I2cError: embedded_hal::i2c::Error,
{
    CrcValidationFailed(CrcFailureReason),
    I2c(I2cError),
}

pub enum CrcFailureReason {
    SerialNumberFirstPair,
    SerialNumberSecondPair,
    TemperatureBytes,
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

impl core::fmt::Display for CrcFailureReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <CrcFailureReason as core::fmt::Debug>::fmt(self, f)
    }
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
