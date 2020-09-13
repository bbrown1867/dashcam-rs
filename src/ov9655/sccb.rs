//! A generic driver for the Serial Camera Control Bus on the OV9655 image sensor. Tested with the
//! STM32F767ZI microcontroller, but in theory should work on any microcontroller implementing the
//! embedded-hal I2C interface.
//!
//! Could this be converted into an SCCB driver for any OmniVision image sensor? Would need to
//! abstract the different registers and other device specific information.

use core::marker::PhantomData;
use embedded_hal::blocking::i2c;
use heapless::{consts, LinearMap};

/// SCCB driver
pub struct SCCB<I2C> {
    // Ensures the same I2C type is used in all calls
    i2c: PhantomData<I2C>,
    // Even though this is constant, keeping it a a member for now in case it ever changes
    address: u8,
}

/// SCCB errors
#[derive(Debug, Eq, PartialEq)]
pub enum SccbError<E> {
    /// I2C Write error
    I2cWrite(E),
    /// I2C Read error
    I2cRead(E),
    /// Read Manf ID error
    ReadManfId,
    /// Read Product ID error
    ReadProdId,
    // Register write-read mismatch
    RegMismatch((u8, u8)),
}

impl<I2C, E> SCCB<I2C>
where
    I2C: i2c::Read<Error = E> + i2c::Write<Error = E>,
{
    /// Creates a new SCCB driver associated with an I2C peripheral
    pub fn new(_i2c: &I2C) -> Self {
        SCCB {
            i2c: PhantomData,
            address: OV9655_ADDRESS,
        }
    }

    /// I2C read wrapper for mapping E --> SccbError
    fn i2c_read(&self, i2c: &mut I2C, buf: &mut [u8]) -> Result<(), SccbError<E>> {
        match i2c.read(self.address, buf) {
            Ok(()) => Ok(()),
            Err(e) => Err(SccbError::I2cRead(e)),
        }
    }

    /// I2C write wrapper for mapping E --> SccbError
    fn i2c_write(&self, i2c: &mut I2C, buf: &[u8]) -> Result<(), SccbError<E>> {
        match i2c.write(self.address, buf) {
            Ok(()) => Ok(()),
            Err(e) => Err(SccbError::I2cWrite(e)),
        }
    }

    //// Read a register, must be two seperate transactions can't use WriteRead
    fn read_register(&self, i2c: &mut I2C, reg: u8) -> Result<u8, SccbError<E>> {
        // Write the address
        self.i2c_write(i2c, &[reg])?;

        // Read the value
        let mut buf = [0x00];
        self.i2c_read(i2c, &mut buf)?;
        Ok(buf[0])
    }

    /// Write a register
    fn write_register(&self, i2c: &mut I2C, reg: u8, val: u8) -> Result<(), SccbError<E>> {
        // Write the address and value
        self.i2c_write(i2c, &[reg, val])
    }

    /// Reset all registers to their default values
    pub fn reset(&self, i2c: &mut I2C) -> Result<(), SccbError<E>> {
        // Setting the upper bit of this register resets all the registers
        let reg = self.read_register(i2c, Register::COM_CNTRL_07)?;
        self.write_register(i2c, Register::COM_CNTRL_07, reg | 0x80)
    }

    /// Check the device ID matches the expected value
    pub fn check_id(&self, i2c: &mut I2C) -> Result<(), SccbError<E>> {
        // Manf ID
        let manf_id_msb: u16 = self.read_register(i2c, Register::MANF_ID_MSB)?.into();
        let manf_id_lsb: u16 = self.read_register(i2c, Register::MANF_ID_LSB)?.into();
        let manf_id: u16 = (manf_id_msb << 8) | manf_id_lsb;
        if manf_id != OV9655_MANF_ID {
            return Err(SccbError::ReadManfId);
        }

        // Product ID
        let product_id_msb: u16 = self.read_register(i2c, Register::PROD_ID_MSB)?.into();
        let product_id_lsb: u16 = self.read_register(i2c, Register::PROD_ID_LSB)?.into();
        let product_id: u16 = (product_id_msb << 8) | product_id_lsb;
        if product_id != OV9655_PROD_ID {
            return Err(SccbError::ReadProdId);
        }

        Ok(())
    }

    /// Perform a sequence of register writes to setup the OV9655 for QVGA (320x240) mode
    pub fn qvga_setup(&self, i2c: &mut I2C) -> Result<(), SccbError<E>> {
        // Registers range from 0x00 to 0xC7, although we don't write every one
        let mut reg_vals = LinearMap::<u8, u8, consts::U200>::new();

        // 30 fps VGA with RGB output data format
        reg_vals.insert(Register::COM_CNTRL_07, 0x63).unwrap();

        // PCLK falling edge (reverse?), HREF active low (reverse?)
        reg_vals.insert(Register::COM_CNTRL_10, 0x18).unwrap();

        // Full output range, RGB 565 data format
        reg_vals.insert(Register::COM_CNTRL_15, 0x10).unwrap();

        // Scale down ON
        reg_vals.insert(Register::COM_CNTRL_16, 0x01).unwrap();

        // Reduce resolution by half both vertically and horizontally (640x480 --> 320x240)
        reg_vals.insert(Register::PIX_OUT_INDX, 0x11).unwrap();

        // Pixel clock output frequency adjustment (note: default value is 0x01)
        reg_vals.insert(Register::PIX_CLK_DIVD, 0x01).unwrap();

        // Horizontal and vertical scaling - TODO: Unsure how this works
        reg_vals.insert(Register::PIX_HOR_SCAL, 0x10).unwrap();
        reg_vals.insert(Register::PIX_VER_SCAL, 0x10).unwrap();

        // TODO: Are registers below necessary?

        // Set the output drive capability to 4x
        reg_vals.insert(Register::COM_CNTRL_01, 0x03).unwrap();

        // Set the exposure step bit high
        reg_vals.insert(Register::COM_CNTRL_05, 0x01).unwrap();

        // Enable HREF at optical black, use optical black as BLC signal
        reg_vals.insert(Register::COM_CNTRL_06, 0xc0).unwrap();

        // Enable auto white balance, gain control, exposure control, etc.
        reg_vals.insert(Register::COM_CNTRL_08, 0xef).unwrap();

        // More gain and exposure settings
        reg_vals.insert(Register::COM_CNTRL_09, 0x3a).unwrap();

        // No mirror and no vertical flip
        reg_vals.insert(Register::MIRROR_VFLIP, 0x00).unwrap();

        // Zoom function ON, black/white correction off
        reg_vals.insert(Register::COM_CNTRL_14, 0x02).unwrap();

        // Enables auto adjusting for de-noise and edge enhancement
        reg_vals.insert(Register::COM_CNTRL_17, 0xc0).unwrap();

        // Use VarioPixel
        reg_vals.insert(Register::VARIO_PX_SEL, 0x0a).unwrap();

        for (reg, val) in reg_vals.iter() {
            self.write_register(i2c, *reg, *val)?;
            let readback = self.read_register(i2c, *reg)?;
            if readback != *val {
                return Err(SccbError::RegMismatch((*reg, readback)));
            }
        }

        Ok(())
    }
}

