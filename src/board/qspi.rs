//! QSPI driver for the MT25QL128ABA located on the STM32F746G Discovery Board.

use stm32f7xx_hal::{
    gpio::{GpioExt, Speed},
    pac::{GPIOB, GPIOD, GPIOE, RCC},
};

/// Initialize and configure the QSPI flash driver.
pub fn init(rcc: &mut RCC, gpiob: GPIOB, gpiod: GPIOD, gpioe: GPIOE) {
    // Enable peripheral in RCC
    rcc.ahb3enr.modify(|_, w| w.qspien().set_bit());

    // Setup GPIO pins
    let gpiob = gpiob.split();
    let gpiod = gpiod.split();
    let gpioe = gpioe.split();

    let _qspi_d0 = gpiod
        .pd11
        .into_alternate_af9()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    let _qspi_d1 = gpiod
        .pd12
        .into_alternate_af9()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    let _qspi_d2 = gpioe
        .pe2
        .into_alternate_af9()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    let _qspi_d3 = gpiod
        .pd13
        .into_alternate_af9()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    let _qspi_clk = gpiob
        .pb2
        .into_alternate_af9()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    let _qspi_ncs = gpiob
        .pb6
        .into_alternate_af9()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    // Configure QSPI registers
}
