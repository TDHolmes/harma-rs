//! Takes care of all of the serial communication & parsing with Pensel
use heapless::spsc::Producer;
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::types;

pub struct PenselSerial {
    port: Box<dyn serialport::SerialPort>,
}

impl PenselSerial {
    pub fn new(name: &str) -> PenselSerial {
        let port = serialport::new(name, 115_200)
            .timeout(std::time::Duration::from_millis(10))
            .open()
            .expect("Failed to open port");

        PenselSerial { port }
    }

    pub fn new_first_matching() -> PenselSerial {
        let ports = serialport::available_ports().expect("No ports found!");
        for p in ports {
            if p.port_name.contains("PENSEL") {
                return PenselSerial::new(&p.port_name);
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

    pub fn parse_data_until(
        &mut self,
        mut accel_queue: Producer<types::AccelerationVec, 4>,
        mut grav_queue: Producer<types::AccelerationVec, 4>,
        should_run: &AtomicBool,
    ) {
        let mut serial_read_buf: [u8; 128] = [0; 128];
        let mut write_index: usize = 0;
        let mut read_index: usize = 0;

        let re_accel = Regex::new(r"A:([-\d]+),([-\d]+),([-\d]+)").unwrap();
        let re_gravity = Regex::new(r"O:([-\d]+),([-\d]+),([-\d]+)").unwrap();
        while should_run.load(Ordering::Acquire) {
            // read out some bytes and try to parse it, line by line
            let bytes_read = self.read_raw(&mut serial_read_buf[write_index..]);
            write_index += bytes_read;

            let str_to_search = std::str::from_utf8(&serial_read_buf[0..write_index]).unwrap();
            for line in str_to_search.lines() {
                if let Some(accel) = re_accel.captures(line) {
                    let x = accel.get(1).unwrap().as_str().parse::<isize>().unwrap();
                    let y = accel.get(2).unwrap().as_str().parse::<isize>().unwrap();
                    let z = accel.get(3).unwrap().as_str().parse::<isize>().unwrap();
                    let accel_pkt = types::AccelerationVec { x, y, z };
                    let _ = accel_queue.enqueue(accel_pkt); // enqueue & don't mind if it fails

                    read_index += line.len();
                } else if let Some(grav) = re_gravity.captures(line) {
                    let x = grav.get(1).unwrap().as_str().parse::<isize>().unwrap();
                    let y = grav.get(2).unwrap().as_str().parse::<isize>().unwrap();
                    let z = grav.get(3).unwrap().as_str().parse::<isize>().unwrap();
                    let grav_pkt = types::GravityVec { x, y, z };
                    let _ = grav_queue.enqueue(grav_pkt); // enqueue & don't mind if it fails

                    read_index += line.len();
                }
            }

            // check if we can reset our indices, or need to move an unparsed chunk forward
            if read_index == write_index {
                write_index = 0;
                read_index = 0;
            } else {
                let unread_range = read_index..write_index;
                write_index = unread_range.len();
                serial_read_buf.copy_within(unread_range, 0);
                read_index = 0;
            }
        }
    }
}
