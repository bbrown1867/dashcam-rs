#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
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

#[entry]
fn main() -> ! {
    rtt_init_print!();

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

    rprintln!("Initializing I2C...\r\n");

    let mut i2c = BlockingI2c::i2c1(
        pac_periph.I2C1,
        (scl, sda),
        Mode::standard(200.khz()),
        clocks,
        &mut rcc.apb1,
        10000,
    );

    rprintln!("I2C initialization complete!\r\n");

    // Writing 0x80 to register 0x12 resets all registers
    sccb_reg_write(&mut i2c, 0x12, 0x80);
    rprintln!("Reset OV9655!\r\n");
    delay.delay_ms(500_u16);

    // Read ID
    let val = sccb_reg_read(&mut i2c, 0x1C);
    rprintln!("Read ID MIDH = {:#x}\r\n", val);
    delay.delay_ms(500_u16);

    let val = sccb_reg_read(&mut i2c, 0x1D);
    rprintln!("Read ID MIDL = {:#x}\r\n", val);
    delay.delay_ms(500_u16);

    let val = sccb_reg_read(&mut i2c, 0x0B);
    rprintln!("Read ID Ver = {:#x}\r\n", val);
    delay.delay_ms(500_u16);

    let val = sccb_reg_read(&mut i2c, 0x0A);
    rprintln!("Read ID PID = {:#x}\r\n", val);
    delay.delay_ms(500_u16);

    loop {
        delay.delay_ms(500_u16);
    }
}

fn sccb_reg_read(
    i2c: &mut BlockingI2c<I2C1, PB8<Alternate<AF4>>, PB9<Alternate<AF4>>>,
    reg: u8,
) -> u8 {
    let buf1 = [reg, 0x00];
    let mut buf2 = [0x00, 0x00];
    match i2c.write_read(0x60, &buf1, &mut buf2) {
        Ok(_) => rprintln!("SCCB register read {:#x} = {:#x} passed.\r\n", reg, buf2[1]),
        Err(e) => rprintln!("SCCB register read failed with error code {:?}.\r\n", e),
    };

    buf2[1]
}

fn sccb_reg_write(
    i2c: &mut BlockingI2c<I2C1, PB8<Alternate<AF4>>, PB9<Alternate<AF4>>>,
    reg: u8,
    val: u8,
) {
    let buf = [reg, val];
    match i2c.write(0x60, &buf) {
        Ok(_) => rprintln!("SCCB register write {:#x} = {:#x} passed.\r\n", reg, val),
        Err(e) => rprintln!("SCCB register write failed with error code {:?}.\r\n", e),
    };
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!\r\n");
    rprintln!("{:?}", _info);
    loop {}
}
