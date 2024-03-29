//! Takes care of all of the serial communication & parsing with Pensel
use heapless::spsc::Producer;
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::types;

const ACCEL_PREFIX: &str = "A:";
const GRAVITY_PREFIX: &str = "G:";

pub struct PenselSerial {
    port: Box<dyn serialport::SerialPort>,
}

impl PenselSerial {
    /// Creates a new instance of [`PenselSerial`] from the provided serial port trait object.
    #[must_use]
    pub fn new(port: Box<dyn serialport::SerialPort>) -> Self {
        Self { port }
    }

    /// Creates a new instance of [`PenselSerial`] from a port named `name`.
    ///
    /// # Panics
    /// If we fail to open the port specified.
    #[must_use]
    pub fn new_from_name(name: &str) -> Self {
        let port = serialport::new(name, 115_200)
            .timeout(std::time::Duration::from_millis(10))
            .open()
            .expect("Failed to open port");

        Self::new(port)
    }

    /// Makes a new instance of [`PenselSerial`] from the first serial port with `PENSEL` in its name.
    ///
    /// # Panics
    /// If we cannot find a matching port or none are available.
    #[must_use]
    pub fn new_first_matching() -> Self {
        let ports = serialport::available_ports().expect("No ports found!");
        for p in ports {
            if p.port_name.contains("PENSEL") {
                return Self::new_from_name(&p.port_name);
            }
        }

        panic!("no matching port found");
    }

    /// Sends the given command over serial. Currently doesn't check if pensel received it properly.
    ///
    /// # Errors
    /// If we fail to write out the command bytes or error out while waiting for a response.
    pub fn send_command(&mut self, command: &str) -> Result<(), serialport::Error> {
        log::debug!("sending command {:?}", command);
        self.port.write_all(command.as_bytes())?;
        self.port.write_all(b"\r")?;
        self.wait_for(command)?;
        Ok(())
    }

