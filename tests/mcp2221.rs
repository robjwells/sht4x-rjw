use embedded_hal_mock::eh1::delay::StdSleep;

#[test]
fn mcp() -> anyhow::Result<()> {
    let mut dev = mcp2221::Handle::open_first(&Default::default())?;
    dev.check_bus()?;
    let di = dev.get_device_info()?;
    println!("{}", di);

    let config = sht40_rjw::Config {
        validate_crc: false,
        ..sht40_rjw::Config::default()
    };
    let mut sensor = sht40_rjw::SHT40::new(dev, config);

    println!("Serial number: {}", sensor.serial_number());

    let measurement = sensor.measure(StdSleep::new());
    println!(
        "{}°C\t{} %RH",
        measurement.temperature, measurement.humidity
    );

    Ok(())
}
