mod bring_up;
mod error;
mod ui;

use bring_up::{init_lcd, init_window};
use error::Result;
fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world From Darwin!");
    // let _ret = init_lcd();
    init_window();

    // Ok(())
}
