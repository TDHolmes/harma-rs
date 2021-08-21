//! Takes care of all of the serial communication & parsing with Pensel
use heapless::spsc::Producer;
use regex::Regex;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::types;

pub struct PenselSerial {
    port: Box<dyn serialport::SerialPort>,
    re_accel: Regex,
    re_gravity: Regex,
}

impl PenselSerial {
    /// Creates a new instance of PenselSerial from the provided serial port trait object
    pub fn new(port: Box<dyn serialport::SerialPort>) -> PenselSerial {
        let re_accel = Regex::new(r"A:([-\d]+),([-\d]+),([-\d]+)").unwrap();
        let re_gravity = Regex::new(r"O:([-\d]+),([-\d]+),([-\d]+)").unwrap();
        PenselSerial {
            port,
            re_accel,
            re_gravity,
        }
    }

    /// Creates a new instance of PenselSerial from a port named `name`
    pub fn new_from_name(name: &str) -> PenselSerial {
        let port = serialport::new(name, 115_200)
            .timeout(std::time::Duration::from_millis(10))
            .open()
            .expect("Failed to open port");

        PenselSerial::new(port)
    }

    /// Makes a new instance of PenselSerial from the first serial port with `PENSEL` in its name
    pub fn new_first_matching() -> PenselSerial {
        let ports = serialport::available_ports().expect("No ports found!");
        for p in ports {
            if p.port_name.contains("PENSEL") {
                return PenselSerial::new_from_name(&p.port_name);
            }
        }

        panic!("no matching port found");
    }

    pub fn read_raw(&mut self, buffer: &mut [u8]) -> usize {
        if let Ok(len_read) = self.port.read(buffer) {
            len_read
        } else {
            0
        }
    }

    pub fn parse_line(&self, line: &str) -> types::ParsedLine {
        if let Some(accel) = self.re_accel.captures(line) {
            let x = accel.get(1).unwrap().as_str().parse::<isize>().unwrap();
            let y = accel.get(2).unwrap().as_str().parse::<isize>().unwrap();
            let z = accel.get(3).unwrap().as_str().parse::<isize>().unwrap();
            return types::ParsedLine::Accel(types::AccelerationVec { x, y, z });
        } else if let Some(grav) = self.re_gravity.captures(line) {
            let x = grav.get(1).unwrap().as_str().parse::<isize>().unwrap();
            let y = grav.get(2).unwrap().as_str().parse::<isize>().unwrap();
            let z = grav.get(3).unwrap().as_str().parse::<isize>().unwrap();
            return types::ParsedLine::Grav(types::GravityVec { x, y, z });
        }

        types::ParsedLine::None
    }

    pub fn parse_data_until(
        &mut self,
        mut accel_queue: Producer<types::AccelerationVec, { types::ACC_QUEUE_SIZE }>,
        mut grav_queue: Producer<types::AccelerationVec, { types::GRAV_QUEUE_SIZE }>,
        should_run: Arc<AtomicBool>,
    ) {
        let mut serial_read_buf: [u8; 128] = [0; 128];
        let mut write_index: usize = 0;
        let mut read_index: usize = 0;

        loop {
            // read out some bytes and try to parse it, line by line
            let bytes_read = self.read_raw(&mut serial_read_buf[write_index..]);
            write_index += bytes_read;

            let str_to_search = std::str::from_utf8(&serial_read_buf[0..write_index]).unwrap();
            for line in str_to_search.lines() {
                let mut parsed = true;
                let parsed_line = self.parse_line(line);
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

    static mut A_QUEUE: Queue<types::AccelerationVec, 4> = Queue::new();
    static mut G_QUEUE: Queue<types::GravityVec, 4> = Queue::new();

    const EXAMPLE_ACCEL_LINE: &str = "A:1,2,3\n";
    const EXAMPLE_GRAVITY_LINE: &str = "O:1,2,3\n";

    #[test]
    fn create_pensel_serial() {
        let port = Box::new(MockSerial::default());
        let _ = PenselSerial::new(port);
    }

    #[test]
    fn parse_accel() {
        let port = Box::new(MockSerial::default());
        let comm = PenselSerial::new(port);

        let res = comm.parse_line(EXAMPLE_ACCEL_LINE);
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
        let port = Box::new(MockSerial::default());
        let comm = PenselSerial::new(port);

        let res = comm.parse_line(EXAMPLE_GRAVITY_LINE);
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
    fn parse_garbage() {
        let port = Box::new(MockSerial::default());
        let comm = PenselSerial::new(port);

        let garbage_line = "derpy derp\n";
        let res = comm.parse_line(garbage_line);
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
            serial.parse_data_until(a_producer, g_producer, should_run_thread_ref)
        });

        let (mut accel_received, mut gravity_recevied) = (false, false);
        while !accel_received || !gravity_recevied {
            if let Some(a) = a_consumer.dequeue() {
                println!("A: {:?}", a);
                accel_received = true;
            }
            if let Some(g) = g_consumer.dequeue() {
                println!("G: {:?}", g);
                gravity_recevied = true;
            }
        }

        should_run.store(false, std::sync::atomic::Ordering::Release);
        sender.join().unwrap();
    }
}
