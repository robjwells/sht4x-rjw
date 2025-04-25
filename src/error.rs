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
