//! OV9655 device driver.

pub mod parallel;
pub mod sccb;

use sccb::{RegMap, Register};

/// Given an empty `RegMap`, fill out the register values for a particular configuration. This
/// configuration was determined by reading the datasheet and setting fields that made sense.
pub fn get_config(reg_vals: &mut RegMap) {
    // 15 fps VGA with RGB output data format
    reg_vals.insert(Register::COM_CNTRL_07, 0x03).unwrap();

    // Pin configuration:
    // --> Bit 6: Change HREF pin to be HSYNC signals
    // --> Bit 4: PCLK reverse
    // --> Bit 3: HREF reverse
    // --> Bit 1: VSYNC negative
    // --> Bit 0: HSYNC negative
    reg_vals.insert(Register::COM_CNTRL_10, 0x40).unwrap();

    // RGB 565 data format with full output range (0x00 --> 0xFF)
    reg_vals.insert(Register::COM_CNTRL_15, 0x10).unwrap();

    // Scale down ON
    reg_vals.insert(Register::COM_CNTRL_16, 0x01).unwrap();

    // Reduce resolution by half both vertically and horizontally (640x480 --> 320x240)
    reg_vals.insert(Register::PIX_OUT_INDX, 0x11).unwrap();

    // Pixel clock output frequency adjustment (note: default value is 0x01)
    reg_vals.insert(Register::PIX_CLK_DIVD, 0x01).unwrap();

    // Horizontal and vertical scaling
    reg_vals.insert(Register::PIX_HOR_SCAL, 0x10).unwrap();
    reg_vals.insert(Register::PIX_VER_SCAL, 0x10).unwrap();
}
