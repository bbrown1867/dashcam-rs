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
        self.write_register(i2c, Register::COM_CNTRL_7, 0x80)
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

    /// Perform a sequence of register writes to setup the OV9655 for QVGA mode
    pub fn qvga_setup(&self, i2c: &mut I2C) -> Result<(), SccbError<E>> {
        // TODO: Revisit these registers

        // Registers range from 0x00 to 0xC7, although we don't write every one
        let mut reg_vals = LinearMap::<u8, u8, consts::U200>::new();
        reg_vals.insert(0x00, 0x00).unwrap();
        reg_vals.insert(0x01, 0x80).unwrap();
        reg_vals.insert(0x02, 0x80).unwrap();
        reg_vals.insert(0x03, 0x02).unwrap();
        reg_vals.insert(0x04, 0x00).unwrap();
        reg_vals.insert(0x09, 0x03).unwrap();
        reg_vals.insert(0x0b, 0x57).unwrap();
        reg_vals.insert(0x0e, 0x01).unwrap();
        reg_vals.insert(0x0f, 0xc0).unwrap();
        reg_vals.insert(0x10, 0x50).unwrap();
        reg_vals.insert(0x11, 0x80).unwrap();
        reg_vals.insert(0x12, 0x63).unwrap();
        reg_vals.insert(0x13, 0xef).unwrap();
        reg_vals.insert(0x14, 0x3a).unwrap();
        reg_vals.insert(0x15, 0x18).unwrap();
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
        reg_vals.insert(0x32, 0x12).unwrap();
        reg_vals.insert(0x33, 0x00).unwrap();
        reg_vals.insert(0x34, 0x3f).unwrap();
        reg_vals.insert(0x35, 0x00).unwrap();
        reg_vals.insert(0x36, 0x3a).unwrap();
        reg_vals.insert(0x38, 0x72).unwrap();
        reg_vals.insert(0x39, 0x57).unwrap();
        reg_vals.insert(0x3a, 0xca).unwrap();
        reg_vals.insert(0x3b, 0x04).unwrap();
        reg_vals.insert(0x3d, 0x99).unwrap();
        reg_vals.insert(0x3e, 0x02).unwrap();
        //reg_vals.insert(0x3f, 0xc1).unwrap();
        reg_vals.insert(0x40, 0xd0).unwrap();
        reg_vals.insert(0x41, 0x41).unwrap();
        reg_vals.insert(0x42, 0xc0).unwrap();
        // reg_vals.insert(0x43, 0x0a).unwrap();
        // reg_vals.insert(0x44, 0xf0).unwrap();
        // reg_vals.insert(0x45, 0x46).unwrap();
        // reg_vals.insert(0x46, 0x62).unwrap();
        // reg_vals.insert(0x47, 0x2a).unwrap();
        // reg_vals.insert(0x48, 0x3c).unwrap();
        // //reg_vals.insert(0x4a, 0xfc).unwrap();
        // reg_vals.insert(0x4b, 0xfc).unwrap();
        // reg_vals.insert(0x4c, 0x7f).unwrap();
        // reg_vals.insert(0x4d, 0x7f).unwrap();
        // reg_vals.insert(0x4e, 0x7f).unwrap();
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
        reg_vals.insert(0x6b, 0x0a).unwrap();
        reg_vals.insert(0x6c, 0x04).unwrap();
        reg_vals.insert(0x6d, 0x55).unwrap();
        reg_vals.insert(0x6e, 0x00).unwrap();
        reg_vals.insert(0x6f, 0x9d).unwrap();
        reg_vals.insert(0x70, 0x21).unwrap();
        reg_vals.insert(0x71, 0x78).unwrap();
        reg_vals.insert(0x72, 0x11).unwrap();
        reg_vals.insert(0x73, 0x01).unwrap();
        reg_vals.insert(0x74, 0x10).unwrap();
        reg_vals.insert(0x75, 0x10).unwrap();
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
        // reg_vals.insert(0xa1, 0x1f).unwrap();
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
        reg_vals.insert(0xc7, 0x81).unwrap();
        reg_vals.insert(0xc9, 0xe0).unwrap();
        reg_vals.insert(0xca, 0xe8).unwrap();
        reg_vals.insert(0xcb, 0xf0).unwrap();
        reg_vals.insert(0xcc, 0xd8).unwrap();
        reg_vals.insert(0xcd, 0x93).unwrap();

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
struct Register;
impl Register {
    pub const PROD_ID_MSB: u8 = 0x0A;
    pub const PROD_ID_LSB: u8 = 0x0B;
    pub const COM_CNTRL_7: u8 = 0x12;
    pub const MANF_ID_MSB: u8 = 0x1C;
    pub const MANF_ID_LSB: u8 = 0x1D;
}
