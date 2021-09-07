//! The Pensel firmware for the Harma project
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_mut_refs)]
#![warn(missing_docs)]
#![no_std]
pub mod cli;
pub mod imu;
pub mod prelude;
pub mod usb_serial;

/// re-export of our HAL and PAC layer, which in turn comes from our BSP
pub use bsp::{hal, pac};
/// re-export of our Board Support Package (BSP) for use in our modules
pub use feather_m0 as bsp;
