use crate::error::Result;
use crate::ui::MyPlatform;
use display_interface_spi::SPIInterface;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{Gpio16, Gpio17};
use esp_idf_hal::units::FromValueType;
use esp_idf_hal::{
    prelude::Peripherals,
    spi::{config::Config, SpiDeviceDriver, SpiDriverConfig},
};
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiDriver, SPI2};
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9341Rgb565;
use mipidsi::{Builder, Display};
use slint::platform::software_renderer::MinimalSoftwareWindow;
slint::include_modules!();
use crate::ui::DisplayWrapper;
use mipidsi::options::{ColorOrder, Orientation};
use static_cell::StaticCell;

// Statically allocate memory for a `u32`.
static BUFFER: StaticCell<[u8; 512]> = StaticCell::new();

pub type FrontDisplayDriver<'d> = mipidsi::Display<
    SpiInterface<'d, SpiDeviceDriver<'d, SpiDriver<'d>>, PinDriver<'d, Gpio17, Output>>,
    ILI9341Rgb565,
    PinDriver<'d, Gpio16, Output>,
>;

pub fn init_lcd<'d>() -> Result<FrontDisplayDriver<'d>> {
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
    // let a = [0_u8; 512];
    let buffer = BUFFER.init([0; 512]);
    let slice: &'static mut [u8] = buffer;
    let dc = PinDriver::output(peripherals.pins.gpio17)?;
    let rst = PinDriver::output(peripherals.pins.gpio16)?;
    // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, slice);
    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();
    // Flip display content horizontally.
    let flipped = Orientation::new().flip_horizontal();
    let _ = display.set_orientation(flipped).unwrap();
    log::info!("Initialize the SPI il9431");
    Ok(display)
}

pub fn init_window() {
    let window = MinimalSoftwareWindow::new(Default::default());
    slint::platform::set_platform(Box::new(MyPlatform {
        window: window.clone(),
    }));
    // Make sure the window covers our entire screen.
    window.set_size(slint::PhysicalSize::new(240, 320));
    let app_window = AppWindow::new().unwrap();
    let mut line_buffer = [slint::platform::software_renderer::Rgb565Pixel(0); 320];
    let mut display = init_lcd().unwrap();
    // let mut display = hal::Display::new(/*...*/);
    loop {
        window.draw_if_needed(|renderer| {
            renderer.render_by_line(DisplayWrapper {
                display: &mut display,
                line_buffer: &mut line_buffer,
            });
        });
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
