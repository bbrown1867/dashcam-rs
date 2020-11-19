//! Prototype car dashboard camera.

#![no_std]
#![no_main]

mod board;
mod frame_buf;
mod nvm;
mod ov9655;
mod util;

use board::{
    display, get_xtal,
    qspi::{self, QspiDriver},
    sdram, setup_button, ButtonPin,
};
use frame_buf::FrameBuffer;
use nvm::NonVolatileMemory;
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

/// Alias for NVM driver that uses QSPI flash.
type NvmDriver = NonVolatileMemory<QspiDriver>;

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
const APP: () = {
    // Static resources.
    struct Resources {
        nvm: NvmDriver,
        fb1: FrameBuffer,
        fb2: FrameBuffer,
        but: ButtonPin,
        dly: Delay,
        pbn: u32,
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
        let but = setup_button(
            &mut rcc,
            pac_periph.SYSCFG,
            pac_periph.EXTI,
            pac_periph.GPIOI,
        );

        // Setup QSPI
        let mut qspi = QspiDriver::new(
            &mut rcc,
            pac_periph.GPIOB,
            pac_periph.GPIOD,
            pac_periph.GPIOE,
            pac_periph.QUADSPI,
        );

        // Clocking: Set HSE to reflect hardware and ramp up SYSCLK to max possible speed
        let mut rcc = rcc.constrain();
        let hse_cfg = HSEClock::new(get_xtal(), HSEClockMode::Oscillator);
        let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();
        let mut dly = Delay::new(cm_periph.SYST, clocks);

        // Test QSPI
        qspi.check_id().unwrap();
        qspi::tests::test_mem(&mut qspi);
        rprintln!("QSPI driver successfully initialized!");

        // LCD screen
        let mut display = display::config();
        display::draw_message(&mut display, "Hello Dashcam!");

        // SDRAM
        let (sdram_ptr, sdram_size) = sdram::init(&clocks, &mut dly);

        // NVM
        let nvm = NvmDriver::new(qspi, 0);
        rprintln!("NVM driver successfully initialized and erased!");

        // OV9655
        ov9655::init(pac_periph.I2C1, &mut rcc.apb1, clocks, &mut dly);

        // Initialize frame buffers: One for capture and one for replay
        let fb1 = FrameBuffer::new(sdram_ptr as u32, sdram_size as u32, FRAME_SIZE);
        let fb2 = FrameBuffer::new(sdram_ptr as u32, sdram_size as u32, FRAME_SIZE);

        // Allow RTT buffer to flush and give time to view screen prior to starting
        rprintln!("Starting image capture...");
        dly.delay_ms(500_u32);

        // Start capture
        ov9655::start();

        // Initialize static resources
        init::LateResources {
            nvm,
            fb1,
            fb2,
            but,
            dly,
            pbn: 0,
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
    #[task(binds = DMA2_STREAM1, priority = 1, resources = [fb1])]
    fn dma_isr(mut cx: dma_isr::Context) {
        // See if a frame capture completed, handle_dma_done will clear pending interrupt
        if ov9655::handle_dma_done() {
            // Update circular frame buffer, must be done in a lock since lower priority task
            let address = cx.resources.fb1.lock(|fb1| fb1.update(true));

            // Draw image on display using DMA2D
            match display::draw_image(address, FRAME_WIDTH, FRAME_HEIGHT) {
                true => rprintln!("Error: Cannot display image. Frame rate too fast!"),
                false => (),
            };
        }
    }

    // Handle a button interrupt. First press saves buffered video to NVM, second press reads
    // saved video from NVM and plays it on the display in a loop.
    #[task(binds = EXTI15_10, priority = 2, resources = [nvm, fb1, fb2, but, dly, pbn])]
    fn button_isr(cx: button_isr::Context) {
        // Clear pending interrupt
        cx.resources.but.clear_interrupt_pending_bit();

        // Handle button presses
        *cx.resources.pbn += 1;
        if *cx.resources.pbn == 1 {
            handle_button1(cx.resources.fb1, cx.resources.nvm);
        } else if *cx.resources.pbn == 2 {
            handle_button2(cx.resources.fb2, cx.resources.nvm, cx.resources.dly);
        }
    }
};

/// Handle the first push button press.
fn handle_button1(fb: &mut FrameBuffer, nvm: &mut NvmDriver) {
    // Stop capturing video
    ov9655::stop();

    // Save buffered video to non-volatile memory
    rprintln!("Saving frames to non-volatile memory!");
    for address in fb {
        nvm.write(address, FRAME_SIZE as usize).unwrap();
    }

    rprintln!("Video saved! Press button to replay saved video.");
}

/// Handle the second push button press.
fn handle_button2(fb: &mut FrameBuffer, nvm: &mut NvmDriver, dly: &mut Delay) {
    // Read buffered video from non-volatile memory into a new frame buffer
    rprintln!("Reading frames from non-volatile memory!");
    let num_frames = nvm.get_write_ptr() / FRAME_SIZE;
    for _ in 0..num_frames {
        let address = fb.update(false);
        nvm.read(address, FRAME_SIZE as usize).unwrap();
    }

    rprintln!("Playing back images in frame buffer!");
    loop {
        // Clone since we do this on a loop, exhausting the iterator each time
        let curr_fb = fb.clone();

        // Iterate on the frame buffer
        for address in curr_fb {
            // Draw image on display using DMA2D
            match display::draw_image(address, FRAME_WIDTH, FRAME_HEIGHT) {
                true => rprintln!("Error: Cannot display image. Frame rate too fast!"),
                false => (),
            };

            // Block to simulate captured frame rate
            dly.delay_ms(FRAME_RATE);
        }
    }
}