// Device address for is 0x60, however the I2C driver will left-shift the provided address by 1
const OV9655_ADDRESS: u8 = 0x30;

// Expected manufacturer ID (weird that it is not "OV" in ASCII...)
const OV9655_MANF_ID: u16 = 0x7FA2;

// Expected product ID (weird that it is not "9655"...)
const OV9655_PROD_ID: u16 = 0x9657;

// Device register addresses
pub struct Register;

#[allow(dead_code)]
impl Register {
    // Common control registers
    pub const COM_CNTRL_01: u8 = 0x04;
    pub const COM_CNTRL_02: u8 = 0x09;
    pub const COM_CNTRL_03: u8 = 0x0C;
    pub const COM_CNTRL_04: u8 = 0x0D;
    pub const COM_CNTRL_05: u8 = 0x0E;
    pub const COM_CNTRL_06: u8 = 0x0F;
    pub const COM_CNTRL_07: u8 = 0x12;
    pub const COM_CNTRL_08: u8 = 0x13;
    pub const COM_CNTRL_09: u8 = 0x14;
    pub const COM_CNTRL_10: u8 = 0x15;
    pub const COM_CNTRL_11: u8 = 0x3B;
    pub const COM_CNTRL_12: u8 = 0x3C;
    pub const COM_CNTRL_13: u8 = 0x3D;
    pub const COM_CNTRL_14: u8 = 0x3E;
    pub const COM_CNTRL_15: u8 = 0x40;
    pub const COM_CNTRL_16: u8 = 0x41;
    pub const COM_CNTRL_17: u8 = 0x42;
    pub const COM_CNTRL_18: u8 = 0x8B;
    pub const COM_CNTRL_19: u8 = 0x8C;
    pub const COM_CNTRL_20: u8 = 0x8D;
    pub const COM_CNTRL_21: u8 = 0xA4;
    pub const COM_CNTRL_22: u8 = 0xB5;

    // Product ID registers
    pub const PROD_ID_MSB: u8 = 0x0A;
    pub const PROD_ID_LSB: u8 = 0x0B;

    // Manufacturer ID registers
    pub const MANF_ID_MSB: u8 = 0x1C;
    pub const MANF_ID_LSB: u8 = 0x1D;

    // Rescaling configuration registers
    pub const PIX_OUT_INDX: u8 = 0x72;
    pub const PIX_CLK_DIVD: u8 = 0x73;
    pub const PIX_HOR_SCAL: u8 = 0x74;
    pub const PIX_VER_SCAL: u8 = 0x75;

    // Misc configuration registers
    pub const MIRROR_VFLIP: u8 = 0x1E;
    pub const VARIO_PX_SEL: u8 = 0x69;
}
