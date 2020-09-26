//! A generic driver for the Serial Camera Control Bus on the OV9655 image sensor. Tested with the
//! STM32F767ZI microcontroller, but in theory should work on any microcontroller implementing the
//! embedded-hal I2C interface.
//!
//! Could this be converted into an SCCB driver for any OmniVision image sensor? Would need to
//! abstract the different registers and other device specific information.

use core::marker::PhantomData;
use embedded_hal::blocking::i2c;
use heapless::{consts, LinearMap};

/// Statically allocated (size 200) linear map for mapping addresses (`u8`) to values (`u8`).
pub type RegMap = LinearMap<u8, u8, consts::U200>;

/// SCCB driver.
pub struct SCCB<I2C> {
    /// Marker to ensure the same I2C type is used in all calls.
    i2c: PhantomData<I2C>,
    /// Device I2C address.
    address: u8,
}

/// SCCB errors.
#[derive(Debug, Eq, PartialEq)]
pub enum SccbError<E> {
    /// I2C write error.
    I2cWrite(E),
    /// I2C read error.
    I2cRead(E),
    /// Manufacturer ID mismatch.
    ReadManfId,
    /// Product ID mismatch.
    ReadProdId,
    /// Register write-readback mismatch.
    RegMismatch((u8, u8)),
}

impl<I2C, E> SCCB<I2C>
where
    I2C: i2c::Read<Error = E> + i2c::Write<Error = E>,
{
    /// Creates a new SCCB driver associated with an I2C peripheral.
    pub fn new(_i2c: &I2C) -> Self {
        SCCB {
            i2c: PhantomData,
            address: OV9655_ADDRESS,
        }
    }

    /// I2C read wrapper for mapping `E --> SccbError`.
    fn i2c_read(&self, i2c: &mut I2C, buf: &mut [u8]) -> Result<(), SccbError<E>> {
        match i2c.read(self.address, buf) {
            Ok(()) => Ok(()),
            Err(e) => Err(SccbError::I2cRead(e)),
        }
    }

    /// I2C write wrapper for mapping `E --> SccbError`.
    fn i2c_write(&self, i2c: &mut I2C, buf: &[u8]) -> Result<(), SccbError<E>> {
        match i2c.write(self.address, buf) {
            Ok(()) => Ok(()),
            Err(e) => Err(SccbError::I2cWrite(e)),
        }
    }

    /// Read a register, must be two seperate transactions and we can't use `WriteRead`.
    fn read_register(&self, i2c: &mut I2C, reg: u8) -> Result<u8, SccbError<E>> {
        // Write the address
        self.i2c_write(i2c, &[reg])?;

        // Read the value
        let mut buf = [0x00];
        self.i2c_read(i2c, &mut buf)?;

        Ok(buf[0])
    }

    /// Write a register.
    fn write_register(&self, i2c: &mut I2C, reg: u8, val: u8) -> Result<(), SccbError<E>> {
        // Write the address and value
        self.i2c_write(i2c, &[reg, val])
    }

    /// Reset all registers to their default values.
    pub fn reset(&self, i2c: &mut I2C) -> Result<(), SccbError<E>> {
        // Setting the upper bit of this register resets all the registers
        let reg = self.read_register(i2c, Register::COM_CNTRL_07)?;
        self.write_register(i2c, Register::COM_CNTRL_07, reg | 0x80)
    }

    /// Check the device ID matches the expected value.
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

    /// Apply a register configuration specified by the linear map.
    pub fn apply_config(
        &self,
        i2c: &mut I2C,
        map: &RegMap,
        do_readback: bool,
    ) -> Result<(), SccbError<E>> {
        for (reg, val) in map.iter() {
            // Write the register
            self.write_register(i2c, *reg, *val)?;

            // Readback to check the write register worked
            if do_readback {
                let readback = self.read_register(i2c, *reg)?;
                if readback != *val {
                    return Err(SccbError::RegMismatch((*reg, readback)));
                }
            }
        }

        Ok(())
    }
}

/// Device address for is 0x60, however the I2C driver will left-shift the provided address by 1
const OV9655_ADDRESS: u8 = 0x30;

/// Expected manufacturer ID (weird that it is not "OV" in ASCII...)
const OV9655_MANF_ID: u16 = 0x7FA2;

/// Expected product ID (weird that it is not "9655"...)
const OV9655_PROD_ID: u16 = 0x9657;

/// Device register addresses.
struct Register;

impl Register {
    // Common control registers
    pub const COM_CNTRL_07: u8 = 0x12;

    // Product ID registers
    pub const PROD_ID_MSB: u8 = 0x0A;
    pub const PROD_ID_LSB: u8 = 0x0B;

    // Manufacturer ID registers
    pub const MANF_ID_MSB: u8 = 0x1C;
    pub const MANF_ID_LSB: u8 = 0x1D;
}
