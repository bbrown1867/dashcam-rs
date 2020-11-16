//! Abstraction layer for write frames to non-volatile memory.

/// Any memory device that implements these traits will be compatible with the NVM driver.
pub trait Mem {
    type Error;

    /// Read `len` bytes at NVM device `src` address into `dest`.
    fn read(&mut self, dest: &mut [u8], src: u32, len: usize) -> Result<(), Self::Error>;

    /// Write `len` bytes in `src` to NVM device `dest` address.
    fn write(&mut self, dest: u32, src: &mut [u8], len: usize) -> Result<(), Self::Error>;

    /// Erase at least `len` bytes at `src`. Return a pair containing (num bytes erased, starting
    /// address for erase). These may differ from input arguments due to NVM device limitations.
    fn erase(&mut self, src: u32, len: usize) -> Result<(u32, u32), Self::Error>;
}

/// Handle for the NVM driver.
pub struct NonVolatileMemory<MEM> {
    _device: MEM,
    write_ptr: u32,
}

impl<MEM, E> NonVolatileMemory<MEM>
where
    MEM: Mem<Error = E>,
{
    pub fn new(device: MEM) -> Self {
        NonVolatileMemory {
            _device: device,
            write_ptr: 0,
        }
    }

    pub fn write(&mut self, _src_address: u32, size: u32) {
        // Erase non-volatile memory

        // Write to pointer...

        // Readback to verify?

        self.write_ptr += size;
    }
}
