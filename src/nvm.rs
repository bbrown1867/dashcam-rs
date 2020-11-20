//! Abstraction layer for reading and writing frames to non-volatile memory.

use core::fmt;

/// Memory device API used by the `NonVolatileMemory` driver.
pub trait Mem {
    type Error;

    /// Read `len` bytes at NVM address `src` address to RAM address `dst`.
    fn read(&mut self, dst: u32, src: u32, len: usize) -> Result<(), Self::Error>;

    /// Write `len` bytes at RAM address `src` to NVM address `dst`.
    fn write(&mut self, dst: u32, src: u32, len: usize) -> Result<(), Self::Error>;

    /// Erase the NVM device, such that any section of it can be written.
    fn erase(&mut self) -> Result<(), Self::Error>;
}

/// Handle for the NVM driver.
pub struct NonVolatileMemory<MEM> {
    /// Memory device handle.
    device: MEM,
    /// Write pointer (NVM address).
    write_ptr: u32,
    /// Read pointer (NVM address).
    read_ptr: u32,
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
            read_ptr: start_addr,
        }
    }

    pub fn get_write_ptr(&mut self) -> u32 {
        self.write_ptr
    }

    /// Write `size` bytes located in RAM at `src_address` to non-volatile memory.
    pub fn write(&mut self, src_address: u32, size: usize) -> Result<(), E> {
        self.device.write(self.write_ptr, src_address, size)?;
        self.write_ptr += size as u32;
        Ok(())
    }

    /// Read `size` bytes located in non-volatile memory to SDRAM at `dst_address`.
    pub fn read(&mut self, dst_address: u32, size: usize) -> Result<(), E> {
        self.device.read(dst_address, self.read_ptr, size)?;
        self.read_ptr += size as u32;
        Ok(())
    }
}
