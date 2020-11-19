//! QSPI driver for the MT25QL128ABA located on the STM32F746G Discovery Board.

use crate::nvm::Mem;
use core::convert::TryInto;
use stm32f7xx_hal::{
    gpio::{GpioExt, Speed},
    pac::{GPIOB, GPIOD, GPIOE, QUADSPI, RCC},
};

/// Handle for the QSPI driver.
pub struct QspiDriver {
    /// QSPI peripheral registers.
    qspi: QUADSPI,
}

/// QSPI errors.
#[derive(Debug, Eq, PartialEq)]
pub enum QspiError {
    /// Flash device ID mismatch.
    ReadDeviceId,
    /// Timeout during a polling transaction.
    Timeout,
    /// Timeout waiting for a write/erase to complete.
    StatusTimeout,
}

/// Commands and other information specific to the MT25Q.
struct FlashDevice;

impl FlashDevice {
    pub const CMD_READ_ID: u8 = 0x9F;
    pub const CMD_MEM_READ: u8 = 0x03;
    pub const CMD_MEM_PROGRAM: u8 = 0x02;
    pub const CMD_BULK_ERASE: u8 = 0xC7;
    pub const CMD_SUBSECT_ERASE: u8 = 0x20;
    pub const CMD_READ_FLAG_STATUS: u8 = 0x70;
    pub const CMD_WRITE_ENABLE: u8 = 0x06;
    pub const DEVICE_ID_MANF: u8 = 0x20;
    pub const DEVICE_ID_MEMT: u8 = 0xBA;
    pub const DEVICE_ID_MEMC: u8 = 0x18;
    pub const DEVICE_MAX_ADDRESS: u32 = 0x00FF_FFFF;
    pub const DEVICE_SUBSECTOR_SIZE: u32 = 4096;
    pub const DEVICE_PAGE_SIZE: u32 = 256;
}

/// QSPI transaction description.
struct QspiTransaction {
    iwidth: u8,
    awidth: u8,
    dwidth: u8,
    instruction: u8,
    address: Option<u32>,
    dummy: u8,
    data_len: Option<usize>,
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

impl QspiDriver {
    /// Initialize and configure the QSPI flash driver.
    pub fn new(rcc: &mut RCC, gpiob: GPIOB, gpiod: GPIOD, gpioe: GPIOE, qspi: QUADSPI) -> Self {
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

            QspiDriver { qspi }
        }
    }

    /// Check the identification bytes of the flash device to validate communication.
    pub fn check_id(&mut self) -> Result<(), QspiError> {
        let transaction = QspiTransaction {
            iwidth: QspiWidth::SING,
            awidth: QspiWidth::NONE,
            dwidth: QspiWidth::SING,
            instruction: FlashDevice::CMD_READ_ID,
            address: None,
            dummy: 0,
            data_len: Some(3),
        };

        let mut device_id = [0, 0, 0];
        self.polling_read(&mut device_id, transaction)?;

        if device_id[0] != FlashDevice::DEVICE_ID_MANF
            || device_id[1] != FlashDevice::DEVICE_ID_MEMT
            || device_id[2] != FlashDevice::DEVICE_ID_MEMC
        {
            Err(QspiError::ReadDeviceId)
        } else {
            Ok(())
        }
    }

    /// Erase `len` bytes at address `src` sector-by-sector. If `src` is not sector aligned, the
    /// start of sector it resides in will be the starting address for the erase. A pair is
    /// returned containing the total number of bytes erased and the erase starting address.
    pub fn sector_erase(&mut self, src: u32, len: usize) -> Result<(u32, u32), QspiError> {
        assert!(len > 0);
        assert!(src + (len as u32) <= FlashDevice::DEVICE_MAX_ADDRESS);

        let mut num_erased_bytes: u32 = 0;
        let mut addr: u32 = src - (src % FlashDevice::DEVICE_SUBSECTOR_SIZE);
        let start_addr = addr;

        // The smallest possible erase is a subsector (4KB)
        while num_erased_bytes < (len as u32) {
            self.write_enable()?;

            let transaction = QspiTransaction {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                dwidth: QspiWidth::NONE,
                instruction: FlashDevice::CMD_SUBSECT_ERASE,
                address: Some(addr & FlashDevice::DEVICE_MAX_ADDRESS),
                dummy: 0,
                data_len: None,
            };

            let mut dummy = [0];
            self.polling_read(&mut dummy, transaction)?;

            num_erased_bytes += FlashDevice::DEVICE_SUBSECTOR_SIZE;
            addr += FlashDevice::DEVICE_SUBSECTOR_SIZE;

            self.poll_status(10000)?;
        }

        Ok((num_erased_bytes, start_addr))
    }

