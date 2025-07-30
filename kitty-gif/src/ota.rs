use crate::error::MyError;
use crate::esp_err;
use core::mem::size_of;
use embedded_svc::{
    http::{client::Client as HttpClient, Headers, Method},
    io::Write,
    utils::io,
};
use esp_idf_svc::http::client::Configuration as HttpConfiguration;
use esp_idf_svc::sys::{EspError, ESP_ERR_IMAGE_INVALID, ESP_ERR_INVALID_RESPONSE};
use esp_idf_svc::{http::client::EspHttpConnection, ota::EspOta};
use esp_idf_svc::{
    http::client::Response,
    ota::{EspFirmwareInfoLoad, EspFirmwareInfoLoader, EspOtaUpdate, FirmwareInfo},
};
use http::header::ACCEPT;
use http::Uri;
use log::{error, info};
use serde::{Deserialize, Serialize};

const FIRMWARE_DOWNLOAD_CHUNK_SIZE: usize = 1024 * 20;
const FIRMWARE_MAX_SIZE: usize = 1024 * 1024 * 6;
const FIRMWARE_MIN_SIZE: usize = size_of::<FirmwareInfo>() + 1024;
const FIRMWARE_VERSION_CHECK_URL: &str = "http://okzoov2-api-dev.oozoo.dev/device/check-sum";

#[derive(Debug, Deserialize)]
pub struct OtaOKZooInfo {
    success: bool,
    message: String,
    pub data: OtaData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OtaData {
    pub version: String,

    #[serde(rename = "fileLink")]
    pub file_link: String,
}
pub struct OtaUpdate {
    client: HttpClient<EspHttpConnection>,
}
impl OtaUpdate {
    pub fn new() -> OtaUpdate {
        //let mut client = HttpClient::wrap(EspHttpConnection::new(&Default::default()).unwrap());

        // let config = &HttpConfiguration {
        //     crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        //     ..Default::default()
        // };
        //let mut client = HttpClient::wrap(EspHttpConnection::new(&config).unwrap());
        let client = HttpClient::wrap(EspHttpConnection::new(&Default::default()).unwrap());
        OtaUpdate { client }
    }
    pub fn get_new_fw_info(&mut self) -> Result<OtaOKZooInfo, EspError> {
        // Prepare headers and URL
        let headers = [("content-length", "0")];
        let url = FIRMWARE_VERSION_CHECK_URL;

        // Send request
        let request = self.client.post(url, &headers).map_err(|e| e.0)?;
        info!("-> POST {url}");
        let mut response = request.submit().map_err(|e| e.0)?;

        // Process response
        let status = response.status();
        info!("<- {status}");
        let mut buf = [0u8; 1024];
        let bytes_read = io::try_read_full(&mut response, &mut buf).map_err(|e| e.0 .0)?;
        info!("Read {bytes_read} bytes");
        match std::str::from_utf8(&buf[0..bytes_read]) {
            Ok(body_string) => {
                let serialized: OtaOKZooInfo = serde_json::from_str(body_string).unwrap();
                if serialized.success == true {
                    info!("Ok zoo OTA Data {:?}", serialized);
                    return Ok(serialized);
                } else {
                    return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
                }
            }
            Err(e) => {
                error!("Error decoding response body: {e}");
                return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
            }
        }
    }
    pub fn firmware_update(&mut self) -> Result<(), EspError> {
        // Prepare payload
        let ota_info = self.get_new_fw_info()?;
        let link_url = &ota_info.data.file_link;
        let headers = [(ACCEPT.as_str(), mime::APPLICATION_OCTET_STREAM.as_ref())];
        if let Ok(u) = Uri::try_from(link_url) {
            let https_url = u.to_string();
            let url = https_url.replacen("https://", "http://", 1);
            let requestsurl = self
                .client
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
                    } else {
                        log::info!("Write ota {} bytes", n);
                    }
                }
                if total_read_len >= file_size {
                    break Ok(());
                }
            };
            if dl_result.is_err() {
                log::error!("Error while downloading firmwre");
                return work.abort();
            }
            if total_read_len < file_size {
                log::error!("Supposed to download {file_size} bytes, but we could only get {total_read_len}. May be network error?");
                return work.abort();
            }
            let _ = work.complete();
            return Ok(());
        } else {
            log::warn!("Invalid URL to download firmware");
            return Err(esp_err!(ESP_ERR_INVALID_RESPONSE));
        }
    }
}

fn get_firmware_info(buff: &[u8]) -> Result<FirmwareInfo, EspError> {
    let mut loader = EspFirmwareInfoLoader::new();
    loader.load(buff)?;
    loader.get_info()
}
