//! I2C1 SCL:   PB8  --> Nucleo CN7.2   (D15)  --> OV9655 SIOC
//! I2C1 SDA:   PB9 <--> Nucleo CN7.4   (D14) <--> OV9655 SIOD
//! MCO2:       PC9  --> Nucleo CN8.4   (D44)  --> OV9655 XCLK
//! DCMI PCLK:  PA6  <-- Nucleo CN7.12  (D12) <--  OV9655 PCLK
//! DCMI HSYNC: PA4  <-- Nucleo CN7.17  (D24) <--  OV9655 HREF
//! DCMI VSYNC: PG9  <-- Nucleo CN11.63       <--  OV9655 VSYNC
//! DCMI D0:    PC6  <-- Nucleo CN7.1   (D16) <-- OV9655 D2
//! DCMI D1:    PC7  <-- Nucleo CN7.11  (D21) <-- OV9655 D3
//! DCMI D2:    PC8  <-- Nucleo CN8.2   (D43) <-- OV9655 D4
//! DCMI D3:    PE1  <-- Nucleo CN11.61       <-- OV9655 D5
//! DCMI D4:    PE4  <-- Nucleo CN9.16  (D57) <-- OV9655 D6
//! DCMI D5:    PB6  <-- Nucleo CN10.13 (D26) <-- OV9655 D7
//! DCMI D6:    PE5  <-- Nucleo CN9.18  (D58) <-- OV9655 D8
//! DCMI D7:    PE6  <-- Nucleo CN9.20  (D59) <-- OV9655 D9

#![no_main]
#![no_std]

use dashcam_rs::ov9655::sccb::SCCB;

use core::panic::PanicInfo;
use core::cell::Cell;
use cortex_m::interrupt::{free, Mutex};
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    gpio::Speed,
    i2c::{BlockingI2c, Mode},
    pac::{self, DCMI, DMA2, RCC},
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
    interrupt
};

// Shared memory between main thread and interrupts
static DCMI_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static DMA2_INT_STATUS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

