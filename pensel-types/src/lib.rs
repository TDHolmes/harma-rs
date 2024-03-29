//! The types shared between pensel FW and the SW that talks to it
#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

/// re-export of [`bno055`] for downstream crates
pub use bno055;
/// re-export of [`bno055::mint`] for downstream crates
pub use bno055::mint;

pub mod cli {
    //! Types specific to pensel's CLI

    /// the command to trigger a forced panic
    pub const CMD_PANIC: &str = "panic";

    /// initiates an MCU reset
    pub const CMD_RESET: &str = "reset";

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

    use core::fmt;

    /// A fixed point 3D vector coming from pensel. Could be linear acceleration or gravity.
    #[repr(transparent)]
    #[derive(Debug, PartialEq, Eq, derive_more::From, derive_more::Deref)]
    pub struct FixedPointVector<const PREFIX: char>(bno055::mint::Vector3<i16>);

    impl<const P: char> core::str::FromStr for FixedPointVector<P> {
        type Err = core::fmt::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            const FUNC_NAME: &str = "FixedPointVector::from_str";
            /// checks for a decimal digit
            const fn is_decimal_digit(input: char) -> bool {
                input.is_ascii_digit() || input == '-'
            }

            log::debug!("{}: parsing {:?}", FUNC_NAME, s);
            if !s.starts_with(P) {
                log::error!("{}: missing prefix {} in {:#?}", FUNC_NAME, P, s);
                return Err(core::fmt::Error);
            }

            // split by comma and look for 3 digits to parse out
            let mut values: [i16; 3] = [0; 3];
            for (count, item) in s.split(',').enumerate() {
                if count == 3 {
                    log::error!("{}: Too many comma separated values in {:#?}", FUNC_NAME, s);
                    return Err(core::fmt::Error);
                }

                // parse out the digit, if we can find it
                if let Some(start_ind) = item.find(is_decimal_digit) {
                    let slice_to_parse = item[start_ind..].trim_end();
                    if let Ok(digit) = i16::from_str(slice_to_parse) {
                        values[count] = digit;
                    } else {
                        log::error!(
                            "{}: failed to convert '{}' to i16",
                            FUNC_NAME,
                            slice_to_parse
                        );
                        return Err(core::fmt::Error);
                    }
                } else {
                    log::error!("{}: failed to find a number in {}", FUNC_NAME, item);
                    return Err(core::fmt::Error);
                }
            }

            Ok(Self::new(values[0], values[1], values[2]))
        }
    }

    impl<const P: char> fmt::Display for FixedPointVector<P> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}:{},{},{}", P, self.x, self.y, self.z)
        }
    }

    impl<const P: char> FixedPointVector<P> {
        /// Initializes a new `FixedPointVector`.
        #[must_use]
        pub const fn new(x: i16, y: i16, z: i16) -> Self {
            Self(bno055::mint::Vector3::<i16> { x, y, z })
        }
    }

    /// Gravity vector
    pub type GravityVector = FixedPointVector<'G'>;

    /// Linear acceleration vector
    pub type AccelerationVector = FixedPointVector<'A'>;
}
