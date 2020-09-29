//! Prototype dashboard camera.

#![no_main]
#![no_std]

pub mod board;
pub mod ov9655;
pub mod util;

use board::stm32f746_disco::*;
use ov9655::parallel::*;
use ov9655::sccb::{RegMap, SCCB};

use core::{cell::Cell, convert::TryInto, panic::PanicInfo};
use cortex_m::interrupt::{free, Mutex};
use cortex_m_rt::entry;
use embedded_graphics::{
    egrectangle, egtext,
    fonts::Font6x8,
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
    primitive_style, text_style,
};
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    i2c::{BlockingI2c, Mode},
    interrupt,
    pac::{self, DCMI, DMA2},
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

// QVGA size
const QVGA_WIDTH: u32 = 320;
const QVGA_HEIGHT: u32 = 240;

// Use the screen size for the frame buffer for now, eventually will be QVGA size
const WIDTH: u16 = screen::DISCO_SCREEN_CONFIG.active_width;
const HEIGHT: u16 = screen::DISCO_SCREEN_CONFIG.active_height;
const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize);
static mut FRAME_BUFFER: [u16; FRAME_SIZE] = [0; FRAME_SIZE];

// Shared memory between main thread and interrupts
static DCMI_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static DMA2_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

#[entry]
/// Program entry point.
fn main() -> ! {
    // Setup RTT for logging
    let channels = rtt_init! {
        up: {
            0: {
                size: 4096
                mode: BlockIfFull
                name: "Terminal"
            }
        }
    };

    set_print_channel(channels.up.0);

    // Get peripherals
    let pac_periph = pac::Peripherals::take().unwrap();
    let cm_periph = cortex_m::Peripherals::take().unwrap();

    // Clock config: Set HSE to reflect the board and ramp up SYSCLK to max possible speed
    let mut rcc = pac_periph.RCC.constrain();
    let hse_cfg = HSEClock::new(board_get_hse(), HSEClockMode::Oscillator);
    let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();

    // Delay configuration
    let mut delay = Delay::new(cm_periph.SYST, clocks);

    // SDRAM configuration
    let (ram_ptr, ram_size) = board_config_sdram(&clocks);
    rprintln!(
        "SDRAM configuration complete! Address = {:?}, Size = {}",
        ram_ptr,
        ram_size
    );

    // SDRAM testing. Two observations:
    //  - Only lower 16-bits seem to go across the data bus
    //  - Only can do word-aligned addresses
    unsafe {
        let mut ram_test: u32 = 0xAAAA_BBBB;

        core::ptr::write_volatile(ram_ptr, ram_test);
        ram_test = core::ptr::read_volatile(ram_ptr);
        rprintln!("Read {:X} from SDRAM.", ram_test);

        ram_test = 0xCCCC_DDDD;
        util::memory_set(0xC000_0004, 1, ram_test);
        util::memory_get(0xC000_0004, 4);

        ram_test = 0xEEEE_FFFF;
        util::memory_set(0xC000_0008, 1, ram_test);
        util::memory_get(0xC000_0008, 4);
    }

    // OV9655 GPIO configuration
    let i2c_pins = board_config_ov9655();

    // I2C1 configuration (SCCB)
    let mut i2c = BlockingI2c::i2c1(
        pac_periph.I2C1,
        i2c_pins,
        Mode::standard(100.khz()),
        clocks,
        &mut rcc.apb1,
        10000,
    );

    // Init SCCB module
    let sccb = SCCB::new(&mut i2c);

    // Establish communication with the OV9655
    sccb.reset(&mut i2c).unwrap();
    delay.delay_ms(1000_u16);
    sccb.check_id(&mut i2c).unwrap();
    rprintln!("Successfully communicated with the OV9655 over SCCB!");

    // Configure the OV9655 for QVGA (320x240) resolution with RGB565
    let mut reg_vals = RegMap::new();
    ov9655::get_config(&mut reg_vals);
    sccb.apply_config(&mut i2c, &reg_vals, false).unwrap();
    rprintln!("QVGA mode setup for the OV9655 complete!");

    // Configure the LCD screen (for debug purposes)
    let mut display = board_config_screen();
    let display = &mut display;

    // Debug
    egrectangle!(
        top_left = (0, 0),
        bottom_right = (479, 271),
        style = primitive_style!(fill_color = Rgb565::new(0, 0b11110, 0b11011))
    )
    .draw(display)
    .ok();

    delay.delay_ms(500_u16);

    egtext!(
        text = "Hello Rust!",
        top_left = (100, 100),
        style = text_style!(font = Font6x8, text_color = RgbColor::WHITE)
    )
    .draw(display)
    .ok();

    // DMA transfer description: QVGA resolution + RGB565 format (2 bytes each pixel)
    let dma_size_bytes: u32 = QVGA_WIDTH * QVGA_HEIGHT * 2;
    let dma_size_words: u32 = dma_size_bytes / 4;
    let mem_addr_sram: u32 = unsafe { &FRAME_BUFFER as *const _ as u32 };

    rprintln!(
        "DMA transfer: {} bytes to address 0x{:X}",
        dma_size_bytes,
        mem_addr_sram
    );

    // Setup DCMI and DMA2 to transfer from the DCMI peripheral into memory
    dcmi_setup();
    dma2_setup(mem_addr_sram, dma_size_words.try_into().unwrap());

    // Allow RTT buffer to flush and give time to view screen prior to starting
    rprintln!("Starting image capture...");
    delay.delay_ms(500_u16);
    start_capture();

    // Debug
    let mut dcmi_bits: [u32; 5] = [0; 5];
    let mut dma2_bits: [u32; 4] = [0; 4];

    // Capture a single image
    let mut num_caps = 0;
    while num_caps < 10 {
        // Poll interrupt shared memory
        let mut dcmi_int_status: u32 = 0;
        let mut dma2_int_status: u32 = 0;
        free(|cs| {
            dcmi_int_status = DCMI_INT_STATUS.borrow(cs).get();
            dma2_int_status = DMA2_INT_STATUS.borrow(cs).get();

            if dcmi_int_status != 0 {
                DCMI_INT_STATUS.borrow(cs).set(0);
            }

            if dma2_int_status != 0 {
                DMA2_INT_STATUS.borrow(cs).set(0);
            }
        });

        // Check if DMA transfer completed
        if dma2_int_status & 0x800 == 0x800 {
            num_caps += 1;
        }

        // Debug
        for x in 0..5 {
            if dcmi_int_status & (1 << x) == (1 << x) {
                dcmi_bits[x] += 1;
            }
        }

        // Debug
        for x in 0..4 {
            let y = 8 + x;
            if dma2_int_status & (1 << y) == (1 << y) {
                dma2_bits[x] += 1;
            }
        }
    }

    // Stop capture
    stop_capture();

    // Debug
    rprintln!("Capture complete!");
    rprintln!("    Num DMA2 Direct Error Interrupts   = {}", dma2_bits[0]);
    rprintln!("    Num DMA2 Transfer Error Interrupts = {}", dma2_bits[1]);
    rprintln!("    Num DMA2 Halfway Interrupts        = {}", dma2_bits[2]);
    rprintln!("    Num DMA2 Done Interrupts           = {}", dma2_bits[3]);
    rprintln!("    Num DCMI Frame Interrupts          = {}", dcmi_bits[0]);
    rprintln!("    Num DCMI Overrun Interrupts        = {}", dcmi_bits[1]);
    rprintln!("    Num DCMI Error Interrupts          = {}", dcmi_bits[2]);
    rprintln!("    Num DCMI VSYNC Interrupts          = {}", dcmi_bits[3]);
    rprintln!("    Num DCMI Line Interrupts           = {}", dcmi_bits[4]);

    // End of program
    loop {}
}

