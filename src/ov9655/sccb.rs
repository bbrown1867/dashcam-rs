//! A generic driver for the Serial Camera Control Bus on the OV9655 image sensor. Tested with the
//! STM32F767ZI microcontroller, but in theory should work on any microcontroller implementing the
//! embedded-hal I2C interface.

use core::marker::PhantomData;
use embedded_hal::blocking::i2c;

/// SCCB driver
pub struct SCCB<I2C> {
    // Ensures the same I2C type is used in all calls
    i2c: PhantomData<I2C>,
    // Even though this is constant, keeping it a a member for now in case it ever changes
    address: u8,
}

/// SCCB errors
#[derive(Debug, Eq, PartialEq)]
pub enum SccbError {
    /// I2C Write error
    I2cWriteError,
    /// I2C Read error
    I2cReadError,
    /// Read Manf ID error
    ReadManfId,
    /// Read Product ID error
    ReadProdId,
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

    /// I2C read wrapper
    fn i2c_read(&self, i2c: &mut I2C, buf: &mut [u8]) -> Result<u8, SccbError> {
        match i2c.read(self.address, buf) {
            Ok(()) => Ok(buf[0]),
            // Could log the exact I2C error of type E here
            Err(_) => Err(SccbError::I2cReadError),
        }
    }

    /// I2C write wrapper
    fn i2c_write(&self, i2c: &mut I2C, buf: &[u8]) -> Result<(), SccbError> {
        match i2c.write(self.address, buf) {
            Ok(()) => Ok(()),
            // Could log the exact I2C error of type E here
            Err(_) => Err(SccbError::I2cWriteError),
        }
    }

    //// Read a register
    pub fn read_register(&self, i2c: &mut I2C, reg: u8) -> Result<u8, SccbError> {
        let mut buf = [reg];
        self.i2c_write(i2c, &[reg])?;
        self.i2c_read(i2c, &mut buf)?;
        Ok(buf[0])
    }

    /// Write a register
    pub fn write_register(&self, i2c: &mut I2C, reg: u8, val: u8) -> Result<(), SccbError> {
        self.i2c_write(i2c, &[reg, val])
    }

    /// Check the device ID matches the expected value
    pub fn check_id(&self, i2c: &mut I2C) -> Result<(), SccbError> {
        let manf_id_msb: u16 = self.read_register(i2c, Register::MANF_ID_MSB)?.into();
        let manf_id_lsb: u16 = self.read_register(i2c, Register::MANF_ID_LSB)?.into();
        let manf_id: u16 = (manf_id_msb << 8) | manf_id_lsb;
        if manf_id != OV9655_MANF_ID {
            return Err(SccbError::ReadManfId);
        }

        let product_id_msb: u16 = self.read_register(i2c, Register::PROD_ID_MSB)?.into();
        let product_id_lsb: u16 = self.read_register(i2c, Register::PROD_ID_LSB)?.into();
        let product_id: u16 = (product_id_msb << 8) | product_id_lsb;
        if product_id != OV9655_PROD_ID {
            return Err(SccbError::ReadProdId);
        }

        Ok(())
    }

    pub fn reset(&self, i2c: &mut I2C) -> Result<(), SccbError> {
        self.write_register(i2c, Register::COM_CNTRL_7, OV9655_RESET_VAL)
    }
}

// TODO: Device specific stuff below here. How to make generic for any OmniVision device???

// Device address for is 0x60, however the I2C driver will left-shift the provided address by 1
const OV9655_ADDRESS: u8 = 0x30;

// Other misc constants
const OV9655_MANF_ID: u16 = 0x7FA2; // Weird that iit s not "OV" in ASCII...
const OV9655_PROD_ID: u16 = 0x9657; // Weird that it is not 9655...
const OV9655_RESET_VAL: u8 = 0x80;

#[non_exhaustive]
struct Register;

impl Register {
    pub const PROD_ID_MSB: u8 = 0x0A;
    pub const PROD_ID_LSB: u8 = 0x0B;
    pub const COM_CNTRL_7: u8 = 0x12;
    pub const MANF_ID_MSB: u8 = 0x1C;
    pub const MANF_ID_LSB: u8 = 0x1D;
}
