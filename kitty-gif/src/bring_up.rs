use crate::error::Result;
use esp_idf_hal::delay::Delay;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::units::FromValueType;
use esp_idf_hal::{
    prelude::Peripherals,
    spi::{config::Config, SpiDeviceDriver, SpiDriverConfig},
};
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9341Rgb565;
use mipidsi::Builder;

/* -------------------------- */

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyle, Triangle},
};

// use thiserror::Error;
pub fn init_lcd() -> Result<()> {
    // let sck =
    // let display_interface = SpiInterface::new(, , buffer)
    let peripherals = Peripherals::take()?;
    // let system = peripherals.SYSTEM.split();
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio13;
    let mosi = peripherals.pins.gpio5;
    let miso = peripherals.pins.gpio12;
    let cs = peripherals.pins.gpio2;
    let config = Config::new().baudrate(26.MHz().into());
    // Define the delay struct, needed for the display driver
    let mut delay = Ets;
    // Define the Data/Command select pin as a digital output
    // let dc =
    // let spi_device = SPI
    // let display_interface = SpiInterface::new(spi, dc, buffer)
    let spi_device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        mosi,
        Some(miso),
        Some(cs),
        &SpiDriverConfig::new(),
        &config,
    )?;
    let mut buffer = [0_u8; 512];
    let dc = PinDriver::output(peripherals.pins.gpio17)?;
    let rst = PinDriver::output(peripherals.pins.gpio16)?;
    // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, &mut buffer);
    // let display_interface_spi = SpiInterface::new(spi, dc, buffer)
    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();
    draw_smiley(&mut display);
    Ok(())
}

fn draw_smiley<T: DrawTarget<Color = Rgb565>>(display: &mut T) {
    // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 100), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display);

    // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 200), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display);

    // Draw an upside down red triangle to represent a smiling mouth
    Triangle::new(
        Point::new(130, 140),
        Point::new(130, 200),
        Point::new(160, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(display);
    // Cover the top part of the mouth with a black triangle so it looks closed instead of open
    Triangle::new(
        Point::new(130, 150),
        Point::new(130, 190),
        Point::new(150, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    .draw(display);
    // Ok(())
}
