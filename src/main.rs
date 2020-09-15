#![no_main]
#![no_std]

use dashcam_rs::ov9655::parallel::*;
use dashcam_rs::ov9655::sccb::SCCB;
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
            0: { // channel number
                size: 4096 // buffer size in bytes
                mode: BlockIfFull // mode (optional, default: NoBlockSkip, see enum ChannelMode)
                name: "Terminal" // name (optional, default: no name)
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
    sccb.qvga_setup(&mut i2c).unwrap();
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
