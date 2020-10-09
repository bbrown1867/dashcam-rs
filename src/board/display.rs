//! Display driver for the LCD screen located on the STM32F746G Discovery Board. Majority of this
//! code was adapted from the `screen` example in the `stm32f7xx-hal` crate, except for
//! `draw_image` which was written from scratch. The screen is for debug purposes only at the
//! moment, the final dashcam would not have a screen.

use embedded_graphics::{
    egrectangle, egtext,
    fonts::Font6x8,
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
    primitive_style, text_style,
};
use stm32f7xx_hal::{
    gpio::Speed,
    ltdc::{Layer, PixelFormat},
    pac,
    prelude::*,
};

/// Number of horizontal pixels on the display.
const DISP_WIDTH: u16 = 480;

/// Number of vertical pixels on the display.
const DISP_HEIGHT: u16 = 272;

/// Number of total pixels on the display.
const DISP_SIZE: usize = (DISP_WIDTH as usize) * (DISP_HEIGHT as usize);

/// SRAM buffer to store display pixel data.
static mut DISP_BUFFER: [u16; DISP_SIZE] = [0; DISP_SIZE];

/// Configure the STM32F746G Discovery Board LCD screen.
/// * Peripherals are stolen, so this should only be done during init!
pub fn config() -> screen::DiscoDisplay<u16> {
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
        .config_layer(Layer::L1, unsafe { &mut DISP_BUFFER }, PixelFormat::RGB565);
    display.controller.enable_layer(Layer::L1);
    display.controller.reload();

    // Enable LCD
    lcd_enable.set_high().ok();

    display
}

/// Color the screen blue and display the welcome message.
pub fn draw_welcome(display: &mut screen::DiscoDisplay<u16>) {
    egrectangle!(
        top_left = (0, 0),
        bottom_right = (479, 271),
        style = primitive_style!(fill_color = Rgb565::BLUE)
    )
    .draw(display)
    .ok();

    egtext!(
        text = "Hello Dashcam!",
        top_left = (100, 100),
        style = text_style!(font = Font6x8, text_color = RgbColor::WHITE)
    )
    .draw(display)
    .ok();
}

/// Draw an image located at `address` on the display using DMA2D. Returns `false` on success and
/// `true` when a DMA2D transfer was already in progress.
pub fn draw_image(address: u32, pix_per_line: u16, num_lines: u16) -> bool {
    assert!(pix_per_line < DISP_WIDTH && num_lines < DISP_HEIGHT);

    unsafe {
        let dma2d_regs = &(*pac::DMA2D::ptr());

        // Test if a transfer is currently in progress
        let is_started = dma2d_regs.cr.read().start().is_start();
        if !is_started {
            // DMA2D_FGMAR = Address of source image
            dma2d_regs.fgmar.write(|w| w.ma().bits(address));

            // DMA2_OMAR = Address of display buffer
            dma2d_regs
                .omar
                .write(|w| w.ma().bits(&DISP_BUFFER as *const _ as u32));

            // DMA2D_NLR = Number of lines in source image and pixels per line in source image
            dma2d_regs
                .nlr
                .write(|w| w.pl().bits(pix_per_line).nl().bits(num_lines));

            // DMA2D_FGOR = Line size for the source image (pixels per line)
            dma2d_regs.fgor.write(|w| w.lo().bits(0));

            // DMA2D_OOR = Line size for the display
            dma2d_regs
                .oor
                .write(|w| w.lo().bits(DISP_WIDTH - pix_per_line));

            // DMA2D_FGPFCCR = RGB565
            dma2d_regs.fgpfccr.write_with_zero(|w| w.cm().rgb565());

            // DMA2D_OPFCCR = RGB565
            dma2d_regs.opfccr.write(|w| w.cm().rgb565());

            // DMA2D_CR = Start transfer!
            dma2d_regs.cr.write_with_zero(|w| w.start().set_bit());
        }

        is_started
    }
}

/// Implementation of the DisplayController traits needed in order to use the embedded-graphics
/// crate with the STM32F746G Discovery Board LCD screen.
mod screen {
    use embedded_graphics::{
        drawable::Pixel,
        geometry::Size,
        pixelcolor::{
            raw::{RawData, RawU16},
            Rgb565,
        },
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
        active_width: super::DISP_WIDTH,
        active_height: super::DISP_HEIGHT,
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
                    crate::board::get_xtal(),
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
            self.controller.draw_pixel(
                Layer::L1,
                coord.x as usize,
                coord.y as usize,
                RawU16::from(color).into_inner(),
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
                    Some(c) => RawU16::from(c).into_inner() as u32,
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
