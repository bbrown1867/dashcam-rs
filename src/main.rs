//! Prototype car dashboard camera.

#![no_std]
#![no_main]

mod board;
mod frame_buf;
mod ov9655;
mod util;

use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    pac::{self, DMA2},
    prelude::_embedded_hal_blocking_delay_DelayMs,
    rcc::{HSEClock, HSEClockMode, RccExt},
    time::U32Ext,
};

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
const APP: () = {
    // Static resources.
    struct Resources {
        fb: frame_buf::FrameBuffer,
    }

    // Program entry point.
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

        // Get peripherals from RTIC
        let pac_periph: pac::Peripherals = cx.device;
        let cm_periph: cortex_m::Peripherals = cx.core;

        // Clocking: Set HSE to reflect the board and ramp up SYSCLK to max possible speed
        let mut rcc = pac_periph.RCC.constrain();
        let hse_cfg = HSEClock::new(board::get_xtal(), HSEClockMode::Oscillator);
        let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();
        let mut delay = Delay::new(cm_periph.SYST, clocks);

        // LCD screen
        let mut display = board::display::config();
        board::display::draw_welcome(&mut display);

        // SDRAM
        let (sdram_ptr, sdram_size) = board::sdram::init(&clocks, &mut delay);

        // OV9655
        ov9655::init(pac_periph.I2C1, &mut rcc.apb1, clocks, &mut delay);

        // Initialize frame buffer
        let fb =
            frame_buf::FrameBuffer::new(sdram_ptr as u32, sdram_size as u32, ov9655::FRAME_SIZE);

        // Allow RTT buffer to flush and give time to view screen prior to starting
        rprintln!("Starting image capture...");
        delay.delay_ms(500_u16);

        // Start capture
        ov9655::start();

        // Initialize static resources
        init::LateResources { fb }
    }

    // Idle task.
    #[idle]
    fn idle(_: idle::Context) -> ! {
        // TODO: Enter low-power mode with WFI?
        loop {
            cortex_m::asm::nop();
        }
    }

    // Handle DMA interrupts. A DMA DONE interrupt indicates a frame was captured in memory.
    #[task(binds = DMA2_STREAM1, priority = 1, resources = [fb])]
    fn dma_isr(cx: dma_isr::Context) {
        // Read and clear interrupt status
        let int_status = unsafe {
            let dma2_regs = &(*DMA2::ptr());
            let int_status = dma2_regs.lisr.read().bits();
            dma2_regs.lifcr.write(|w| w.bits(int_status));
            int_status
        };

        // TODO: Remove this eventually
        if cx.resources.fb.num_caps == 1000 {
            rprintln!("Done!");
            ov9655::stop();
            return;
        }

        // See if a frame capture completed
        if int_status & 0x800 == 0x800 {
            // Update circular frame buffer
            let frame_buffer = cx.resources.fb.next().unwrap();

            // Draw image on display using DMA2D
            match board::display::draw_image(
                frame_buffer,
                ov9655::FRAME_WIDTH,
                ov9655::FRAME_HEIGHT,
            ) {
                true => rprintln!("Error: Cannot display image. Frame rate faster than DMA2D!"),
                false => (),
            };

            rprintln!("Capture complete into frame buffer = {:X}", frame_buffer);
        }
    }
};
