use std::cell::RefCell;
//use std::fmt::format;
use crate::ui::Esp32Platform;
use esp_idf_hal::delay::{Delay, Ets, FreeRtos};
use std::rc::Rc;
use std::sync::Mutex;
use std::thread;
//use esp_idf_hal::gpio::{Gpio39, Gpio41};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::sys::{esp_get_free_heap_size, EspError};
use esp_idf_hal::units::FromValueType;
use esp_idf_hal::{
    prelude::Peripherals,
    spi::{config::Config, SpiDeviceDriver, SpiDriverConfig},
};
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::spi::SpiDriver;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use mipidsi::Builder;
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::platform::{PointerEventButton, WindowEvent};
slint::include_modules!();
use crate::ui::DisplayWrapper;
use mipidsi::options::{ColorInversion, ColorOrder};
use slint::{
    ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel, SetPlatformError
};
use static_cell::StaticCell;
use std::time::{Duration, Instant};
////////
use cst816s::Cst328;
use esp_idf_hal::i2c;
// WiFi
use crate::ota::*;
use crate::RgbaFrameData;
use esp_idf_svc::wifi::*;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use hmac::{Hmac, Mac};
use image::EncodableLayout;
use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;
use sha2::Sha256;
use crate::gif::chirplunk_3_eat_rgba8::CHIRPLUNK_3_EAT_RGBA8_FRAMES;
use crate::gif::chirplunk_3_jump_rgba8::CHIRPLUNK_3_JUMP_RGBA8_FRAMES;
use crate::gif::chirplunk_3_sit_rgba8::CHIRPLUNK_3_SIT_RGBA8_FRAMES;
use crate::gif::chirplunk_3_sleep_rgba8::CHIRPLUNK_3_SLEEP_RGBA8_FRAMES;
use crate::gif::lunafluff_4_eat_rgba8::LUNAFLUFF_4_EAT_RGBA8_FRAMES;
use crate::gif::lunafluff_4_jump_rgba8::LUNAFLUFF_4_JUMP_RGBA8_FRAMES;
use crate::gif::lunafluff_4_sit_rgba8::LUNAFLUFF_4_SIT_RGBA8_FRAMES;
use crate::gif::lunafluff_4_sleep_rgba8::LUNAFLUFF_4_SLEEP_RGBA8_FRAMES;
use crate::gif::mechapup_3_eat_rgba8::MECHAPUP_3_EAT_RGBA8_FRAMES;
use crate::gif::mechapup_3_jump_rgba8::MECHAPUP_3_JUMP_RGBA8_FRAMES;
use crate::gif::mechapup_3_sit_rgba8::MECHAPUP_3_SIT_RGBA8_FRAMES;
use crate::gif::mechapup_3_sleep_rgba8::MECHAPUP_3_SLEEP_RGBA8_FRAMES;

use esp_idf_svc::http::client::EspHttpConnection;
use serde::{Deserialize, Serialize};

use embedded_svc::{
    http::{client::Client as HttpClient, Method},
    io::Write,
    utils::io,
    wifi::{AuthMethod, ClientConfiguration, Configuration},
};

const DEVICE_ID: &str = "58db0095571ee686bdc5cfa3a7368eb9";
const SEERET_KEY: &str = "0bffd683ac83273d91c1d82d89f9d786";
const DISPLAY_WIDTH: u32 = 240;
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

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct IpAPIResponse {
    query: String,
    status: String,
    country: String,
    #[serde(rename = "countryCode")]
    country_code: String,
    region: String,
    #[serde(rename = "regionName")]
    region_name: String,
    city: String,
    zip: String,
    lat: f32,
    lon: f32,
    timezone: String,
    isp: String,
    org: String,
    r#as: String,
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

/// Send an HTTP GET request.
fn get_request() -> Result<IpAPIResponse, ()> {
    // Prepare headers and URL
    let mut client = HttpClient::wrap(EspHttpConnection::new(&Default::default()).unwrap());
    let headers = [("accept", "text/plain")];
    let url = "http://ip-api.com/json/";

    // Send request
    //
    // Note: If you don't want to pass in any headers, you can also use `client.get(url, headers)`.
    let request = client.request(Method::Get, url, &headers).unwrap();
    log::info!("-> GET {url}");
    let mut response = request.submit().unwrap();

    // Process response
    let status = response.status();
    log::info!("<- {status}");
    let mut buf = [0u8; 1024];
    let bytes_read = io::try_read_full(&mut response, &mut buf)
        .map_err(|e| e.0)
        .unwrap();
    log::info!("Read {bytes_read} bytes");
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => {
            log::info!(
                "Response body (truncated to {} bytes): {:?}",
                buf.len(),
                body_string
            );
            let ip_api: IpAPIResponse = serde_json::from_str(body_string).unwrap();
            return Ok(ip_api);
        }
        Err(e) => {
            log::error!("Error decoding response body: {e}");
            return Err(());
        }
    };
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
    let buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(pixels.as_bytes(), img_size, img_size);

    Image::from_rgba8(buffer)
}

