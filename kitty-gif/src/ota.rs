use core::mem::size_of;
use log::*;

use embedded_svc::{
    http::{client::Client as HttpClient, Headers, Method},
    io::Write,
    utils::io,
};
use esp_idf_svc::{http::client::Response, ota::{EspFirmwareInfoLoad, EspFirmwareInfoLoader, EspOtaUpdate, FirmwareInfo}};
use log::{error, info};
use esp_idf_svc::{http::client::EspHttpConnection, ota::EspOta};
use crate::error::MyError;
use serde::{Serialize, Deserialize};
use http::header::{ACCEPT};
use http::Uri;
use esp_idf_svc::sys::{EspError, ESP_ERR_IMAGE_INVALID, ESP_ERR_INVALID_RESPONSE};

const FIRMWARE_DOWNLOAD_CHUNK_SIZE: usize = 1024 * 20;
const FIRMWARE_MAX_SIZE: usize = 1024 * 1024 * 6;
const FIRMWARE_MIN_SIZE: usize = size_of::<FirmwareInfo>() + 1024;

#[macro_export]
macro_rules! esp_err {
    ($x:ident) => {
        EspError::from_infallible::<{ $x }>()
    };
}

#[derive(Debug, Deserialize)]
struct OtaOKZooInfo {
    success: bool,
    message: String,
    data: OtaData
}

#[derive(Serialize, Deserialize, Debug)]
struct OtaData {
    version: String,
    fileLink: String
}
pub struct OtaUpdate {
    client:HttpClient<EspHttpConnection>,
    pub version: Option<String>,
    pub url: Option<String>
}
impl OtaUpdate {
    pub fn new() -> OtaUpdate{
        //let mut client = HttpClient::wrap(EspHttpConnection::new(&Default::default()).unwrap());
        use esp_idf_svc::http::client::{Configuration as HttpConfiguration, EspHttpConnection};
        
        // let config = &HttpConfiguration {
        //     crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        //     ..Default::default()
        // }; 
        //let mut client = HttpClient::wrap(EspHttpConnection::new(&config).unwrap());
        let client = HttpClient::wrap(EspHttpConnection::new(&Default::default()).unwrap());
        //9let ret = get_new_firmware_version(&mut client);
        OtaUpdate {
            client,
            version: None,
            url: None
        }
    }
    pub fn get_new_fw_info(&mut self, version: bool) -> Result<(), EspError> {


        // Prepare headers and URL
        let headers = [
            ("content-length", "0"),
        ];
        let url = "http://okzoov2-api-dev.oozoo.dev/device/check-sum";

        // Send request
        let request = self.client.post(url, &headers).unwrap();
        log::info!("-> POST {url}");
        let mut response = request.submit().unwrap();

        // Process response
        let status = response.status();
        log::info!("<- {status}");
        let mut buf = [0u8; 1024];
        let bytes_read = io::try_read_full(&mut response, &mut buf).map_err(|e| e.0).unwrap();
        log::info!("Read {bytes_read} bytes");
        match std::str::from_utf8(&buf[0..bytes_read]) {
            Ok(body_string) => {
                let serialized: OtaOKZooInfo = serde_json::from_str(body_string).unwrap();
                if serialized.success == true {
                    self.url = Some(String::from(&serialized.data.fileLink));
                    self.version = Some(String::from(&serialized.data.version));
                    return Ok(());
                } else {
                    return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
                }
            }
            Err(e) => 
            {
                error!("Error decoding response body: {e}");
                return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
            }
        }
    }
    pub fn firmware_update(&mut self) -> Result<(), EspError>{
        // Prepare payload
        let _ = self.get_new_fw_info(false)?;
        if let Some(ref link_url) = self.url {
            let headers = [(ACCEPT.as_str(), mime::APPLICATION_OCTET_STREAM.as_ref())];
            if let Ok(u) = Uri::try_from(link_url) {
                let https_url = u.to_string();
                let url = https_url.replacen("https://", "http://", 1);
                let requestsurl = self.client
                    .request(Method::Get, &url, &headers)
                    .map_err(|e| e.0)?;
                    let mut response = requestsurl.submit().map_err(|e| e.0)?;
                    if response.status() != 200 {
                        log::info!("Bad HTTP response: {}", response.status());
                        return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
                    }
                    let file_size = response.content_len().unwrap_or(0) as usize;
                    if file_size <= FIRMWARE_MIN_SIZE {
                        log::info!(
                            "File size is {file_size}, too small to be a firmware! No need to proceed further."
                        );
                        return Err(esp_err!(ESP_ERR_IMAGE_INVALID));
                    }
                    if file_size > FIRMWARE_MAX_SIZE {
                        log::info!("File is too big ({file_size} bytes).");
                        return Err(esp_err!(ESP_ERR_IMAGE_INVALID));
                    }
                    let mut ota = EspOta::new()?;
                    let mut work = ota.initiate_update()?;
                    let mut buff = vec![0; FIRMWARE_DOWNLOAD_CHUNK_SIZE];
                    let mut total_read_len: usize = 0;
                    let mut got_info = false;
                    let dl_result = loop {
                        let n = response.read(&mut buff).unwrap_or_default();
                        total_read_len += n;
                        if !got_info {
                            match get_firmware_info(&buff[..n]) {
                                Ok(info) => log::info!("Firmware to be downloaded: {info:?}"),
                                Err(e) => {
                                    log::error!("Failed to get firmware info from downloaded bytes!");
                                    break Err(e);
                                }
                            };
                            got_info = true;
                        }
                        if n > 0 {
                            if let Err(e) = work.write(&buff[..n]) {
                                log::error!("Failed to write to OTA. {e}");
                                break Err(e);
                            }
                        }
                        if total_read_len >= file_size {
                            break Ok(());
                        }
                    };
                    if dl_result.is_err() { 
                        return work.abort();
                    }
                    if total_read_len < file_size {
                        log::error!("Supposed to download {file_size} bytes, but we could only get {total_read_len}. May be network error?");
                        return work.abort();
                    }
                    return work.complete();
            } else {
                log::warn!("Invalid URL to download firmware");
                return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
            }
        } else {
            return Err(esp_err!(ESP_ERR_INVALID_RESPONSE)); 
        }
    }
}

