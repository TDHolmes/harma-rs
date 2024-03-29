//! Manages the command line interface. Uses `menu` under the hood.
use heapless::spsc::{Producer, Queue};

use pensel_types::cli as pt_cli;

/// The size of our CLI queue structures. Current largest output: `help imu` at ~200 bytes
pub const CLI_QUEUE_SIZE: usize = 512;

static mut MENU_BUFFER: [u8; CLI_QUEUE_SIZE] = [0; CLI_QUEUE_SIZE];

/// The queue for our CLI abstraction to write out to the serial port
pub static mut CLI_OUTPUT_QUEUE: Queue<u8, { CLI_QUEUE_SIZE }> = Queue::new();

/// How our `menu` based CLI outputs to the user. Not for direct consumption.
pub struct Output<'a, const N: usize> {
    /// Bytes coming from our CLI to be output to the serial port
    cli_output_queue: Producer<'a, u8, N>,
}

impl<'a, const N: usize> Output<'a, { N }> {
    const fn new(cli_output_queue: Producer<'a, u8, N>) -> Output<'a, N> {
        Output { cli_output_queue }
    }
}

impl<'a, const N: usize> core::fmt::Write for Output<'a, { N }> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        #[cfg(debug_assertions)]
        let high_watermark = N * 3 / 4;

        for byte in s.bytes() {
            #[cfg(debug_assertions)]
            {
                if self.cli_output_queue.len() > high_watermark {
                    // dump out CLI queue bytes. Disable interrupts to make this
                    //   sketchy global unsafe mess a little less unsafe. We're
                    //   crashing and burning anyways
                    cortex_m::interrupt::free(|_| unsafe {
                        let cli_queue = &mut CLI_OUTPUT_QUEUE;
                        let recent_bytes = [
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                            cli_queue.dequeue_unchecked(),
                        ];
                        panic!(
                            "Output hit high watermark. Recent input: '{}'",
                            core::str::from_utf8_unchecked(&recent_bytes)
                        );
                    });
                }
            }
            if self.cli_output_queue.enqueue(byte).is_err() {
                return Err(core::fmt::Error);
            }
        }
        Ok(())
    }
}

/// The type we need to return if we want an item in the CLI
pub type Item = menu::Item<'static, Output<'static, CLI_QUEUE_SIZE>>;

/// Our encapsulation of the CLI
pub struct Cli<'a, const N: usize> {
    /// the CLI runner
    runner: menu::Runner<'a, Output<'a, N>>,
}

impl<'a> Cli<'a, CLI_QUEUE_SIZE> {
    /// Creates our Cli encapsulation
    ///
    /// # Parameters
    /// `cli_output_queue`: where we write our bytes to be sent to the serial port by the application
    #[must_use]
    pub fn new(
        cli_output_queue: Producer<'static, u8, CLI_QUEUE_SIZE>,
    ) -> Cli<'static, CLI_QUEUE_SIZE> {
        let buffer = unsafe { &mut MENU_BUFFER };
        let runner = menu::Runner::new(&ROOT_MENU, buffer, Output::new(cli_output_queue));

        Cli { runner }
    }

    /// Give a byte coming from our serial connection to our CLI runner
    pub fn input_from_serial(&mut self, byte: u8) {
        self.runner.input_byte(byte);
    }

    /// Give the bytes coming from our serial connection to our CLI runner
    pub fn input_from_serial_bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.input_from_serial(*b);
        }
    }
}

const PANIC_CLI_ITEM: Item = Item {
    item_type: menu::ItemType::Callback {
        function: panic,
        parameters: &[],
    },
    command: pt_cli::CMD_PANIC,
    help: Some("Tests our panic handling by forcing one to happen"),
};

const RESET_CLI_ITEM: Item = Item {
    item_type: menu::ItemType::Callback {
        function: reset,
        parameters: &[],
    },
    command: pt_cli::CMD_RESET,
    help: Some("initiates an MCU reset"),
};

const ROOT_MENU: menu::Menu<Output<CLI_QUEUE_SIZE>> = menu::Menu {
    label: "root",
    items: &[
        &PANIC_CLI_ITEM,
        &RESET_CLI_ITEM,
        &crate::imu::IMU_CLI_ITEM,
        &crate::usb_serial_log::LOG_CLI_ITEM,
    ],
    entry: None,
    exit: None,
};

fn panic<const N: usize>(
    _menu: &menu::Menu<Output<N>>,
    _item: &menu::Item<Output<N>>,
    _args: &[&str],
    _context: &mut Output<N>,
) {
    panic!("test panic");
}

fn reset<const N: usize>(
    _menu: &menu::Menu<Output<N>>,
    _item: &menu::Item<Output<N>>,
    _args: &[&str],
    _context: &mut Output<N>,
) {
    cortex_m::peripheral::SCB::sys_reset();
}
