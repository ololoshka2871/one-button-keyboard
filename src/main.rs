#![no_main]
#![no_std]
#![allow(static_mut_refs)]

mod config;
mod data_sorage;
mod report;

use packed_struct::PackedStructSlice;
use usbd_hid::descriptor::SerializedDescriptor;

use rtic::app;

use panic_probe as _;

#[cfg(feature = "defmt")]
use defmt_rtt as _; // global logger

#[cfg(feature = "stm32f103")]
#[path = "main/main-f103.rs"]
mod main;

#[cfg(feature = "stm32f072")]
#[path = "main/main-f072.rs"]
mod main;

fn pull_raw_output<B: usb_device::bus::UsbBus>(
    hid_ctrl: &mut usbd_hid::hid_class::HIDClass<'static, B>,
    ctrl_report: &mut [u8],
) -> Option<data_sorage::ReportPattern> {
    match hid_ctrl.pull_raw_output(ctrl_report) {
        Ok(size) => match data_sorage::ReportPattern::unpack_from_slice(&ctrl_report[..size]) {
            Ok(pattern) => {
                #[cfg(feature = "defmt")]
                defmt::info!("New pattern: {}", &pattern);
                return Some(pattern);
            }
            Err(_e) => {
                #[cfg(feature = "defmt")]
                defmt::error!(
                    "Unpack error: {:#X} ({})",
                    &ctrl_report[..size],
                    defmt::Debug2Format(&_e)
                );
            }
        },
        Err(usbd_hid::UsbError::WouldBlock) => { /* ok */ }
        Err(_e) => {
            #[cfg(feature = "defmt")]
            defmt::error!("USB Command error: {}", _e)
        }
    }

    None
}

fn store_report(pattern: data_sorage::ReportPattern, storage: &mut data_sorage::DataStorage) {
    storage.report_pattern = pattern;

    cortex_m::interrupt::free(|cs| {
        if let Err(_e) = storage.save(cs) {
            #[cfg(feature = "defmt")]
            defmt::error!("Failed to save settings: {}", defmt::Debug2Format(&_e))
        }
    })
}
