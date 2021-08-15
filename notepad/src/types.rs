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
