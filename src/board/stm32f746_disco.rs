//! Board specific functions for the STM32F746G Discovery Board.

use stm32f7xx_hal::{
    pac, gpio,
    gpio::{Alternate, Speed, AF4},
    time::MegaHertz,
    prelude::*,
};

/// Configure GPIOs for alternate functions and return the I2C pins since they are needed for
/// I2C driver. Note that the peripherals are stolen, so this should only be done during init
/// to be safe.
///
/// Pin configuration:
///
///     I2C1 SCL:   PB8  --> OV9655 SIOC
///     I2C1 SDA:   PB9 <--> OV9655 SIOD
///     (HW OSC 24M)     --> OV9655 XCLK
///     DCMI PCLK:  PA6  <-- OV9655 PCLK
///     DCMI HSYNC: PA4  <-- OV9655 HREF
///     DCMI VSYNC: PG9  <-- OV9655 VSYNC
///     DCMI D0:    PH9  <-- OV9655 D2
///     DCMI D1:    PH10 <-- OV9655 D3
///     DCMI D2:    PH11 <-- OV9655 D4
///     DCMI D3:    PH12 <-- OV9655 D5
///     DCMI D4:    PH14 <-- OV9655 D6
///     DCMI D5:    PD3  <-- OV9655 D7
///     DCMI D6:    PE5  <-- OV9655 D8
///     DCMI D7:    PE6  <-- OV9655 D9
pub fn configure_pins() -> (
    gpio::gpiob::PB8<Alternate<AF4>>,
    gpio::gpiob::PB9<Alternate<AF4>>,
) {
    let pac_periph = unsafe { pac::Peripherals::steal() };
    let gpioa = pac_periph.GPIOA.split();
    let gpiob = pac_periph.GPIOB.split();
    let gpiod = pac_periph.GPIOD.split();
    let gpioe = pac_periph.GPIOE.split();
    let gpiog = pac_periph.GPIOG.split();
    let gpioh = pac_periph.GPIOH.split();

    // Configure I2C1 for OV9655 SCCB
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

    // Configure DCMI for OV9655 parallel
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

    let _dcmi_d0 = gpioh
        .ph9
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d1 = gpioh
        .ph10
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d2 = gpioh
        .ph11
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d3 = gpioh
        .ph12
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d4 = gpioh
        .ph14
        .into_alternate_af13()
        .internal_pull_up(true)
        .set_open_drain()
        .set_speed(Speed::VeryHigh);

    let _dcmi_d5 = gpiod
        .pd3
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

    (scl, sda)
}

/// The 25 MHz external oscillator on the board (X2) is the source for HSE
pub fn get_hse_freq() -> MegaHertz {
    25.mhz()
}
