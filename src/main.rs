//! I2C1 SCL: PB8  --> Nucleo CN7.2 (D15)  --> OV9655 SIOC
//! I2C1 SDA: PB9 <--> Nucleo CN7.4 (D14) <--> OV9655 SIOD
//! MCO2:     PC9  --> Nucleo CN8.4 (D44)  --> OV9655 XCLK

#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init, set_print_channel};
use stm32f7xx_hal::{
    delay::Delay,
    gpio::{
        gpiob::{PB8, PB9},
        Alternate, Speed, AF4,
    },
    i2c::{BlockingI2c, Mode},
    pac::{self, I2C1, RCC},
    prelude::*,
    rcc::{HSEClock, HSEClockMode},
};

// Device address for writes is 0x60 and reads is 0x61, however the STM32F7 HAL I2C driver will
// left-shift the provided address by 1. Also reads (0x61) is never used, because even register
// reads require writing the address of the register we wish to read.
const OV9655_SLAVE_ADDRESS: u8 = 0x30;

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

    // Configure I2C4 for Serial Camera Control Bus
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

    rprintln!("Initializing I2C...");

    let mut i2c = BlockingI2c::i2c1(
        pac_periph.I2C1,
        (scl, sda),
        Mode::standard(100.khz()),
        clocks,
        &mut rcc.apb1,
        10000,
    );

    rprintln!("I2C initialization complete!");

    // OV9655: Reset all registers
    sccb_reg_write(&mut i2c, 0x12, 0x80);
    rprintln!("OV9655 reset complete!");
    delay.delay_ms(1000_u16);

    // OV9655: Read ID
    let manf_id_msb: u16 = sccb_reg_read(&mut i2c, 0x1C).into();
    let manf_id_lsb: u16 = sccb_reg_read(&mut i2c, 0x1D).into();
    let manf_id: u16 = (manf_id_msb << 8) | manf_id_lsb;
    rprintln!("OV9655 Manf ID = {:#x}", manf_id);

    let product_id_msb: u16 = sccb_reg_read(&mut i2c, 0x0A).into();
    let product_id_lsb: u16 = sccb_reg_read(&mut i2c, 0x0B).into();
    let product_id: u16 = (product_id_msb << 8) | product_id_lsb;
    rprintln!("OV9655 Product ID = {:#x}", product_id);

    for reg in 0x0..0x0B {
        sccb_reg_read(&mut i2c, reg);
    }

    loop {
        delay.delay_ms(500_u16);
    }
}

fn sccb_reg_read(
    i2c: &mut BlockingI2c<I2C1, PB8<Alternate<AF4>>, PB9<Alternate<AF4>>>,
    reg: u8,
) -> u8 {
    let buf1 = [reg];
    match i2c.write(OV9655_SLAVE_ADDRESS, &buf1) {
        Ok(_) => (),
        Err(e) => rprintln!("SCCB register read failed with error code {:?}.", e),
    };

    let mut buf2 = [0x00];
    match i2c.read(OV9655_SLAVE_ADDRESS, &mut buf2) {
        Ok(_) => rprintln!("SCCB register read {:#x} = {:#x} passed.", reg, buf2[0]),
        Err(e) => rprintln!("SCCB register read failed with error code {:?}.", e)
    }

    buf2[0]
}

fn sccb_reg_write(
    i2c: &mut BlockingI2c<I2C1, PB8<Alternate<AF4>>, PB9<Alternate<AF4>>>,
    reg: u8,
    val: u8,
) {
    let buf = [reg, val];
    match i2c.write(OV9655_SLAVE_ADDRESS, &buf) {
        Ok(_) => rprintln!("SCCB register write {:#x} = {:#x} passed.", reg, val),
        Err(e) => rprintln!("SCCB register write failed with error code {:?}.", e),
    };
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!");
    rprintln!("{:?}", _info);
    loop {}
}
