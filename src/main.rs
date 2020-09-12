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
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    gpio::Speed,
    i2c::{BlockingI2c, Mode},
    pac::{self, DCMI, RCC},
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

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

    /*
        DCMI setup steps:
            - DCMI periph:
                - DCMI_CaptureMode_Continuous
                - DCMI_SynchroMode_Hardware
                - DCMI_PCKPolarity_Falling
                - DCMI_VSPolarity_High
                - DCMI_HSPolarity_High
                - DCMI_CaptureRate_All_Frame
                - DCMI_ExtendedDataMode_8b
            - VSYNC interrupt enabled in DCMI periph but none other
            - NVIC enable DCMI interrupts
        - DMA2 setup
            - Enable AHB1 periph clock for DMA2
            - DMA_Channel_1
            - DMA_PeripheralBaseAddr = 0x50050028
            - DMA_Memory0BaseAddr = (Pick a RAM bank)
            - DMA_BufferSize = ??? 320 ???
            - DMA_PeripheralInc_Disable
            - DMA_MemoryInc_Disable
            - DMA_PeripheralDataSize_Word
            - DMA_MemoryDataSize_HalfWord
            - DMA_Mode_Circular ???
            - DMA_Priority_High
            - DMA_FIFOMode_Enable
            - DMA_FIFOThreshold_Full
            - DMA_MemoryBurst_Single
            - DMA_PeripheralBurst_Single

        Then ENABLE DCMI and DMA2, tghen ENABLE DCMI capture command
    */

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

    loop {
        delay.delay_ms(500_u16);
    }
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    rprintln!("{:?}", _info);
    loop {}
}
