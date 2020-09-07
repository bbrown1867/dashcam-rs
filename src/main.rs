#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
use nucleof767zi_rs::{Leds, LED1};
use stm32f7xx_hal::{delay::Delay, device, prelude::*};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let pac_periph = device::Peripherals::take().unwrap();
    let cm_periph = cortex_m::Peripherals::take().unwrap();

    let rcc = pac_periph.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(216.mhz()).freeze();
    let mut delay = Delay::new(cm_periph.SYST, clocks);

    let gpiob = pac_periph.GPIOB.split();
    let mut leds = Leds::new(gpiob);

    let mut counter = 0;
    loop {
        rprintln!("Hello World! ({})\r\n", counter);
        delay.delay_ms(500_u16);
        leds[LED1].toggle();
        counter = counter + 1;
    }
}
