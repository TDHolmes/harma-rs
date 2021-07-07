#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [EVSYS])]
mod app {
    use pensel::cli;
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

    macro_rules! write_string {
        ($cx:tt, $($args:tt)*) => {{
            $cx.resources
                .usb_serial
                .lock(|usb_serial| pensel::serial_write!(usb_serial, $($args)*));
        }};
    }

    macro_rules! write_bytes {
        ($cx:tt, $message:expr) => {{
            $cx.resources
                .usb_serial
                .lock(|usb_serial| usb_serial.write($message));
        }};
    }

    #[resources]
    struct Resources {
        #[lock_free]
        red_led: gpio::Pin<gpio::PA17, gpio::Output<gpio::PushPull>>,
        usb_serial: usbserial::UsbSerial<'static>,
        #[lock_free]
        cli_output: heapless::spsc::Consumer<'static, u8, { cli::CLI_QUEUE_SIZE }>,
        #[lock_free]
        cli_runner: menu::Runner<'static, cli::CliOutput<{ cli::CLI_QUEUE_SIZE }>>,
        #[lock_free]
        cli_user_input_in: heapless::spsc::Producer<'static, u8, { cli::CLI_QUEUE_SIZE }>,
        #[lock_free]
        cli_user_input_out: heapless::spsc::Consumer<'static, u8, { cli::CLI_QUEUE_SIZE }>,
    }

    #[monotonic(binds = RTC, default = true)]
    type RtcMonotonic = Rtc<Count32Mode>;

    #[init]
    fn init(cx: init::Context) -> (init::LateResources, init::Monotonics) {
        // some static muts that can be safely used later on because RTIC is great
        static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
        static mut CLI_WRITE_QUEUE: heapless::spsc::Queue<u8, { cli::CLI_QUEUE_SIZE }> =
            heapless::spsc::Queue::new();
        static mut CLI_INPUT_QUEUE: heapless::spsc::Queue<u8, { cli::CLI_QUEUE_SIZE }> =
            heapless::spsc::Queue::new();
        static mut CLI_BUFFER: [u8; cli::CLI_QUEUE_SIZE] = [0; cli::CLI_QUEUE_SIZE];

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

        // CLI
        let (cli_producer, cli_output) = CLI_WRITE_QUEUE.split();
        let (cli_user_input_in, cli_user_input_out) = CLI_INPUT_QUEUE.split();
        let cli_writer = cli::CliOutput::new(cli_producer);
        let cli_runner = menu::Runner::new(&cli::ROOT_MENU, CLI_BUFFER, cli_writer);

        (
            init::LateResources {
                red_led,
                usb_serial,
                cli_output,
                cli_runner,
                cli_user_input_in,
                cli_user_input_out,
            },
            init::Monotonics(rtc),
        )
    }

    #[idle(resources = [usb_serial, cli_output, cli_runner, cli_user_input_out])]
    fn idle(mut cx: idle::Context) -> ! {
        let cli_user_input = cx.resources.cli_user_input_out;
        let cli_runner = cx.resources.cli_runner;
        let cli_output = cx.resources.cli_output;

        loop {
            while let Some(byte) = cli_user_input.dequeue() {
                cli_runner.input_byte(byte);
            }

            while let Some(byte) = cli_output.dequeue() {
                write_bytes!(cx, &[byte]);
            }

            cortex_m::asm::wfi();
        }
    }

    #[task(resources = [red_led])]
    fn blink(cx: blink::Context) {
        cx.resources.red_led.toggle().unwrap();
        blink::spawn_after(1_u32.seconds()).ok();
    }

    #[task(binds = USB, resources=[usb_serial, cli_user_input_in], priority = 2)]
    fn poll_usb(mut cx: poll_usb::Context) {
        let mut read_buffer: [u8; 32] = [0; 32];
        let bytes_read = cx
            .resources
            .usb_serial
            .lock(|usb_serial| usb_serial.poll(&mut read_buffer));

        if bytes_read != 0 {
            for byte in &read_buffer[0..bytes_read] {
                if cx.resources.cli_user_input_in.enqueue(*byte).is_err() {
                    write_string!(cx, "Err!\r\n");
                }
            }
        }
    }
}
