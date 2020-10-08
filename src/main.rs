//! Prototype dashboard camera.

#![no_main]
#![no_std]

pub mod board;
pub mod ov9655;
pub mod util;

use board::stm32f746_disco::*;
use ov9655::parallel::*;
use ov9655::sccb::{RegMap, SCCB};

use core::{cell::Cell, convert::TryInto};
use cortex_m::interrupt::{free, Mutex};
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
    pac::{self, DMA2},
    rcc::{HSEClock, HSEClockMode, RccExt},
    time::U32Ext,
};

use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;

// QVGA size + RGB565 format (2 bytes per pixel)
pub const QVGA_WIDTH: u16 = 320;
pub const QVGA_HEIGHT: u16 = 240;
pub const QVGA_SIZE: u32 = (QVGA_WIDTH as u32) * (QVGA_HEIGHT as u32) * 2;

// Shared memory between main thread and interrupts
static DMA2_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        delay: Delay,
        frame_buffer1: u32,
        frame_buffer2: u32,
    }

    /// Program entry point.
    #[init]
    fn init(cx: init::Context) -> init::LateResources {
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
        let cm_periph: cortex_m::Peripherals = cx.core;
        let pac_periph: pac::Peripherals = cx.device;

        // Clock config: Set HSE to reflect the board and ramp up SYSCLK to max possible speed
        let mut rcc = pac_periph.RCC.constrain();
        let hse_cfg = HSEClock::new(board_get_hse(), HSEClockMode::Oscillator);
        let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();

        // Delay configuration
        let mut delay = Delay::new(cm_periph.SYST, clocks);

        // SDRAM configuration
        let (sdram_ptr, sdram_size) = board_config_sdram(&clocks);
        rprintln!(
            "SDRAM configuration complete! Address = {:?}, Size = {}",
            sdram_ptr,
            sdram_size
        );

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
            style = primitive_style!(fill_color = Rgb565::BLUE)
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

        // DMA transfer description
        let dma_size_words: u32 = QVGA_SIZE / 4;
        let frame_buffer1: u32 = sdram_ptr as u32;
        let frame_buffer2: u32 = frame_buffer1 + QVGA_SIZE;

        rprintln!(
            "DMA transfer: {} bytes to address 0x{:X} and 0x{:X}",
            QVGA_SIZE,
            frame_buffer1,
            frame_buffer2
        );

        // Setup DCMI and DMA2 to transfer from the DCMI peripheral into memory
        dcmi_setup();
        dma2_setup(
            frame_buffer1,
            frame_buffer2,
            dma_size_words.try_into().unwrap(),
        );

        init::LateResources {
            delay,
            frame_buffer1,
            frame_buffer2,
        }
    }

    /// Idle task.
    #[idle(resources = [delay, frame_buffer1, frame_buffer2])]
    fn idle(cx: idle::Context) -> ! {
        let delay = cx.resources.delay;
        let frame_buffer1 = *cx.resources.frame_buffer1;
        let frame_buffer2 = *cx.resources.frame_buffer2;

        // Allow RTT buffer to flush and give time to view screen prior to starting
        rprintln!("Starting image capture...");
        delay.delay_ms(500_u16);
        start_capture();

        // Capture a single image
        let mut num_caps = 0;
        while num_caps < 1000 {
            // Poll interrupt shared memory
            let mut dma2_int_status: u32 = 0;
            free(|cs| {
                dma2_int_status = DMA2_INT_STATUS.borrow(cs).get();

                if dma2_int_status != 0 {
                    DMA2_INT_STATUS.borrow(cs).set(0);
                }
            });

            // Check if DMA transfer completed
            if dma2_int_status & 0x800 == 0x800 {
                // Determine which frame buffer in the ping-pong DMA
                let frame_buffer = match num_caps % 2 {
                    0 => frame_buffer1,
                    _ => frame_buffer2,
                };

                rprintln!("Capture complete into frame buffer = {:X}", frame_buffer);

                // Draw image on display using DMA2D
                match board_draw_image(frame_buffer, QVGA_WIDTH, QVGA_HEIGHT) {
                    true => rprintln!("\tCannot display image. Frame rate faster than DMA2D!"),
                    false => (),
                };

                num_caps += 1;
            }
        }

        // Stop capture
        stop_capture();

        // End of program
        loop {}
    }

    /// DMA2 Stream 1 interrupt handler.
    #[task(binds = DMA2_STREAM1, priority = 1)]
    fn dma_isr(_: dma_isr::Context) {
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
};