fn get_firmware_info(buff: &[u8]) -> Result<FirmwareInfo, EspError> {
    let mut loader = EspFirmwareInfoLoader::new();
    loader.load(buff)?;
    loader.get_info()
}





// fn get_running_version(ota: &EspOta) -> Result<String<24>, MyError> {
//     let slot = ota.get_running_slot().map_err(|_| MyError::GetSlotFailed)?;

//     let firmware = slot
//         .firmware
//         .as_ref()
//         .ok_or(MyError::FirmwareMissing)?;

//     Ok(String::from(firmware.version))
// }

// pub fn check_for_updates(
//     client: &mut HttpClient<EspHttpConnection>,
//     url: &str,
// ) -> Result<(), MyError> {
//     let mut ota = EspOta::new().map_err(|_| MyError::OtaInit)?;

//     let current_version = get_running_version(&ota)?;
//     info!("Current version: {current_version}");

//     info!("Checking for updates...");

//     let headers = [
//         ("Accept", "application/octet-stream"),
//         ("X-Esp32-Version", &current_version),
//     ];

//     let request = client
//         .request(Method::Get, url, &headers)
//         .map_err(MyError::Http)?;

//     let response = request.submit().map_err(|_| MyError::SendRequest)?;

//     if response.status() == http_status::NOT_MODIFIED {
//         info!("Already up to date");
//     } else if response.status() == http_status::OK {
//         info!("An update is available, updating...");
//         let mut update = ota.initiate_update().map_err(|_| MyError::InitiateUpdate)?;

//         match download_update(response, &mut update).map_err(|_| MyError::DownloadUpdate) {
//             Ok(_) => {
//                 info!("Update done. Restarting...");
//                 update.complete().map_err(|_| MyError::CompleteUpdate)?;
//                 esp_idf_svc::hal::reset::restart();
//             }
//             Err(err) => {
//                 error!("Update failed: {err}");
//                 update.abort().map_err(|_| MyError::AbortUpdate)?;
//             }
//         };
//     }

//     Ok(())
// }

// fn download_update(
//     mut response: Response<&mut EspHttpConnection>,
//     update: &mut EspOtaUpdate<'_>,
// ) -> Result<(), MyError> {
//     let mut buffer = [0_u8; 1024];

//     let update_info = read_firmware_info(&mut buffer, &mut response, update)?;
//     info!("Update version: {}", update_info.version);

//     io::utils::copy(response, update, &mut buffer).map_err(|_| MyError::CopyFailed)?;

