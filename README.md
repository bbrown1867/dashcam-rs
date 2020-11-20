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
    * A HAL driver for the QSPI peripheral on the STM32F746G. Supports indirect mode with polling or DMA.
        * TODO: Upstream to `stm32f7xx-hal`.
    * A higher-level driver for the 16 MB MT25Q flash device.

## Hardware Architecture
![](img/design.jpg)

## Software Architecture
![](img/software.jpg)

## Limitations and Next Steps

### Memory
The biggest limitation with this prototype is the amount of memory, both volatile and non-volatile. The SDRAM chip used for frame buffering is 8 MB and the QSPI flash chip for saving frames is 16 MB. As a result, we can only buffer around 2-3 seconds of video. As described in detail in this [issue](https://github.com/bbrown1867/dashcam-rs/issues/2), somewhere between 275 MB - 1.3 GB of RAM is needed to buffer a few minutes of video. For non-volatile memory (flash), probably at least 2x to 10x of the RAM size is desired to store multiple clips during a drive. To increase both memories, a new hardware platform is needed. Since the SDRAM chip is connected via a high-speed interface, a custom PCB would be needed.

There are a couple other ways to resolve this problem:
* Reduce resolution and frame rate.
    * This is not very desirable since resolution is already quite low.
    * The OV9655 only supports 15 or 30 fps, so manual downsampling would be required for anything less than 15 fps.
* Change the buffering methodology.
    * Buffer only a few seconds, then write to flash/non-volatile memory continuously. Saving a video is simply updating metadata and files in non-volatile memory.
    * This would be more work software and more application logic.
    * Writing to the current flash memory device is very slow, so a different one may be needed.
    * Overall this solution is more messy than the current implementation.

### Filesystem
Currently non-volatile memory (flash) is accessed using raw data and addresses. To support saving and organizing multiple video clips, a filesystem is most likely needed. A stable, mature embedded filesystem most likely does not exist in the embedded Rust ecosystem, so this may be a case for migrating to an embedded Linux platform rather than bare-metal.

## Further Reading
* [Original conceptual design](doc/dashcam_design.md)
* [Original requirements](doc/dashcam_reqs.md)
