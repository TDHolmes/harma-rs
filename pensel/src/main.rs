#![no_std]
#![no_main]

use pensel::{bal, cli, imu::Imu, prelude::*, usb_serial, usb_serial_log};

use panic_persist as _;

use bsp::entry;
use hal::{delay::Delay, prelude::*};
use pac::{CorePeripherals, Peripherals};

#[entry]
fn main() -> ! {
    // initialize core peripherals
    let peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let (pins, mut board) = bal::Bal::init(peripherals);

    // initialize GPIOs
    let mut _red_led: bsp::RedLed = pins.d13.into();

    // initialize clocks/peripherals
    let i2c = board.i2c(400.khz(), pins.sda, pins.scl);
    let usb_allocator = board.usb_allocator(pins.usb_dp, pins.usb_dm);
    let mut delay = Delay::new(core.SYST, &mut board.clocks);

    usb_serial::init(&mut core.NVIC, usb_allocator);

    // Wait for us to have the terminal open
    while !usb_serial::user_present() {
        cortex_m::asm::wfi();
    }

    // initialize the CLI
    usb_serial_log::init().unwrap();
    let (cli_producer, mut cli_bytes_to_write) = unsafe { cli::CLI_OUTPUT_QUEUE.split() };
    let mut cli = cli::Cli::new(cli_producer);
    let mut serial_read_queue = usb_serial::get_serial_input_pipe();

    // Check if there was a panic message, if so, send to UART
    if let Some(msg) = panic_persist::get_panic_message_bytes() {
        log::error!("panic from previous boot:");
        let mut bytes_written = 0;
        // Write it out in chunks, waiting for USB interrupt handler to run in between before trying to shove more bytes
        while bytes_written != msg.len() {
            let chunk_written = usb_serial::get(|usbserial| usbserial.write(&msg[bytes_written..]));
            bytes_written += chunk_written;
            cortex_m::asm::wfi();
        }
    }

    let mut imu = Imu::new(&mut delay, i2c);

    // workloop forever
    loop {
        log::trace!("loop");
        // handle our CLI
        while let Some(new_byte) = cli_bytes_to_write.dequeue() {
            usb_serial::get(|usbserial| usbserial.write(&[new_byte]));
        }
        while let Some(new_byte) = serial_read_queue.dequeue() {
            cli.input_from_serial(new_byte);
        }

        // Get gravity vector
        let angles_res = imu.gravity_fixed();
        if let Some(angles) = angles_res {
            log::info!(
                "G:{},{},{}",
                (angles.x as isize * 10),
                (angles.y as isize * 10),
                (angles.z as isize * 10)
            );
        }

        // get acceleration
        let lin_accel = imu.linear_acceleration_fixed();
        if let Some(acc) = lin_accel {
            log::info!(
                "A:{},{},{}",
                (acc.x as isize * 10),
                (acc.y as isize * 10),
                (acc.z as isize * 10)
            );
        }
    }
}
