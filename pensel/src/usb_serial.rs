//! helper structure for working with USB serial communication
use crate::prelude::*;

use usb_device::{class_prelude::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use hal::{clock::GenericClockController, usb::UsbBus};

pub struct UsbSerial<'a> {
    usb_serial: SerialPort<'a, UsbBus>,
    usb_dev: UsbDevice<'a, UsbBus>,
}

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;

impl<'a> UsbSerial<'a> {
    /// Initializes everything we need for USB serial communication
    pub fn new(
        usb: pac::USB,
        clocks: &mut GenericClockController,
        pm: &mut pac::PM,
        dm: impl Into<bsp::UsbDm>,
        dp: impl Into<bsp::UsbDp>,
    ) -> UsbSerial<'a> {
        let usb_allocator = unsafe {
            USB_ALLOCATOR = Some(bsp::usb_allocator(usb, clocks, pm, dm, dp));
            USB_ALLOCATOR.as_ref().unwrap()
        };
        let usb_serial = SerialPort::new(usb_allocator);
        let usb_dev = UsbDeviceBuilder::new(usb_allocator, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .build();
        UsbSerial {
            usb_serial,
            usb_dev,
        }
    }

    /// Polls the USB device and reads out any available serial data.
    ///
    /// To be called in the `USB` interrupt handler.
    pub fn poll(&mut self, read_buffer: &mut [u8]) -> usize {
        let mut total_bytes_read = 0;
        self.usb_dev.poll(&mut [&mut self.usb_serial]);

        if let Ok(bytes_read) = self.usb_serial.read(read_buffer) {
            total_bytes_read = bytes_read;
        }

        total_bytes_read
    }

    /// Writes bytes to USB serial
    ///
    /// # Arguments
    /// * bytes: raw bytes to write
    ///
    /// # Returns
    /// Number of bytes successfully written
    pub fn write(&mut self, bytes: &[u8]) -> usize {
        match self.usb_serial.write(bytes) {
            Ok(count) => count,
            Err(_) => 0,
        }
    }

    /// Writes a message over USB serial
    ///
    /// # Arguments
    /// * message: The message to write to the USB port
    ///
    /// # Returns
    /// number of bytes successfully written
    pub fn write_str(&mut self, message: &str) -> usize {
        let message_bytes = message.as_bytes();
        match self.usb_serial.write(message_bytes) {
            Ok(count) => count,
            Err(_) => 0,
        }
    }
}

/// Writes the given message out over USB serial.
///
/// # Arguments
/// * usbserial: The `static mut Option<UsbSerial>`
/// * println args: variable arguments passed along to `ufmt::uwrite!`
///
/// # Warning
/// as this function deals with a static mut, and it is also accessed in the
/// USB interrupt handler, we both have unsafe code for unwrapping a static mut
/// as well as disabling of interrupts while we do so.
///
/// # Safety
/// the only time the static mut is used, we have interrupts disabled so we know
/// we have sole access
#[macro_export]
macro_rules! serial_write {
    ($usbserial:ident, $($tt:tt)+) => {{
        let mut s: heapless::String<64> = heapless::String::new();
        ufmt::uwrite!(
            ufmt_utils::WriteAdapter(&mut s), $($tt)*
        )
        .unwrap();
        cortex_m::interrupt::free(|_| {
            unsafe {
                $usbserial.as_mut().unwrap().write_str(s.as_str());
            }
        });
    }};
}