    fn wait_for(&mut self, line: &str) -> Result<(), std::io::Error> {
        let mut write_index = 0;
        let mut read_buf: [u8; 1024] = [0; 1024];

        loop {
            let size_read = self.port.read(&mut read_buf[write_index..])?;
            let sub_string =
                std::str::from_utf8(&read_buf[write_index..write_index + size_read]).unwrap();
            log::debug!("wait_for - {:?}", sub_string);
            write_index += size_read;
            let string = std::str::from_utf8(&read_buf[0..write_index]).unwrap();
            if string.contains(line) {
                return Ok(());
            }
            if write_index == read_buf.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::OutOfMemory,
                    "Ran out of buffer waiting",
                ));
            }
        }
    }

    #[must_use]
    /// Parses `line` into [`types::ParsedLine`].
    pub fn parse_line(line: &str) -> types::ParsedLine {
        if line.starts_with(ACCEL_PREFIX) {
            if let Ok(accel) = types::imu::AccelerationVector::from_str(line) {
                return types::ParsedLine::Accel(accel);
            }
        } else if line.starts_with(GRAVITY_PREFIX) {
            if let Ok(gravity) = types::imu::GravityVector::from_str(line) {
                return types::ParsedLine::Grav(gravity);
            }
        }

        types::ParsedLine::None
    }

    /// Parses data into `accel_queue` and `grav_queue` as long as `should_run` is `true`.
    ///
    /// # Panics
    /// If we get non UTF-8 compliant characters from the underlying serial port.
    pub fn parse_data_until(
        &mut self,
        mut accel_queue: Producer<types::imu::AccelerationVector, { types::ACC_QUEUE_SIZE }>,
        mut grav_queue: Producer<types::imu::GravityVector, { types::GRAV_QUEUE_SIZE }>,
        should_run: &Arc<AtomicBool>,
    ) {
        let mut serial_read_buf: [u8; 128] = [0; 128];
        let mut write_index: usize = 0;
        let mut read_index: usize = 0;

        loop {
            // read out some bytes and try to parse it, line by line
            let read_res = self.port.read(&mut serial_read_buf[write_index..]);
            if let Ok(bytes_read) = read_res {
                write_index += bytes_read;
            } else if let Err(error) = read_res {
                if error.kind() == std::io::ErrorKind::BrokenPipe {
                    eprintln!("serial port disconnected");
                    should_run.as_ref().store(false, Ordering::Release);
                }
            }

            let str_to_search = std::str::from_utf8(&serial_read_buf[0..write_index])
                .expect("non UTF-8 compliant characters received");
            for line in str_to_search.lines() {
                let mut parsed = true;
                let parsed_line = Self::parse_line(line);
                match parsed_line {
                    types::ParsedLine::Accel(acc) => accel_queue.enqueue(acc).unwrap_or(()),
                    types::ParsedLine::Grav(grav) => grav_queue.enqueue(grav).unwrap_or(()),
                    types::ParsedLine::None => parsed = false,
                };
                if parsed {
                    read_index += line.len();
                }
            }

            // check if we can reset our indices, or need to move an unparsed chunk forward
            if read_index == write_index {
                write_index = 0;
            } else {
                let unread_range = read_index..write_index;
                write_index = unread_range.len();
                serial_read_buf.copy_within(unread_range, 0);
            }
            read_index = 0;

            // check if we've been requested to halt
            if !should_run.as_ref().load(Ordering::Acquire) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod comm_test {
    use super::*;
    use crate::mock_serial::MockSerial;
    use heapless::spsc::Queue;

    static mut A_QUEUE: Queue<types::imu::AccelerationVector, { types::ACC_QUEUE_SIZE }> =
        Queue::new();
    static mut G_QUEUE: Queue<types::imu::GravityVector, { types::GRAV_QUEUE_SIZE }> = Queue::new();

    const EXAMPLE_ACCEL_LINE: &str = "A:1,2,3\n";
    const EXAMPLE_GRAVITY_LINE: &str = "G:1,2,3\n";
    const EXAMPLE_GRAVITY_LINE_NEG: &str = "G:-1,2,-3\n";

    #[test]
    fn create_pensel_serial() {
        let port = Box::new(MockSerial::default());
        let _ = PenselSerial::new(port);
    }

    #[test]
    fn parse_accel() {
        let res = PenselSerial::parse_line(EXAMPLE_ACCEL_LINE);
        let accel_pkt = match res {
            types::ParsedLine::Accel(g) => g,
            _ => panic!(
                "Line {} parsed incorrectly to {:#?}",
                EXAMPLE_ACCEL_LINE, res
            ),
        };

        assert_eq!(accel_pkt.x, 1);
        assert_eq!(accel_pkt.y, 2);
        assert_eq!(accel_pkt.z, 3);
    }

    #[test]
    fn parse_grav() {
        let res = PenselSerial::parse_line(EXAMPLE_GRAVITY_LINE);
        let grav_pkt = match res {
            types::ParsedLine::Grav(g) => g,
            _ => panic!(
                "Line '{}' parsed incorrectly to {:#?}",
                EXAMPLE_GRAVITY_LINE, res
            ),
        };

        assert_eq!(grav_pkt.x, 1);
        assert_eq!(grav_pkt.y, 2);
        assert_eq!(grav_pkt.z, 3);
    }

    #[test]
    fn parse_grav_neg() {
        let res = PenselSerial::parse_line(EXAMPLE_GRAVITY_LINE_NEG);
        let grav_pkt = match res {
            types::ParsedLine::Grav(g) => g,
            _ => panic!(
                "Line '{}' parsed incorrectly to {:#?}",
                EXAMPLE_GRAVITY_LINE_NEG, res
            ),
        };

        assert_eq!(grav_pkt.x, -1);
        assert_eq!(grav_pkt.y, 2);
        assert_eq!(grav_pkt.z, -3);
    }

    #[test]
    fn parse_garbage() {
        let garbage_line = "derpy derp\n";
        let res = PenselSerial::parse_line(garbage_line);
        match res {
            types::ParsedLine::None => (),
            _ => panic!("Line '{}' parsed incorrectly to {:#?}", garbage_line, res),
        }
    }

    #[test]
    fn parse_until_basic() {
        use std::io::Write;

        let should_run = Arc::new(AtomicBool::new(true));
        let should_run_thread_ref = should_run.clone();

        let mut port = Box::new(MockSerial::default());

        // prime the pipes with some lovely data
        port.write(EXAMPLE_ACCEL_LINE.as_bytes()).unwrap();
        port.write(EXAMPLE_GRAVITY_LINE.as_bytes()).unwrap();

        let mut serial = PenselSerial::new(port);

        let (a_producer, mut a_consumer) = unsafe { A_QUEUE.split() };
        let (g_producer, mut g_consumer) = unsafe { G_QUEUE.split() };

        let sender = std::thread::spawn(move || {
            serial.parse_data_until(a_producer, g_producer, &should_run_thread_ref);
        });

        let (mut accel_received, mut gravity_received) = (false, false);
        while !accel_received || !gravity_received {
            if let Some(a) = a_consumer.dequeue() {
                log::debug!("A: {:?}", a);
                accel_received = true;
            }
            if let Some(g) = g_consumer.dequeue() {
                log::debug!("G: {:?}", g);
                gravity_received = true;
            }
        }

        should_run.store(false, std::sync::atomic::Ordering::Release);
        sender.join().unwrap();
    }
}
