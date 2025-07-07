use std::cell::RefCell;
use std::fmt::format;
use std::rc::Rc;
use std::sync::Mutex;
use std::thread;
use crate::ui::MyPlatform;
use esp_idf_hal::delay::{Delay, Ets, FreeRtos};
use esp_idf_hal::gpio::{Gpio39, Gpio41};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::units::FromValueType;
use esp_idf_hal::{
    prelude::Peripherals,
    spi::{config::Config, SpiDeviceDriver, SpiDriverConfig},
};
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::spi::SpiDriver;
use mipidsi::interface::SpiInterface;
use mipidsi::models::{ST7789};
use mipidsi::Builder;
use slint::platform::software_renderer::{MinimalSoftwareWindow};
use slint::platform::{PointerEventButton, WindowEvent};
slint::include_modules!();
use crate::ui::DisplayWrapper;
use mipidsi::options::{ColorInversion, ColorOrder};
use slint::{ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use static_cell::StaticCell;
use std::time::{Duration, Instant};
////////
use cst816s::Cst328;
use esp_idf_hal::{i2c};
// WiFi
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_svc::wifi::*;
use sha256::{digest, try_digest};
use qrcodegen::Mask;
use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;
use qrcodegen::QrSegment;
use qrcodegen::Version;
use crate::FrameData;
use crate::RgbaFrameData;
// use crate::cat_dance_frames::CAT_DANCE_FRAMES;
//use crate::cat_eating_frames::CAT_EATING_FRAMES;
use crate::cat_eating_rgba8::CAT_EATING_RGBA8_FRAMES;
// use crate::cat_playing_frames::CAT_PLAYING_FRAMES;

static BUFFER: StaticCell<[u8; 512]> = StaticCell::new();
const SSID: &str = "TUE";
const PASSWORD: &str = "Gemtek@123";

const DEVICE_ID: &str = "58db0095571ee686bdc5cfa3a7368eb9";
const SEERET_KEY: &str = "0bffd683ac83273d91c1d82d89f9d786";
// Frame data structure
// Animation controller
struct AnimationController {
    current_frame: usize,
    last_frame_time: Instant,
    is_playing: bool,
    frames: &'static [RgbaFrameData],
}



impl AnimationController {
    fn new(frames: &'static [RgbaFrameData]) -> Self {
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
#[warn(dead_code)]
    fn stop(&mut self) {
        self.is_playing = false;
    }

    fn update(&mut self) -> Option<&RgbaFrameData> {
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
fn create_slint_image_from_frame(frame: &RgbaFrameData) -> Image {
    
    let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        &frame.data,
        frame.width as u32,
        frame.height as u32,
    );

    Image::from_rgba8(buffer)
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration).unwrap();

    wifi.start().unwrap();
    log::info!("Wifi started");

    // wifi.connect().unwrap();
    // log::info!("Wifi connected");
    // wifi.wait_netif_up().unwrap();
    // log::info!("Wifi netif up");

}
fn print_qr(qr: &QrCode) {
	let border: i32 = 4;
	for y in -border .. qr.size() + border {
		for x in -border .. qr.size() + border {
			let c: char = if qr.get_module(x, y) { '█' } else { ' ' };
			print!("{0}{0}", c);
		}
		println!();
	}
	println!();
}
fn to_svg_string(qr: &QrCode, border: i32) -> String {
	assert!(border >= 0, "Border must be non-negative");
	let mut result = String::new();
	result += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
	result += "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n";
	let dimension = qr.size().checked_add(border.checked_mul(2).unwrap()).unwrap();
	result += &format!(
		"<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n", dimension);
	result += "\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n";
	result += "\t<path d=\"";
	for y in 0 .. qr.size() {
		for x in 0 .. qr.size() {
			if qr.get_module(x, y) {
				if x != 0 || y != 0 {
					result += " ";
				}
				result += &format!("M{},{}h1v1h-1z", x + border, y + border);
			}
		}
	}
	result += "\" fill=\"#000000\"/>\n";
	result += "</svg>\n";
	result
}

pub fn init_window() {
    let peripherals = Peripherals::take().unwrap();
    let window = MinimalSoftwareWindow::new(slint::platform::software_renderer::RepaintBufferType::ReusedBuffer);
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    slint::platform::set_platform(Box::new(MyPlatform {
        window: window.clone(),
    }))
    .unwrap();
    // Make sure the window covers our entire screen.
    window.set_size(slint::PhysicalSize::new(240, 320));

    let app = AppWindow::new().unwrap();


    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop,
    ).unwrap();

    connect_wifi(&mut wifi);
    // let ip_info = wifi.wifi().sta_netif().get_ip_info().unwrap();
    // log::info!("Wifi DHCP info: {ip_info:?}");


    let mut pwr_en = PinDriver::output(peripherals.pins.gpio7).unwrap();
    pwr_en.set_high().unwrap();
    
//////////////////
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio40; //MTDO
    let mosi = peripherals.pins.gpio45; //
    let miso = peripherals.pins.gpio46;
    let cs = peripherals.pins.gpio42; //MTDI
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
    ).unwrap();
    let buffer = BUFFER.init([0; 512]);
    let slice: &'static mut [u8] = buffer;
    let dc = PinDriver::output(peripherals.pins.gpio41).unwrap(); //MTDI
    let rst = PinDriver::output(peripherals.pins.gpio39).unwrap(); //MTCK
    // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, slice);
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .color_order(ColorOrder::Rgb).invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();
    //log::info!("Initialize the ST7798");

    /////////////////////////////////// Touch peripheral init
    let mut touch_int = PinDriver::input(peripherals.pins.gpio4).unwrap();
    let mut touch_rst = PinDriver::output(peripherals.pins.gpio2).unwrap();
    let touch_sda_pin = peripherals.pins.gpio1;
    let touch_scl_pin = peripherals.pins.gpio3;
    let touch_i2c_config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = peripherals.i2c0;
    let mut touch_i2c = I2cDriver::new(i2c, touch_sda_pin, touch_scl_pin, &touch_i2c_config).unwrap();
    let mut delay_source:Delay = Default::default();
    let bus: &'static shared_bus::BusManager<Mutex<i2c::I2cDriver<'_>>> = shared_bus::new_std!(i2c::I2cDriver = touch_i2c).unwrap();

    let mut touch = Cst328::new(bus.acquire_i2c(), delay_source);
    touch.reset(&mut touch_rst, &mut delay_source).unwrap();

    let mut line_buffer = [slint::platform::software_renderer::Rgb565Pixel(0); 240];

    //Create animation controller with pre-processed frames
    let controller = Rc::new(RefCell::new(AnimationController::new(&CAT_EATING_RGBA8_FRAMES)));

    {
        let mut ctrl = controller.borrow_mut();
        ctrl.start();
    }

    // Animation timer
    let controller_clone = controller.clone();
    let app_weak = app.as_weak();
    let timer = slint::Timer::default();
    
    // timer.start(
    //     slint::TimerMode::Repeated,
    //     Duration::from_millis(16),
    //     move || {
    //         let app = match app_weak.upgrade() {
    //             Some(app) => {
    //                 app
    //             }
    //             None => return,
    //         };

    //         let mut ctrl = controller_clone.borrow_mut();
    //         if let Some(frame) = ctrl.update() {
    //             let image = create_slint_image_from_frame(frame);
    //             app.set_current_frame(image);
    //             //log::info!("Set frame");
    //         }
    //     },
    // );
    //timer.stop();

    let mut bl = PinDriver::output(peripherals.pins.gpio5).unwrap();
    let mut last_touch = None;
    // topbard.show().unwrap();
    let weak = app.as_weak();
    app.global::<VirtualKeyboardHandler>().on_key_pressed({
        let weak = weak.clone();
        move |key| {
            let copy = key.clone();
            weak.unwrap()
                .window()
                .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: key.clone() });
            weak.unwrap()
                .window()
                .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: key });
            //log::info!("Key pressed {}", copy);
        }
    });
    let list_ssid = wifi.scan().unwrap();
    let ssids: Vec<SharedString> = list_ssid
    .iter()
    .map(|ap| SharedString::from(ap.ssid.as_str()))
    .collect();
    log::info!("{:?}", ssids);
    // let list = ModelRc::from(Rc::new(VecModel::from(ssids)));
    // app.set_scanned_ssid(list);
    wifi.connect().unwrap();
    wifi.wait_netif_up().unwrap();
    let mac = wifi.wifi().sta_netif().get_mac().unwrap();
    //let qr = generate_qr_code(format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]));
    //app.set_qr_image(qr);
    loop {
        bl.set_high().unwrap();
        slint::platform::update_timers_and_animations();
        if wifi.wifi().is_connected().unwrap() {
           if let Configuration::Client(ssid_connected) = wifi.wifi().get_configuration().unwrap() {
                let ip_info = wifi.wifi().sta_netif().get_mac().unwrap();
                app.set_connected_status(WiFiConnectParameters{
                connected: true,
                ssid : ssid_connected.ssid.to_string().into(),
                mac:     SharedString::from(format!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    ip_info[0], ip_info[1], ip_info[2], ip_info[3], ip_info[4], ip_info[5]
                ))
               });
           }
        }

        // match app.get_screen_state() {
        //     ScreenState::Game => {
        //         let mut ctrl = controller.borrow_mut();
        //         // ctrl.start();
        //         timer.restart();
        //     }
        //     _ =>  {
        //         let mut ctrl = controller.borrow_mut();
        //         // ctrl.stop();
        //         timer.stop();
        //     }
        // };

        match touch.get_xy_data() {
            Ok(Some(event_touch)) => {
                let pos = slint::PhysicalPosition::new(event_touch.x_cord as i32, event_touch.y_cord as i32)
                    .to_logical(window.scale_factor());
                let event = if let Some(previous_pos) = last_touch.replace(pos) {
                    // If the position changed, send a PointerMoved event.
                    if previous_pos != pos {
                        WindowEvent::PointerMoved { position: pos }
                    } else {
                        // If the position is unchanged, skip event generation.
                        continue;
                    }
                } else {
                    // No previous touch recorded, generate a PointerPressed event.
                    WindowEvent::PointerPressed {
                        position: pos,
                        button: PointerEventButton::Left,
                    }
                };
                // Dispatch the event to Slint.
                log::info!("{:?}", event);
                window.try_dispatch_event(event).unwrap();
            },
            Ok(None) => {
                if let Some(pos) = last_touch.take() {
                    window.try_dispatch_event(WindowEvent::PointerReleased {
                        position: pos,
                        button: PointerEventButton::Left,
                    }).unwrap();
                    window.try_dispatch_event(WindowEvent::PointerExited).unwrap();
                }
            },
            Err(_) => {
                todo!("Implement errror handle if have to");
            }
        }

        window.draw_if_needed(|renderer| {
            renderer.render_by_line(DisplayWrapper {
                display: &mut display,
                line_buffer: &mut line_buffer,
            });
        });
        //log::info!("hehe");
    }
}
