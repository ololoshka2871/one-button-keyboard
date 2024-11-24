use usbd_hid_macros::gen_hid_descriptor;
use usbd_hid::descriptor::{generator_prelude::*, AsInputReport, SerializedDescriptor};

/// KeyboardReport describes a report and its companion descriptor that can be
/// used to send keyboard button presses to a host and receive the status of the
/// keyboard LEDs.
#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = KEYBOARD) = {
        (usage_page = KEYBOARD, usage_min = 0xE0, usage_max = 0xE7) = {
            #[packed_bits 8] #[item_settings data,variable,absolute] modifier=input;
        };
        (usage_page = KEYBOARD, usage_min = 0x00, usage_max = 0xDD) = {
            #[item_settings data,array,absolute] keycodes=input;
        };
    }
)]
pub struct KeyboardReport {
    pub modifier: u8,
    pub keycodes: [u8; 6],
}

impl KeyboardReport {
    pub fn empty() -> Self {
        Self {
            modifier: 0,
            keycodes: [0u8; 6],
        }
    }

    pub const fn size() -> usize {
        6 + 1
    }
}

#[gen_hid_descriptor(
    (collection = LOGICAL, usage_page = VENDOR_DEFINED_START, usage = 0x00) = {
        (usage_min = 0x00, usage_max = 0xFF) = { 
            #[item_settings data,array,absolute] set_report_pattern=output;
        };
        (usage_min = 0x00, usage_max = 0xFF) = { 
            #[item_settings data,array,absolute] get_report_pattern=input;
        };
    }
)]
#[derive(Default)]
pub struct ControlDesctiptor {
    pub set_report_pattern: [u8; 7],
    pub get_report_pattern: [u8; 7],
}