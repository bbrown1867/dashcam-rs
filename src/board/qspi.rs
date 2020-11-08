//! QSPI driver for the MT25QL128ABA located on the STM32F746G Discovery Board.

use core::convert::TryInto;
use rtt_target::rprintln;
use stm32f7xx_hal::{
    gpio::{GpioExt, Speed},
    pac::{GPIOB, GPIOD, GPIOE, QUADSPI, RCC},
};

/// QSPI errors.
#[derive(Debug, Eq, PartialEq)]
pub enum QspiError {
    /// Flash device ID mismatch.
    ReadDeviceId,
    /// Timeout during a polling transaction.
    Timeout,
}

/// Commands and other information specific to the MT25Q.
struct FlashDevice;

impl FlashDevice {
    pub const CMD_READ_ID: u8 = 0x9F;

    pub const DEVICE_ID_MANF: u8 = 0x20;
    pub const DEVICE_ID_MEMT: u8 = 0xBA;
    pub const DEVICE_ID_MEMC: u8 = 0x18;
}

/// QSPI functional mode.
struct QspiMode;

#[allow(dead_code)]
impl QspiMode {
    pub const INDIRECT_WRITE: u8 = 0b00;
    pub const INDIRECT_READ: u8 = 0b01;
    pub const AUTO_POLLING: u8 = 0b10;
    pub const MEMORY_MAPPED: u8 = 0b11;
}

/// QSPI transactions contain configurable instruction, address, and data fields.
struct QspiWidth;

#[allow(dead_code)]
impl QspiWidth {
    pub const NONE: u8 = 0b00;
    pub const SING: u8 = 0b01;
    pub const DUAL: u8 = 0b10;
    pub const QUAD: u8 = 0b11;
}

/// Initialize and configure the QSPI flash driver.
pub fn init(rcc: &mut RCC, gpiob: GPIOB, gpiod: GPIOD, gpioe: GPIOE, qspi: QUADSPI) {
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
        .into_alternate_af10()
        .internal_pull_up(true)
        .set_speed(Speed::VeryHigh);

    // Configure QSPI
    unsafe {
        // Single flash mode with a QSPI clock prescaler of 2 (216 / 2 = 108 MHz)
        qspi.cr.modify(|_, w| {
            w.prescaler()
                .bits(1)
                .fsel()
                .clear_bit()
                .dfm()
                .clear_bit()
                .en()
                .set_bit()
        });

        // Set the device size to 16 MB (2^(1 + 23))
        qspi.dcr.modify(|_, w| w.fsize().bits(23));
    }
}

pub fn check_id() -> Result<(), QspiError> {
    let mut device_id = [0, 0, 0];
    polling_read(&mut device_id, 3, FlashDevice::CMD_READ_ID)?;
    if device_id[0] != FlashDevice::DEVICE_ID_MANF
        || device_id[1] != FlashDevice::DEVICE_ID_MEMT
        || device_id[2] != FlashDevice::DEVICE_ID_MEMC
    {
        Err(QspiError::ReadDeviceId)
    } else {
        Ok(())
    }
}

fn polling_read(buf: &mut [u8], len: usize, instruction: u8) -> Result<(), QspiError> {
    let qspi_regs = unsafe { &(*QUADSPI::ptr()) };
    unsafe {
        qspi_regs.dlr.write(|w| w.bits(len as u32 - 1));
        qspi_regs.ccr.write(|w| {
            w.fmode()
                .bits(QspiMode::INDIRECT_READ)
                // The transaction has data and it is single wire
                .dmode()
                .bits(QspiWidth::SING)
                // The transaction has instruction and it is single wire
                .imode()
                .bits(QspiWidth::SING)
                // The instruction is READ_ID
                .instruction()
                .bits(instruction)
        });
    }

    let timeout = 10000;
    let mut cnt: u32 = 0;
    let mut idx: usize = 0;
    while idx < len {
        // Check if there are bytes in the FIFO
        let num_bytes = qspi_regs.sr.read().flevel().bits();
        if num_bytes > 0 {
            // Read a word
            let val = qspi_regs.dr.read().data().bits();

            // Unpack the word
            let cnt = if num_bytes >= 4 { 4 } else { num_bytes };
            for i in 0..cnt {
                buf[idx] = ((val & (0xFF << i * 8)) >> i * 8).try_into().unwrap();
                idx += 1;
            }
        } else {
            cnt += 1;
            if cnt == timeout {
                return Err(QspiError::Timeout);
            }
        }
    }

    Ok(())
}
