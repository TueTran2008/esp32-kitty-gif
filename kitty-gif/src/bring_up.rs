use std::cell::RefCell;
use std::fmt::format;
use std::rc::Rc;
use std::sync::Mutex;
use std::thread;
use crate::ui::MyPlatform;
use esp_idf_hal::delay::{Delay, Ets, FreeRtos};
use esp_idf_hal::gpio::{Gpio39, Gpio41};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::sys::{esp_get_free_heap_size, xPortGetFreeHeapSize};
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
use crate::RgbaFrameData;
use crate::ota::*;
use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;
use sha256::{digest};
use sha2::{Sha256, Digest};
use image::{EncodableLayout, ImageBuffer, Rgb, Rgba};
use hmac::{Hmac, Mac};

use crate::chirplunk_3_eat_rgba8::{self, CHIRPLUNK_3_EAT_RGBA8_FRAMES};
use crate::chirplunk_3_jump_rgba8::CHIRPLUNK_3_JUMP_RGBA8_FRAMES;
use crate::chirplunk_3_sit_rgba8::{self, CHIRPLUNK_3_SIT_RGBA8_FRAMES};
use crate::chirplunk_3_sleep_rgba8::{self, CHIRPLUNK_3_SLEEP_RGBA8_FRAMES};

use crate::lunafluff_4_eat_rgba8::*;
use crate::lunafluff_4_jump_rgba8::*;
use crate::lunafluff_4_sit_rgba8::*;
use crate::lunafluff_4_sleep_rgba8::*;

use crate::mechapup_3_eat_rgba8::*;
use crate::mechapup_3_jump_rgba8::*;
use crate::mechapup_3_sit_rgba8::*;
use crate::mechapup_3_sleep_rgba8::*;

const DEVICE_ID: &str = "58db0095571ee686bdc5cfa3a7368eb9";
const SEERET_KEY: &str = "0bffd683ac83273d91c1d82d89f9d786";
// use crate::cat_playing_frames::CAT_PLAYING_FRAMES;

static BUFFER: StaticCell<[u8; 512]> = StaticCell::new();
const SSID: &str = "TUE";
const PASSWORD: &str = "Gemtek@123";

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
    /// Switch to new GIF frames
    fn switch_gif(&mut self, new_frames: &'static [RgbaFrameData]) {
        self.frames = new_frames;
        self.current_frame = 0;
        self.last_frame_time = Instant::now();
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

fn setup_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) {
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
}

// Convert QR code to Slint image
pub fn qr_to_slint_image(qr: &QrCode, scale: u32, border: u32) -> Image {
    let qr_size = qr.size() as u32;
    let img_size = (qr_size + 2 * border) * scale;

    // Create a vector to hold pixel data (RGBA8)
    let mut pixels = vec![255u8; (img_size * img_size * 4) as usize]; // default to white

    for y in 0..qr_size {
        for x in 0..qr_size {
            if qr.get_module(x as i32, y as i32) {
                let px = (x + border) * scale;
                let py = (y + border) * scale;

                for dy in 0..scale {
                    for dx in 0..scale {
                        let idx = (((py + dy) * img_size + (px + dx)) * 4) as usize;
                        pixels[idx..idx + 4].copy_from_slice(&[0, 0, 0, 255]); // black pixel
                    }
                }
            }
        }
    }

    // Create SharedPixelBuffer from raw pixel data
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(pixels.as_bytes(), img_size, img_size);

    Image::from_rgba8(buffer)
}

