// use esp_idf_svc::wifi::*;
// use crate::error::Result;
// use esp_idf_svc::wifi::EspWifi;
// use std::sync::{mpsc, Arc, Mutex};
// use std::thread;
// use std::{rc::Rc, cell::RefCell};
// use esp_idf_svc::wifi::BlockingWifi;
// use esp_idf_svc::wifi::Configuration;
// use esp_idf_svc::wifi::ClientConfiguration;
// use esp_idf_svc::wifi::AuthMethod;
// use std::sync::mpsc::{channel, Sender, Receiver};

// pub enum WiFiAction {
//     Connect { ssid: String, password: String },
//     Disconnect,
//     GetStatus,
//     Scan
// }
// pub (crate) struct WiFi {
//     wifi: Arc<Mutex<BlockingWifi<EspWifi<'static>>>>,
//     wifi_command: Sender<WiFiAction>,
// }

// // impl WiFi {
// //     pub (crate) fn new(wifi_client: BlockingWifi<EspWifi<'static>>) -> Self {
// //         let (wifi_cmd_send, wifi_cmd_receiver) = channel::<WiFiAction>();
// //         let wifi = WiFi { 
// //             wifi: Arc::new(Mutex::new(wifi_client)),
// //             wifi_command: wifi_cmd_send
// //         };
// //         let wifi_task = wifi.wifi.clone();
// //         let wifi_task = wifi_task.lock().unwrap();
// //         let wifi_thread = thread::spawn(move || {
// //             wifi_task.start()?;
// //             log::info!("Wifi started in the thread");
// //             loop {
// //                 match wifi_cmd_receiver.recv() {
// //                     Ok(WiFiAction::Connect { ssid, password }) => {
// //                         log::info!("Connection to the WiFi network: SSID {} PASS {}", ssid, password);
// //                         wifi_task.connect();
// //                     }
// //                     Ok(WiFi)
// //                     Err(_) => {
// //                         log::error!("Unknow WiFi command");
// //                     }
// //                 }
// //             }
// //         });
// //         wifi

// //     }
// //     // pub (crate) fn update_configuration(&mut self, ssid: &str, password: &str) -> Result<()> {
// //     //     let wifi = self.wifi.clone();
// //     //     let mut wifi = wifi.borrow_mut();

// //     //     let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
// //     //         ssid: ssid.try_into().unwrap(),
// //     //         bssid: None,
// //     //         auth_method: AuthMethod::WPA2Personal,
// //     //         password: password.try_into().unwrap(),
// //     //         channel: None,
// //     //         ..Default::default()
// //     //     });

// //     //     wifi.set_configuration(&wifi_configuration)?;
// //     //     Ok(())
// //     // }
// //     async fn connect_wifi(&mut self, ssid: &str, password: &str) ->  Result<()> {
// //         let wifi = self.wifi.clone();
// //         let mut wifi = wifi.borrow_mut();
// //         let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
// //             ssid: ssid.try_into().unwrap(),
// //             bssid: None,
// //             auth_method: AuthMethod::WPA2Personal,
// //             password: password.try_into().unwrap(),
// //             channel: None,
// //             ..Default::default()
// //         });

// //         wifi.set_configuration(&wifi_configuration)?;

// //         wifi.start().await?;
// //         log::info!("Wifi started");

// //         wifi.connect().await?;
// //         log::info!("Wifi connected");

// //         wifi.wait_netif_up().await?;
// //         info!("Wifi netif up");

// //         Ok(())
// //     }
// // }