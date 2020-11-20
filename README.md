# dashcam-rs

## Overview
This project is a prototype for a car dashboard camera, created to learn more about the Rust programming language and the embedded Rust ecosystem. The prototype implements the basic functionality needed by a car dash cam:
* Capture live video, with a display for preview.
* Buffer past video, ideally several minutes of it.
* Save buffered video to non-volatile memory on user intervention. For example, after a car accident.

## Demo
The STM32F746G Discovery Board board is used for the hardware platform with an OV9655 CMOS camera attached.

The dash cam buffers as many past frames as possible in SDRAM. On a user button press, the past frames are saved to flash memory. There is a small (~8 second) delay, as write operations for this particular flash device must be done one page at a time (256 bytes). On the next button press, the saved frames are read from flash into SDRAM and played continuously in a loop.

## Embedded Rust Community
This project relies heavily on a lot of great open-source software created by the embedded Rust community, including:
* [RTIC](https://github.com/rtic-rs/cortex-m-rtic/): A small concurrency framework for Cortex-M processors, like a mini-RTOS.
* [cortex-m](https://github.com/rust-embedded/cortex-m) and [cortex-m-rt](https://github.com/rust-embedded/cortex-m-rt): Low-level device support for Cortex-M processors.
* [stm32f7xx-hal](https://github.com/stm32-rs/stm32f7xx-hal): Hardware abstraction layer (drivers) for the STM32F7 part family.
* [embedded-hal](https://github.com/rust-embedded/embedded-hal): Hardware abstraction layer traits for common peripherals.
* [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics): 2D graphics library for embedded devices.

Hopefully, this project adds the following contributions to the embedded Rust community:
* [OV9655 device driver](src/ov9655)
    * Abstract SCCB driver for device configuration using `embedded-hal`.
    * Parallel interface driver for the STM32F746G using DCIM and DMA2, including live video capture with ping-pong DMA.
* [QSPI flash driver](src/board/qspi.rs)
    * A HAL driver for the QUADSPI peripheral on the STM32F746G. Supports indirect mode with polling or DMA.
        * `TODO:`: Upstream to `stm32f7xx-hal`.
    * A higher-level driver for the MT25QL128ABA QSPI flash device.

## Hardware Architecture
![](img/design.jpg)

## Software Architecture

## Limitations and Next Steps
