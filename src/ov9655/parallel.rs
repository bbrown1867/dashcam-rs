//! A driver for the parallel data bus on the OV9655 using the STM32F7 DCMI peripheral and DMA2
//! to transfer image sensor data into memory. Enables DCMI and DMA2 clocks in RCC. Does not do
//! any GPIO configuration.

use stm32f7xx_hal::pac::{DCMI, DMA2, RCC};

// DMA2-Stream 1-Channel 1 is used to interface with DCMI
const DMA_STREAM: usize = 1;
const DMA_CHANNEL: u8 = 1;

// DCMI data register address
const DCMI_DR_ADDR: u32 = 0x5005_0000 + 0x28;

/// Setup the DCMI peripheral to interface with the OV9655.
pub fn dcmi_setup() {
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };
    let rcc_regs = unsafe { &(*RCC::ptr()) };

    // Enable peripheral clock
    rcc_regs.ahb2enr.modify(|_, w| w.dcmien().set_bit());

    // Set up SYNC signal polarity and capture mode
    dcmi_regs
        .cr
        .write(|w| w.vspol().set_bit().hspol().clear_bit().cm().clear_bit());

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
}

/// Setup DMA2 to transfer image data from DCMI to memory. Does not update
/// the address registers, that must be done seperately `update_addr0`
/// with and `update_addr1` functions since address may change during
/// ping-pong DMA.
pub fn dma2_setup(dma_size: u16) {
    let dma2_regs = unsafe { &(*DMA2::ptr()) };
    let rcc_regs = unsafe { &(*RCC::ptr()) };

    // Enable peripheral clock
    rcc_regs.ahb1enr.modify(|_, w| w.dma2en().set_bit());

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
                // Flow controller (0 = DMA, 1 = peripheral)
                .pfctrl()
                .clear_bit()
                // Direction
                .dir()
                .peripheral_to_memory()
                // Circular mode
                .circ()
                .set_bit()
                // Peripheral address increment
                .pinc()
                .clear_bit()
                // Memory address increment
                .minc()
                .set_bit()
                // Peripheral transfer size
                .psize()
                .bits32()
                // Memory transfer size
                .msize()
                .bits32()
                // Priority level
                .pl()
                .high()
                // Double buffer mode
                .dbm()
                .set_bit()
                // Peripheral burst
                .pburst()
                .single()
                // Memory burst
                .mburst()
                .single()
                // Channel
                .chsel()
                .bits(DMA_CHANNEL)
        });
    }

    // Configure addresses and size
    dma2_regs.st[DMA_STREAM]
        .ndtr
        .write(|w| w.ndt().bits(dma_size));
    dma2_regs.st[DMA_STREAM]
        .par
        .write(|w| w.pa().bits(DCMI_DR_ADDR));
}

/// Set DMA2 address 0 register.
pub fn dma2_update_addr0(address: u32) {
    let dma2_regs = unsafe { &(*DMA2::ptr()) };

    dma2_regs.st[DMA_STREAM]
        .m0ar
        .write(|w| w.m0a().bits(address));
}

/// Set DMA2 address 1 register.
pub fn dma2_update_addr1(address: u32) {
    let dma2_regs = unsafe { &(*DMA2::ptr()) };

    dma2_regs.st[DMA_STREAM]
        .m1ar
        .write(|w| w.m1a().bits(address));
}

/// Read and clear low interrupt status register and return `true` if the transfer is complete.
pub fn dma2_isr() -> bool {
    unsafe {
        let dma2_regs = &(*DMA2::ptr());

        // Did the DMA done interrupt fire?
        let dma_done = dma2_regs.lisr.read().tcif1().is_complete();

        // Read and clear interrupt status
        let int_status = dma2_regs.lisr.read().bits();
        dma2_regs.lifcr.write(|w| w.bits(int_status));

        dma_done
    }
}

/// Start DCMI capture. Programs registers for both DMA2 and DCMI peripherals.
pub fn start_capture() {
    let dma2_regs = unsafe { &(*DMA2::ptr()) };
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Enable DMA2
    dma2_regs.st[DMA_STREAM].cr.modify(|_, w| w.en().set_bit());

    // Enable the DCMI peripheral and start capture
    dcmi_regs
        .cr
        .modify(|_, w| w.enable().set_bit().capture().set_bit());
}

/// Stop DCMI capture. Programs registers for both DMA2 and DCMI peripherals.
pub fn stop_capture() {
    let dma2_regs = unsafe { &(*DMA2::ptr()) };
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Disable DMA2
    dma2_regs.st[DMA_STREAM]
        .cr
        .modify(|_, w| w.en().clear_bit());

    // Disable the DCMI peripheral and stop capture
    dcmi_regs
        .cr
        .modify(|_, w| w.enable().clear_bit().capture().clear_bit());
}
