//! Miscellaneous helper functions.

use core::panic::PanicInfo;
use rtt_target::rprintln;

/// Sets `size` items of type T located at `addr` to `val`.
#[allow(dead_code)]
pub fn memory_set<T: Copy>(addr: u32, size: usize, val: T) {
    for i in 0..size {
        unsafe {
            let curr: *mut T = (addr + i as u32) as *mut T;
            core::ptr::write_volatile(curr, val);
        }
    }
}

/// Prints `size` bytes located at `addr` using RTT.
#[allow(dead_code)]
pub fn memory_get(addr: u32, size: usize) {
    rprintln!("{} bytes located at address {:X}:", size, addr);

    for i in 0..size {
        unsafe {
            let curr: *mut u8 = (addr + i as u32) as *mut u8;
            let val: u8 = core::ptr::read_volatile(curr);
            rprintln!("\t{:X}", val);
        }
    }
}

/// Custom handler to use RTT when a panic occurs.
#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    rprintln!("{:?}", _info);
    loop {}
}
