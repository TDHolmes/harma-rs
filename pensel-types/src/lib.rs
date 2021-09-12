//! The types shared between pensel FW and the SW that talks to it
#![no_std]
#![warn(missing_docs)]
pub use bno055;
pub use bno055::mint;

pub mod cli {
    //! Types specific to pensel's CLI

    /// the command to trigger a forced panic
    pub const CMD_PANIC: &str = "panic";

    /// the command to control the IMU
    pub const CMD_IMU: &str = "imu";
    /// argument to `imu` command to enable streaming of the gravity vector
    pub const ARG_GRAVITY: &str = "gravity";
    /// argument to `imu` command to enable streaming of the accel vector
    pub const ARG_ACCEL: &str = "accel";

    /// control our logging facilities
    pub const CMD_LOG: &str = "log";
    /// change log level
    pub const ARG_LEVEL_SET: &str = "level";
    /// retrieve current log level
    pub const ARG_LEVEL_GET: &str = "level-get";
}

pub mod imu {
    //! Types specific to pensel's IMU
    /// A fixed point 3D vector coming from pensel. Could be linear acceleration or gravity.
    pub type FixedPointVector = bno055::mint::Vector3<i16>;
}
