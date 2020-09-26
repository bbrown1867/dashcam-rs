//! Prototype dashboard camera.

#![no_main]
#![no_std]

pub mod board;
pub mod ov9655;
pub mod util;

use board::stm32f746_disco::{board_config_ov9655, board_config_screen, board_get_hse, screen};
use ov9655::parallel::*;
use ov9655::sccb::{RegMap, SCCB};

use core::cell::Cell;
use core::convert::TryInto;
use core::panic::PanicInfo;
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

// Use the screen size for the frame buffer for now, eventually will be QVGA size
const WIDTH: u16 = screen::DISCO_SCREEN_CONFIG.active_width;
const HEIGHT: u16 = screen::DISCO_SCREEN_CONFIG.active_height;
const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize);
static mut FRAME_BUFFER: [u16; FRAME_SIZE] = [0; FRAME_SIZE];

// Shared memory between main thread and interrupts
static DCMI_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static DMA2_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

#[entry]
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

    // Configure the LCD screen (debug only)
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

    // DMA transfer description: QVGA resolution (320x240) + RGB565 format (2 bytes each pixel)
    let dma_size_bytes: u32 = 320 * 240 * 2;
    let dma_size_words: u32 = dma_size_bytes / 4;
    let mem_addr_sram: u32 = unsafe { &FRAME_BUFFER as *const _ as u32 };

    rprintln!(
        "Setting up DMA transfer of {} bytes to {:X}...",
        dma_size_bytes,
        mem_addr_sram
    );

    // Setup DCMI and DMA2 to transfer one image worth of words into memory
    dcmi_setup();
    dma2_setup(mem_addr_sram, dma_size_words.try_into().unwrap());

    /* Allow RTT buffer to flush and view screen */
    delay.delay_ms(500_u16);

    // Start capture!
    start_capture();

    // Debug
    let mut dcmi_b0: u32 = 0;
    let mut dcmi_b1: u32 = 0;
    let mut dcmi_b2: u32 = 0;
    let mut dcmi_b3: u32 = 0;
    let mut dcmi_b4: u32 = 0;

    // Capture a single image
    rprintln!("DCMI and DMA setup complete!");
    rprintln!("Starting image capture...");
    let mut cap_done = false;
    let mut timeout = 0;
    while !cap_done && timeout < 2000 {
        // Poll control reg
        let dcmi_regs = unsafe { &(*DCMI::ptr()) };
        let dcmi_cr = dcmi_regs.cr.read().bits();

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

        // Debug
        if dcmi_int_status & 0x1 == 0x1 {
            dcmi_b0 += 1;
        }

        if dcmi_int_status & 0x2 == 0x2 {
            dcmi_b1 += 1;
        }

        if dcmi_int_status & 0x4 == 0x04 {
            dcmi_b2 += 1;
        }

        if dcmi_int_status & 0x8 == 0x08 {
            dcmi_b3 += 1;
        }

        if dcmi_int_status & 0x10 == 0x10 {
            dcmi_b4 += 1;
        }

        // Stop after DMA transfer complete
        if dma2_int_status & 0x800 == 0x800 {
            rprintln!("Capture complete!");
            rprintln!("    DMA transfer complete.");
            cap_done = true;
        }

        // Stop after we capture a single frame
        if dcmi_int_status & 0x1 == 0x1 {
            rprintln!("Capture complete!");
            rprintln!("    DCMI CR  = {:X}", dcmi_cr);
            cap_done = true;
        }

        // Debug
        if dcmi_int_status != 0 || dma2_int_status != 0 {
            rprintln!("Interrupt fired!");
            rprintln!("    DCMI Int = {:X}", dcmi_int_status);
            rprintln!("    DMA2 Int = {:X}", dma2_int_status);
            rprintln!("    DCMI CR  = {:X}", dcmi_cr);
        }

        timeout += 1;
        delay.delay_ms(1_u16);
    }

    // Stop capture
    stop_capture();

    // Debug

    rprintln!("Num Frame Interrupts   = {}", dcmi_b0);
    rprintln!("Num Overrun Interrupts = {}", dcmi_b1);
    rprintln!("Num Error Interrupts   = {}", dcmi_b2);
    rprintln!("Num VSYNC Interrupts   = {}", dcmi_b3);
    rprintln!("Num Line Interrupts    = {}", dcmi_b4);

    // End of program
    loop {}
}

#[interrupt]
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
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    rprintln!("{:?}", _info);
    loop {}
}
