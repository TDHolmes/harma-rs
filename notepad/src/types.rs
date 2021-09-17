//! Types notepad uses

/// the shared vector type pensel firmware produces
pub use pensel_types::imu;

/// The possible outcomes of parsing a line of data from Pensel
#[derive(PartialEq, Debug)]
pub enum ParsedLine {
    None,
    Grav(imu::GravityVector),
    Accel(imu::AccelerationVector),
}

pub const ACC_QUEUE_SIZE: usize = 100;
pub const GRAV_QUEUE_SIZE: usize = 100;

#[cfg(test)]
mod test_types {
    use super::*;

    #[test]
    fn partial_eq_parsed_line() {
        let mut test_line = ParsedLine::None;

        assert_eq!(test_line, ParsedLine::None);
        test_line = ParsedLine::Accel(imu::AccelerationVector::new(1, 2, 3));
        assert_ne!(test_line, ParsedLine::None);
    }
}
