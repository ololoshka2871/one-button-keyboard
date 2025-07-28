use super::*;

use stm32f0xx_hal::{prelude::*, timers::Event, usb::{Peripheral, UsbBus, UsbBusType}};

#[app(device = stm32f0xx_hal::pac, peripherals = true, dispatchers = [FLASH])]
mod app {
    use stm32f0xx_hal::timers::Timer;

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
        timer: stm32f0xx_hal::timers::Timer<stm32f0xx_hal::pac::TIM2>,
        button: stm32f0xx_hal::gpio::gpioa::PA2<stm32f0xx_hal::gpio::Input<stm32f0xx_hal::gpio::PullDown>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        #[cfg(feature = "defmt")]
        defmt::info!("Init...");

        let mut flash = ctx.device.FLASH;

        let mut rcc =  ctx.device.RCC
            .configure()
            .hsi48()
            .enable_crs(ctx.device.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut flash);

        #[cfg(feature = "defmt")]
        defmt::info!("Clocks ready");

        let gpioa = ctx.device.GPIOA.split(&mut rcc);

        let usb = stm32f0xx_hal::usb::Peripheral {
            usb: ctx.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: gpioa.pa12,
        };

        let usb_bus = cortex_m::singleton!(
            : usb_device::bus::UsbBusAllocator<UsbBus<Peripheral>> = UsbBus::new(usb)
        )
        .unwrap();

        let mut timer =  Timer::tim2(ctx.device.TIM2, (1_000 / config::HID_I2C_POLL_INTERVAL_MS as u32).hz(), &mut rcc);   
        timer.listen(Event::TimeOut);

        #[cfg(feature = "defmt")]
        defmt::info!("Timer ready");

        let hid_kbd = usbd_hid::hid_class::HIDClass::new(
            usb_bus,
            report::KeyboardReport::desc(),
            config::HID_I2C_POLL_INTERVAL_MS,
        );

        #[cfg(feature = "defmt")]
        defmt::info!("HID ready");

        let hid_ctrl = usbd_hid::hid_class::HIDClass::new(
            usb_bus,
            report::ControlDesctiptor::desc(),
            config::HID_I2C_POLL_INTERVAL_MS,
        );

        #[cfg(feature = "defmt")]
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

        #[cfg(feature = "defmt")]
        defmt::info!("USB device ready");

        let button = cortex_m::interrupt::free(|cs| gpioa.pa2.into_pull_down_input(cs));
        #[cfg(feature = "defmt")]
        defmt::info!("Button ready");

        //---------------------------------------------------------------------

        let storage = data_sorage::DataStorage::load(flash);

        #[cfg(feature = "defmt")]
        defmt::info!("Saved report: {}", storage.report_pattern);

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

        let new_state = button.is_high().unwrap();
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

        timer.wait().ok();
    }

    //-------------------------------------------------------------------------

    #[idle(shared = [usb_dev, hid_kbd, hid_ctrl, storage])]
    fn idle(ctx: idle::Context) -> ! {
        let mut ctrl_report = [0u8; 64];

        let mut usb_dev = ctx.shared.usb_dev;
        let mut hid_kbd = ctx.shared.hid_kbd;
        let mut hid_ctrl = ctx.shared.hid_ctrl;
        let mut storage = ctx.shared.storage;

        loop {
            if (&mut usb_dev, &mut hid_kbd, &mut hid_ctrl)
                .lock(|usb_dev, hid_kbd, hid_ctrl| usb_dev.poll(&mut [hid_kbd, hid_ctrl]))
            {
                let new_report_assigned = hid_ctrl.lock(|hid_ctrl| pull_raw_output(hid_ctrl, &mut ctrl_report));

                if let Some(pattern) = new_report_assigned {
                    storage.lock(|storage| store_report(pattern, storage));
                }
            }
        }
    }
}
