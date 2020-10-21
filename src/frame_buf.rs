//! Circular frame buffer which updates ping-pong DMA registers using the Iterator pattern.

#[derive(Debug)]
pub struct FrameBuffer {
    mem_base: u32,
    frame_size: u32,
    pub num_frames: u32,
    pub num_caps: u32,
}

impl FrameBuffer {
    /// Creates a new FrameBuffer object.
    pub fn new(base: u32, size: u32, fsize: u32) -> Self {
        crate::ov9655::update_addr0(base);
        crate::ov9655::update_addr1(base + fsize);

        FrameBuffer {
            mem_base: base,
            frame_size: fsize,
            num_frames: size / fsize,
            num_caps: 0,
        }
    }

    /// Convert an index in the circular buffer to an address.
    fn get_addr(&self, index: u32) -> u32 {
        self.mem_base + index * self.frame_size
    }
}

impl Iterator for FrameBuffer {
    type Item = u32;

    /// Return the current address and update the next address in the ping-pong DMA registers.
    fn next(&mut self) -> Option<u32> {
        let curr_index = self.num_caps % self.num_frames;
        let curr_addr = self.get_addr(curr_index);

        // Note: DMA controller will automatically move onto address in self.num_caps + 1 index
        let next_index = (self.num_caps + 2) % self.num_frames;
        let next_addr = self.get_addr(next_index);

        match self.num_caps % 2 {
            0 => crate::ov9655::update_addr0(next_addr),
            _ => crate::ov9655::update_addr1(next_addr),
        };

        self.num_caps += 1;

        Some(curr_addr)
    }
}
