# SHT4x embedded-hal driver

An [`embedded-hal`], [`no_std`] driver for the [Sensirion SHT4x series][sht4x]
of temperature and humidity sensors.

This crate provides both blocking and async drivers for the SHT4x, with CRC
validation of all read data, and easy access to measurements in celsius (°C),
fahrenheit (°F), and percent relative humidity (%RH), all as `f32`.

The SHT4x communicates over I2C, exposing temperature and humidity measurement
at three precision levels, as well as high-precision measurement after
pre-heating. This crate supports all functions of the SHT4x as listed in
section 4.5 of the [datasheet].

## Example usage

```rust
# use embedded_hal_mock::eh1::i2c::{Mock, Transaction};
# use sht40_rjw::blocking::SHT40;
# fn main() -> anyhow::Result<()> {
#   let mut delay = embedded_hal_mock::eh1::delay::NoopDelay::new();
#   let expectations = [
#     // Request the sensor serial number.
#     Transaction::write(0x44, vec![0x89]),
#     // Receive a fake sensor serial number.
#     Transaction::read(0x44, vec![0x01, 0x02, 0x17, 0x3, 0x4, 0x68]),
#     // Request a high-precision read.
#     Transaction::write(0x44, vec![0xFD]),
#     // Receive a fake temp & humidity measurement.
#     Transaction::read(0x44, vec![0x12, 0x34, 0x37, 0x56, 0x78, 0x7D])
#   ];
#   let i2c = Mock::new(&expectations);
let mut sensor = SHT40::new(i2c, Default::default());
let serial_number = sensor.serial_number()?;
let measurement = sensor.measure(&mut delay)?;

defmt::info!(
    "SHT40 sensor with serial {}, currently: {}°C, {}%RH",
    serial_number,
    measurement.celsius(),
    measurement.humidity()
);
#   sensor.destroy().done();    // Call done on the I2C mock.
#   Ok(())
# }
```

## Blocking and async

A blocking driver, for use with [`embedded-hal`], is available as
[`blocking::SHT40`]. This is included as a default feature.

An async driver, for use with [`embedded-hal-async`], is available as
[`asynch::SHT40`] (note the extra "h" because of keyword clash). This
is available after enabling the `async` feature.

If you are using the async driver, you can remove the blocking driver by
passing `--no-default-features` to `cargo add`, or disable the default features
in your `Cargo.toml`, like so:

```toml
[dependencies]
sht4x_rjw = { version = "0.1.0", default-features = false, features = ["async"] }
```

## Sensor configuration and delay mode

When you create the sensor, it takes a configuration struct, [`common::Config`].
Here are its default values:

```rust
# use sht40_rjw::common::{Config, ReadingMode, DelayMode};
Config {
    reading_mode: ReadingMode::HighPrecision,
    delay_mode: DelayMode::Typical,
};
```

This sets the default measurement mode (high precision, without heating)
and sets the delay before reading the measurement to the "typical" values
listed in the datasheet.

**NOTE** that the SHT4x does not respond to reads that occur before the
requested measurement is ready, which will cause the measurement method to
fail with an [`i2c::Error`]. If you find that you are encountering NACK errors
when attempting to measure, try switching to [`DelayMode::Maximum`].
Please see section 3.2 of the [datasheet] for timings.

## Sensor I2C address

The sensor struct uses a default I2C address of `0x44`, as this appears to be
the most common. However, sensors with part numbers including `-B` and `-C`
have I2C addresses of `0x45` and `0x46`, respectively. See section 9 of the
[datasheet].

Should you need to use an address other than `0x44`, instantiate the struct
as normal and write to its `address` field, as so:

```rust
# use sht40_rjw::blocking::SHT40;
# let i2c = embedded_hal_mock::eh1::i2c::Mock::new(&[]);
let mut sensor = SHT40::new(i2c, Default::default());
sensor.address = 0x46;
# sensor.destroy().done()
```

10-bit I2C addresses are not supported.

## I have an SHT40, what is this SHT4x?

SHT4x refers to the whole series of sensors, all of which work in the same
way over I2C. So this crate _should_ work with whichever variant you have,
though currently it has only been tested with the SHT40.

The SHT40 is the least accurate of the series, the SHT45 the most accurate, and
the SHT41 between the two. The SHT43 meanwhile is subject to particular
calibration and accreditation to an ISO standard. See section 2 of the
[datasheet] for details.

There is also an SHT4x**A**, which is intended for automotive usage. This
appears to operate in the same way, with a caveat that the listed heater power
is reduced at lower supply voltages. Please see the [SHT4x**A**
datasheet][sht4xa].

## Similar crates

You may prefer to use the following drivers for the SHT4x:

- [`sht4x`](https://github.com/sirhcel/sht4x)
- [`sensor-temp-humidity-sht40`](https://github.com/lc525/sensor-temp-humidity-sht40-rs)

## Licence

The `sht4x_rjw` crate is licensed under the [Apache License, Version 2.0], or
the [MIT License], at your option.

[`embedded-hal`]: https://docs.rs/embedded-hal/latest/embedded_hal/
[`no_std`]: https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute
[sht4x]: https://developer.sensirion.com/product-support/sht4x-humidity-and-temperature-sensor
[datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
[sht4xa]: https://sensirion.com/media/documents/C43ACD8C/67BD838A/HT_DS_Datasheet_SHT4xA_3.pdf
[`embedded-hal-async`]: https://docs.rs/embedded-hal-async/latest/embedded_hal_async/
[`i2c::Error`]: embedded_hal::i2c::Error
[`DelayMode::Maximum`]: crate::common::DelayMode::Maximum
[Apache License, Version 2.0]: https://opensource.org/license/apache-2-0
[MIT License]: https://opensource.org/license/mit
