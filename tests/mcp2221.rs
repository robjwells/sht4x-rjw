use embedded_hal_mock::eh1::delay::StdSleep;
use sht40_rjw::blocking::SHT40;

#[test]
fn mcp() -> anyhow::Result<()> {
    let mut dev = mcp2221::Handle::open_first(&Default::default())?;
    dev.check_bus()?;
    let di = dev.get_device_info()?;
    println!("{}", di);

    let mut sensor = SHT40::new(dev, Default::default());

    println!("Serial number: {}", sensor.serial_number()?);

    let measurement = sensor.measure(StdSleep::new())?;
    println!(
        "{}°C\t{} %RH",
        measurement.temperature, measurement.humidity
    );

    Ok(())
}
