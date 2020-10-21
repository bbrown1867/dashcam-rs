//! Support for off-chip, board specific devices.
//! * Note: The OV9655 is not part of this module and has a seperate module.

pub mod display;
pub mod sdram;

use stm32f7xx_hal::{
    gpio::{gpioi, Edge, ExtiPin, Floating, GpioExt, Input},
    pac::{EXTI, GPIOI, RCC, SYSCFG},
    time::{MegaHertz, U32Ext},
};

pub type ButtonPin = gpioi::PI11<Input<Floating>>;

/// 25 MHz external oscillator (X2) is the HSE clock source.
pub fn get_xtal() -> MegaHertz {
    25.mhz()
}

/// Configure push button PI11 as an external interrupt.
pub fn setup_button(rcc: &mut RCC, mut syscfg: SYSCFG, mut exti: EXTI, gpio: GPIOI) -> ButtonPin {
    let gpioi = gpio.split();
    let mut button = gpioi.pi11.into_floating_input();
    button.make_interrupt_source(&mut syscfg, rcc);
    button.trigger_on_edge(&mut exti, Edge::RISING);
    button.enable_interrupt(&mut exti);
    button
}