//     Ok(())
// }

// fn read_firmware_info(
//     buffer: &mut [u8],
//     response: &mut Response<&mut EspHttpConnection>,
//     update: &mut EspOtaUpdate,
// ) -> Result<FirmwareInfo, MyError> {
//     let update_info_load = EspFirmwareInfoLoad {};
//     let mut update_info = FirmwareInfo {
//         version: Default::default(),
//         released: Default::default(),
//         description: Default::default(),
//         signature: Default::default(),
//         download_id: Default::default(),
//     };

//     loop {
//         let n = response.read(buffer).map_err(MyError::IoError)?;
//         if n == 0 {
//             break; // EOF
//         }

//         update.write(&buffer[..n]).map_err(|_| MyError::OtaWriteFailed)?;

//         let parsed = update_info_load
//             .fetch(&buffer[..n], &mut update_info)
//             .map_err(|_| MyError::FirmwareInfoParseFailed)?;

//         if parsed {
//             return Ok(update_info);
//         }
//     }

//     Err(MyError::FirmwareInfoNotFound)
// }

// use http::header::ACCEPT;
// use http::Uri;
// use embedded_svc::ota::FirmwareInfo;
// use embedded_svc::http::{client::Client, Method};
// use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
// use esp_idf_svc::ota::EspOta;
// use esp_idf_svc::sys::{EspError, ESP_ERR_IMAGE_INVALID, ESP_ERR_INVALID_RESPONSE};

// const FIRMWARE_DOWNLOAD_CHUNK_SIZE: usize = 1024 * 20;
// // Not expect firmware bigger than 2MB
// const FIRMWARE_MAX_SIZE: usize = 1024 * 1024 * 2;
// const FIRMWARE_MIN_SIZE: usize = size_of::<FirmwareInfo>() + 1024;

// pub fn simple_download_and_update_firmware(url: Uri) -> Result<(), EspError> {
//     let mut client = Client::wrap(EspHttpConnection::new(&Configuration {
//         buffer_size: Some(1024 * 4),
//         ..Default::default()
//     }).unwrap());
//     let headers = [(ACCEPT.as_str(), mime::APPLICATION_OCTET_STREAM.as_ref())];
//     let surl = url.to_string();
//     let request = client
//         .request(Method::Get, &surl, &headers)
//         .map_err(|e| e.0).unwrap();
//     let mut response = request.submit().map_err(|e| e.0).unwrap();
//     if response.status() != 200 {
//         log::info!("Bad HTTP response: {}", response.status());
//         return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
//     }
//     let file_size = response.content_len().unwrap_or(0) as usize;
//     if file_size <= FIRMWARE_MIN_SIZE {
//         log::info!(
//             "File size is {file_size}, too small to be a firmware! No need to proceed further."
//         );
//         return Err(esp_err!(ESP_ERR_IMAGE_INVALID));
//     }
//     if file_size > FIRMWARE_MAX_SIZE {
//         log::info!("File is too big ({file_size} bytes).");
//         return Err(esp_err!(ESP_ERR_IMAGE_INVALID));
//     }
//     let mut ota = EspOta::new().unwrap();
//     let mut work = ota.initiate_update().unwrap();
//     let mut buff = vec![0; FIRMWARE_DOWNLOAD_CHUNK_SIZE];
//     let mut total_read_len: usize = 0;
//     let mut got_info = false;
//     let dl_result = loop {
//         let n = response.read(&mut buff).unwrap_or_default();
//         total_read_len += n;
//         if !got_info {
//             match get_firmware_info(&buff[..n]) {
//                 Ok(info) => log::info!("Firmware to be downloaded: {info:.unwrap()}"),
//                 Err(e) => {
//                     log::error!("Failed to get firmware info from downloaded bytes!");
//                     break Err(e);
//                 }
//             };
//             got_info = true;
//         }
//         if n > 0 {
//             if let Err(e) = work.write(&buff[..n]) {
//                 log::error!("Failed to write to OTA. {e}");
//                 break Err(e);
//             }
//         }
//         if total_read_len >= file_size {
//             break Ok(());
//         }
//     };
//     if dl_result.is_err() {
//         return work.abort();
//     }
//     if total_read_len < file_size {
//         log::error!("Supposed to download {file_size} bytes, but we could only get {total_read_len}. May be network error.unwrap()");
//         return work.abort();
//     }
//     work.complete()
// }