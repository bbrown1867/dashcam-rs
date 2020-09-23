//! Prototype dashboard camera.

#![no_main]
#![no_std]

pub mod board;
pub mod ov9655;

use ov9655::parallel::*;
use ov9655::sccb::{RegMap, Register, SCCB};
use board::stm32f746_disco::{configure_pins, get_hse_freq};

use core::cell::Cell;
use core::convert::TryInto;
use core::panic::PanicInfo;
use cortex_m::interrupt::{free, Mutex};
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    pac::{self, DCMI, DMA2},
    i2c::{BlockingI2c, Mode},
    interrupt,
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

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
    let hse_cfg = HSEClock::new(get_hse_freq(), HSEClockMode::Oscillator);
    let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();

    // Delay configuration
    let mut delay = Delay::new(cm_periph.SYST, clocks);

    // GPIO configuration
    let i2c_pins = configure_pins();

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
    qvga_setup(&mut reg_vals);
    sccb.apply_config(&mut i2c, &reg_vals).unwrap();
    rprintln!("QVGA mode setup for the OV9655 complete!");

    // DMA transfer description: QVGA resolution (320x240) + RGB565 format (2 bytes each pixel)
    let dma_size_bytes: u32 = 320 * 240 * 2;
    let dma_size_words: u32 = dma_size_bytes / 4;
    let mem_addr_sram: u32 = 0x2001_0000;

    // Setup the DCMI peripheral to interface with the OV9655
    dcmi_setup();

    // Debug code, will remove later
    memory_set(mem_addr_sram, dma_size_bytes, 0xAA);
    memory_get(mem_addr_sram, 4);

    // Setup DMA2 to transfer one image worth of words into memory
    dma2_setup(mem_addr_sram, dma_size_words.try_into().unwrap());

    // Start capture!
    dcmi_capture();

    // Capture a single image
    rprintln!("DCMI and DMA setup complete!");
    rprintln!("Starting image capture...");
    let mut cap_done = false;
    while !cap_done {
        // Wait for the interrupt to fire
        free(|cs| {
            let dcmi_int_status = DCMI_INT_STATUS.borrow(cs).get();
            let dma2_int_status = DMA2_INT_STATUS.borrow(cs).get();
            if dcmi_int_status != 0 || dma2_int_status != 0 {
                rprintln!("DCMI Int = {:X}", dcmi_int_status);
                rprintln!("DMA2 Int = {:X}", dma2_int_status);

                // Debug code, will remove later
                let dcmi_regs = unsafe { &(*DCMI::ptr()) };
                rprintln!("DCMI CR = {:X}", dcmi_regs.cr.read().bits());
                rprintln!("DCMI SR = {:X}", dcmi_regs.sr.read().bits());

                // Stop after we capture a single frame (for now)
                if dcmi_int_status & 0x1 == 0x1 {
                    rprintln!("Capture complete!");
                    cap_done = true;
                }

                DCMI_INT_STATUS.borrow(cs).set(0);
                DMA2_INT_STATUS.borrow(cs).set(0);
            }
        });
    }

    // Debug code, will remove later
    memory_get(mem_addr_sram, 4);
    memory_get(mem_addr_sram + (dma_size_bytes / 2), 4);

    loop {
        delay.delay_ms(500_u16);
    }
}

fn memory_set(addr: u32, size: u32, val: u8) {
    for i in 0..size {
        unsafe {
            let curr: *mut u8 = (addr + i) as *mut u8;
            core::ptr::write_volatile(curr, val);
        }
    }
}

fn memory_get(addr: u32, size: u32) {
    rprintln!("{} bytes located at address {:X}:", size, addr);

    for i in 0..size {
        unsafe {
            let curr: *mut u8 = (addr + i) as *mut u8;
            let val: u8 = core::ptr::read_volatile(curr);
            rprintln!("\t{:X}", val);
        }
    }
}

