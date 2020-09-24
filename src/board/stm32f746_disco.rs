//! Board specific functions for the STM32F746G Discovery Board.

use stm32f7xx_hal::{
    gpio::{self, Alternate, Speed, AF4},
    ltdc::{Layer, PixelFormat},
    pac,
    prelude::*,
    time::MegaHertz,
};

// TODO: Will need to share this memory with OV9655 frame buffer */
const WIDTH: u16 = 480;
const HEIGHT: u16 = 272;
const FB_GRAPHICS_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize);
static mut FB_LAYER1: [u16; FB_GRAPHICS_SIZE] = [0; FB_GRAPHICS_SIZE];

/// The 25 MHz external oscillator on the board (X2) is the source for HSE
pub fn board_get_hse() -> MegaHertz {
    25.mhz()
}

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

/// Configure the LCD screen (for debug purposes). Note that the peripherals are stolen, so this
/// should only be done during init to be safe.
pub fn board_config_screen() -> screen::DiscoDisplay<u16> {
    let pac_periph = unsafe { pac::Peripherals::steal() };
    let gpioe = pac_periph.GPIOE.split();
    let gpiog = pac_periph.GPIOG.split();
    let gpioi = pac_periph.GPIOI.split();
    let gpioj = pac_periph.GPIOJ.split();
    let gpiok = pac_periph.GPIOK.split();

    // LCD data and timing signals
    gpioe.pe4.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_B0
    gpiog.pg12.into_alternate_af9().set_speed(Speed::VeryHigh); // LTCD_B4
    gpioi.pi9.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_VSYNC
    gpioi.pi10.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_HSYNC
    gpioi.pi13.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_INT
    gpioi.pi14.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_CLK
    gpioi.pi15.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R0
    gpioj.pj0.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R1
    gpioj.pj1.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R2
    gpioj.pj2.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R3
    gpioj.pj3.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R4
    gpioj.pj4.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R5
    gpioj.pj5.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R6
    gpioj.pj6.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_R7
    gpioj.pj7.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G0
    gpioj.pj8.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G1
    gpioj.pj9.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G2
    gpioj.pj10.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G3
    gpioj.pj11.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G4
    gpioj.pj13.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_B1
    gpioj.pj14.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_B2
    gpioj.pj15.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_B3
    gpiok.pk0.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G5
    gpiok.pk1.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G6
    gpiok.pk2.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_G7
    gpiok.pk4.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_B5
    gpiok.pk5.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_B6
    gpiok.pk6.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_D7
    gpiok.pk7.into_alternate_af14().set_speed(Speed::VeryHigh); // LTCD_E

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
        .config_layer(Layer::L1, unsafe { &mut FB_LAYER1 }, PixelFormat::RGB565);
    display.controller.enable_layer(Layer::L1);
    display.controller.reload();

    // Enable LCD! */
    lcd_enable.set_high().ok();

    display
}

/// This module is copied from the screen example in the stm32f7xx-hal crate.
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
    }

    impl DrawTarget<Rgb565> for DiscoDisplay<u16> {
        type Error = core::convert::Infallible;

        /// Draw a `Pixel` that has a color defined
        fn draw_pixel(&mut self, pixel: Pixel<Rgb565>) -> Result<(), Self::Error> {
            let Pixel(coord, color) = pixel;

            // Convert to RGB565
            let value: u16 = (color.b() as u16 & 0x1F)
                | ((color.g() as u16 & 0x3F) << 5)
                | ((color.r() as u16 & 0x1F) << 11);

            self.controller
                .draw_pixel(Layer::L1, coord.x as usize, coord.y as usize, value);
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

                let color = match item.style.fill_color {
                    Some(c) => {
                        (c.b() as u32 & 0x1F)
                            | ((c.g() as u32 & 0x3F) << 5)
                            | ((c.r() as u32 & 0x1F) << 11)
                    }
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
            Size::new(480, 272)
        }
    }
}
