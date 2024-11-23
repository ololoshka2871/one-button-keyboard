#![no_main]
#![no_std]

mod config;
mod data_sorage;

use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::timer::Event;
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
        storage: data_sorage::DataStorage,
    }

    #[local]
    struct Local {
        timer: stm32f1xx_hal::timer::Counter<stm32f1xx_hal::pac::TIM2, 1000>,
        button: stm32f1xx_hal::gpio::PB9<stm32f1xx_hal::gpio::Input<stm32f1xx_hal::gpio::PullDown>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        defmt::info!("Init...");

        let mut flash = ctx.device.FLASH.constrain();
        let rcc = ctx.device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(config::XTAL_FREQ.Hz())
            .sysclk(48.MHz())
            .pclk1(24.MHz())
            .freeze(&mut flash.acr);

        defmt::info!("Clocks ready");

        let _dma_channels = ctx.device.DMA1.split(); // for defmt

        let mut _afio = ctx.device.AFIO.constrain();
        let gpioa = ctx.device.GPIOA.split();
        let mut gpiob = ctx.device.GPIOB.split();

        // let mut usb_pull_up = gpioa.pa10.into_push_pull_output_with_state(
        // &mut gpioa.crh,
        // if !config::USB_PULLUP_ACTVE_LEVEL {
        // stm32f1xx_hal::gpio::PinState::High
        // } else {
        // stm32f1xx_hal::gpio::PinState::Low
        // },
        // );

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

        let mut timer = ctx.device.TIM2.counter_ms(&clocks);
        timer
            .start((config::HID_I2C_POLL_INTERVAL_MS as u32).millis())
            .ok();
        timer.listen(Event::Update);

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

        let button = gpiob.pb9.into_pull_down_input(&mut gpiob.crh);
        defmt::info!("Button ready");

        //---------------------------------------------------------------------

        let storage = data_sorage::DataStorage::load(flash);
        defmt::info!("Saved report: {}", storage.report_pattern);

        //---------------------------------------------------------------------

        // usb_pull_up.toggle(); // enable USB
        // defmt::info!("USB enabled");

        //---------------------------------------------------------------------

        (
            Shared {
                hid,
                usb_dev,
                storage,
            },
            Local { timer, button },
        )
    }

    #[task(binds = TIM2, shared = [hid, storage], local = [timer, button, prev_btn_state: bool = false], priority = 1)]
    fn timer_isr(ctx: timer_isr::Context) {
        let timer = ctx.local.timer;
        let button = ctx.local.button;
        let prev_btn_state = ctx.local.prev_btn_state;
        let mut hid = ctx.shared.hid;
        let mut storage = ctx.shared.storage;

        let new_state = button.is_high();
        if new_state != *prev_btn_state {
            *prev_btn_state = new_state;
            let report = if new_state {
                storage.lock(|storage| (&storage.report_pattern).into())
            } else {
                usbd_hid::descriptor::KeyboardReport {
                    modifier: 0,
                    reserved: 0,
                    leds: 0,
                    keycodes: [0, 0, 0, 0, 0, 0],
                }
            };

            hid.lock(|hid| hid.push_input(&report)).ok();
        }

        timer.clear_interrupt(Event::Update);
    }

    //-------------------------------------------------------------------------

    #[idle(shared = [usb_dev, hid])]
    fn idle(ctx: idle::Context) -> ! {
        let mut usb_dev = ctx.shared.usb_dev;
        let mut hid = ctx.shared.hid;

        loop {
            (&mut usb_dev, &mut hid).lock(|usb_dev, hid| usb_dev.poll(&mut [hid]));

            hid.lock(|hid| {
                let mut report = [0u8; 64];
                if let Ok(report_info) = hid.pull_raw_report(&mut report) {
                    defmt::warn!(
                        "New report: ty:={}, id={} {}",
                        report_info.report_type as u8,
                        report_info.report_id,
                        &report[..report_info.len]
                    );
                }
            });
        }
    }

    //-------------------------------------------------------------------------

    #[task(shared = [storage], priority = 1)]
    async fn saver(mut ctx: saver::Context) {
        ctx.shared.storage.lock(|storage| {
            cortex_m::interrupt::free(|cs| {
                if let Err(e) = storage.save(cs) {
                    defmt::error!("Failed to save settings: {}", defmt::Debug2Format(&e))
                }
            })
        })
    }
}
