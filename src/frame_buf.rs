//! Circular frame buffer which updates ping-pong DMA registers.

/// `FrameBuffer` is intialized with a base address, frame size (bytes), and the number frames
/// that can be stored. The `FrameBuffer` counts the number of frames written and stores them in
/// SDRAM via the OV9655 DMA address registers, in a circular buffer fashion.
#[derive(Clone, Debug)]
pub struct FrameBuffer {
    /// Base address for the frame buffer. This field does not change after calling `new`.
    mem_base: u32,

    /// Size of a single frame in bytes. This field does not change after calling `new`.
    frame_size: u32,

    /// Number of frames in the frame buffer. This field does not change after calling `new`.
    num_frames: u32,

    /// Number of frames currently captured (calls to `update`). Increases forever.
    num_caps: u32,

    /// Helper variable for the `Iterator` trait. Increases until the frame buffer is fully walked.
    iter_cnt: u32,
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
            iter_cnt: 0,
        }
    }

    /// Update frame buffer, `self.num_caps` completed and `self.num_caps + 1` is now underway.
    /// Return the current address.
    pub fn update(&mut self) -> u32 {
        // Get current and next addresses
        let curr_addr = self.get_addr(self.num_caps);
        let next_addr = self.get_addr(self.num_caps + 2);

        // Replace the address in `self.num_caps` with `self.num_caps + 2`
        match self.num_caps % 2 {
            0 => crate::ov9655::update_addr0(next_addr),
            _ => crate::ov9655::update_addr1(next_addr),
        };

        // Update and return current address
        self.num_caps += 1;
        curr_addr
    }

    /// Convert an index in the circular buffer to an address.
    fn get_addr(&self, index: u32) -> u32 {
        self.mem_base + (index % self.num_frames) * self.frame_size
    }
}

/// Allow for easy iteration of the frames in the frame buffer.
impl Iterator for FrameBuffer {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        // Usually the buffer will be full, but handle edge case where it is not
        let limit = match self.num_caps < self.num_frames {
            true => self.num_caps,
            false => self.num_frames,
        };

        // We will return the next address until the limit, then return None
        if self.iter_cnt < limit {
            // Walk over the frame buffer
            let start_index = self.num_caps - limit;
            let curr_addr = self.get_addr(start_index + self.iter_cnt);

            // Update iterator and return
            self.iter_cnt += 1;
            Some(curr_addr)
        } else {
            None
        }
    }
}
