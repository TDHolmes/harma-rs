use mint;

/// The gravity vector determined by Pensel
pub type GravityVec = mint::Vector3<isize>;

/// The net linear acceleration determined by Pensel
pub type AccelerationVec = mint::Vector3<isize>;

/// The possible outcomes of parsing a line of data from Pensel
#[derive(PartialEq, Debug)]
pub enum ParsedLine {
    None,
    Grav(GravityVec),
    Accel(AccelerationVec),
}

#[cfg(test)]
mod test_types {
    use super::*;

    #[test]
    fn partial_eq_parsed_line() {
        let mut test_line = ParsedLine::None;

        assert_eq!(test_line, ParsedLine::None);
        test_line = ParsedLine::Accel(AccelerationVec { x: 1, y: 2, z: 3 });
        assert_ne!(test_line, ParsedLine::None);
    }
}
