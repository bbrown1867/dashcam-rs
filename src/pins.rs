//! GPIO pin configuration for the board.

use stm32f7xx_hal::{
    device, gpio,
    gpio::{Alternate, Speed, AF4},
    prelude::*,
};

/// Configure GPIOs for alternate functions and return the I2C pins since they are needed for
/// I2C driver. Note that the peripherals are stolen, so this should only be done during init
/// to be safe.
///
/// Pin configuration for the Nucleo-F767ZI:
///
///     I2C1 SCL:   PB8  --> Nucleo CN7.2   (D15)  --> OV9655 SIOC
///     I2C1 SDA:   PB9 <--> Nucleo CN7.4   (D14) <--> OV9655 SIOD
///     MCO2:       PC9  --> Nucleo CN8.4   (D44)  --> OV9655 XCLK
///     DCMI PCLK:  PA6  <-- Nucleo CN7.12  (D12) <--  OV9655 PCLK
///     DCMI HSYNC: PA4  <-- Nucleo CN7.17  (D24) <--  OV9655 HREF
///     DCMI VSYNC: PG9  <-- Nucleo CN11.63       <--  OV9655 VSYNC
///     DCMI D0:    PC6  <-- Nucleo CN7.1   (D16) <--  OV9655 D2
///     DCMI D1:    PC7  <-- Nucleo CN7.11  (D21) <--  OV9655 D3
///     DCMI D2:    PC8  <-- Nucleo CN8.2   (D43) <--  OV9655 D4
///     DCMI D3:    PE1  <-- Nucleo CN11.61       <--  OV9655 D5
///     DCMI D4:    PE4  <-- Nucleo CN9.16  (D57) <--  OV9655 D6
///     DCMI D5:    PB6  <-- Nucleo CN10.13 (D26) <--  OV9655 D7
///     DCMI D6:    PE5  <-- Nucleo CN9.18  (D58) <--  OV9655 D8
///     DCMI D7:    PE6  <-- Nucleo CN9.20  (D59) <--  OV9655 D9
pub fn pin_config_nucleo() -> (
    gpio::gpiob::PB8<Alternate<AF4>>,
    gpio::gpiob::PB9<Alternate<AF4>>,
) {
    let pac_periph = unsafe { device::Peripherals::steal() };
    let gpioa = pac_periph.GPIOA.split();
    let gpiob = pac_periph.GPIOB.split();
    let gpioc = pac_periph.GPIOC.split();
    let gpioe = pac_periph.GPIOE.split();
    let gpiog = pac_periph.GPIOG.split();

    // Configure MCO2 for OV9655 XCLK
    let _xclk = gpioc.pc9.into_alternate_af0().set_speed(Speed::VeryHigh);

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

    (scl, sda)
}
