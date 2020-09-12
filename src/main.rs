//! I2C1 SCL: PB8  --> Nucleo CN7.2 (D15)  --> OV9655 SIOC
//! I2C1 SDA: PB9 <--> Nucleo CN7.4 (D14) <--> OV9655 SIOD
//! MCO2:     PC9  --> Nucleo CN8.4 (D44)  --> OV9655 XCLK

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
    pac::{self, RCC},
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

    // Nucleo board:
    //     HSE = 8 MHz, use as SYSCLK source.
    //
    //     Configure microcontroller clock output (MCO) 2 on pin PC9 for OV9655 XCLK. OV9655
    //     requires that 10 MHz <= XCLK <= 48 MHz. Can't use HSE (too slow) so use SYSCLK which
    //     will be 216 MHz, but prescale it by 5 to slow it down to 43.2 MHz.
    let rcc_regs = unsafe { &(*RCC::ptr()) };
    rcc_regs.cfgr.modify(|_, w| w.mco2().sysclk());
    rcc_regs.cfgr.modify(|_, w| w.mco2pre().div5());

    let mut rcc = pac_periph.RCC.constrain();
    let hse_cfg = HSEClock::new(8.mhz(), HSEClockMode::Oscillator);
    let clocks = rcc.cfgr.hse(hse_cfg).sysclk(216.mhz()).freeze();

    let gpioc = pac_periph.GPIOC.split();
    gpioc.pc9.into_alternate_af0().set_speed(Speed::VeryHigh);

    // Delay
    let mut delay = Delay::new(cm_periph.SYST, clocks);

    // Configure I2C1 for Serial Camera Control Bus
    let gpiob = pac_periph.GPIOB.split();

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
    let sccb = SCCB::new(&mut i2c);
    sccb.reset(&mut i2c).unwrap();
    sccb.check_id(&mut i2c).unwrap();

    rprintln!("SCCB initialization complete!");

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