fn qvga_setup(reg_vals: &mut RegMap) {
    // 15 fps VGA with RGB output data format
    reg_vals.insert(Register::COM_CNTRL_07, 0x03).unwrap();

    // Pin configuration:
    // --> Bit 6: Set this bit to change HREF pin to be HSYNC signals
    // --> Bit 4: PCLK reverse - PCLK is falling edge in datasheet, so reverse is rising
    // --> Bit 3: HREF reverse - HREF active high in datasheet, so reverse is active low
    // --> Bit 1: VSYNC negative - VSYNC active low in datasheet, so reverse is active high
    // --> Bit 0: HSYNC negative - HSYNC polarity unclear in datasheet, ignore and use HREF
    reg_vals.insert(Register::COM_CNTRL_10, 0x00).unwrap();

    // RGB 565 data format with full output range (0x00 --> 0xFF)
    reg_vals.insert(Register::COM_CNTRL_15, 0x10).unwrap();

    // Scale down ON
    reg_vals.insert(Register::COM_CNTRL_16, 0x01).unwrap();

    // Reduce resolution by half both vertically and horizontally (640x480 --> 320x240)
    reg_vals.insert(Register::PIX_OUT_INDX, 0x11).unwrap();

    // Pixel clock output frequency adjustment (note: default value is 0x01)
    reg_vals.insert(Register::PIX_CLK_DIVD, 0x01).unwrap();

    // Horizontal and vertical scaling - TODO: Unsure how this works
    reg_vals.insert(Register::PIX_HOR_SCAL, 0x10).unwrap();
    reg_vals.insert(Register::PIX_VER_SCAL, 0x10).unwrap();

    // TODO: Are registers below necessary?

    // Set the output drive capability to 4x
    reg_vals.insert(Register::COM_CNTRL_01, 0x03).unwrap();

    // Set the exposure step bit high
    reg_vals.insert(Register::COM_CNTRL_05, 0x01).unwrap();

    // Enable HREF at optical black, use optical black as BLC signal
    reg_vals.insert(Register::COM_CNTRL_06, 0xc0).unwrap();

    // Enable auto white balance, gain control, exposure control, etc.
    reg_vals.insert(Register::COM_CNTRL_08, 0xef).unwrap();

    // More gain and exposure settings
    reg_vals.insert(Register::COM_CNTRL_09, 0x3a).unwrap();

    // No mirror and no vertical flip
    reg_vals.insert(Register::MIRROR_VFLIP, 0x00).unwrap();

    // Zoom function ON, black/white correction off
    reg_vals.insert(Register::COM_CNTRL_14, 0x02).unwrap();

    // Enables auto adjusting for de-noise and edge enhancement
    reg_vals.insert(Register::COM_CNTRL_17, 0xc0).unwrap();
}

#[interrupt]
fn DCMI() {
    free(|cs| {
        // If main thread is not processing a previous interrupt
        if DCMI_INT_STATUS.borrow(cs).get() == 0 {
            // Read interrupt status
            let dcmi_regs = unsafe { &(*DCMI::ptr()) };
            let int_status = dcmi_regs.ris.read().bits();

            // If an interrupt fired
            if int_status != 0 {
                // Signal interrupt status to main thread
                DCMI_INT_STATUS.borrow(cs).set(int_status);

                // Clear the pending interrupt
                unsafe {
                    dcmi_regs.icr.write(|w| w.bits(int_status));
                }
            }
        }
    });
}

#[interrupt]
fn DMA2_STREAM1() {
    free(|cs| {
        // If main thread is not processing a previous interrupt
        if DMA2_INT_STATUS.borrow(cs).get() == 0 {
            // Read interrupt status
            let dma2_regs = unsafe { &(*DMA2::ptr()) };
            let int_status = dma2_regs.lisr.read().bits();

            // If an interrupt fired
            if int_status != 0 {
                // Signal interrupt status to main thread
                DMA2_INT_STATUS.borrow(cs).set(int_status);

                // Clear the pending interrupt
                unsafe {
                    dma2_regs.lifcr.write(|w| w.bits(int_status));
                }
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
