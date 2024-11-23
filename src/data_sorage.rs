#![allow(unexpected_cfgs)]

use core::mem::MaybeUninit;

use cortex_m::interrupt::CriticalSection;
use packed_struct::prelude::*;

#[cfg(any(feature = "stm32f103",))]
use stm32f1xx_hal::flash::{Error as FlashError, FLASH_START};

#[cfg(any(
    feature = "stm32f030",
    feature = "stm32f031",
    feature = "stm32f038",
    feature = "stm32f042",
    feature = "stm32f048",
    feature = "stm32f051",
    feature = "stm32f058",
    feature = "stm32f070",
    feature = "stm32f071",
    feature = "stm32f072",
    feature = "stm32f078",
    feature = "stm32f091",
    feature = "stm32f098",
))]
use stm32f0xx_hal::{
    flash::{Error as FlashError, FlashExt, FLASH_START, PAGE_SIZE},
    pac::FLASH,
};

use crate::report::KeyboardReport;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SettingsStoreError {
    FlashError(FlashError),
    PackingError(PackingError),
}

#[derive(PackedStruct, Clone, Default, defmt::Format)]
pub struct ReportPattern {
    #[packed_field(size_bytes = "1")]
    pub modifier: u8,
    #[packed_field(element_size_bytes = "1")]
    pub keycodes: [u8; 6],
}

impl Into<KeyboardReport> for &ReportPattern {
    fn into(self) -> KeyboardReport {
        KeyboardReport {
            modifier: self.modifier,
            keycodes: self.keycodes,
        }
    }
}

#[derive(PackedStruct, Clone)]
struct SaveStruct {
    #[packed_field(size_bytes = "7")]
    report_pattern: ReportPattern,
    #[packed_field(size_bytes = "2", endian = "lsb")]
    crc: u16,
}

#[link_section = ".uninit_settings.save_struct"]
static mut SAVE_SPACE: MaybeUninit<[u8; 9]> = MaybeUninit::uninit();

pub struct DataStorage {
    pub report_pattern: ReportPattern,

    #[cfg(any(feature = "stm32f103",))]
    pub flash: stm32f1xx_hal::flash::Parts,

    #[cfg(any(
        feature = "stm32f030",
        feature = "stm32f031",
        feature = "stm32f038",
        feature = "stm32f042",
        feature = "stm32f048",
        feature = "stm32f051",
        feature = "stm32f058",
        feature = "stm32f070",
        feature = "stm32f071",
        feature = "stm32f072",
        feature = "stm32f078",
        feature = "stm32f091",
        feature = "stm32f098",
    ))]
    pub flash: FLASH,
}

impl DataStorage {
    pub fn load(
        #[cfg(any(feature = "stm32f103",))] flash: stm32f1xx_hal::flash::Parts,

        #[cfg(any(
            feature = "stm32f030",
            feature = "stm32f031",
            feature = "stm32f038",
            feature = "stm32f042",
            feature = "stm32f048",
            feature = "stm32f051",
            feature = "stm32f058",
            feature = "stm32f070",
            feature = "stm32f071",
            feature = "stm32f072",
            feature = "stm32f078",
            feature = "stm32f091",
            feature = "stm32f098",
        ))]
        flash: FLASH,
    ) -> Self {
        let save_space = unsafe { SAVE_SPACE.assume_init_ref() };

        let crc = crc16(save_space);
        // crc(data + correct_crc) == 0
        if crc == 0 {
            if let Ok(res) = SaveStruct::unpack(&save_space) {
                return Self {
                    report_pattern: res.report_pattern,
                    flash,
                };
            }
        }
        let mut res = Self {
            flash,
            report_pattern: ReportPattern::default(),
        };

        cortex_m::interrupt::free(|cs| res.save(cs)).ok();

        res
    }

    pub fn save(&mut self, _cs: &CriticalSection) -> Result<(), SettingsStoreError> {
        let save_struct = SaveStruct {
            report_pattern: self.report_pattern.clone(),
            crc: 0,
        };

        let mut packed = save_struct
            .pack()
            .map_err(|e| SettingsStoreError::PackingError(e))?;

        {
            let len = packed.len();
            let crc = crc16(&packed[0..len - core::mem::size_of::<u16>()]);
            let crc_pos = &mut packed[len - core::mem::size_of::<u16>()..];
            crc_pos.copy_from_slice(&crc.to_le_bytes());
        }

        let save_space =
            (unsafe { SAVE_SPACE.assume_init_ref().as_ptr() as u32 } - FLASH_START) as u32;

        #[cfg(any(feature = "stm32f103",))]
        {
            const SECTOR_SIZE: usize = 1024;

            let mut writer = self.flash.writer(
                stm32f1xx_hal::flash::SectorSize::Sz1K,
                stm32f1xx_hal::flash::FlashSize::Sz64K,
            );
            writer
                .erase(save_space, SECTOR_SIZE)
                .map_err(|e| SettingsStoreError::FlashError(e))?;
            writer
                .write(save_space, &packed)
                .map_err(|e| SettingsStoreError::FlashError(e))
        }

        #[cfg(any(
            feature = "stm32f030",
            feature = "stm32f031",
            feature = "stm32f038",
            feature = "stm32f042",
            feature = "stm32f048",
            feature = "stm32f051",
            feature = "stm32f058",
            feature = "stm32f070",
            feature = "stm32f071",
            feature = "stm32f072",
            feature = "stm32f078",
            feature = "stm32f091",
            feature = "stm32f098",
        ))]
        {
            use embedded_storage::nor_flash::NorFlash;

            NorFlash::erase(&mut unlocked_flash, save_space, save_space + PAGE_SIZE)
                .map_err(|e| SettingsStoreError::FlashError(e))?;

            NorFlash::write(&mut unlocked_flash, save_space, &packed)
                .map_err(|e| SettingsStoreError::FlashError(e))
        }
    }
}

fn crc16(data: &[u8]) -> u16 {
    crc16::State::<crc16::AUG_CCITT>::calculate(data)
}
