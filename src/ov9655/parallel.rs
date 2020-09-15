//! A driver for the parallel data bus on the OV9655 using the STM32F7 DCMI peripheral and DMA2
//! to transfer image sensor data into on-chip memory. Assumes that GPIO and RCC are setup prior
//! to using this module.

use cortex_m::peripheral::NVIC;
use stm32f7xx_hal::device::{interrupt, DCMI, DMA2};

const DMA_CHANNEL: usize = 1;

pub fn dcmi_setup() {
    // TODO: No HAL driver exists for DCMI
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Set both SYNC signals to be active high and use snapshot mode. Default fields not set:
    //     - Hardware sync (ESS = 0)
    //     - 8-bit data mode (EDM = 00)
    //     - PCLK polarity falling (PCKPOL = 0)
    //     - Capture all frames (FCRC = 0)
    dcmi_regs
        .cr
        .write(|w| w.vspol().set_bit().hspol().set_bit().cm().set_bit());

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

pub fn dcmi_capture() {
    // TODO: No HAL driver exists for DCMI
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Enable the DCMI peripheral and start capture
    dcmi_regs
        .cr
        .modify(|_, w| w.enable().set_bit().capture().set_bit());
}

pub fn dma2_setup(dma_size: u16, dest_addr: u32) {
    // TODO: No HAL driver exists for DMA with DCMI
    let dma2_regs = unsafe { &(*DMA2::ptr()) };

    unsafe {
        // Configure DMA
        dma2_regs.st[DMA_CHANNEL].cr.write(|w| {
            w
                // Enable DME interrupt
                .dmeie()
                .set_bit()
                // Enable TCIE interrupt
                .teie()
                .set_bit()
                // Disable HTIE interrupt
                .htie()
                .clear_bit()
                // Enable TCIC interrupt
                .tcie()
                .set_bit()
                // Peripheral is flow controller
                .pfctrl()
                .set_bit()
                // Direction: Peripheral to memory
                .dir()
                .bits(0)
                // Disable circular mode
                .circ()
                .clear_bit()
                // Don't increment peripheral address
                .pinc()
                .clear_bit()
                // Increment the memory address
                .minc()
                .set_bit()
                // Transfer a word at a time from the peripheral
                .psize()
                .bits(0)
                // Place into memory in half-word alignment (RGB565)
                .msize()
                .bits(1)
                // Priority level is high
                .pl()
                .bits(0x3)
                // No double buffer mode for now (change for ping-pong)
                .dbm()
                .clear_bit()
                // No peripheral burst, single word
                .pburst()
                .bits(0)
                // No memory burst, single word
                .mburst()
                .bits(0)
                // Channel = 1
                .chsel()
                .bits(1)
        });

        // Configure FIFO
        dma2_regs.st[DMA_CHANNEL].fcr.write(|w| {
            w
                // Set FIFO threshold to full
                .fth()
                .bits(0x3)
                // Enable FIFO mode (disable direct mode)
                .dmdis()
                .set_bit()
                // Enable FEIE interrupt
                .feie()
                .set_bit()
        });

        // Enable DMA2 interrupts
        NVIC::unmask::<interrupt>(interrupt::DMA2_STREAM1);
    }

    // Configure addresses and size
    let dcmi_dr_addr: u32 = 0x5005_0000 + 0x28;
    dma2_regs.st[DMA_CHANNEL]
        .ndtr
        .write(|w| w.ndt().bits(dma_size));
    dma2_regs.st[DMA_CHANNEL]
        .par
        .write(|w| w.pa().bits(dcmi_dr_addr));
    dma2_regs.st[DMA_CHANNEL]
        .m0ar
        .write(|w| w.m0a().bits(dest_addr));

    // Enable DMA2
    dma2_regs.st[DMA_CHANNEL].cr.modify(|_, w| w.en().set_bit());
}
