//! SDRAM driver for the IS42S32400F-6BL located on the STM32F746G Discovery Board. Adapted from
//! the example code in the `stm32-fmc` crate. Note that the board has 16/32 data lines connected,
//! so the is42s16400j_7 is used since the parameters are pretty close to the same and the
//! IS42S32400F-6BL is not in `stm32-fmc` at the moment.

use stm32_fmc::devices::is42s16400j_7;
use stm32f7xx_hal::{
    delay::Delay,
    fmc::FmcExt,
    gpio::{GpioExt, Speed},
    pac,
    rcc::Clocks,
};

/// Helper macro for SDRAM pins.
macro_rules! fmc_pins {
    ($($pin:expr),*) => {
        (
            $(
                $pin.into_push_pull_output()
                    .set_speed(Speed::VeryHigh)
                    .into_alternate_af12()
                    .internal_pull_up(true)
            ),*
        )
    };
}

/// Configure STM32F746G Discovery Board SDRAM. The FMC driver is used from the HAL, which is used
/// in conjunction with the [stm32-rs/stm32-fmc](https://github.com/stm32-rs/stm32-fmc/) crate.
/// * This board only has 16 out of 32 data lines wired to the IS42S32400F.
/// * The function returns a raw pointer to the SDRAM address space and size in bytes.
/// * Peripherals are stolen, so this should only be done during init!
pub fn init(clocks: &Clocks, delay: &mut Delay) -> (*mut u32, usize) {
    let pac_periph = unsafe { pac::Peripherals::steal() };

    let gpioc = pac_periph.GPIOC.split();
    let gpiod = pac_periph.GPIOD.split();
    let gpioe = pac_periph.GPIOE.split();
    let gpiof = pac_periph.GPIOF.split();
    let gpiog = pac_periph.GPIOG.split();
    let gpioh = pac_periph.GPIOH.split();

    let fmc_io = fmc_pins! {
        gpiof.pf0,  // A0
        gpiof.pf1,  // A1
        gpiof.pf2,  // A2
        gpiof.pf3,  // A3
        gpiof.pf4,  // A4
        gpiof.pf5,  // A5
        gpiof.pf12, // A6
        gpiof.pf13, // A7
        gpiof.pf14, // A8
        gpiof.pf15, // A9
        gpiog.pg0,  // A10
        gpiog.pg1,  // A11
        gpiog.pg4,  // BA0
        gpiog.pg5,  // BA1
        gpiod.pd14, // D0
        gpiod.pd15, // D1
        gpiod.pd0,  // D2
        gpiod.pd1,  // D3
        gpioe.pe7,  // D4
        gpioe.pe8,  // D5
        gpioe.pe9,  // D6
        gpioe.pe10, // D7
        gpioe.pe11, // D8
        gpioe.pe12, // D9
        gpioe.pe13, // D10
        gpioe.pe14, // D11
        gpioe.pe15, // D12
        gpiod.pd8,  // D13
        gpiod.pd9,  // D14
        gpiod.pd10, // D15
        gpioe.pe0,  // NBL0
        gpioe.pe1,  // NBL1
        gpioc.pc3,  // SDCKEn
        gpiog.pg8,  // SDCLK
        gpiog.pg15, // SDNCAS
        gpioh.ph3,  // SDNEn
        gpiof.pf11, // SDNRAS
        gpioh.ph5   // SDNWE
    };

    // Create SDRAM object using IS42S32400F implementation
    let mut sdram = pac_periph
        .FMC
        .sdram(fmc_io, is42s16400j_7::Is42s16400j {}, clocks);

    // Initialize and return raw pointer and size in bytes
    let ram_ptr: *mut u32 = sdram.init(delay);
    let ram_size: usize = (16 * 1024 * 1024) / 2;
    (ram_ptr, ram_size)
}
