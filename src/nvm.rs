//! Abstraction layer for write frames to non-volatile memory.

use core::{fmt, slice::from_raw_parts_mut};

/// Any memory device that implements these traits will be compatible with the NVM driver.
pub trait Mem {
    type Error;

    /// Read `len` bytes at NVM device `src` address into `dest`.
    fn read(&mut self, dest: &mut [u8], src: u32, len: usize) -> Result<(), Self::Error>;

    /// Write `len` bytes in `src` to NVM device `dest` address.
    fn write(&mut self, dest: u32, src: &mut [u8], len: usize) -> Result<(), Self::Error>;

    /// Erase the NVM device, such that any section of it can be written.
    fn erase(&mut self) -> Result<(), Self::Error>;
}

/// Handle for the NVM driver.
pub struct NonVolatileMemory<MEM> {
    /// Memory device handle.
    device: MEM,
    /// Write pointer.
    write_ptr: u32,
}

impl<MEM, E> NonVolatileMemory<MEM>
where
    MEM: Mem<Error = E>,
    E: fmt::Debug,
{
    /// Initialize the NVM driver.
    pub fn new(mut device: MEM, start_addr: u32) -> Self {
        device.erase().expect("Could not erase NVM device!");
        NonVolatileMemory {
            device,
            write_ptr: start_addr,
        }
    }

    /// Save `size` bytes located in RAM at `src_address` to non-volatile memory.
    pub fn write(&mut self, src_address: u32, size: usize) -> Result<(), E> {
        let src_buf: &mut [u8] = unsafe { from_raw_parts_mut(src_address as *mut u8, size) };
        self.device.write(self.write_ptr, src_buf, size)?;
        self.write_ptr += size as u32;
        Ok(())
    }
}
