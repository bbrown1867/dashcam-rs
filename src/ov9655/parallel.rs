//! A driver for the parallel data bus on the OV9655 using the STM32F7 DCMI peripheral and DMA2
//! to transfer image sensor data into memory. Assumes that GPIO and RCC are setup prior to using
//! this module.

use cortex_m::peripheral::NVIC;
use stm32f7xx_hal::device::{interrupt, DCMI, DMA2};

// DMA2-Stream 1-Channel 1 is used to interface with DCMI
const DMA_STREAM: usize = 1;
const DMA_CHANNEL: u8 = 1;

// DCMI data register address (TODO: Get from PAC)
const DCMI_DR_ADDR: u32 = 0x5005_0000 + 0x28;

/// Setup the DCMI peripheral to interface with the OV9655.
pub fn dcmi_setup() {
    // TODO: No HAL driver exists for DCMI
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Set both SYNC signals to be active high and use snapshot mode. Default fields not set:
    //     - VSYNC active low (0)
    //     - HSYNC (HREF) active high (1)
    //     - Hardware sync (ESS = 0)
    //     - 8-bit data mode (EDM = 00)
    //     - PCLK polarity falling (PCKPOL = 0)
    //     - Capture all frames (FCRC = 0)
    dcmi_regs
        .cr
        .write(|w| w.vspol().clear_bit().hspol().set_bit().cm().set_bit());

    // Enable all of the interrupts
    dcmi_regs.ier.write(|w| {
        w.line_ie()
            .set_bit()
            .vsync_ie()
            .set_bit()
            .err_ie()
            .set_bit()
            .ovr_ie()
            .set_bit()
            .frame_ie()
            .set_bit()
    });

    // Enable DCMI interrupt
    unsafe {
        NVIC::unmask::<interrupt>(interrupt::DCMI);
    }
}

/// Initiate DCMI capture.
pub fn dcmi_capture() {
    // TODO: No HAL driver exists for DCMI
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Enable the DCMI peripheral and start capture
    dcmi_regs
        .cr
        .modify(|_, w| w.enable().set_bit().capture().set_bit());
}

/// Setup DMA2 to transfer image data from DCMI to memory.
pub fn dma2_setup(dest_addr: u32, dma_size: u16) {
    // TODO: No HAL driver exists for DMA with DCMI
    let dma2_regs = unsafe { &(*DMA2::ptr()) };

    unsafe {
        // Clear any stale interrupts
        let dma2_int_status_lo = dma2_regs.lisr.read().bits();
        let dma2_int_status_hi = dma2_regs.hisr.read().bits();
        dma2_regs.lifcr.write(|w| w.bits(dma2_int_status_lo));
        dma2_regs.hifcr.write(|w| w.bits(dma2_int_status_hi));

        // Configure DMA
        dma2_regs.st[DMA_STREAM].cr.write(|w| {
            w
                // DME interrupt
                .dmeie()
                .set_bit()
                // TCIE interrupt
                .teie()
                .set_bit()
                // HTIE interrupt
                .htie()
                .set_bit()
                // TCIE interrupt
                .tcie()
                .set_bit()
                // Flow controller (DMA or peripheral)
                .pfctrl()
                .clear_bit()
                // Direction
                .dir()
                .peripheral_to_memory()
                // Circular mode
                .circ()
                .clear_bit()
                // Peripheral address increment
                .pinc()
                .clear_bit()
                // Memory address increment
                .minc()
                .set_bit()
                // Peripheral transfer size (in words)
                .psize()
                .bits32()
                // Memory transfer size (in words)
                .msize()
                .bits32()
                // Priority level
                .pl()
                .high()
                // Double buffer mode
                .dbm()
                .clear_bit()
                // Peripheral burst
                .pburst()
                .single()
                // Memory burst
                .mburst()
                .incr4()
                // Channel
                .chsel()
                .bits(DMA_CHANNEL)
        });

        // Configure FIFO
        dma2_regs.st[DMA_STREAM].fcr.write(|w| {
            w
                // FIFO threshold
                .fth()
                .full()
                // FIFO mode (not direct mode)
                .dmdis()
                .set_bit()
                // FEIE interrupt
                .feie()
                .set_bit()
        });

        // Enable DMA2 interrupts
        NVIC::unmask::<interrupt>(interrupt::DMA2_STREAM1);
    }

    // Configure addresses and size
    dma2_regs.st[DMA_STREAM]
        .ndtr
        .write(|w| w.ndt().bits(dma_size));
    dma2_regs.st[DMA_STREAM]
        .par
        .write(|w| w.pa().bits(DCMI_DR_ADDR));
    dma2_regs.st[DMA_STREAM]
        .m0ar
        .write(|w| w.m0a().bits(dest_addr));

    // Enable DMA2
    dma2_regs.st[DMA_STREAM].cr.modify(|_, w| w.en().set_bit());
}
