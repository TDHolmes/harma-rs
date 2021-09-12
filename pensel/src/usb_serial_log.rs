//! implementation of `log`
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

use core::sync::atomic;

use crate::{cli, serial_write, usb_serial};
use pensel_types::cli as pt_cli;

struct UsbSerialLogger {
    enabled_level: atomic::AtomicUsize,
}

static LOGGER: UsbSerialLogger = UsbSerialLogger::new();

impl UsbSerialLogger {
    const fn new() -> UsbSerialLogger {
        UsbSerialLogger {
            // 0 is not a valid log level, but can't seem to initialize here for some reason, so do it later in `init`
            enabled_level: atomic::AtomicUsize::new(0),
        }
    }

    fn set_level(&self, new_level: Level) {
        self.enabled_level
            .store(new_level as usize, atomic::Ordering::Release);
    }

    fn get_level(&self) -> Level {
        let level = self.enabled_level.load(atomic::Ordering::Acquire);

        // I hate this, but `log` doesn't expose it's `from_usize` method
        if level == Level::Error as usize {
            Level::Error
        } else if level == Level::Warn as usize {
            Level::Warn
        } else if level == Level::Info as usize {
            Level::Info
        } else if level == Level::Debug as usize {
            Level::Debug
        } else if level == Level::Trace as usize {
            Level::Trace
        } else {
            panic!("impossible log level: {}", level);
        }
    }
}

impl log::Log for UsbSerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() as usize <= self.enabled_level.load(atomic::Ordering::Acquire)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();
            if level != Level::Info {
                usb_serial::get(|_| serial_write!("{}: {}\n", record.level(), record.args()));
            } else {
                // leave INFO prefix off for expected normal output
                usb_serial::get(|_| serial_write!("{}\n", record.args()));
            }
        }
    }

    fn flush(&self) {}
}

/// Initializes the USB serial based logger
pub fn init() -> Result<(), SetLoggerError> {
    #[cfg(debug_assertions)]
    let max_level = LevelFilter::Trace;
    #[cfg(not(debug_assertions))]
    let max_level = LevelFilter::Debug;

    #[cfg(debug_assertions)]
    const LEVEL: Level = Level::Debug;
    #[cfg(not(debug_assertions))]
    const LEVEL: Level = Level::Info;

    LOGGER.set_level(LEVEL);
    cortex_m::interrupt::free(|_| unsafe {
        log::set_logger_racy(&LOGGER).map(|()| log::set_max_level(max_level))
    })
}

fn log_control<const N: usize>(
    _menu: &menu::Menu<cli::CliOutput<N>>,
    item: &menu::Item<cli::CliOutput<N>>,
    args: &[&str],
    context: &mut cli::CliOutput<N>,
) {
    use core::fmt::Write;

    let mut handled = false;
    if let Ok(Some(level)) = menu::argument_finder(item, args, pt_cli::ARG_LEVEL_SET) {
        use core::str::FromStr;

        if let Ok(level) = Level::from_str(level) {
            LOGGER.set_level(level);
            handled = true;
        } else {
            writeln!(context, "failed to parse '{}'", level).unwrap();
        }
    }
    if let Ok(Some(_)) = menu::argument_finder(item, args, pt_cli::ARG_LEVEL_GET) {
        writeln!(context, "level: {}\n", LOGGER.get_level().as_str()).unwrap();
        handled = true;
    }

    if !handled {
        writeln!(context, "invalid usage").unwrap();
    }
}

/// Method to put our CLI entry in for IMU control
pub const LOG_CLI_ITEM: cli::CliItem = cli::CliItem {
    item_type: menu::ItemType::Callback {
        function: log_control,
        parameters: &[
            menu::Parameter::NamedValue {
                parameter_name: pt_cli::ARG_LEVEL_SET,
                argument_name: pt_cli::ARG_LEVEL_SET,
                help: Some("sets our log level"),
            },
            menu::Parameter::Named {
                parameter_name: pt_cli::ARG_LEVEL_GET,
                help: Some("gets our log level"),
            },
        ],
    },
    command: pt_cli::CMD_LOG,
    help: Some("Controls how our log functions"),
};