#[interrupt]
/// DCMI interrupt handler. Determines which interrupts fired and passes to main thread.
fn DCMI() {
    // Read and clear interrupt status
    let int_status = unsafe {
        let dcmi_regs = &(*DCMI::ptr());
        let int_status = dcmi_regs.ris.read().bits();
        dcmi_regs.icr.write(|w| w.bits(int_status));
        int_status
    };

    // Signal interrupt status to main thread
    free(|cs| {
        // If main thread is not processing a previous interrupt
        if DCMI_INT_STATUS.borrow(cs).get() == 0 {
            // If an interrupt fired
            if int_status != 0 {
                DCMI_INT_STATUS.borrow(cs).set(int_status);
            }
        }
    });
}

#[interrupt]
/// DMA2 Stream 1 interrupt handler. Determines which interrupts fired and passes to main thread.
fn DMA2_STREAM1() {
    // Read and clear interrupt status
    let int_status = unsafe {
        let dma2_regs = &(*DMA2::ptr());
        let int_status = dma2_regs.lisr.read().bits();
        dma2_regs.lifcr.write(|w| w.bits(int_status));
        int_status
    };

    // Signal interrupt status to main thread
    free(|cs| {
        // If main thread is not processing a previous interrupt
        if DMA2_INT_STATUS.borrow(cs).get() == 0 {
            // If an interrupt fired
            if int_status != 0 {
                DMA2_INT_STATUS.borrow(cs).set(int_status);
            }
        }
    });
}

#[inline(never)]
#[panic_handler]
/// Custom handler to use RTT when a panic occurs.
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    rprintln!("{:?}", _info);
    loop {}
}
