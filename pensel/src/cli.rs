//! Manages the command line interface. Uses `menu` under the hood.
use heapless::spsc::Producer;
use menu::{Item, ItemType, Menu};

pub const CLI_QUEUE_SIZE: usize = 256;

static mut MENU_BUFFER: [u8; CLI_QUEUE_SIZE] = [0; CLI_QUEUE_SIZE];

struct CliOutput<'a, const N: usize> {
    /// Bytes coming from our CLI to be output to the serial port
    cli_output_queue: Producer<'a, u8, N>,
}

impl<'a, const N: usize> CliOutput<'a, { N }> {
    fn new(cli_output_queue: Producer<'a, u8, N>) -> CliOutput<'a, N> {
        CliOutput { cli_output_queue }
    }
}

impl<'a, const N: usize> core::fmt::Write for CliOutput<'a, { N }> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        for byte in s.bytes() {
            if self.cli_output_queue.enqueue(byte).is_err() {
                return Err(core::fmt::Error);
            }
        }
        Ok(())
    }
}

/// Our encapsulation of the CLI
pub struct Cli<'a, const N: usize> {
    /// the CLI runner
    runner: menu::Runner<'a, CliOutput<'a, N>>,
}

impl<'a> Cli<'a, CLI_QUEUE_SIZE> {
    /// Creates our Cli encapsulation
    ///
    /// # Parameters
    /// `cli_output_queue`: where we write our bytes to be sent to the serial port by the application
    pub fn new(
        cli_output_queue: Producer<'static, u8, CLI_QUEUE_SIZE>,
    ) -> Cli<'static, CLI_QUEUE_SIZE> {
        let buffer = unsafe { &mut MENU_BUFFER };
        let runner = menu::Runner::new(&ROOT_MENU, buffer, CliOutput::new(cli_output_queue));

        Cli { runner }
    }

    /// Give a byte coming from our serial connection to our CLI runner
    pub fn input_from_serial(&mut self, byte: u8) {
        self.runner.input_byte(byte);
    }

    /// Give the bytes coming from our serial connection to our CLI runner
    pub fn input_from_serial_bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.input_from_serial(*b)
        }
    }
}

const ROOT_MENU: Menu<CliOutput<CLI_QUEUE_SIZE>> = Menu {
    label: "root",
    items: &[&Item {
        item_type: ItemType::Callback {
            function: panic,
            parameters: &[],
        },
        command: "panic",
        help: Some("Tests our panic handling by forcing one to happen"),
    }],
    entry: None,
    exit: None,
};

fn panic<const N: usize>(
    _menu: &Menu<CliOutput<N>>,
    _item: &Item<CliOutput<N>>,
    _args: &[&str],
    _context: &mut CliOutput<N>,
) {
    panic!("test panic");
}