    /// Poll the status register until not busy. Necessary after write/erase operations.
    fn poll_status(&mut self, timeout: u32) -> Result<(), QspiError> {
        let mut cnt = 0;
        let mut status = 0;
        while status & 0x80 == 0 {
            status = match self.read_flag_status() {
                Ok(status) => status,
                Err(e) => return Err(e),
            };

            cnt += 1;
            if cnt == timeout {
                return Err(QspiError::StatusTimeout);
            }
        }

        Ok(())
    }

    /// Write enable.
    fn write_enable(&mut self) -> Result<(), QspiError> {
        let transaction = QspiTransaction {
            iwidth: QspiWidth::SING,
            awidth: QspiWidth::NONE,
            dwidth: QspiWidth::NONE,
            instruction: FlashDevice::CMD_WRITE_ENABLE,
            address: None,
            dummy: 0,
            data_len: None,
        };

        let mut dummy = [0];
        self.polling_read(&mut dummy, transaction)
    }

    /// Read flag status register.
    fn read_flag_status(&mut self) -> Result<u8, QspiError> {
        let transaction = QspiTransaction {
            iwidth: QspiWidth::SING,
            awidth: QspiWidth::NONE,
            dwidth: QspiWidth::SING,
            instruction: FlashDevice::CMD_READ_FLAG_STATUS,
            address: None,
            dummy: 0,
            data_len: Some(1),
        };

        let mut status = [0];
        self.polling_read(&mut status, transaction)?;

        Ok(status[0])
    }

    /// Polling indirect read. Can also be used to perform transactions with no data.
    fn polling_read(
        &mut self,
        buf: &mut [u8],
        transaction: QspiTransaction,
    ) -> Result<(), QspiError> {
        self.setup_transaction(QspiMode::INDIRECT_READ, &transaction);

        match transaction.data_len {
            Some(len) => {
                let timeout = 10000;
                let mut cnt: u32 = 0;
                let mut idx: usize = 0;
                while idx < len {
                    // Check if there are bytes in the FIFO
                    let num_bytes = self.qspi.sr.read().flevel().bits();
                    if num_bytes > 0 {
                        // Read a word
                        let word = self.qspi.dr.read().data().bits();

                        // Unpack the word
                        let num_unpack = if num_bytes >= 4 { 4 } else { num_bytes };
                        for i in 0..num_unpack {
                            buf[idx] = ((word & (0xFF << i * 8)) >> i * 8).try_into().unwrap();
                            idx += 1;
                        }
                    } else {
                        cnt += 1;
                        if cnt == timeout {
                            return Err(QspiError::Timeout);
                        }
                    }
                }
            }
            None => (),
        }

        Ok(())
    }

    /// Polling indirect write.
    fn polling_write(
        &mut self,
        buf: &mut [u8],
        transaction: QspiTransaction,
        start_idx: usize,
    ) -> Result<(), QspiError> {
        self.setup_transaction(QspiMode::INDIRECT_WRITE, &transaction);

        match transaction.data_len {
            Some(len) => {
                let timeout = 10000;
                let mut cnt: u32 = 0;
                let mut idx: usize = 0;
                while idx < len {
                    // Check if the FIFO is empty
                    let num_bytes = self.qspi.sr.read().flevel().bits();
                    if num_bytes == 0 {
                        // Pack the word
                        let mut word: u32 = 0;
                        let num_pack = if (len - idx) >= 4 { 4 } else { len - idx };
                        for i in 0..num_pack {
                            word |= (buf[start_idx + idx] as u32) << (i * 8);
                            idx += 1;
                        }

                        // Write a word
                        unsafe {
                            self.qspi.dr.write(|w| w.data().bits(word));
                        }
                    } else {
                        cnt += 1;
                        if cnt == timeout {
                            return Err(QspiError::Timeout);
                        }
                    }
                }
            }
            None => (),
        }

        Ok(())
    }

    /// Map from QspiTransaction to QSPI registers.
    fn setup_transaction(&mut self, fmode: u8, transaction: &QspiTransaction) {
        unsafe {
            match transaction.data_len {
                Some(len) => self.qspi.dlr.write(|w| w.bits(len as u32 - 1)),
                None => (),
            };

            // Note: This part always has 24-bit addressing (adsize)
            self.qspi.ccr.write(|w| {
                w.fmode()
                    .bits(fmode)
                    .imode()
                    .bits(transaction.iwidth)
                    .admode()
                    .bits(transaction.awidth)
                    .dmode()
                    .bits(transaction.dwidth)
                    .adsize()
                    .bits(0b10)
                    .abmode()
                    .bits(QspiWidth::NONE)
                    .dcyc()
                    .bits(transaction.dummy)
                    .instruction()
                    .bits(transaction.instruction)
            });

            match transaction.address {
                Some(addr) => self.qspi.ar.write(|w| w.bits(addr)),
                None => (),
            };
        }
    }
}

