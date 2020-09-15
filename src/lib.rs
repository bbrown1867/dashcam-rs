//! A prototype dashboard camera.

#![no_std]

/// Drivers and helper functions for using the OV9655.
pub mod ov9655 {
    pub mod parallel;
    pub mod sccb;
}

pub mod pins;
