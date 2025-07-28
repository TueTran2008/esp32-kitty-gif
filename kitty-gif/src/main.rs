#[macro_use]
mod macros;

//mod bring_up;
mod consts;
mod error;
mod gif;
mod lcd;
mod ota;
mod ui;
mod wifi;

use embedded_svc::wifi::Wifi;
use esp_idf_svc::wifi::{Configuration, WifiDeviceId};
use mipidsi::Builder;
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::platform::{PointerEventButton, WindowEvent};
use std::thread;
use std::time::Duration;
slint::include_modules!();
use error::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, EspWifi, WifiEvent},
};
use lcd::lcd_init;
use slint::SharedString;
use std::{cell::RefCell, rc::Rc};
use ui::Esp32Platform;
// Frame data structure
#[derive(Clone)]
pub struct FrameData {
    pub data: &'static [u16], // RGB565 format
    pub delay_ms: u32,
    pub width: u16,
    pub height: u16,
}
pub struct RgbaFrameData {
    pub data: &'static [u8],
    pub delay_ms: u32,
    pub width: u16,
    pub height: u16,
}

fn setup_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: "TUE".try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: "Gemtek@123".try_into().unwrap(),
        channel: None,
        ..Default::default()
    });
    wifi.set_configuration(&wifi_configuration).unwrap();
    wifi.start().unwrap();
    log::info!("Wifi started");
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world From Okzoo!");
    let peripherals = Peripherals::take()?;
    //let wifi_peripherals = Rc::clone(&peripherals);
    let wifi_modem = peripherals.modem;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    unsafe {
        let board_lcd_panel = lcd_init()?;
    }
    sys_loop.subscribe::<WifiEvent, _>(move |event| {
        match event {
            WifiEvent::StaConnected { .. } => {
                log::info!("WiFi connected");
            }
            WifiEvent::StaDisconnected { .. } => {
                log::info!("WiFi disconnected");
            }
            // WifiEvent::StaGotIp { .. } => {
            //     log::info!("WiFi got IP address");
            // }
            other => {
                log::info!("Wifi event: {:?}", other);
            }
        }
    })?;

    let window = MinimalSoftwareWindow::new(
        slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
    );
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(wifi_modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop.clone(),
    )?;
    let wifi: Rc<RefCell<BlockingWifi<EspWifi<'_>>>> = Rc::new(RefCell::new(wifi));
    let wifi_status = Rc::clone(&wifi);
    let wifi_setup = Rc::clone(&wifi);

    slint::platform::set_platform(Box::new(Esp32Platform {
        window: window.clone(),
    }))
    .unwrap();

    setup_wifi(&mut wifi_setup.borrow_mut());

    let app = AppWindow::new()?;
    app.global::<WiFiScan>().on_connected_wifi(move || {
        //let weak = weak.clone();
        let connected_status = wifi_status.borrow_mut().is_connected().unwrap();
        let wifi_mac = wifi_status
            .borrow_mut()
            .wifi()
            .get_mac(WifiDeviceId::Sta)
            .unwrap();
        if let Configuration::Client(ssid_connected) =
            wifi_status.borrow_mut().wifi().get_configuration().unwrap()
        {
            let ip_info = wifi_status
                .borrow_mut()
                .wifi()
                .sta_netif()
                .get_mac()
                .unwrap();
            WiFiConnectParameters {
                connected: connected_status,
                ssid: ssid_connected.ssid.to_string().into(),
                mac: SharedString::from(format!(
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    ip_info[0], ip_info[1], ip_info[2], ip_info[3], ip_info[4], ip_info[5]
                )),
            }
        } else {
            WiFiConnectParameters {
                connected: false,
                ssid: SharedString::from("None"),
                mac: SharedString::from(""),
            }
        }
    });

    wifi_setup.borrow_mut().wifi_mut().connect()?;

    loop {
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