/// Implementation of `Mem` traits for the MT25QL128ABA using the ST32F7 QSPI peripheral.
impl Mem for QspiDriver {
    type Error = QspiError;

    /// Blocking read implementation for QSPI flash.
    fn read(&mut self, dest: &mut [u8], src: u32, len: usize) -> Result<(), QspiError> {
        assert!(len > 0);
        assert!(src + (len as u32) <= FlashDevice::DEVICE_MAX_ADDRESS);

        let transaction = QspiTransaction {
            iwidth: QspiWidth::SING,
            awidth: QspiWidth::SING,
            dwidth: QspiWidth::SING,
            instruction: FlashDevice::CMD_MEM_READ,
            address: Some(src & FlashDevice::DEVICE_MAX_ADDRESS),
            dummy: 0,
            data_len: Some(len),
        };

        self.polling_read(dest, transaction)
    }

    /// Blocking write implementation for QSPI flash.
    fn write(&mut self, dest: u32, src: &mut [u8], len: usize) -> Result<(), QspiError> {
        assert!(len > 0);
        assert!(dest + (len as u32) <= FlashDevice::DEVICE_MAX_ADDRESS);

        let mut outer_idx: usize = 0;
        let mut curr_addr: u32 = dest;
        let mut curr_len: usize = len;

        // Constraints for writes: (1) Must be <= 256 bytes, (2) must not cross a page boundry
        while curr_len > 0 {
            self.write_enable()?;

            let start_page = curr_addr - (curr_addr % FlashDevice::DEVICE_PAGE_SIZE);
            let end_page = start_page + FlashDevice::DEVICE_PAGE_SIZE;
            let size: usize = if curr_addr + (curr_len as u32) > end_page {
                (end_page - curr_addr) as usize
            } else {
                curr_len
            };

            let transaction = QspiTransaction {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: FlashDevice::CMD_MEM_PROGRAM,
                address: Some(curr_addr & FlashDevice::DEVICE_MAX_ADDRESS),
                dummy: 0,
                data_len: Some(size),
            };

            self.polling_write(src, transaction, outer_idx)?;

            self.poll_status(10000)?;

            curr_addr += size as u32;
            curr_len -= size;
            outer_idx += size;
        }

        Ok(())
    }

    /// Blocking erase implementation for QSPI flash. This takes several seconds.
    fn erase(&mut self) -> Result<(), QspiError> {
        self.write_enable()?;

        let transaction = QspiTransaction {
            iwidth: QspiWidth::SING,
            awidth: QspiWidth::NONE,
            dwidth: QspiWidth::NONE,
            instruction: FlashDevice::CMD_BULK_ERASE,
            address: None,
            dummy: 0,
            data_len: None,
        };

        let mut dummy = [0];
        self.polling_read(&mut dummy, transaction)?;
        self.poll_status(1000000)
    }
}

/// Tests for the QSPI flash driver.
pub mod tests {
    use super::*;

    /// Simple loopback memory test for the QSPI flash driver. Performs two checks:
    /// - Erase/read: After erasing, all bytes in the region [ADDR:ADDR+LEN) should be 0xFF.
    /// - Write/read: After writing, all bytes in the region [ADDR:ADDR+LEN) should match the
    ///   values written.
    /// Note:
    /// - The test addr is an odd, non page aligned address to stress the `memory_write` function.
    /// - The test length is greater than one subsector to stress the `memory_erase` function.
    pub fn test_mem(dut: &mut QspiDriver) {
        const ADDR: u32 = 0x7003;
        const LEN: usize = 4121;
        let mut read_buffer: [u8; LEN] = [0; LEN];
        let mut write_buffer: [u8; LEN] = [0; LEN];
        for i in 0..LEN {
            write_buffer[i] = i as u8;
        }

        match dut.sector_erase(ADDR, LEN) {
            Ok(pair) => {
                let (num_erase, addr_erase) = pair;
                assert!(LEN <= num_erase as usize);
                assert!(addr_erase <= ADDR);
            }
            Err(e) => panic!("Erase failed with error = {:?}", e),
        };
        dut.read(&mut read_buffer, ADDR, LEN).unwrap();
        for i in 0..LEN {
            assert!(read_buffer[i] == 0xFF);
        }

        dut.write(ADDR, &mut write_buffer, LEN).unwrap();
        dut.read(&mut read_buffer, ADDR, LEN).unwrap();
        for i in 0..LEN {
            if write_buffer[i] != read_buffer[i] {
                panic!(
                    "Error: Mismatch at address {:X}. Expected {:X} but read {:X}",
                    ADDR + i as u32,
                    write_buffer[i],
                    read_buffer[i]
                );
            }
        }
    }
}
