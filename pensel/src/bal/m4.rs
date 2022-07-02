use super::BoardAbstractionLayer;
use crate::prelude::*;
use hal::{clock::GenericClockController, time::Hertz, usb::UsbBus};

use usb_device::class_prelude::UsbBusAllocator;

/// Our board abstraction layer encapsulation
pub struct Bal {
    mclk: pac::MCLK,
    i2c_sercom: Option<pac::SERCOM2>,
    usb: Option<pac::USB>,
    /// our clock controller
    pub clocks: GenericClockController,
}

impl BoardAbstractionLayer for Bal {
    type I2C = bsp::I2c;
    type Pins = bsp::Pins;

    fn init(mut peripherals: pac::Peripherals) -> (Self::Pins, Self) {
        let clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.MCLK,
            &mut peripherals.OSC32KCTRL,
            &mut peripherals.OSCCTRL,
            &mut peripherals.NVMCTRL,
        );
        let pins = bsp::Pins::new(peripherals.PORT);

        (
            pins,
            Self {
                mclk: peripherals.MCLK,
                i2c_sercom: Some(peripherals.SERCOM2),
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
            &mut self.mclk,
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
            dm,
            dp,
            self.usb.take().unwrap(),
            &mut self.clocks,
            &mut self.mclk,
        )
    }
}

/// List of USB interrupts to enable/disable when needed
pub const USB_INTERRUPTS: [pac::interrupt; 3] = [
    pac::interrupt::USB_OTHER,
    pac::interrupt::USB_TRCPT0,
    pac::interrupt::USB_TRCPT1,
];