fn qr_convert_to_rgba8(monochrome: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for &pixel in monochrome {
        // Assume 0 = black, anything else = white
        let (r, g, b, a) = if pixel == 0 {
            (0, 0, 0, 255) // Black
        } else {
            (255, 255, 255, 255) // White
        };

        rgba.extend_from_slice(&[r, g, b, a]);
    }

    rgba
}
fn hmac_sha256_hex_short(key: &str, message: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");

    mac.update(message.as_bytes());

    let result = mac.finalize().into_bytes();
    let full_hex = format!("{:x}", result);

    let short = if full_hex.len() > 20 {
        let start = &full_hex[..10];
        let end = &full_hex[full_hex.len() - 10..];
        format!("{}{}", start, end)
    } else {
        full_hex
    };

    short
}
fn generate_qr_code(mac: &[u8; 6]) -> Image {
    let mac_str = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    let input_sha = format!("{}-{}-{}", DEVICE_ID, "1751897409", mac_str);

    let hash = hmac_sha256_hex_short(SEERET_KEY, &input_sha);

    let data_qr = format!("{}{}{}{}", DEVICE_ID, "1751897409", mac_str, hash);
    log::info!("{}", data_qr);
    //let result = qrcode_generator::to_png_to_vec(&[DEVICE_ID, "1751897409", &mac, &val].concat(), QrCodeEcc::Low, 512).unwrap();
    let result = QrCode::encode_text(&data_qr, QrCodeEcc::High).unwrap();
    qr_to_slint_image(&result, 2, 4)
}

fn short_device_id(id: &str) -> String {
    if id.len() >= 8 {
        format!("{}...{}", &id[..4], &id[id.len() - 4..])
    } else {
        id.to_string() // Return as-is if too short
    }
}

