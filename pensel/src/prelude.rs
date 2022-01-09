//! useful import of the HAL, BSP, and PAC

/// re-export of our Board Support Package (BSP) for use in our modules
#[cfg(feature = "feather_m0")]
pub use feather_m0 as bsp;
#[cfg(feature = "feather_m4")]
pub use feather_m4 as bsp;

pub use crate::bal::BoardAbstractionLayer as bal_BoardAbstractionLayer;
/// re-export of our HAL and PAC layer, which in turn comes from our BSP
pub use bsp::{hal, pac};
