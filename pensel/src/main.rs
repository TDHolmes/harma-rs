#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [EVSYS])]
mod app {
    use pensel::prelude::*;
    use pensel::usbserial;

    use cortex_m::peripheral::NVIC;
    use hal::clock::{ClockGenId, ClockSource, GenericClockController};
    use hal::gpio::v2 as gpio;
    use hal::pac::{interrupt, Peripherals};
    use hal::prelude::*;
    use hal::rtc::{Count32Mode, Rtc};
    use rtic_monotonic::Extensions;

    use hal::usb::UsbBus;
    use usb_device::bus::UsbBusAllocator;
    use usb_device::prelude::*;
    use usbd_serial::{SerialPort, USB_CLASS_CDC};

    #[resources]
    struct Resources {
        #[lock_free]
        red_led: gpio::Pin<gpio::PA17, gpio::Output<gpio::PushPull>>,
        usb_serial: usbserial::UsbSerial<'static>,
    }

    #[monotonic(binds = RTC, default = true)]
    type RtcMonotonic = Rtc<Count32Mode>;

    #[init]
    fn init(cx: init::Context) -> (init::LateResources, init::Monotonics) {
        static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;

        // setup some basic peripherals...
        let mut peripherals: Peripherals = cx.device;
        let pins = bsp::Pins::new(peripherals.PORT);
        let mut core: rtic::export::Peripherals = cx.core;
        let mut clocks = GenericClockController::with_external_32kosc(
            peripherals.GCLK,
            &mut peripherals.PM,
            &mut peripherals.SYSCTRL,
            &mut peripherals.NVMCTRL,
        );

        // ... configure RTC
        let rtc_clock_src = clocks
            .configure_gclk_divider_and_source(ClockGenId::GCLK2, 1, ClockSource::XOSC32K, false)
            .unwrap();
        clocks.configure_standby(ClockGenId::GCLK2, true);
        let rtc_clock = clocks.rtc(&rtc_clock_src).unwrap();
        let rtc = Rtc::count32_mode(peripherals.RTC, rtc_clock.freq(), &mut peripherals.PM);

        // ... setup USB stuff
        *USB_ALLOCATOR = Some(bsp::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        let bus_allocator = USB_ALLOCATOR.as_ref().unwrap();
        let usb_serial = SerialPort::new(&bus_allocator);
        let usb_dev = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .build();
        let usb_serial = usbserial::UsbSerial::new(usb_serial, usb_dev);
        unsafe {
            // enable interrupts
            core.NVIC.set_priority(interrupt::USB, 1);
            NVIC::unmask(interrupt::USB);
        }

        // ... Red LED to blink
        let red_led: bsp::RedLed = pins.d13.into();
        blink::spawn().unwrap();

        (
            init::LateResources {
                red_led,
                usb_serial,
            },
            init::Monotonics(rtc),
        )
    }

    #[task(resources = [red_led, usb_serial])]
    fn blink(mut cx: blink::Context) {
        cx.resources.red_led.toggle().unwrap();
        blink::spawn_after(1_u32.seconds()).ok();
        cx.resources
            .usb_serial
            .lock(|usb_serial| pensel::serial_write!(usb_serial, "blink!\r\n"));
    }

    #[task(binds = USB, resources=[usb_serial], priority = 2)]
    fn poll_usb(mut cx: poll_usb::Context) {
        let mut read_buffer: [u8; 32] = [0; 32];
        cx.resources
            .usb_serial
            .lock(|usb_serial| usb_serial.poll(&mut read_buffer));
    }
}
