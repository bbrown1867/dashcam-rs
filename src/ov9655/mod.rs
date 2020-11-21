//! OV9655 device driver.

mod parallel;
mod pins;
mod sccb;

use core::convert::TryInto;
use sccb::{RegMap, SCCB};
use stm32f7xx_hal::{
    delay::Delay,
    i2c::{BlockingI2c, Mode},
    pac::I2C1,
    prelude::_embedded_hal_blocking_delay_DelayMs,
    rcc::{Clocks, APB1},
    time::U32Ext,
};

/// Number of horizontal pixels for QQVGA resolution.
pub const FRAME_WIDTH: u16 = 160;

/// Number of vertical pixels for QQVGA resolution.
pub const FRAME_HEIGHT: u16 = 120;

/// Number of total bytes in one frame using RGB565 format (2 pixels per byte).
pub const FRAME_SIZE: u32 = (FRAME_WIDTH as u32) * (FRAME_HEIGHT as u32) * 2;

/// Time between frames in milliseconds
pub const FRAME_RATE: u32 = 33_u32;

/// Initialize the OV9655 device driver.
/// * Performs camera configuration using the SCCB (I2C) port.
/// * Sets up DCMI and DMA2 to handle data capture.
/// * User must call `start_capture` to begin capturing frames.
/// * User must call `update_addrX` to setup ping-pong DMA addresses.
pub fn init(i2c1: I2C1, apb1: &mut APB1, clocks: Clocks, delay: &mut Delay) {
    // Pin configuration
    let i2c_pins = pins::pin_config_stm32f746g_disco();

    // I2C1 configuration (OV9655 SCCB)
    let mut i2c = BlockingI2c::i2c1(
        i2c1,
        i2c_pins,
        Mode::standard(100.khz()),
        clocks,
        apb1,
        10000,
    );

    // Init SCCB module
    let sccb = SCCB::new(&mut i2c);

    // Establish communication with the OV9655
    sccb.reset(&mut i2c).unwrap();
    delay.delay_ms(1000_u16);
    sccb.check_id(&mut i2c).unwrap();

    // Generate register map
    let mut reg_vals = RegMap::new();
    get_config(&mut reg_vals);

    // Configure the OV9655 using the register map
    sccb.apply_config(&mut i2c, &reg_vals, false).unwrap();

    // Setup DCMI and DMA2 to transfer from the DCMI peripheral into memory
    let dma_size_words = FRAME_SIZE / 4;
    parallel::dcmi_setup();
    parallel::dma2_setup(dma_size_words.try_into().unwrap());
}

/// Start capturing frames continuously.
pub fn start() {
    parallel::start_capture();
}

/// Stop capturing frames.
pub fn stop() {
    parallel::stop_capture();
}

/// Update camera frame data destination memory address 0.
pub fn update_addr0(address: u32) {
    parallel::dma2_update_addr0(address);
}

/// Update camera frame data destination memory address 1.
pub fn update_addr1(address: u32) {
    parallel::dma2_update_addr1(address);
}

/// Handle the frame interrupt. Returns `true` if a frame capture completed, `false` otherwise.
pub fn handle_dma_done() -> bool {
    return parallel::dma2_isr();
}

