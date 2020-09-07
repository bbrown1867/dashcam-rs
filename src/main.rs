#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
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
        Mode::fast(200.khz()),
        clocks,
        &mut rcc.apb1,
        10000,
    );

    rprintln!("I2C initialization complete!\r\n");

    // Writing 0x80 to register 0x12 resets all registers
    i2c.write(0x60, &[0x12, 0x80]).unwrap();
    rprintln!("Reset OV9655!\r\n");
    delay.delay_ms(500_u16);

    // Read ID
    let buf: &mut [u8] = &mut [0x00, 0x00];

    i2c.write_read(0x60, &[0x1C, 0x00], buf).unwrap();
    rprintln!("Read ID MIDH = {}", buf[1]);
    delay.delay_ms(500_u16);

    i2c.write_read(0x60, &[0x1D, 0x00], buf).unwrap();
    rprintln!("Read ID MIDL = {}", buf[1]);
    delay.delay_ms(500_u16);

    i2c.write_read(0x60, &[0x0B, 0x00], buf).unwrap();
    rprintln!("Read ID Ver = {}", buf[1]);
    delay.delay_ms(500_u16);

    i2c.write_read(0x60, &[0x0B, 0x00], buf).unwrap();
    rprintln!("Read ID Ver = {}", buf[1]);
    delay.delay_ms(500_u16);

    i2c.write_read(0x60, &[0x0A, 0x00], buf).unwrap();
    rprintln!("Read ID PID = {}", buf[1]);
    delay.delay_ms(500_u16);

    loop {
        delay.delay_ms(500_u16);
    }
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    rprintln!("Panicked!\r\n");
    rprintln!("{:?}", _info);
    loop {}
}
