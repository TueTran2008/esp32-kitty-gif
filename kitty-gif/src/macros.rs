//use esp_idf_svc::sys::EspError;
#[macro_export]
macro_rules! esp_err {
    ($x:ident) => {
        EspError::from_infallible::<{ $x }>()
    };
}