fn qr_convert_to_rgba8(monochrome: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for &pixel in monochrome {
        // Assume 0 = black, anything else = white
        let (r, g, b, a) = if pixel == 0 {
            (0, 0, 0, 255)     // Black
        } else {
            (255, 255, 255, 255) // White
        };

        rgba.extend_from_slice(&[r, g, b, a]);
    }

    rgba
}
fn hmac_sha256_hex_short(key: &str, message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");

    mac.update(message.as_bytes());

    let result = mac.finalize().into_bytes();
    let full_hex = format!("{:x}", result);

    let short = if full_hex.len() > 20 {
        let start = &full_hex[..10];
        let end = &full_hex[full_hex.len() - 10..];
        format!("{}...{}", start, end)
    } else {
        full_hex
    };

    short
}
fn generate_qr_code(mac: &[u8;6]) -> Image {
    let mac_str = format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
    let input_sha = format!("{}-{}-{}", DEVICE_ID, "1751897409", mac_str);

    let hash = hmac_sha256_hex_short(SEERET_KEY, &input_sha);


    let data_qr = format!("{}{}{}{}", DEVICE_ID, "1751897409", mac_str, hash);
    log::info!("{}", data_qr);
    //let result = qrcode_generator::to_png_to_vec(&[DEVICE_ID, "1751897409", &mac, &val].concat(), QrCodeEcc::Low, 512).unwrap();
    let result = QrCode::encode_text(&data_qr, QrCodeEcc::High).unwrap();
    qr_to_slint_image(&result, 2, 4)
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
    log::info!("before appwindowFree Heap: {} bytes", unsafe {esp_get_free_heap_size()});
    let app = AppWindow::new().unwrap();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop,
    ).unwrap();

    setup_wifi(&mut wifi);
    let wifi_sta_mac = wifi.wifi().get_mac(WifiDeviceId::Sta).unwrap();
    let qr_pic = generate_qr_code(&wifi_sta_mac);

    let mut pwr_en = PinDriver::output(peripherals.pins.gpio7).unwrap();
    pwr_en.set_high().unwrap();
    
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
    let mut board_ota = OtaUpdate::new();
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

    let mut line_buffer = [slint::platform::software_renderer::Rgb565Pixel(0); 320];

    //Create animation controller with pre-processed frames
    let controller = Rc::new(RefCell::new(AnimationController::new(&CHIRPLUNK_3_SIT_RGBA8_FRAMES)));

    {
        let mut ctrl = controller.borrow_mut();
        ctrl.start();
    }

    //Animation timer
    let controller_clone = controller.clone();
    let app_weak = app.as_weak();
    let timer = slint::Timer::default();
    
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let app = match app_weak.upgrade() {
                Some(app) => {
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
    timer.stop();

    let mut bl = PinDriver::output(peripherals.pins.gpio5).unwrap();
    let mut last_touch = None;
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
        }
    });
    // let ota_version = board_ota.version.unwrap();
    // app.global::<UpdateFwCallback>().on_get_version(||{

    // });
    app.global::<UpdateFwCallback>().on_exec_update(move ||{
        board_ota.firmware_update();
    });
    app.global::<AnimationSwitch>().on_animation_switch(move |action, animal| {
                log::info!("{:?} - {:?}", action, animal);
                let mut ctrl = controller.borrow_mut();
                match animal {
                    Animal::Chirplunk => {
                        match action {
                            GameState::Normal => {
                                ctrl.switch_gif(&CHIRPLUNK_3_SIT_RGBA8_FRAMES);
                            },
                            GameState::Eating => {
                                ctrl.switch_gif(&CHIRPLUNK_3_EAT_RGBA8_FRAMES);
                            },
                            GameState::Sleeping => {
                                ctrl.switch_gif(&CHIRPLUNK_3_SLEEP_RGBA8_FRAMES);
                            },
                            GameState::Playing => {
                                ctrl.switch_gif(&CHIRPLUNK_3_JUMP_RGBA8_FRAMES);
                            }
                        }
                    },
                    Animal::Mechapup => {
                        match action {
                            GameState::Normal => {
                                ctrl.switch_gif(&MECHAPUP_3_SIT_RGBA8_FRAMES);
                            },
                            GameState::Eating => {
                                ctrl.switch_gif(&MECHAPUP_3_EAT_RGBA8_FRAMES);
                            },
                            GameState::Sleeping => {
                                ctrl.switch_gif(&MECHAPUP_3_SLEEP_RGBA8_FRAMES);
                            },
                            GameState::Playing => {
                                ctrl.switch_gif(&MECHAPUP_3_JUMP_RGBA8_FRAMES);
                            }
                        }
                    },
                    Animal::LunaFluff => {
                        match action {
                            GameState::Normal => {
                                ctrl.switch_gif(&LUNAFLUFF_4_SIT_RGBA8_FRAMES);
                            },
                            GameState::Eating => {
                                ctrl.switch_gif(&LUNAFLUFF_4_EAT_RGBA8_FRAMES);
                            },
                            GameState::Sleeping => {
                                ctrl.switch_gif(&LUNAFLUFF_4_SLEEP_RGBA8_FRAMES);
                            },
                            GameState::Playing => {
                                ctrl.switch_gif(&LUNAFLUFF_4_JUMP_RGBA8_FRAMES);
                            }
                        }
                    }
                }
    });
    // let list_ssid = wifi.scan().unwrap();
    // let ssids: Vec<SharedString> = list_ssid
    // .iter()
    // .map(|ap| SharedString::from(ap.ssid.as_str()))
    // .collect();
    // log::info!("{:?}", ssids);
    // let list = ModelRc::from(Rc::new(VecModel::from(ssids)));
    // app.set_scanned_ssid(list);
    log::info!("before wifi SetQR: {} bytes", unsafe {esp_get_free_heap_size()});
    {
        app.set_qr_image(qr_pic);
        app.set_deviceID(SharedString::from(DEVICE_ID));
    }
    log::info!("before wifi Heap: {} bytes", unsafe {esp_get_free_heap_size()});
    wifi.connect().unwrap();
    wifi.wait_netif_up().unwrap();

    // let mac = wifi.wifi().sta_netif().get_mac().unwrap();
    //let qr = generate_qr_code(format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]));
    //app.set_qr_image(qr);
    log::info!("before http Heap: {} bytes", unsafe {esp_get_free_heap_size()});

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

        match app.get_screen_state() {
            ScreenState::Game => {
                //let mut ctrl = controller.borrow_mut();
                timer.restart();
            }
            _ =>  {
                //let mut ctrl = controller.borrow_mut();
                timer.stop();
            }
        };


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
        //Rendering 320x240 takes more than 200ms :(, which is suck
        
        window.draw_if_needed(|renderer| {
            //log::info!("Before render");
            renderer.render_by_line(DisplayWrapper {
                display: &mut display,
                line_buffer: &mut line_buffer,
            });
            //log::info!("After render");
        });
        
    }
}
