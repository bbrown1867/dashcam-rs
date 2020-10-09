//! Pin configuration for the OV9655.

use stm32f7xx_hal::{
    gpio::{self, Alternate, GpioExt, Speed, AF4},
    pac,
};

/// Configure the STM32F746G Discovery Board pins connected to the OV9655 via the camera
/// connector (P1).
/// * Return the I2C pins since they are needed for the I2C driver.
/// * Peripherals are stolen, so this should only be done during init!
pub fn pin_config_stm32f746g_disco() -> (
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
