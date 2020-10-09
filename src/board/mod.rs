//! Support for off-chip, board specific devices.
//! * Note: The OV9655 is not part of this module and has a seperate module.

pub mod display;
pub mod sdram;

use stm32f7xx_hal::time::{MegaHertz, U32Ext};

/// 25 MHz external oscillator (X2) is the HSE clock source.
pub fn get_xtal() -> MegaHertz {
    25.mhz()
}
