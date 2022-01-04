use super::BoardAbstractionLayer;
use crate::prelude::*;
use hal::{clock::GenericClockController, time::Hertz, usb::UsbBus};

use usb_device::class_prelude::UsbBusAllocator;

/// Our board abstraction layer encapsulation
pub struct Bal {
    pm: pac::PM,
    i2c_sercom: Option<pac::SERCOM3>,
    usb: Option<pac::USB>,
    /// our clock controller
    pub clocks: GenericClockController,
}

impl BoardAbstractionLayer for Bal {
    type I2C = bsp::I2C;
    type Pins = bsp::Pins;

    fn init(mut peripherals: pac::Peripherals) -> (Self::Pins, Self) {
        let clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.PM,
            &mut peripherals.SYSCTRL,
            &mut peripherals.NVMCTRL,
        );
        let pins = bsp::Pins::new(peripherals.PORT);
        (
            pins,
            Self {
                pm: peripherals.PM,
                i2c_sercom: Some(peripherals.SERCOM3),
                usb: Some(peripherals.USB),
                clocks,
            },
        )
    }

    fn i2c(
        &mut self,
        baud: impl Into<Hertz>,
        sda: impl Into<bsp::Sda>,
        scl: impl Into<bsp::Scl>,
    ) -> Self::I2C {
        bsp::i2c_master(
            &mut self.clocks,
            baud.into(),
            self.i2c_sercom.take().unwrap(),
            &mut self.pm,
            sda,
            scl,
        )
    }

    fn usb_allocator(
        &mut self,
        dp: impl Into<bsp::UsbDp>,
        dm: impl Into<bsp::UsbDm>,
    ) -> UsbBusAllocator<UsbBus> {
        bsp::usb_allocator(
            self.usb.take().unwrap(),
            &mut self.clocks,
            &mut self.pm,
            dm,
            dp,
        )
    }
}

/// List of USB interrupts to enable/disable when needed
pub const USB_INTERRUPTS: [pac::interrupt; 1] = [pac::interrupt::USB];
