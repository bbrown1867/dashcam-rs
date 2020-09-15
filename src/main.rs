//! A prototype dashboard camera.

#![no_main]
#![no_std]

use dashcam_rs::ov9655::parallel::*;
use dashcam_rs::ov9655::sccb::{RegMap, Register, SCCB};
use dashcam_rs::pins::pin_config_nucleo;

use core::cell::Cell;
use core::panic::PanicInfo;
use cortex_m::interrupt::{free, Mutex};
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    device::{self, DCMI, DMA2, RCC},
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
    let pac_periph = device::Peripherals::take().unwrap();
    let cm_periph = cortex_m::Peripherals::take().unwrap();

    /********** BEGIN: CLOCK CONFG **********/

    // Nucleo board: HSE = 8 MHz, use as SYSCLK source
    let mut rcc = pac_periph.RCC.constrain();
    let hse_cfg = HSEClock::new(8.mhz(), HSEClockMode::Oscillator);
    let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();

    // TODO: Clock config for MCO2, DCMI, and DMA2 don't appear to be in HAL
    let rcc_regs = unsafe { &(*RCC::ptr()) };

    // Configure microcontroller clock output (MCO) 2 for OV9655 XCLK. OV9655 requires that it is
    // 10 MHz <= XCLK <= 48 MHz. Can't use HSE (too slow) so use SYSCLK which will be 216 MHz.
    // Prescale it by 5 to slow it down to 43.2 MHz.
    rcc_regs.cfgr.modify(|_, w| w.mco2().sysclk());
    rcc_regs.cfgr.modify(|_, w| w.mco2pre().div5());

    // Enable DCMI and DMA2 clocks
    rcc_regs.ahb2enr.modify(|_, w| w.dcmien().set_bit());
    rcc_regs.ahb1enr.modify(|_, w| w.dma2en().set_bit());

    /********** END: CLOCK CONFG **********/

    // Delay configuration
    let mut delay = Delay::new(cm_periph.SYST, clocks);

    // GPIO configuration
    let i2c_pins = pin_config_nucleo();

    // I2C1 configuration (SCCB)
    let mut i2c = BlockingI2c::i2c1(
        pac_periph.I2C1,
        i2c_pins,
        Mode::standard(100.khz()),
        clocks,
        &mut rcc.apb1,
        10000,
    );

    // Init SCCB module and establish communication with the OV9655
    let sccb = SCCB::new(&mut i2c);
    sccb.reset(&mut i2c).unwrap();
    delay.delay_ms(1000_u16);
    sccb.check_id(&mut i2c).unwrap();
    rprintln!("SCCB initialization complete!");

    // Configure the OV9655 for QVGA (320x240) resolution with RGB565
    let mut reg_vals = RegMap::new();
    qvga_setup(&mut reg_vals);
    sccb.apply_config(&mut i2c, &reg_vals).unwrap();
    rprintln!("QVGA setup complete!");

    // Setup the DCMI peripheral to interface with the OV9655
    dcmi_setup();

    // Setup DMA2 to transfer one row of pixels (16-bit each) into memory
    let dma_size: u16 = 320;
    let mem_addr_sram2: u32 = 0x2007_C000;
    dma2_setup(dma_size, mem_addr_sram2);

    // Start capture!
    dcmi_capture();

    loop {
        // Wait for the interrupt to fire
        free(|cs| {
            let dcmi_int_status = DCMI_INT_STATUS.borrow(cs).get();
            let dma2_int_status = DMA2_INT_STATUS.borrow(cs).get();
            if dcmi_int_status != 0 || dma2_int_status != 0 {
                let buffer_pointer = mem_addr_sram2 as *const _;
                let buffer: [u16; 4] = unsafe { *buffer_pointer };

                rprintln!("DCMI Int = {}", dcmi_int_status);
                rprintln!("DMA2 Int = {}", dma2_int_status);
                rprintln!("Buffer:");
                rprintln!("\t{}", buffer[0]);
                rprintln!("\t{}", buffer[1]);
                rprintln!("\t{}", buffer[2]);
                rprintln!("\t{}", buffer[3]);

                DCMI_INT_STATUS.borrow(cs).set(0);
                DMA2_INT_STATUS.borrow(cs).set(0);
            }
        });
    }
}

fn qvga_setup(reg_vals: &mut RegMap) {
    // 15 fps VGA with RGB output data format
    reg_vals.insert(Register::COM_CNTRL_07, 0x03).unwrap();

    // Pin configuration:
    // --> Bit 6: Set to 1 to change HREF to HSYNC, which STM32 DCMI expects
    // --> Bit 4: PCLK reverse, assuming that means falling edge
    // --> Bit 1: VSYNC negative, unclear what this means - we are using active high
    // --> Bit 0: HSYNC negative, unclear what this means - we are using active high
    reg_vals.insert(Register::COM_CNTRL_10, 0x50).unwrap();

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
            let mut int_status = dma2_regs.lisr.read().bits();

            // Mask away interrupts that aren't channel 1
            int_status &= 0xF40;

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
