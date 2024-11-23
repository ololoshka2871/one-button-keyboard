#![allow(unexpected_cfgs)]

use std::{env, fs::File, io::Write, path::PathBuf};

#[cfg(any(
    feature = "stm32f030",
    feature = "stm32f031",
    feature = "stm32f038",
    feature = "stm32f042",
    feature = "stm32f048",
    feature = "stm32f051",
    feature = "stm32f058",
))]
const FLASH_SIZE: u32 = 32;

#[cfg(any(
    feature = "stm32f070",
    feature = "stm32f071",
    feature = "stm32f072",
    feature = "stm32f078",
    feature = "stm32f091",
    feature = "stm32f098",
))]
const FLASH_SIZE: u32 = 64;

#[cfg(any(feature = "stm32f103c8",))]
const FLASH_SIZE: u32 = 64;

#[cfg(any(feature = "stm32f103cb",))]
const FLASH_SIZE: u32 = 128;

//-------------------------------------------------------------------------

#[cfg(any(
    feature = "stm32f030",
    feature = "stm32f031",
    feature = "stm32f038",
    feature = "stm32f042",
    feature = "stm32f048",
    feature = "stm32f051",
    feature = "stm32f058",
))]
const FLASH_PAGE_SIZE: u32 = 1;

#[cfg(any(
    feature = "stm32f070",
    feature = "stm32f071",
    feature = "stm32f072",
    feature = "stm32f078",
    feature = "stm32f091",
    feature = "stm32f098",
))]
const FLASH_PAGE_SIZE: u32 = 2;

#[cfg(any(feature = "stm32f103",))]
const FLASH_PAGE_SIZE: u32 = 1;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(
            format!(
                r#"
MEMORY
{{
  /* NOTE K = KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x08000000, LENGTH = {flash_size}K - {flas_page_size}K
  SETTINGS : ORIGIN = 0x08000000 + {flash_size}K - {flas_page_size}K,  LENGTH = {flas_page_size}K
  RAM : ORIGIN = 0x20000000, LENGTH = {ram_size}K
}}

SECTIONS {{
  .uninit_settings (NOLOAD) : ALIGN(4)
  {{
    . = ALIGN(4);
    __suninit_settings = .;
    *(.uninit_settings .uninit_settings.*)
    . = ALIGN(4);
    __euninit_settings = .;
    }} > SETTINGS
}}
"#,
                flash_size = FLASH_SIZE,
                flas_page_size = FLASH_PAGE_SIZE,
                ram_size = 6,
            )
            .as_bytes(),
        )
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=build.rs");
}
