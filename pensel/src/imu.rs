//! Encapsulation of the details of our IMU
use crate::cli;

use core::sync::atomic;

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

static CLI_CONTROL_STREAM_GRAVITY: atomic::AtomicBool = atomic::AtomicBool::new(true);
static CLI_CONTROL_STREAM_ACCEL: atomic::AtomicBool = atomic::AtomicBool::new(true);

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
        // initialize the IMU
        let mut bno = bno055::Bno055::new(i2c).with_alternative_address();
        bno.init(delay).expect("bno init err");
        bno.set_mode(bno055::BNO055OperationMode::NDOF, delay)
            .expect("set_mode fail");

        bno.set_calibration_profile(BNO055_CALIBRATION, delay)
            .expect("set_calibration_profile fail");

        Imu { bno }
    }

    /// Retrieves the current gravity vector as calculated by the bno055
    pub fn gravity_fixed(&mut self) -> Option<bno055::mint::Vector3<i16>> {
        if CLI_CONTROL_STREAM_GRAVITY.load(atomic::Ordering::Acquire) {
            return self.bno.gravity_fixed().ok();
        }

        None
    }

    /// Retrieves the current linear acceleration from the bno055
    pub fn linear_acceleration_fixed(&mut self) -> Option<bno055::mint::Vector3<i16>> {
        if CLI_CONTROL_STREAM_ACCEL.load(atomic::Ordering::Acquire) {
            return self.bno.linear_acceleration_fixed().ok();
        }

        None
    }
}

const GRAVITY_ARG: &'static str = "gravity";
const ACCEL_ARG: &'static str = "accel";

fn imu_control<const N: usize>(
    _menu: &menu::Menu<cli::CliOutput<N>>,
    item: &menu::Item<cli::CliOutput<N>>,
    args: &[&str],
    _context: &mut cli::CliOutput<N>,
) {
    let enable_accel = menu::argument_finder(item, args, ACCEL_ARG).is_ok();
    let enable_grav = menu::argument_finder(item, args, GRAVITY_ARG).is_ok();

    CLI_CONTROL_STREAM_GRAVITY.store(enable_grav, atomic::Ordering::Release);
    CLI_CONTROL_STREAM_ACCEL.store(enable_accel, atomic::Ordering::Release);
}

/// Method to put our CLI entry in for IMU control
pub const fn generate_imu_item() -> cli::CliItem {
    menu::Item {
        item_type: menu::ItemType::Callback {
            function: imu_control,
            parameters: &[
                menu::Parameter::Named {
                    parameter_name: ACCEL_ARG,
                    help: Some("Enable streaming of accel vector"),
                },
                menu::Parameter::Named {
                    parameter_name: GRAVITY_ARG,
                    help: Some("Enable streaming of gravity vector"),
                },
            ],
        },
        command: "imu",
        help: Some("Controls how our IMU functions"),
    }
}
