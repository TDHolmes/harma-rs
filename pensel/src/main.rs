#![no_std]
#![no_main]

use pensel::{bsp, cli, hal, imu::Imu, pac, serial_write, usb_serial};

use heapless::spsc::Queue;
use panic_persist as _;

use bsp::entry;
use hal::{clock::GenericClockController, delay::Delay, prelude::*};
use pac::{CorePeripherals, Peripherals};

/// The queue for our CLI abstraction to write out to the serial port
static mut CLI_OUTPUT_QUEUE: Queue<u8, { cli::CLI_QUEUE_SIZE }> = Queue::new();

#[entry]
fn main() -> ! {
    // initialize core peripherals
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);

    // initialize GPIOs
    let pins = bsp::Pins::new(peripherals.PORT);
    let mut _red_led: bsp::RedLed = pins.d13.into();
    let i2c = bsp::i2c_master(
        &mut clocks,
        400.khz(),
        peripherals.SERCOM3,
        &mut peripherals.PM,
        pins.sda,
        pins.scl,
    );

    // initialize USB
    usb_serial::init(
        peripherals.USB,
        &mut core.NVIC,
        &mut clocks,
        &mut peripherals.PM,
        pins.usb_dm,
        pins.usb_dp,
    );

    // Wait for us to have the terminal open
    while !usb_serial::user_present() {
        cortex_m::asm::wfi();
    }

    // initialize the CLI
    let (cli_producer, mut cli_bytes_to_write) = unsafe { CLI_OUTPUT_QUEUE.split() };
    let mut cli = cli::Cli::new(cli_producer);
    let mut serial_read_queue = usb_serial::get_serial_input_pipe();

    // Check if there was a panic message, if so, send to UART
    if let Some(msg) = panic_persist::get_panic_message_bytes() {
        serial_write!("panic from previous boot:\n");
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
        // handle our CLI
        if let Some(new_byte) = cli_bytes_to_write.dequeue() {
            usb_serial::get(|usbserial| usbserial.write(&[new_byte]));
        }
        if let Some(new_byte) = serial_read_queue.dequeue() {
            cli.input_from_serial(new_byte);
        }

        // Get gravity vector
        let angles_res = imu.gravity_fixed();
        if let Ok(angles) = angles_res {
            serial_write!(
                "G:{},{},{}\n",
                (angles.x as isize * 10),
                (angles.y as isize * 10),
                (angles.z as isize * 10)
            );
        }

        // get acceleration
        let lin_accel = imu.linear_acceleration_fixed();
        if let Ok(acc) = lin_accel {
            serial_write!(
                "A:{},{},{}\n",
                (acc.x as isize * 10),
                (acc.y as isize * 10),
                (acc.z as isize * 10)
            );
        }
    }
}
