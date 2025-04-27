use embedded_hal_mock::eh1::delay::StdSleep;
use sht4x_rjw::blocking::SHT4x;

#[test]
fn mcp() -> anyhow::Result<()> {
    let mut dev = mcp2221::Handle::open_first(&Default::default())?;
    dev.check_bus()?;
    let di = dev.get_device_info()?;
    println!("{}", di);

    let mut sensor = SHT4x::new(dev, Default::default());

    println!("Serial number: {}", sensor.serial_number()?);

    let measurement = sensor.measure(StdSleep::new())?;
    println!(
        "{}°C\t{} %RH",
        measurement.celsius(), measurement.humidity()
    );

    Ok(())
}
