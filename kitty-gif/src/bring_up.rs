use crate::error::Result;
use crate::ui::MyPlatform;
// use esp_idf_hal::delay::Delay;
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
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::ComponentHandle;
slint::include_modules!();

pub fn init_lcd() -> Result<()> {
    let peripherals = Peripherals::take()?;
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio13;
    let mosi = peripherals.pins.gpio5;
    let miso = peripherals.pins.gpio12;
    let cs = peripherals.pins.gpio2;
    let config = Config::new().baudrate(26.MHz().into());
    // Define the delay struct, needed for the display driver
    let mut delay = Ets;
    // Define the Data/Command select pin as a digital output
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
    let mut _display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();
    Ok(())
}

pub fn init_window() {
    let window = MinimalSoftwareWindow::new(Default::default());
    slint::platform::set_platform(Box::new(MyPlatform {
        window: window.clone(),
    }));
    // Make sure the window covers our entire screen.
    window.set_size(slint::PhysicalSize::new(320, 240));
    let app_window = AppWindow::new().unwrap();
    loop {
        slint::platform::update_timers_and_animations();
    }
    // ui.on_request_increase_value({
    //     let ui_handle = ui.as_weak();
    //     move || {
    //         let ui = ui_handle.unwrap();
    //         ui.set_counter(ui.get_counter() + 1);
    //     }
    // });
    //
    // ui.run()?;
}
