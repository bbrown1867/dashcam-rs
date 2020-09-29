//! Board specific functions for the STM32F746G Discovery Board.

use crate::FRAME_BUFFER;
use stm32_fmc::devices::is42s32800g_6;
use stm32f7xx_hal::{
    delay::Delay,
    gpio::{self, Alternate, Speed, AF4},
    ltdc::{Layer, PixelFormat},
    pac,
    prelude::*,
    rcc::Clocks,
    time::MegaHertz,
};

/// The 25 MHz external oscillator on the board (X2) is the source for HSE
pub fn board_get_hse() -> MegaHertz {
    25.mhz()
}

/// Configure the STM32F746G Discovery Board pins connected to the OV9655 via the camera
/// connector (P1).
/// * Return the I2C pins since they are needed for the I2C driver.
/// * Peripherals are stolen, so this should only be done during init!
pub fn board_config_ov9655() -> (
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
/// * The SDRAM chip on the board is the IS42S32400F, but the driver does not support this at
///   the time of writing. Instead, the IS42S32800G is used since that is supported. They seem
///   to be mostly the same but the one on this board has half the size (128 MB vs. 256 MB).
/// * This board only has 16/32 data lines wired to the SDRAM part, so only half the available
///   128 MB is available.
/// * The function returns a raw pointer to the SDRAM address space and size in bytes.
/// * Peripherals are stolen, so this should only be done during init!
pub fn board_config_sdram(clocks: &Clocks) -> (*mut u32, usize) {
    let pac_periph = unsafe { pac::Peripherals::steal() };
    let cm_periph = unsafe { cortex_m::Peripherals::steal() };
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

    // Create SDRAM object using IS42S32800g implementation
    let mut sdram = pac_periph
        .FMC
        .sdram(fmc_io, is42s32800g_6::Is42s32800g {}, clocks);

    // Initialize and return raw pointer and size in bytes
    let mut delay = Delay::new(cm_periph.SYST, *clocks);
    let ram_ptr: *mut u32 = sdram.init(&mut delay);
    let ram_size: usize = 0x400_0000;
    (ram_ptr, ram_size)
}

/// Configure the STM32F746G Discovery Board LCD screen.
/// * This is for debug purposes only at the moment, final dashcam would not have a screen.
/// * This code is adapted from the screen example in the `stm32f7xx-hal` crate.
/// * Peripherals are stolen, so this should only be done during init!
pub fn board_config_screen() -> screen::DiscoDisplay<u16> {
    let pac_periph = unsafe { pac::Peripherals::steal() };
    let gpioe = pac_periph.GPIOE.split();
    let gpiog = pac_periph.GPIOG.split();
    let gpioi = pac_periph.GPIOI.split();
    let gpioj = pac_periph.GPIOJ.split();
    let gpiok = pac_periph.GPIOK.split();

    // LCD data and timing signals
    gpioe.pe4.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiog.pg12.into_alternate_af9().set_speed(Speed::VeryHigh);
    gpioi.pi9.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioi.pi10.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioi.pi13.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioi.pi14.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioi.pi15.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj0.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj1.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj2.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj3.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj4.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj5.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj6.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj7.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj8.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj9.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj10.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj11.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj13.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj14.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioj.pj15.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk0.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk1.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk2.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk4.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk5.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk6.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiok.pk7.into_alternate_af14().set_speed(Speed::VeryHigh);

    // LCD control signals
    let mut lcd_enable = gpioi.pi12.into_push_pull_output();
    let mut lcd_backlight = gpiok.pk3.into_push_pull_output();

    // Disable LCD at first to avoid LCD bleed
    lcd_enable.set_low().ok();

    // Enable the backlight
    lcd_backlight.set_high().ok();

    // Init display
    let mut display = screen::DiscoDisplay::new(pac_periph.LTDC, pac_periph.DMA2D);

    // Configure display
    display
        .controller
        .config_layer(Layer::L1, unsafe { &mut FRAME_BUFFER }, PixelFormat::RGB565);
    display.controller.enable_layer(Layer::L1);
    display.controller.reload();

    // Enable LCD */
    lcd_enable.set_high().ok();

    display
}

/// Implementation of the DisplayController traits needed in order to use the embedded-graphics
/// crate with the STM32F746G Discovery Board LCD screen. This module is copied from the screen
/// example in the stm32f7xx-hal crate.
pub mod screen {
    use embedded_graphics::{
        drawable::Pixel,
        geometry::Size,
        pixelcolor::{Rgb565, RgbColor},
        primitives,
        style::{PrimitiveStyle, Styled},
        DrawTarget,
    };

    use stm32f7xx_hal::{
        ltdc::{DisplayConfig, DisplayController, Layer, PixelFormat, SupportedWord},
        pac::{DMA2D, LTDC},
        rcc::{HSEClock, HSEClockMode},
    };

    /// STM32F7-DISCO board display
    pub const DISCO_SCREEN_CONFIG: DisplayConfig = DisplayConfig {
        active_width: 480,
        active_height: 272,
        h_back_porch: 13,
        h_front_porch: 30,
        h_sync: 30,
        v_back_porch: 2,
        v_front_porch: 2,
        v_sync: 10,
        frame_rate: 60,
        h_sync_pol: false,
        v_sync_pol: false,
        no_data_enable_pol: true,
        pixel_clock_pol: false,
    };

    pub struct DiscoDisplay<T: 'static + SupportedWord> {
        pub controller: DisplayController<T>,
    }

    impl<T: 'static + SupportedWord> DiscoDisplay<T> {
        pub fn new(ltdc: LTDC, dma2d: DMA2D) -> DiscoDisplay<T> {
            let controller = DisplayController::new(
                ltdc,
                dma2d,
                PixelFormat::RGB565,
                DISCO_SCREEN_CONFIG,
                Some(&HSEClock::new(
                    super::board_get_hse(),
                    HSEClockMode::Oscillator,
                )),
            );

            DiscoDisplay { controller }
        }

        /// Convert from the PixelColor type into RGB565 format (16-bits per pixel)
        pub fn color2rgb(color: Rgb565) -> u16 {
            ((color.b() as u16 & 0x1F) << 0)
                | ((color.g() as u16 & 0x3F) << 5)
                | ((color.r() as u16 & 0x1F) << 11)
        }
    }

    impl DrawTarget<Rgb565> for DiscoDisplay<u16> {
        type Error = core::convert::Infallible;

        /// Draw a `Pixel` that has a color defined
        fn draw_pixel(&mut self, pixel: Pixel<Rgb565>) -> Result<(), Self::Error> {
            let Pixel(coord, color) = pixel;
            self.controller.draw_pixel(
                Layer::L1,
                coord.x as usize,
                coord.y as usize,
                DiscoDisplay::<u16>::color2rgb(color),
            );
            Ok(())
        }

        /// Draw a rectangle, hardware accelerated by DMA2D
        fn draw_rectangle(
            &mut self,
            item: &Styled<primitives::Rectangle, PrimitiveStyle<Rgb565>>,
        ) -> Result<(), Self::Error> {
            if item.style.stroke_color.is_none() {
                let top_left = (
                    item.primitive.top_left.x as usize,
                    item.primitive.top_left.y as usize,
                );

                let bottom_right = (
                    item.primitive.bottom_right.x as usize,
                    item.primitive.bottom_right.y as usize,
                );

                let color: u32 = match item.style.fill_color {
                    Some(c) => DiscoDisplay::<u16>::color2rgb(c).into(),
                    None => 0u32,
                };

                unsafe {
                    self.controller
                        .draw_rectangle(Layer::L1, top_left, bottom_right, color);
                }
            } else {
                self.draw_iter(item).unwrap();
            }

            Ok(())
        }

        /// Return the size of the screen
        fn size(&self) -> Size {
            Size::new(
                DISCO_SCREEN_CONFIG.active_width.into(),
                DISCO_SCREEN_CONFIG.active_height.into(),
            )
        }
    }
}
