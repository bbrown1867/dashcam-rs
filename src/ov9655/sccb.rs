//! A generic driver for the Serial Camera Control Bus on the OV9655 image sensor. Tested with the
//! STM32F767ZI microcontroller, but in theory should work on any microcontroller implementing the
//! embedded-hal I2C interface.
//!
//! Could this be converted into an SCCB driver for any OmniVision image sensor? Would need to
//! abstract the different registers and other device specific information.

use core::marker::PhantomData;
use embedded_hal::blocking::i2c;
use heapless::{consts, LinearMap};

// Type alias for register map
pub type RegMap = LinearMap<u8, u8, consts::U200>;

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

    /// Apply a register configuration specified by the linear map
    pub fn apply_config(&self, i2c: &mut I2C, map: &RegMap) -> Result<(), SccbError<E>> {
        for (reg, val) in map.iter() {
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
    pub const MIRROR_VFLIP: u8 = 0x1E;
}
