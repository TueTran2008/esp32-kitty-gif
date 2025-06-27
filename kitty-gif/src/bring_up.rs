use std::path::Path;
use std::slice::SliceIndex;
use std::thread;

use crate::error::Result;
use crate::ui::MyPlatform;
use esp_idf_hal::delay::{self, Ets, FreeRtos};
use esp_idf_hal::gpio::{Gpio16, Gpio39, Gpio41, Gpio5};
use esp_idf_hal::sys::xPortGetFreeHeapSize;
use esp_idf_hal::units::FromValueType;
use esp_idf_hal::{
    prelude::Peripherals,
    spi::{config::Config, SpiDeviceDriver, SpiDriverConfig},
};
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::spi::SpiDriver;
use image::{ImageBuffer, Rgba};
use mipidsi::interface::SpiInterface;
use mipidsi::models::{ILI9341Rgb565, ST7789};
use mipidsi::Builder;
use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};
slint::include_modules!();
use crate::ui::DisplayWrapper;
use mipidsi::options::{ColorInversion, ColorOrder, Orientation};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
// use slint::SharedPixelBuffer::R
// use crate::FrameData;
use static_cell::StaticCell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use esp_idf_svc::hal::sys;
// include!("frame_0.rs");
// include!(concat!(env!(""), "/frame.rs"));
// Statically allocate memory for a `u32`.
// Include the generated frame data
include!("generated_frames.rs");
static BUFFER: StaticCell<[u8; 512]> = StaticCell::new();

pub type FrontDisplayDriver<'d> = mipidsi::Display<
    SpiInterface<'d, SpiDeviceDriver<'d, SpiDriver<'d>>, PinDriver<'d, Gpio41, Output>>,
    ST7789,
    PinDriver<'d, Gpio39, Output>,
>;

pub fn init_lcd<'d>() -> Result<(FrontDisplayDriver<'d>, PinDriver<'d, Gpio5, Output>)> {
    let peripherals = Peripherals::take()?;
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio40;
    let mosi = peripherals.pins.gpio45;
    let miso = peripherals.pins.gpio46;
    let cs = peripherals.pins.gpio42;
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
    let dc = PinDriver::output(peripherals.pins.gpio41)?;
    let rst = PinDriver::output(peripherals.pins.gpio39)?;
    // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, slice);
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .color_order(ColorOrder::Rgb).invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();
    log::info!("Initialize the ST7798");
    let bl = PinDriver::output(peripherals.pins.gpio5)?;
    Ok((display, bl))
}
// Frame data structure
// Animation controller
struct AnimationController {
    current_frame: usize,
    last_frame_time: Instant,
    is_playing: bool,
    frames: &'static [FrameData],
}

impl AnimationController {
    fn new(frames: &'static [FrameData]) -> Self {
        Self {
            current_frame: 0,
            last_frame_time: Instant::now(),
            is_playing: false,
            frames,
        }
    }

    fn start(&mut self) {
        self.is_playing = true;
        self.last_frame_time = Instant::now();
        //println!("Start animation controller");
    }

    fn stop(&mut self) {
        self.is_playing = false;
    }

    fn update(&mut self) -> Option<&FrameData> {
        if !self.is_playing || self.frames.is_empty() {
            return None;
        }

        let current_time = Instant::now();
        let current_frame_data = &self.frames[self.current_frame];

        if current_time.duration_since(self.last_frame_time)
            >= Duration::from_millis(current_frame_data.delay_ms as u64)
        {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_frame_time = current_time;
        }

        Some(&self.frames[self.current_frame])
    }

    fn get_current_frame_index(&self) -> usize {
        self.current_frame
    }

    fn get_total_frames(&self) -> usize {
        self.frames.len()
    }
}

// Convert RGB565 to RGBA8 for Slint
fn rgb565_to_rgba8(rgb565_data: &[u16], width: u16, height: u16) -> Vec<u8> {
    let mut rgba_data = Vec::with_capacity((width as usize) * (height as usize) * 4);

    for &pixel in rgb565_data {
        // Extract RGB components from RGB565
        let r = ((pixel >> 11) & 0x1F) as u8;
        let g = ((pixel >> 5) & 0x3F) as u8;
        let b = (pixel & 0x1F) as u8;

        // Convert to 8-bit values
        let r8 = (r << 3) | (r >> 2);
        let g8 = (g << 2) | (g >> 4);
        let b8 = (b << 3) | (b >> 2);

        rgba_data.extend_from_slice(&[r8, g8, b8, 255]); // Alpha = 255 (opaque)
    }

    rgba_data
}

// Create Slint image from frame data
fn create_slint_image_from_frame(frame: &FrameData) -> Image {
    let rgba_data = rgb565_to_rgba8(frame.data, frame.width, frame.height);

    let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        &rgba_data,
        frame.width as u32,
        frame.height as u32,
    );

    Image::from_rgba8(buffer)
}

pub fn init_window() {
    let window = MinimalSoftwareWindow::new(slint::platform::software_renderer::RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(MyPlatform {
        window: window.clone(),
    }))
    .unwrap();
    // Make sure the window covers our entire screen.
    window.set_size(slint::PhysicalSize::new(240, 320));
    let app = AppWindow::new().unwrap();

    let mut line_buffer = [slint::platform::software_renderer::Rgb565Pixel(0); 240];
    let mut display = init_lcd().unwrap();
    // Create animation controller with pre-processed frames
    let controller = Rc::new(RefCell::new(AnimationController::new(&ANIMATION_FRAMES)));

    // Set initial state
    app.set_total_frames(ANIMATION_FRAMES.len() as i32);
    app.set_status_text("Animation loaded".into());

    {
        let mut ctrl = controller.borrow_mut();
        ctrl.start();
    }

    // Animation timer
    let controller_clone = controller.clone();
    let app_weak = app.as_weak();
    let timer = slint::Timer::default();
    let _ = display.1.set_high();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let app = match app_weak.upgrade() {
                Some(app) => {
                    //inf("Something is in my mind");
                    app
                }

                None => return,
            };

            let mut ctrl = controller_clone.borrow_mut();
            if let Some(frame) = ctrl.update() {
                let image = create_slint_image_from_frame(frame);
                app.set_current_frame(image);
            }
        },
    );
    let total_memory = ANIMATION_FRAMES.len() * 160 * 160 * 2; // RGB565 = 2 bytes per pixel
    println!("Total animation memory usage: {} KB", total_memory / 1024);
    loop {
        slint::platform::update_timers_and_animations();
        window.draw_if_needed(|renderer| {
            renderer.render_by_line(DisplayWrapper {
                display: &mut display.0,
                line_buffer: &mut line_buffer,
            });
        });
        FreeRtos::delay_ms(1);
    }
}
