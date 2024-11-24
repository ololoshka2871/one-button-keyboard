#![no_main]
#![no_std]

mod config;
mod data_sorage;
mod report;

use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::timer::Event;
use stm32f1xx_hal::usb::Peripheral;
use stm32f1xx_hal::usb::UsbBus;

use stm32f1xx_hal::usb::UsbBusType;

use usbd_hid::descriptor::SerializedDescriptor;

use defmt_rtt as _; // global logger

use panic_probe as _;

use rtic::app;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [RTCALARM, FLASH])]
mod app {
    use packed_struct::PackedStructSlice;

    use super::*;

    #[shared]
    struct Shared {
        hid_kbd: usbd_hid::hid_class::HIDClass<'static, UsbBusType>,
        hid_ctrl: usbd_hid::hid_class::HIDClass<'static, UsbBusType>,
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

        let mut gpioa = ctx.device.GPIOA.split();
        let mut gpiob = ctx.device.GPIOB.split();

        let (usb_pull_up, usb_dp) = if let Some(usb_pull_up_lvl) = config::USB_PULLUP_ACTVE_LEVEL {
            // pa10 or replace to your pin
            let usb_pull_up = gpioa.pa10.into_push_pull_output_with_state(
                &mut gpioa.crh,
                if !usb_pull_up_lvl {
                    stm32f1xx_hal::gpio::PinState::High
                } else {
                    stm32f1xx_hal::gpio::PinState::Low
                },
            );
            (Some(usb_pull_up), gpioa.pa12.into_push_pull_output(&mut gpioa.crh))
        } else {
            // https://github.com/will-hart/pedalrs/blob/dd33bf753c9d482c38a8365cc925822f105b12cd/src/configure/stm32f103.rs#L77
            // BluePill board has a pull-up resistor on the D+ line.
            // Pull the D+ pin down to send a RESET condition to the USB bus.
            // This forced reset is needed only for development, without it host
            // will not reset your device when you upload new firmware.
            let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
            usb_dp.set_low();
            cortex_m::asm::delay(1000);

            (None, usb_dp)
        };

        let usb = stm32f1xx_hal::usb::Peripheral {
            usb: ctx.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
        };

        let usb_bus = cortex_m::singleton!(
            : usb_device::bus::UsbBusAllocator<UsbBus<Peripheral>> = UsbBus::new(usb)
        )
        .unwrap();

        let mut timer = ctx.device.TIM2.counter_ms(&clocks);
        timer
            .start((config::HID_I2C_POLL_INTERVAL_MS as u32).millis())
            .ok();
        timer.listen(Event::Update);
        defmt::info!("Timer ready");

        let hid_kbd = usbd_hid::hid_class::HIDClass::new(
            usb_bus,
            report::KeyboardReport::desc(),
            config::HID_I2C_POLL_INTERVAL_MS,
        );
        defmt::info!("HID ready");

        let hid_ctrl = usbd_hid::hid_class::HIDClass::new(
            usb_bus,
            report::ControlDesctiptor::desc(),
            config::HID_I2C_POLL_INTERVAL_MS,
        );
        defmt::info!("HID2 ready");

        let usb_dev = usb_device::device::UsbDeviceBuilder::new(
            usb_bus,
            usb_device::prelude::UsbVidPid(config::USB_VID, config::USB_PID),
        )
        .manufacturer("Shilo.XyZ")
        .product("OneButtonKeyboard")
        .serial_number(stm32_device_signature::device_id_hex())
        .composite_with_iads()
        .build();
        defmt::info!("USB device ready");

        let button = gpiob.pb9.into_pull_down_input(&mut gpiob.crh);
        defmt::info!("Button ready");

        //---------------------------------------------------------------------

        let storage = data_sorage::DataStorage::load(flash);
        defmt::info!("Saved report: {}", storage.report_pattern);

        //---------------------------------------------------------------------

        if let Some(mut usb_pull_up) = usb_pull_up {
            usb_pull_up.toggle(); // enable USB
            defmt::info!("USB enabled");
        }

        //---------------------------------------------------------------------

        (
            Shared {
                hid_kbd,
                hid_ctrl,
                usb_dev,
                storage,
            },
            Local { timer, button },
        )
    }

    #[task(binds = TIM2, shared = [hid_kbd, hid_ctrl, storage], local = [
        timer, button, 
        prev_btn_state: bool = false,
        counter: u32 = 0,
    ], priority = 1)]
    fn timer_isr(ctx: timer_isr::Context) {
        let timer = ctx.local.timer;
        let button = ctx.local.button;
        let prev_btn_state = ctx.local.prev_btn_state;
        let counter = ctx.local.counter;
        let mut hid_kbd = ctx.shared.hid_kbd;
        let mut hid_ctrl = ctx.shared.hid_ctrl;
        let mut storage = ctx.shared.storage;

        let new_state = button.is_high();
        if new_state != *prev_btn_state || *counter == 10 {
            *prev_btn_state = new_state;
            let report = if new_state {
                storage.lock(|storage| (&storage.report_pattern).into())
            } else {
                report::KeyboardReport::empty()
            };

            hid_kbd.lock(|hid_kbd| hid_kbd.push_input(&report)).ok();
        }

        if *counter == 10 {
            *counter = 0;

            let mut res = report::ControlDesctiptor::default();
            let pattern = storage.lock(|storage| storage.report_pattern.clone());
            pattern.pack_to_slice(&mut res.get_report_pattern).ok();
            hid_ctrl.lock(|hid_ctrl| hid_ctrl.push_input(&res).ok());
        } else {
            *counter += 1;
        }

        timer.clear_interrupt(Event::Update);
    }

    //-------------------------------------------------------------------------

    #[idle(shared = [usb_dev, hid_kbd, hid_ctrl, storage])]
    fn idle(ctx: idle::Context) -> ! {
        use packed_struct::PackedStructSlice;

        let mut ctrl_report = [0u8; 64];

        let mut usb_dev = ctx.shared.usb_dev;
        let mut hid_kbd = ctx.shared.hid_kbd;
        let mut hid_ctrl = ctx.shared.hid_ctrl;
        let mut storage = ctx.shared.storage;

        loop {
            if (&mut usb_dev, &mut hid_kbd, &mut hid_ctrl)
                .lock(|usb_dev, hid_kbd, hid_ctrl| usb_dev.poll(&mut [hid_kbd, hid_ctrl]))
            {
                if let Some(pattern) = hid_ctrl.lock(|hid_ctrl| {
                    match hid_ctrl.pull_raw_output(&mut ctrl_report) {
                        Ok(size) => {
                            match data_sorage::ReportPattern::unpack_from_slice(
                                &ctrl_report[..size],
                            ) {
                                Ok(pattern) => {
                                    defmt::info!("New pattern: {}", &pattern);
                                    return Some(pattern);
                                }
                                Err(e) => defmt::error!(
                                    "Unpack error: {:#X} ({})",
                                    &ctrl_report[..size],
                                    defmt::Debug2Format(&e)
                                ),
                            }
                        }
                        Err(usbd_hid::UsbError::WouldBlock) => { /* ok */ }
                        Err(e) => {
                            defmt::error!("USB Command error: {}", e)
                        }
                    }

                    None
                }) {
                    storage.lock(|storage| {
                        storage.report_pattern = pattern;

                        cortex_m::interrupt::free(|cs| {
                            if let Err(e) = storage.save(cs) {
                                defmt::error!(
                                    "Failed to save settings: {}",
                                    defmt::Debug2Format(&e)
                                )
                            }
                        })
                    })
                }
            }
        }
    }
}
