//! Abstraction layer for write frames to non-volatile memory.

/// Handle for the NVM driver.
pub struct NonVolatileMemory {
    write_ptr: u32,
}

impl NonVolatileMemory {
    pub fn new() -> Self {
        NonVolatileMemory { write_ptr: 0 }
    }

    pub fn write(&mut self, _src_address: u32, size: u32) {
        // Erase non-volatile memory

        // Write to pointer...

        // Readback to verify?

        self.write_ptr += size;
    }
}
