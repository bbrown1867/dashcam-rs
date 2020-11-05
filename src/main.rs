//! Prototype car dashboard camera.

#![no_std]
#![no_main]

mod board;
mod frame_buf;
mod nvm;
mod ov9655;
mod util;

use ov9655::{FRAME_HEIGHT, FRAME_RATE, FRAME_SIZE, FRAME_WIDTH};
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    gpio::ExtiPin,
    pac,
    prelude::_embedded_hal_blocking_delay_DelayMs,
    rcc::{HSEClock, HSEClockMode, RccExt},
    time::U32Ext,
};

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
const APP: () = {
    // Static resources.
    struct Resources {
        nvm: nvm::NonVolatileMemory,
        fb: frame_buf::FrameBuffer,
        button: board::ButtonPin,
        delay: Delay,
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
        let mut rcc = pac_periph.RCC;

        // Setup button
        let button = board::setup_button(
            &mut rcc,
            pac_periph.SYSCFG,
            pac_periph.EXTI,
            pac_periph.GPIOI,
        );

        // Setup QSPI
        board::qspi::init(
            &mut rcc,
            pac_periph.GPIOB,
            pac_periph.GPIOD,
            pac_periph.GPIOE,
            pac_periph.QUADSPI,
        );

        board::qspi::check_id().unwrap();

        // Clocking: Set HSE to reflect the board and ramp up SYSCLK to max possible speed
        let mut rcc = rcc.constrain();
        let hse_cfg = HSEClock::new(board::get_xtal(), HSEClockMode::Oscillator);
        let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();
        let mut delay = Delay::new(cm_periph.SYST, clocks);

        // LCD screen
        let mut display = board::display::config();
        board::display::draw_welcome(&mut display);

        // SDRAM
        let (sdram_ptr, sdram_size) = board::sdram::init(&clocks, &mut delay);

        // NVM
        let nvm = nvm::NonVolatileMemory::new();

        // OV9655
        ov9655::init(pac_periph.I2C1, &mut rcc.apb1, clocks, &mut delay);

        // Initialize frame buffer
        let fb = frame_buf::FrameBuffer::new(sdram_ptr as u32, sdram_size as u32, FRAME_SIZE);

        // Allow RTT buffer to flush and give time to view screen prior to starting
        rprintln!("Starting image capture...");
        delay.delay_ms(500_u32);

        // Start capture
        ov9655::start();

        // Initialize static resources
        init::LateResources {
            nvm,
            fb,
            button,
            delay,
        }
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
    fn dma_isr(mut cx: dma_isr::Context) {
        // See if a frame capture completed, handle_dma_done will clear pending interrupt
        if ov9655::handle_dma_done() {
            // Update circular frame buffer, must be done in a lock since lower priority task
            let address = cx.resources.fb.lock(|fb| fb.update());

            // Draw image on display using DMA2D
            match board::display::draw_image(address, FRAME_WIDTH, FRAME_HEIGHT) {
                true => rprintln!("Error: Cannot display image. Frame rate too fast!"),
                false => (),
            };
        }
    }

    // Handle a button interrupt. At the moment this does a playback of frames in SDRAM.
    #[task(binds = EXTI15_10, priority = 2, resources = [nvm, fb, button, delay])]
    fn button_isr(cx: button_isr::Context) {
        let nvm: &mut nvm::NonVolatileMemory = cx.resources.nvm;
        let fb: &mut frame_buf::FrameBuffer = cx.resources.fb;
        let button: &mut board::ButtonPin = cx.resources.button;
        let delay: &mut Delay = cx.resources.delay;

        // Clear pending interrupt
        button.clear_interrupt_pending_bit();

        // Stop capturing frames
        ov9655::stop();

        // Write frames to non-volatile memory
        let curr_fb = fb.clone();
        for address in curr_fb {
            nvm.write(address, FRAME_SIZE);
        }

        // Now cycle through the frames in the buffer and display them
        rprintln!("Playing back images in frame buffer!");
        loop {
            // Clone since we do this on a loop
            let curr_fb = fb.clone();

            // Iterate on the frame buffer
            for address in curr_fb {
                // Draw image on display using DMA2D
                match board::display::draw_image(address, FRAME_WIDTH, FRAME_HEIGHT) {
                    true => rprintln!("Error: Cannot display image. Frame rate too fast!"),
                    false => (),
                };

                // Block to simulate captured frame rate
                delay.delay_ms(FRAME_RATE);
            }
        }
    }
};
