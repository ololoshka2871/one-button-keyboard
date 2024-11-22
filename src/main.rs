#![no_main]
#![no_std]

mod config;

use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::usb::Peripheral;

use stm32_usbd::UsbBus;

use stm32f1xx_hal::usb::UsbBusType;

use usbd_hid::descriptor::SerializedDescriptor;

use defmt_rtt as _; // global logger

use panic_probe as _;

use rtic::app;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [RTCALARM, FLASH])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        hid: usbd_hid::hid_class::HIDClass<'static, UsbBusType>,
        usb_dev: usb_device::device::UsbDevice<'static, UsbBusType>,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        defmt::info!("Init...");

        let mut flash = ctx.device.FLASH.constrain();
        let rcc = ctx.device.RCC.constrain();

        let _clocks = rcc
            .cfgr
            .use_hse(config::XTAL_FREQ.Hz())
            .sysclk(48.MHz())
            .pclk1(24.MHz())
            .freeze(&mut flash.acr);

        defmt::info!("Clocks ready");

        let _dma_channels = ctx.device.DMA1.split(); // for defmt

        let mut _afio = ctx.device.AFIO.constrain();
        let mut gpioa = ctx.device.GPIOA.split();

        let mut usb_pull_up = gpioa.pa10.into_push_pull_output_with_state(
            &mut gpioa.crh,
            if !config::USB_PULLUP_ACTVE_LEVEL {
                stm32f1xx_hal::gpio::PinState::High
            } else {
                stm32f1xx_hal::gpio::PinState::Low
            },
        );

        let usb = stm32f1xx_hal::usb::Peripheral {
            usb: ctx.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: gpioa.pa12,
        };

        let usb_bus = cortex_m::singleton!(
            : usb_device::bus::UsbBusAllocator<UsbBus<Peripheral>> = stm32_usbd::UsbBus::new(usb)
        )
        .unwrap();

        defmt::info!("USB ready");

        let hid = usbd_hid::hid_class::HIDClass::new(
            usb_bus,
            usbd_hid::descriptor::KeyboardReport::desc(),
            config::HID_I2C_POLL_INTERVAL_MS,
        );

        defmt::info!("HID ready");

        let usb_dev = usb_device::device::UsbDeviceBuilder::new(
            usb_bus,
            usb_device::prelude::UsbVidPid(0x16c0, 0x314f),
        )
        .manufacturer("Shilo.XyZ")
        .product("OneButtonKeyboard")
        .serial_number(stm32_device_signature::device_id_hex())
        .build();

        defmt::info!("USB device ready");

        // Initialize the systick interrupt & obtain the token to prove that we did
        //Mono::start(delay.release().release(), clocks.sysclk().to_Hz());

        defmt::info!("Systick ready");

        //---------------------------------------------------------------------

        usb_pull_up.toggle(); // enable USB
        defmt::info!("USB enabled");

        //---------------------------------------------------------------------


        (Shared { hid, usb_dev }, Local {})
    }

    //-------------------------------------------------------------------------

    #[task(binds = USB_HP_CAN_TX, shared = [usb_dev, hid], priority = 4)]
    fn usb_tx(ctx: usb_tx::Context) {
        let usb_dev = ctx.shared.usb_dev;
        let hid = ctx.shared.hid;

        (usb_dev, hid).lock(|usb_dev, hid| usb_dev.poll(&mut [hid]));
    }

    #[task(binds = USB_LP_CAN_RX0, shared = [usb_dev, hid], priority = 4)]
    fn usb_rx0(ctx: usb_rx0::Context) {
        let usb_dev = ctx.shared.usb_dev;
        let hid = ctx.shared.hid;

        (usb_dev, hid).lock(|usb_dev, hid| usb_dev.poll(&mut [hid]));
    }
}
