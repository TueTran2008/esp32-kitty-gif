use esp_idf_svc::systime::EspSystemTime;
use slint::platform::{software_renderer::MinimalSoftwareWindow, Platform};
use std::rc::Rc;

pub struct MyPlatform {
    pub window: Rc<MinimalSoftwareWindow>,
}

impl Platform for MyPlatform {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        // Since on MCUs, there can be only one window, just return a clone of self.window.
        // We'll also use the same window in the event loop.

        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> core::time::Duration {
        EspSystemTime.now()
    }
}
