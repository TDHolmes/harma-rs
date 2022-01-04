//! Encapsulation of the details of our IMU
use crate::cli;
use pensel_types::{bno055, cli as pt_cli, imu};

use core::sync::atomic;
use log;

use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Write, WriteRead},
};

/// Our encapsulation of an IMU. Currently a bno055
pub struct Imu<I> {
    bno: bno055::Bno055<I>,
}

const BNO055_CALIBRATION: bno055::BNO055Calibration = bno055::BNO055Calibration {
    acc_offset_x_lsb: 2,
    acc_offset_x_msb: 0,
    acc_offset_y_lsb: 252,
    acc_offset_y_msb: 255,
    acc_offset_z_lsb: 231,
    acc_offset_z_msb: 255,
    mag_offset_x_lsb: 215,
    mag_offset_x_msb: 254,
    mag_offset_y_lsb: 174,
    mag_offset_y_msb: 1,
    mag_offset_z_lsb: 228,
    mag_offset_z_msb: 1,
    gyr_offset_x_lsb: 1,
    gyr_offset_x_msb: 0,
    gyr_offset_y_lsb: 0,
    gyr_offset_y_msb: 0,
    gyr_offset_z_lsb: 0,
    gyr_offset_z_msb: 0,
    acc_radius_lsb: 232,
    acc_radius_msb: 3,
    mag_radius_lsb: 241,
    mag_radius_msb: 2,
};

static CLI_CONTROL_STREAM_GRAVITY: atomic::AtomicBool = atomic::AtomicBool::new(false);
static CLI_CONTROL_STREAM_ACCEL: atomic::AtomicBool = atomic::AtomicBool::new(false);

impl<I, E> Imu<I>
where
    I: Write<Error = E> + WriteRead<Error = E>,
    E: core::fmt::Debug,
{
    /// Initializes our IMU.
    ///
    /// # Arguments
    /// `delay`: Facility for bno055 to delay during initialization/mode changes
    /// `i2c`: I2C bus that the bno055 is connected to
    pub fn new(delay: &mut dyn DelayMs<u16>, i2c: I) -> Self {
        log::debug!("initializing IMU");
        let mut bno = bno055::Bno055::new(i2c).with_alternative_address();
        bno.init(delay).expect("bno init err");
        bno.set_mode(bno055::BNO055OperationMode::NDOF, delay)
            .expect("set_mode fail");

        bno.set_calibration_profile(BNO055_CALIBRATION, delay)
            .expect("set_calibration_profile fail");

        Self { bno }
    }

    /// Retrieves the current gravity vector as calculated by the bno055
    pub fn gravity_fixed(&mut self) -> Option<imu::GravityVector> {
        if CLI_CONTROL_STREAM_GRAVITY.load(atomic::Ordering::Acquire) {
            log::debug!("IMU - gravity_fixed");
            if let Ok(g_vec) = self.bno.gravity_fixed() {
                return Some(g_vec.into());
            }
        }

        None
    }

    /// Retrieves the current linear acceleration from the bno055
    pub fn linear_acceleration_fixed(&mut self) -> Option<imu::AccelerationVector> {
        if CLI_CONTROL_STREAM_ACCEL.load(atomic::Ordering::Acquire) {
            log::debug!("IMU - linear_acceleration_fixed");
            if let Ok(a_vec) = self.bno.linear_acceleration_fixed() {
                return Some(a_vec.into());
            }
        }

        None
    }
}

fn imu_control<const N: usize>(
    _menu: &menu::Menu<cli::Output<N>>,
    item: &menu::Item<cli::Output<N>>,
    args: &[&str],
    _context: &mut cli::Output<N>,
) {
    let mut enable_accel = false;
    let mut enable_grav = false;
    if let Ok(Some(_)) = menu::argument_finder(item, args, pt_cli::ARG_ACCEL) {
        enable_accel = true;
    }

    if let Ok(Some(_)) = menu::argument_finder(item, args, pt_cli::ARG_GRAVITY) {
        enable_grav = true;
    }

    CLI_CONTROL_STREAM_ACCEL.store(enable_accel, atomic::Ordering::Release);
    CLI_CONTROL_STREAM_GRAVITY.store(enable_grav, atomic::Ordering::Release);
}

/// Method to put our CLI entry in for IMU control
pub const IMU_CLI_ITEM: cli::Item = cli::Item {
    item_type: menu::ItemType::Callback {
        function: imu_control,
        parameters: &[
            menu::Parameter::Named {
                parameter_name: pt_cli::ARG_ACCEL,
                help: Some("Enable streaming of accel vector"),
            },
            menu::Parameter::Named {
                parameter_name: pt_cli::ARG_GRAVITY,
                help: Some("Enable streaming of gravity vector"),
            },
        ],
    },
    command: pt_cli::CMD_IMU,
    help: Some("Controls how our IMU functions"),
};
