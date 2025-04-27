# SHT4x embedded-hal driver

An [`embedded-hal`], [`no_std`] driver for the [Sensirion SHT4x series][sht4x]
of I2C temperature and humidity sensors with **blocking** and **async**
support. The driver implements all features described in section 4.5 of the
[datasheet].

# Features

By default, this crate contains a blocking driver, [`blocking::SHT4x`].

Optional features include:

- **Async** support via [`embedded-hal-async`]. Use the `async` feature flag
  and the [`asynch::SHT4x`] driver struct. The blocking and async drivers are
  otherwise identical.
- **[`defmt`]** support through the `defmt` feature flag.
- **Fixed-point** conversions (instead of `f32` floating-point) through the
  `fixed` feature flag and the [`fixed`] crate.

You can remove the blocking driver by passing `--no-default-features` to
`cargo add`, or adding `default-features = false` to the dependency spec in
your `Cargo.toml`.

[`defmt`]: https://defmt.ferrous-systems.com/

# Example usage

```rust
# use embedded_hal_mock::eh1::i2c::{Mock, Transaction};
# use sht4x_rjw::blocking::SHT4x;
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
let mut sensor = SHT4x::new(i2c, Default::default());
let serial_number = sensor.serial_number()?;
let measurement = sensor.measure(&mut delay)?;

defmt::info!(
    "SHT4x sensor with serial {}, currently: {}°C, {}%RH",
    serial_number,
    measurement.celsius(),
    measurement.humidity()
);
#   sensor.destroy().done();    // Call done on the I2C mock.
#   Ok(())
# }
```

# Driver operation

Construct the driver by passing in an [I2C interface] and configuration struct
(used to set defaults for [`SHT4x::measure()`]). You can retrieve the I2C
interface with [`SHT4x::destroy()`].

You can choose the type of measurement conducted (varying in repeatability
and heater use) with [`ReadingMode`]. The default value of [`Config`] is set
for high-repeatability measurements.

When reading measurements from the sensor (or performing a soft reset), pass
in an implementation of [`embedded_hal::delay::DelayNs`]. (This is done to avoid
the driver having to take ownership of the delay struct, as it can be less easy
to share these than I2C interfaces.) The length of the delay is controlled by
[`DelayMode`].

Temperature and humidity measurements are provided through [`Measurement`],
which has methods for converting the raw two-byte sensor measurement into
recognisable units. The raw measurements can also be accessed, and the
conversion functions are available in the [`conversions`] module.

Data is read from the sensor as two groups of three bytes: two data bytes and
one CRC byte for error detection. These CRC bytes are always checked before the
data bytes are made available for conversion. Should an error be detected in
the data read from the sensor, the [`Error`] enum will contain the bytes in
question (both data bytes and the CRC byte read from the sensor).

[I2C interface]: embedded_hal::i2c::I2c
[`SHT4x::destroy()`]: crate::blocking::SHT4x::destroy()
[`SHT4x::measure()`]: crate::blocking::SHT4x::measure()
[`ReadingMode`]: crate::common::ReadingMode
[`Measurement`]: crate::common::Measurement
[`conversions`]: crate::conversions
[`Error`]: crate::error::Error
[`Config`]: crate::common::Config
[`DelayMode`]: crate::common::DelayMode

# Sensor I2C address

The sensor struct uses a default I2C address of `0x44`, as this appears to be
the most common. However, sensors with part numbers including `-B` and `-C`
have I2C addresses of `0x45` and `0x46`, respectively. See section 9 of the
[datasheet].

Should you need to use an address other than `0x44`, instantiate the struct
as normal and write to its `address` field, as so:

```rust
# use sht4x_rjw::blocking::SHT4x;
# let i2c = embedded_hal_mock::eh1::i2c::Mock::new(&[]);
let mut sensor = SHT4x::new(i2c, Default::default());
sensor.address = 0x46;
# sensor.destroy().done()
```

10-bit I2C addresses are not supported.

# Sensor variant support

There are (as of this writing) four parts in the SHT4x line: the [SHT40], [SHT41],
[SHT43] and [SHT45]. There is also the SHT4x**A** line of automotive parts. All of
these sensors share the same I2C interface, so this library _should_ work with
all of them, though at the moment it has only been tested with the SHT40.

**Note** that there appear to be some differences in the characteristics of the
automotive parts (slower timings, for instance) which are not accounted for at
present. If this affects you please open an issue.

# Similar crates

You may prefer to use the following drivers for the SHT4x:

- [`sht4x`](https://github.com/sirhcel/sht4x)
- [`sensor-temp-humidity-sht40`](https://github.com/lc525/sensor-temp-humidity-sht40-rs)

# Licence

The `sht4x_rjw` crate is licensed under the [Apache License, Version 2.0], or
the [MIT License], at your option.

[`embedded-hal`]: https://docs.rs/embedded-hal/latest/embedded_hal/
[`embedded-hal-async`]: https://docs.rs/embedded-hal-async/latest/embedded_hal_async/
[`no_std`]: https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute
[sht4x]: https://developer.sensirion.com/product-support/sht4x-humidity-and-temperature-sensor
[datasheet]: https://sensirion.com/media/documents/33FD6951/67EB9032/HT_DS_Datasheet_SHT4x_5.pdf
[SHT40]: https://sensirion.com/products/catalog/SHT40
[SHT41]: https://sensirion.com/products/catalog/SHT41
[SHT43]: https://sensirion.com/products/catalog/SHT43
[SHT45]: https://sensirion.com/products/catalog/SHT45
[Apache License, Version 2.0]: https://opensource.org/license/apache-2-0
[MIT License]: https://opensource.org/license/mit
