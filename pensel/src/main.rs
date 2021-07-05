#![no_std]
#![no_main]

use panic_halt as _;
use rtic;

extern crate feather_m0 as bsp;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [EVSYS])]
mod app {

    use bsp::hal;
    use hal::clock::{ClockGenId, ClockSource, GenericClockController};
    use hal::gpio::v2 as gpio;
    use hal::pac::Peripherals;
    use hal::prelude::*;
    use hal::rtc::{Count32Mode, Rtc};
    use rtic_monotonic::Extensions;

    #[resources]
    struct Resources {
        red_led: gpio::Pin<gpio::PA17, gpio::Output<gpio::PushPull>>,
    }

    #[monotonic(binds = RTC, default = true)]
    type RtcMonotonic = Rtc<Count32Mode>;

    #[init]
    fn init(cx: init::Context) -> (init::LateResources, init::Monotonics) {
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
        // We can use the RTC in standby for maximum power savings
        core.SCB.set_sleepdeep();

        // ... configure RTC
        let rtc_clock_src = clocks
            .configure_gclk_divider_and_source(ClockGenId::GCLK2, 1, ClockSource::XOSC32K, false)
            .unwrap();
        clocks.configure_standby(ClockGenId::GCLK2, true);
        let rtc_clock = clocks.rtc(&rtc_clock_src).unwrap();
        let rtc = Rtc::count32_mode(peripherals.RTC, rtc_clock.freq(), &mut peripherals.PM);

        // ... Red LED to blink
        let red_led: bsp::RedLed = pins.d13.into();
        blink::spawn().unwrap();

        (init::LateResources { red_led }, init::Monotonics(rtc))
    }

    #[task(resources = [red_led])]
    fn blink(mut _cx: blink::Context) {
        _cx.resources.red_led.lock(|led| led.toggle().unwrap());
        blink::spawn_after(1_u32.seconds()).ok();
    }
}
