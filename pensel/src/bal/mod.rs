//! Board Abstraction Layer for using different boards
use crate::prelude::*;
use hal::{time::Hertz, usb::UsbBus};
use usb_device::class_prelude::UsbBusAllocator;

#[cfg(feature = "feather_m0")]
mod m0;
#[cfg(feature = "feather_m4")]
mod m4;

#[cfg(feature = "feather_m0")]
pub use m0::*;
#[cfg(feature = "feather_m4")]
pub use m4::*;

#[cfg(all(feature = "feather_m0", feature = "feather_m4"))]
compile_error!("Must select one and only one board!");

/// The abstraction layer for working between board types
pub trait BoardAbstractionLayer {
    /// This board's concrete I2C peripheral
    type I2C;
    /// This board's concrete pins type
    type Pins;

    /// Initializes our [`BoardAbstractionLayer`] struct
    fn init(peripherals: pac::Peripherals) -> (Self::Pins, Self);

    /// Initializes this board's I2C peripheral and returns it, consuming those pins.
    fn i2c(
        &mut self,
        baud: impl Into<Hertz>,
        sda: impl Into<bsp::Sda>,
        scl: impl Into<bsp::Scl>,
    ) -> Self::I2C;

    /// Initializes this board's USB allocator and returns it, consuming those pins.
    fn usb_allocator(
        &mut self,
        dp: impl Into<bsp::UsbDp>,
        dm: impl Into<bsp::UsbDm>,
    ) -> UsbBusAllocator<UsbBus>;
}
