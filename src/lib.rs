#![cfg_attr(not(test), no_std)]

pub mod jobs;
#[cfg(any(feature = "ota_mqtt_data", feature = "ota_http_data"))]
pub mod ota;
pub mod shadows;

#[cfg(test)]
pub mod test;