pub fn init_window() {
    let peripherals = Rc::new(RefCell::new(Peripherals::take().unwrap()));
    let window = MinimalSoftwareWindow::new(
        slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
    );
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    slint::platform::set_platform(Box::new(Esp32Platform {
        window: window.clone(),
        peripherals: Rc::clone(&peripherals),
    }))
    .unwrap();
    // Make sure the window covers our entire screen.
    window.set_size(slint::PhysicalSize::new(240, 320));
    let window_1 = Rc::downgrade(&window);
    log::info!("before appwindowFree Heap: {} bytes", unsafe {
        esp_get_free_heap_size()
    });
    let app = AppWindow::new().unwrap();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop,
    )
    .unwrap();

    let pwr_status = PinDriver::input(peripherals.pins.gpio6).unwrap();
    let mut pwr_en = PinDriver::output(peripherals.pins.gpio7).unwrap();

    if pwr_status.is_low() {
        FreeRtos::delay_ms(10);
        let mut counter_delay: u16 = 0;
        while pwr_status.is_low() {
            counter_delay = counter_delay + 1;
            if counter_delay > 1 {
                FreeRtos::delay_ms(5);
                log::info!("BUG 2: {}\n", pwr_status.is_low());
                app.set_screen_state(ScreenState::HomeLock);
                pwr_en.set_high().unwrap();
            }
        }
    } else {
        log::info!("Power system off");
    }
    let power_manager_thread = thread::spawn(move || {
        loop {
            //log::info!("Thread 1: running...");
            if pwr_status.is_low() {
                FreeRtos::delay_ms(20);
                let mut counter_delay: u16 = 0;
                while pwr_status.is_low() {
                    counter_delay = counter_delay + 1;
                    FreeRtos::delay_ms(5);
                    if (counter_delay > 80) {
                        log::info!("Set Power system Off.\n");
                        pwr_en.set_low().unwrap();
                        FreeRtos::delay_ms(1000);
                        //app.set_screen_state(ScreenState::HomeLock);
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(20));
            thread::sleep(Duration::from_millis(800));
        }
    });
    // let power_en = PinDriver::output(peripherals.pins.gpio7);
    setup_wifi(&mut wifi);
    //setup_wifi(&mut scan_wifi);

    let wifi_sta_mac = wifi.wifi().get_mac(WifiDeviceId::Sta).unwrap();
    let qr_pic = generate_qr_code(&wifi_sta_mac);
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio40; //MTDO
    let mosi = peripherals.pins.gpio45; //
    let miso = peripherals.pins.gpio46;
    let cs = peripherals.pins.gpio42; //MTDI
    let config = Config::new().baudrate(40.MHz().into());
    // Define the delay struct, needed for the display driver
    let mut display_delay = Delay::new(500_000u32);
    // Define the Data/Command select pin as a digital output
    let spi_config = SpiDriverConfig::new().dma(esp_idf_hal::spi::Dma::Auto(4096 as usize));
    let spi_device =
        SpiDeviceDriver::new_single(spi, sclk, mosi, Some(miso), Some(cs), &spi_config, &config)
            .unwrap();
    let buffer = BUFFER.init([0; 512]);
    let slice: &'static mut [u8] = buffer;
    let dc = PinDriver::output(peripherals.pins.gpio41).unwrap(); //MTDI
    let rst = PinDriver::output(peripherals.pins.gpio39).unwrap(); //MTCK
                                                                   // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, slice);
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .color_order(ColorOrder::Rgb)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut display_delay)
        .unwrap();

    /////////////////////////////////// Touch peripheral init
    let mut touch_int = PinDriver::input(peripherals.pins.gpio4).unwrap();
    let mut touch_rst = PinDriver::output(peripherals.pins.gpio2).unwrap();
    let touch_sda_pin = peripherals.pins.gpio1;
    let touch_scl_pin = peripherals.pins.gpio3;
    let touch_i2c_config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = peripherals.i2c0;
    let mut touch_i2c =
        I2cDriver::new(i2c, touch_sda_pin, touch_scl_pin, &touch_i2c_config).unwrap();
    let mut delay_source: Delay = Default::default();
    let bus: &'static shared_bus::BusManager<Mutex<i2c::I2cDriver<'_>>> =
        shared_bus::new_std!(i2c::I2cDriver = touch_i2c).unwrap();

    let mut touch = Cst328::new(bus.acquire_i2c(), delay_source);
    touch.reset(&mut touch_rst, &mut delay_source).unwrap();

    let mut line_buffer_1 =
        [slint::platform::software_renderer::Rgb565Pixel(0); DISPLAY_WIDTH as usize];
    let mut line_buffer_2 =
        [slint::platform::software_renderer::Rgb565Pixel(0); DISPLAY_WIDTH as usize];

    //Create animation controller with pre-processed frames
    let controller = Rc::new(RefCell::new(AnimationController::new(
        &CHIRPLUNK_3_SIT_RGBA8_FRAMES,
    )));

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
                Some(app) => app,
                None => return,
            };
            let mut ctrl = controller_clone.borrow_mut();
            if let Some(frame) = ctrl.update() {
                let image = create_slint_image_from_frame(frame);
                app.set_current_frame(image);
            }
        },
    );
    //timer.stop();

    let mut bl = PinDriver::output(peripherals.pins.gpio5).unwrap();
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
            log::info!("key pressed {:?}", copy);
        }
    });
    //let ota_version = board_ota.version.unwrap();
    app.global::<UpdateFwCallback>().on_get_version(|| {
        let mut ota = OtaUpdate::new();
        let _ = ota.get_new_fw_info(true).unwrap();
        let version = ota.version.unwrap();
        SharedString::from(version)
    });
    app.global::<UpdateFwCallback>().on_exec_update(move || {
        let mut ota = OtaUpdate::new();
        let _ = ota.firmware_update();
    });
    app.global::<AnimationSwitch>()
        .on_animation_switch(move |action, animal| {
            log::info!("{:?} - {:?}", action, animal);
            let mut ctrl = controller.borrow_mut();
            match animal {
                Animal::Chirplunk => match action {
                    GameState::Normal => {
                        ctrl.switch_gif(&CHIRPLUNK_3_SIT_RGBA8_FRAMES);
                    }
                    GameState::Eating => {
                        ctrl.switch_gif(&CHIRPLUNK_3_EAT_RGBA8_FRAMES);
                    }
                    GameState::Sleeping => {
                        ctrl.switch_gif(&CHIRPLUNK_3_SLEEP_RGBA8_FRAMES);
                    }
                    GameState::Playing => {
                        ctrl.switch_gif(&CHIRPLUNK_3_JUMP_RGBA8_FRAMES);
                    }
                },
                Animal::Mechapup => match action {
                    GameState::Normal => {
                        ctrl.switch_gif(&MECHAPUP_3_SIT_RGBA8_FRAMES);
                    }
                    GameState::Eating => {
                        ctrl.switch_gif(&MECHAPUP_3_EAT_RGBA8_FRAMES);
                    }
                    GameState::Sleeping => {
                        ctrl.switch_gif(&MECHAPUP_3_SLEEP_RGBA8_FRAMES);
                    }
                    GameState::Playing => {
                        ctrl.switch_gif(&MECHAPUP_3_JUMP_RGBA8_FRAMES);
                    }
                },
                Animal::LunaFluff => match action {
                    GameState::Normal => {
                        ctrl.switch_gif(&LUNAFLUFF_4_SIT_RGBA8_FRAMES);
                    }
                    GameState::Eating => {
                        ctrl.switch_gif(&LUNAFLUFF_4_EAT_RGBA8_FRAMES);
                    }
                    GameState::Sleeping => {
                        ctrl.switch_gif(&LUNAFLUFF_4_SLEEP_RGBA8_FRAMES);
                    }
                    GameState::Playing => {
                        ctrl.switch_gif(&LUNAFLUFF_4_JUMP_RGBA8_FRAMES);
                    }
                },
            }
        });
    wifi.connect().unwrap();
    wifi.wait_netif_up().unwrap();
    let wifi_rc = Rc::new(RefCell::new(wifi));
    // let active_scan = Rc::clone(&wifi_rc);
    // let active_scan = active_scan.borrow_mut();
    app.global::<WiFiScan>().on_activate_wifi_scan(move || {
        let weak = weak.clone();
        let list_ssid = wifi_rc.clone().borrow_mut().scan().unwrap();
        let ssids: Vec<SharedString> = list_ssid
            .iter()
            .map(|ap| SharedString::from(ap.ssid.as_str()))
            .collect();
        log::info!("{:?}", ssids);
        let list = ModelRc::from(Rc::new(VecModel::from(ssids)));
        weak.unwrap().set_scanned_ssid(list);
    });

    log::info!("before wifi SetQR: {} bytes", unsafe {
        esp_get_free_heap_size()
    });
    {
        app.set_qr_image(qr_pic);
        app.set_deviceID(SharedString::from(short_device_id(DEVICE_ID)));
    }
    log::info!("before wifi Heap: {} bytes", unsafe {
        esp_get_free_heap_size()
    });
    // let mac = wifi.wifi().sta_netif().get_mac().unwrap();
    //let qr = generate_qr_code(format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]));
    //app.set_qr_image(qr);
    log::info!("before http Heap: {} bytes", unsafe {
        esp_get_free_heap_size()
    });
    //let api_response = get_request().unwrap();
    let mut last_touch = None;
    // let wifi_connected = Rc::clone(&wifi_rc);
    // let wifi_connected = wifi_connected.borrow();

    loop {
        bl.set_high().unwrap();
        slint::platform::update_timers_and_animations();

        // if wifi_connected.is_connected().unwrap() {
        //    if let Configuration::Client(ssid_connected) = wifi_connected.wifi().get_configuration().unwrap() {
        //         let ip_info = wifi_connected.wifi().sta_netif().get_mac().unwrap();
        //         app.set_connected_status(WiFiConnectParameters{
        //         connected: true,
        //         ssid : ssid_connected.ssid.to_string().into(),
        //         mac:     SharedString::from(format!(
        //             "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        //             ip_info[0], ip_info[1], ip_info[2], ip_info[3], ip_info[4], ip_info[5]
        //         ))
        //        });
        //    }
        // }

        // match app.get_screen_state() {
        //     ScreenState::Game => {
        //         // let mut ctrl = controller.borrow_mut();
        //         // ctrl.start();
        //         timer.restart();
        //     }
        //     _ =>  {
        //         timer.stop();
        //     }
        // };

        match touch.get_xy_data() {
            Ok(Some(event_touch)) => {
                let pos = slint::PhysicalPosition::new(
                    event_touch.x_cord as i32,
                    event_touch.y_cord as i32,
                )
                .to_logical(window.scale_factor());
                let event = if let Some(previous_pos) = last_touch.replace(pos) {
                    // If the position changed, send a PointerMoved event.
                    if previous_pos != pos {
                        WindowEvent::PointerMoved { position: pos }
                    } else {
                        FreeRtos::delay_ms(50);
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
                FreeRtos::delay_ms(50);
            }
            Ok(None) => {
                if let Some(pos) = last_touch.take() {
                    let event_release = WindowEvent::PointerReleased {
                        position: pos,
                        button: PointerEventButton::Left,
                    };
                    let event_exit = WindowEvent::PointerExited;
                    //log::info!("{:?}", event_exit);
                    log::info!("{:?}", event_release);
                    window.try_dispatch_event(event_release).unwrap();
                    window.try_dispatch_event(event_exit).unwrap();
                }
                //continue;
            }
            Err(_) => {
                log::info!("Error when reading touch");
                todo!("Implement errror handle if have to");
            }
        }
        //Rendering 320x240 takes more than 200ms :(, which is suck

        window.draw_if_needed(|renderer| {
            let render_start = Instant::now();
            renderer.render_by_line(DisplayWrapper {
                display: &mut display,
                line_buffer: &mut line_buffer_1,
            });
            let render_duration = render_start.elapsed();
            log::info!("Render time: {:?}", render_duration);
        });
        // if window.has_active_animations() {
        //     continue;
        // }
        //FreeRtos::delay_ms(50);
    }
}
