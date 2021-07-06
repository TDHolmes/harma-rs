use crate::prelude::*;

use hal::usb::UsbBus;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

pub struct UsbSerial<'a> {
    usb_serial: SerialPort<'a, UsbBus>,
    usb_dev: UsbDevice<'a, UsbBus>,
}

impl<'a> UsbSerial<'a> {
    pub fn new(
        usb_serial: SerialPort<'a, UsbBus>,
        usb_dev: UsbDevice<'a, UsbBus>,
    ) -> UsbSerial<'a> {
        UsbSerial {
            usb_serial,
            usb_dev,
        }
    }

    pub fn poll(&mut self, read_buffer: &mut [u8]) -> usize {
        let mut total_bytes_read = 0;
        self.usb_dev.poll(&mut [&mut self.usb_serial]);

        if let Ok(bytes_read) = self.usb_serial.read(read_buffer) {
            total_bytes_read = bytes_read;
        }

        total_bytes_read
    }

    /// Writes a message over USB serial
    ///
    /// # Arguments
    /// * message: The message to write to the USB port
    ///
    /// # Returns
    /// number of bytes successfully written
    pub fn write_to_usb(&mut self, message: &str) -> usize {
        let message_bytes = message.as_bytes();
        match self.usb_serial.write(message_bytes) {
            Ok(count) => count,
            Err(_) => 0,
        }
    }
}

/// Writes the given message out over USB serial.
#[macro_export]
macro_rules! serial_write {
    ($usbserial:tt, $($tt:tt)+) => {{
        let mut s: heapless::String<64> = heapless::String::new();
        ufmt::uwrite!(
            ufmt_utils::WriteAdapter(&mut s), $($tt)*
        )
        .unwrap();
        $usbserial.write_to_usb(s.as_str());
    }};
}