/// Given an empty `RegMap`, fill out the register values for QQVGA (160x120) resolution with
/// RGB565.
fn get_config(reg_vals: &mut RegMap) {
    // 30 fps VGA with VarioPixel and RGB output data format
    reg_vals.insert(0x12, 0x63).unwrap();

    // Don't change HREF to HSYNC (b6), don't reverse SYNC polarity (b1, b0), falling PCLK (b4)
    reg_vals.insert(0x15, 0x00).unwrap();

    // RGB 565 data format with full output range (0x00 --> 0xFF)
    reg_vals.insert(0x40, 0x10).unwrap();

    // Scale down ON
    reg_vals.insert(0x41, 0x01).unwrap();

    // Reduce resolution by half both vertically and horizontally (640x480 --> 320x240)
    reg_vals.insert(0x72, 0x22).unwrap();

    // Pixel clock output frequency adjustment
    reg_vals.insert(0x73, 0x02).unwrap();

    // Horizontal and vertical scaling
    reg_vals.insert(0x74, 0x10).unwrap();
    reg_vals.insert(0x75, 0x10).unwrap();

    // These registers are copied from the STM32F7 BSP, need to dig into them more
    reg_vals.insert(0x00, 0x00).unwrap();
    reg_vals.insert(0x01, 0x80).unwrap();
    reg_vals.insert(0x02, 0x80).unwrap();
    reg_vals.insert(0x03, 0x02).unwrap();
    reg_vals.insert(0x04, 0x03).unwrap();
    reg_vals.insert(0x09, 0x01).unwrap();
    reg_vals.insert(0x0b, 0x57).unwrap();
    reg_vals.insert(0x0e, 0x61).unwrap();
    reg_vals.insert(0x0f, 0x40).unwrap();
    reg_vals.insert(0x11, 0x01).unwrap();
    reg_vals.insert(0x13, 0xc7).unwrap();
    reg_vals.insert(0x14, 0x3a).unwrap();
    reg_vals.insert(0x16, 0x24).unwrap();
    reg_vals.insert(0x17, 0x18).unwrap();
    reg_vals.insert(0x18, 0x04).unwrap();
    reg_vals.insert(0x19, 0x01).unwrap();
    reg_vals.insert(0x1a, 0x81).unwrap();
    reg_vals.insert(0x1e, 0x00).unwrap();
    reg_vals.insert(0x24, 0x3c).unwrap();
    reg_vals.insert(0x25, 0x36).unwrap();
    reg_vals.insert(0x26, 0x72).unwrap();
    reg_vals.insert(0x27, 0x08).unwrap();
    reg_vals.insert(0x28, 0x08).unwrap();
    reg_vals.insert(0x29, 0x15).unwrap();
    reg_vals.insert(0x2a, 0x00).unwrap();
    reg_vals.insert(0x2b, 0x00).unwrap();
    reg_vals.insert(0x2c, 0x08).unwrap();
    reg_vals.insert(0x32, 0xa4).unwrap();
    reg_vals.insert(0x33, 0x00).unwrap();
    reg_vals.insert(0x34, 0x3f).unwrap();
    reg_vals.insert(0x35, 0x00).unwrap();
    reg_vals.insert(0x36, 0x3a).unwrap();
    reg_vals.insert(0x38, 0x72).unwrap();
    reg_vals.insert(0x39, 0x57).unwrap();
    reg_vals.insert(0x3a, 0xcc).unwrap();
    reg_vals.insert(0x3b, 0x04).unwrap();
    reg_vals.insert(0x3d, 0x99).unwrap();
    reg_vals.insert(0x3e, 0x0e).unwrap();
    reg_vals.insert(0x3f, 0xc1).unwrap();
    reg_vals.insert(0x42, 0xc0).unwrap();
    reg_vals.insert(0x43, 0x0a).unwrap();
    reg_vals.insert(0x44, 0xf0).unwrap();
    reg_vals.insert(0x45, 0x46).unwrap();
    reg_vals.insert(0x46, 0x62).unwrap();
    reg_vals.insert(0x47, 0x2a).unwrap();
    reg_vals.insert(0x48, 0x3c).unwrap();
    reg_vals.insert(0x4a, 0xfc).unwrap();
    reg_vals.insert(0x4b, 0xfc).unwrap();
    reg_vals.insert(0x4c, 0x7f).unwrap();
    reg_vals.insert(0x4d, 0x7f).unwrap();
    reg_vals.insert(0x4e, 0x7f).unwrap();
    reg_vals.insert(0x4f, 0x98).unwrap();
    reg_vals.insert(0x50, 0x98).unwrap();
    reg_vals.insert(0x51, 0x00).unwrap();
    reg_vals.insert(0x52, 0x28).unwrap();
    reg_vals.insert(0x53, 0x70).unwrap();
    reg_vals.insert(0x54, 0x98).unwrap();
    reg_vals.insert(0x58, 0x1a).unwrap();
    reg_vals.insert(0x59, 0x85).unwrap();
    reg_vals.insert(0x5a, 0xa9).unwrap();
    reg_vals.insert(0x5b, 0x64).unwrap();
    reg_vals.insert(0x5c, 0x84).unwrap();
    reg_vals.insert(0x5d, 0x53).unwrap();
    reg_vals.insert(0x5e, 0x0e).unwrap();
    reg_vals.insert(0x5f, 0xf0).unwrap();
    reg_vals.insert(0x60, 0xf0).unwrap();
    reg_vals.insert(0x61, 0xf0).unwrap();
    reg_vals.insert(0x62, 0x00).unwrap();
    reg_vals.insert(0x63, 0x00).unwrap();
    reg_vals.insert(0x64, 0x02).unwrap();
    reg_vals.insert(0x65, 0x20).unwrap();
    reg_vals.insert(0x66, 0x00).unwrap();
    reg_vals.insert(0x69, 0x0a).unwrap();
    reg_vals.insert(0x6b, 0x5a).unwrap();
    reg_vals.insert(0x6c, 0x04).unwrap();
    reg_vals.insert(0x6d, 0x55).unwrap();
    reg_vals.insert(0x6e, 0x00).unwrap();
    reg_vals.insert(0x6f, 0x9d).unwrap();
    reg_vals.insert(0x70, 0x21).unwrap();
    reg_vals.insert(0x71, 0x78).unwrap();
    reg_vals.insert(0x76, 0x01).unwrap();
    reg_vals.insert(0x77, 0x02).unwrap();
    reg_vals.insert(0x7A, 0x12).unwrap();
    reg_vals.insert(0x7B, 0x08).unwrap();
    reg_vals.insert(0x7C, 0x16).unwrap();
    reg_vals.insert(0x7D, 0x30).unwrap();
    reg_vals.insert(0x7E, 0x5e).unwrap();
    reg_vals.insert(0x7F, 0x72).unwrap();
    reg_vals.insert(0x80, 0x82).unwrap();
    reg_vals.insert(0x81, 0x8e).unwrap();
    reg_vals.insert(0x82, 0x9a).unwrap();
    reg_vals.insert(0x83, 0xa4).unwrap();
    reg_vals.insert(0x84, 0xac).unwrap();
    reg_vals.insert(0x85, 0xb8).unwrap();
    reg_vals.insert(0x86, 0xc3).unwrap();
    reg_vals.insert(0x87, 0xd6).unwrap();
    reg_vals.insert(0x88, 0xe6).unwrap();
    reg_vals.insert(0x89, 0xf2).unwrap();
    reg_vals.insert(0x8a, 0x24).unwrap();
    reg_vals.insert(0x8c, 0x80).unwrap();
    reg_vals.insert(0x90, 0x7d).unwrap();
    reg_vals.insert(0x91, 0x7b).unwrap();
    reg_vals.insert(0x9d, 0x02).unwrap();
    reg_vals.insert(0x9e, 0x02).unwrap();
    reg_vals.insert(0x9f, 0x7a).unwrap();
    reg_vals.insert(0xa0, 0x79).unwrap();
    reg_vals.insert(0xa1, 0x40).unwrap();
    reg_vals.insert(0xa4, 0x50).unwrap();
    reg_vals.insert(0xa5, 0x68).unwrap();
    reg_vals.insert(0xa6, 0x4a).unwrap();
    reg_vals.insert(0xa8, 0xc1).unwrap();
    reg_vals.insert(0xa9, 0xef).unwrap();
    reg_vals.insert(0xaa, 0x92).unwrap();
    reg_vals.insert(0xab, 0x04).unwrap();
    reg_vals.insert(0xac, 0x80).unwrap();
    reg_vals.insert(0xad, 0x80).unwrap();
    reg_vals.insert(0xae, 0x80).unwrap();
    reg_vals.insert(0xaf, 0x80).unwrap();
    reg_vals.insert(0xb2, 0xf2).unwrap();
    reg_vals.insert(0xb3, 0x20).unwrap();
    reg_vals.insert(0xb4, 0x20).unwrap();
    reg_vals.insert(0xb5, 0x00).unwrap();
    reg_vals.insert(0xb6, 0xaf).unwrap();
    reg_vals.insert(0xb6, 0xaf).unwrap();
    reg_vals.insert(0xbb, 0xae).unwrap();
    reg_vals.insert(0xbc, 0x7f).unwrap();
    reg_vals.insert(0xbd, 0x7f).unwrap();
    reg_vals.insert(0xbe, 0x7f).unwrap();
    reg_vals.insert(0xbf, 0x7f).unwrap();
    reg_vals.insert(0xbf, 0x7f).unwrap();
    reg_vals.insert(0xc0, 0xaa).unwrap();
    reg_vals.insert(0xc1, 0xc0).unwrap();
    reg_vals.insert(0xc2, 0x01).unwrap();
    reg_vals.insert(0xc3, 0x4e).unwrap();
    reg_vals.insert(0xc6, 0x05).unwrap();
    reg_vals.insert(0xc7, 0x82).unwrap();
    reg_vals.insert(0xc9, 0xe0).unwrap();
    reg_vals.insert(0xca, 0xe8).unwrap();
    reg_vals.insert(0xcb, 0xf0).unwrap();
    reg_vals.insert(0xcc, 0xd8).unwrap();
    reg_vals.insert(0xcd, 0x93).unwrap();
}
