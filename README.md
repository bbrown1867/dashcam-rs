# dashcam-rs

Car dashboard camera developed using embedded Rust.

## Goals
* Do a sizable project using embedded Rust, requiring:
    * Some kind of framework or RTOS, like [RTIC](https://github.com/rtic-rs/cortex-m-rtic).
    * Device driver development, maybe using [embedded-hal](https://github.com/rust-embedded/embedded-hal).
* Build something I would actually use.
    * I've always wanted a dash cam for my car.
    * Many COTS dash cams have a pretty high price point ($100 - $200), I think I can do it for less.
    * Many COTS dash cams have features I don't need or want (e.g. network upload).

## Conceptual Design
The conceptual design is shown the diagram below. The image data path is shown on the arrows in the STM32F7.
![](img/design.jpg)

* The dash cam will have 3 main modes of operation described below. It will also have some way of uploading video from the dash cam to a host (e.g. SD card, USB).
    * `OFF` - Self explanatory.
    * `ON` - Continuous recording mode.
        * This is not the expected normal mode of operation, but may be useful for recording specific events.
        * The user can `START` recording by pressing the push button and `STOP` recording by pressing the same push button.
        * After stopping the recording, the dash cam will save the buffer to bulk storage and timestamp it.
    * `STANDBY` - Continuous buffering mode.
        * This is the expected mode of operation of the dash cam, when you want to record an incident or event.
        * The dashcam will buffer N minutes of video (TBD or configurable).
        * When the push button is pressed, the dash cam will `CAPTURE` the buffer to bulk storage and timestamp it.
* Future enhancements:
    * Custom PCB.
    * Professional packaging.
    * Use of a GPS to record location.
    * Use of an accelerometer to record speed and/or transition from `OFF` to `STANDBY` automatically.
    * Use of microcontroller low-power modes.
    * Host software for uploading and organizing videos.

## High-Level Requirements

### Features
1. The dash cam shall provide the capability to continuously capture live video.
2. The dash cam shall provide the capability to buffer N minutes of live video, prior to saving or discarding.
3. The dash cam shall provide the capability to save the live video to bulk storage.
4. The dash cam shall provide the capability to upload the video from bulk storage to a host device using a physical connection.
5. The dash cam shall provide the user a push button to `START`, `STOP`, or `CAPTURE` the video stream.
6. The dash cam shall provide the user a switch to change between `ON`, `OFF`, and `STANDBY` modes of operation.
7. The dash cam shall provide the capability to timestamp all video saved to bulk storage.

### Constraints
1. The dash cam shall use an STM32F746NG microcontroller for all processing.
    * __Rationale__: I have [experience using the STM32F7](https://github.com/bbrown1867/nucleof767zi-rs) family of devices.
    * __Rationale__: There is a [development board for the STM32F746NG](https://www.st.com/en/evaluation-tools/32f746gdiscovery.html) that has SDRAM, QSPI flash, camera connector, and SD card connector.
    * __Rationale__: This device is intended for high-performance embedded applications like audio/video.
    * __Rationale__: This device contains hardware for video (DCIM) and bulk storage (QSPI, SDIO).
2. The dash cam shall use an OV9655 color CMOS camera for video capture.
    * __Rationale__: Compatible with DCIM on the STM32F746NG and example C code exists
    * __Rationale__: Has lots of desirable features like high resolution (1.3 MP), color, multiple output data formats, 30 fps.
    * __Rationale__: Several development boards exist ([Waveshare](https://www.waveshare.com/ov9655-camera-board.htm), [STM32F4DIS-CAM](https://www.newark.com/stmicroelectronics/stm32f4dis-cam/module-1-3mp-camera-f4-discovery/dp/47W1732)) that will easily connect to a microcontroller development board.
    * _Note: OmniVision does not market this product anymore and I can't find any sellers for the chip itself. This could be a problem when making a custom PCB. The only in-stock OmniVision product with the same parallel interface I could find on DigiKey was the OVM7692. However it has a much lower resolution and no datasheet in the public domain._

## Low-Level Requirements (Tasks)
1. OV9655 device driver development.
    * Driver will involve three peripherals: I2C for configuration (SCCB), DCIM for capturing pixels, and DMA to transfer from DCIM to memory.
    * DCIM driver does not currently exist in the [Rust HAL for STM32F7](https://github.com/stm32-rs/stm32f7xx-hal). Also the DMA driver in the Rust HAL does not support DCIM.
    * SCCB portion should use `embedded-hal` traits to be easily portable.
    * Any HAL improvements will be done in [my fork](https://github.com/bbrown1867/stm32f7xx-hal) and merged into upstream if it makes sense.
    * Initial goal shall be to capture a low-quality (QVGA, RGB565) image into SRAM (320x240x2 = 153.6 KB).
2. Continuous capture and frame buffering.
    * The software will need to be extended to continuously capture frames and store in a frame buffer.
    * Circular programming of DMA controller.
    * Since anything remotely high-quality will be larger than on-chip SRAM (512 KB), off-chip memory (e.g. SDRAM) is needed.
3. Write video to non-volatile memory.
    * After the frame buffer is working, will need to save it to bulk storage on a user request.
    * Seems like a filesystem will be needed here.
    * Setup push button to trigger the save action.
    * Timestamping functionality from microcontroller timer.
4. Framework integration (e.g. RTIC).
    * Eventually software design will get large enough where a framework or RTOS is needed.
    * Create a software architecture and split into tasks, events, modules, etc.