#[entry]
fn main() -> ! {
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

    let pac_periph = pac::Peripherals::take().unwrap();
    let cm_periph = cortex_m::Peripherals::take().unwrap();

    // GPIOs used
    let gpioa = pac_periph.GPIOA.split();
    let gpiob = pac_periph.GPIOB.split();
    let gpioc = pac_periph.GPIOC.split();
    let gpioe = pac_periph.GPIOE.split();
    let gpiog = pac_periph.GPIOG.split();

    // Nucleo board: HSE = 8 MHz, use as SYSCLK source
    let mut rcc = pac_periph.RCC.constrain();
    let hse_cfg = HSEClock::new(8.mhz(), HSEClockMode::Oscillator);
    let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();

    // Configure microcontroller clock output (MCO) 2 on pin PC9 for OV9655 XCLK. OV9655
    // requires that 10 MHz <= XCLK <= 48 MHz. Can't use HSE (too slow) so use SYSCLK which
    // will be 216 MHz, but prescale it by 5 to slow it down to 43.2 MHz.
    let rcc_regs = unsafe { &(*RCC::ptr()) };
    rcc_regs.cfgr.modify(|_, w| w.mco2().sysclk());
    rcc_regs.cfgr.modify(|_, w| w.mco2pre().div5());
    gpioc.pc9.into_alternate_af0().set_speed(Speed::VeryHigh);

    // Delay
    let mut delay = Delay::new(cm_periph.SYST, clocks);

    // Configure I2C1 for Serial Camera Control Bus
    let scl = gpiob
        .pb8
        .into_alternate_af4()
        .internal_pull_up(true)
        .set_open_drain();
    let sda = gpiob
        .pb9
        .into_alternate_af4()
        .internal_pull_up(true)
        .set_open_drain();

    let mut i2c = BlockingI2c::i2c1(
        pac_periph.I2C1,
        (scl, sda),
        Mode::standard(100.khz()),
        clocks,
        &mut rcc.apb1,
        10000,
    );

    // Establish communication with the OV9655 using the SCCB
    let i2c_ref = &mut i2c;
    let sccb = SCCB::new(i2c_ref);
    sccb.reset(i2c_ref).unwrap();
    delay.delay_ms(1000_u16);
    sccb.check_id(i2c_ref).unwrap();
    rprintln!("SCCB initialization complete!");

    // QVGA size setup
    sccb.qvga_setup(i2c_ref).unwrap();
    rprintln!("QVGA setup complete!");

    // Enable AHB2 periph clock for DCMI
    rcc_regs.ahb2enr.modify(|_, w| w.dcmien().set_bit());

    // Configure DCMI GPIO pins
    let _dcmi_pclk = gpioa
        .pa6
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_hsync = gpioa
        .pa4
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_vsync = gpiog
        .pg9
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d0 = gpioc
        .pc6
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d1 = gpioc
        .pc7
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d2 = gpioc
        .pc8
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d3 = gpioe
        .pe1
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d4 = gpioe
        .pe4
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d5 = gpiob
        .pb6
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d6 = gpioe
        .pe5
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d7 = gpioe
        .pe6
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    // No HAL driver exists for DCMI
    let dcmi_regs = unsafe { &(*DCMI::ptr()) };

    // Set both SYNC signals to be active high and use snapshot mode for now
    // By default: We use hardware sync (ESS = 0), 8-bit data mode (EDM = 00), PCLK polarity
    //             falling (PCKPOL = 0), capture all frames (FCRC = 0)
    dcmi_regs.cr.write(|w| {
        // Active high VSYNC
        w.vspol()
            .set_bit()
            // Active high HSYNC
            .hspol()
            .set_bit()
            // Snapshot
            .cm()
            .set_bit()
    });

    // Enable the VSYNC interrupt
    dcmi_regs.ier.write(|w| w.vsync_ie().set_bit());

    // Enable DCMI interrupt
    unsafe {
        NVIC::unmask::<interrupt>(interrupt::DCMI);
    }

    // Enable DMA clocks
    rcc_regs.ahb1enr.modify(|_, w| w.dma2en().set_bit());

    let dma_size: u16 = 320;
    let dcmi_addr: u32 = 0x5005_0000 + 0x28;
    let mem_addr: u32 = 0x2007_C000; // SRAM2: 16 KB
    unsafe {
        let dma2_regs = &(*DMA2::ptr());

        // Configure DMA
        dma2_regs.st[1].cr.write(|w| {
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
                // PINCOS has no meaning since PINC is 0
                // .pincos()
                // .clear_bit()
                // Priority level is high
                .pl()
                .bits(0x3)
                // No double buffer mode for now (change for ping-pong)
                .dbm()
                .clear_bit()
                // CT has no meaning when DBM = 0
                // .ct()
                // .clear_bit()
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

        dma2_regs.st[1].fcr.write(|w| {
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

        dma2_regs.st[1].ndtr.write(|w| w.ndt().bits(dma_size));
        dma2_regs.st[1].par.write(|w| w.pa().bits(dcmi_addr));
        dma2_regs.st[1].m0ar.write(|w| w.m0a().bits(mem_addr));

        // Enable DMA2 interrupts
        NVIC::unmask::<interrupt>(interrupt::DMA2_STREAM1);

        // Enable DMA
        dma2_regs.st[1].cr.modify(|_, w| w.en().set_bit());
    }

    // Start capture!
    dcmi_regs
        .cr
        .modify(|_, w| w.enable().set_bit().capture().set_bit());

    loop {
        // Wait for the interrupt to fire
        free(|cs| {
            let dcmi_int_status = DCMI_INT_STATUS.borrow(cs).get();
            let dma2_int_status = DMA2_INT_STATUS.borrow(cs).get();
            if dcmi_int_status != 0 || dma2_int_status != 0 {
                let buffer_pointer = mem_addr as *const _;
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
