# dashcam-rs

Car dashboard camera developed using embedded Rust.

## Goals
* Do a sizable project using embedded Rust, requiring:
    * Some kind of framework or RTOS, like [RTIC](https://github.com/rtic-rs/cortex-m-rtic).
    * HAL driver development (microcontroller peripherals).
    * Device driver development (off-chip devices).
* Build something I would actually use.
    * I've always wanted a dash cam for my car.
    * Many COTS dash cams have a pretty high price point ($100 - $200).
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
1. The dash cam shall provide the capability to capture live video.
2. The dash cam shall provide the capability to save the live video to bulk storage.
3. The dash cam shall provide the capability to upload the video from bulk storage to a host device using a physical connection.
4. The dash cam shall provide the user a push button to `START`, `STOP`, or `CAPTURE` the video stream.
5. The dash cam shall provide the user a switch to change between `ON`, `OFF`, and `STANDBY` modes of operation.
6. The dash cam shall provide the capability to timestamp all video saved to bulk storage.

### Constraints
1. The dash cam shall use an STM32F767ZI microcontroller for all processing.
    * Rationale: I already own a development kit for this part and have experience using it (see [nucleof767zi-rs](https://github.com/bbrown1867/nucleof767zi-rs)).
    * Rationale: This part is intended for high-performance embedded applications, like audio/video.
    * Rationale: This part contains hardware support for video (DCIM) and bulk storage (QSPI flash, SD card)
2. The dash cam shall use an OV9655 color CMOS camera for video capture.
    * Rationale: Compatible with DCIM on the STM32F767ZI and example C code exists.
    * Rationale: Has lots of desirable features like high resolution (1.3 MP), color, multiple output data formats.
    * Rationale: [Development board](https://www.waveshare.com/ov9655-camera-board.htm) is available for purchase that will connect easily to a microcontroller development kit.
    * _Note: OmniVision does not market this product anymore and I can't find any sellers for the chip itself. This could be a problem when making a custom PCB. The only in-stock OmniVision product with the same parallel interface I could find on DigiKey was the OVM7692. However it has a much lower resolution and no datasheet in the public domain._

## Low-Level Requirements (Tasks)
1. DCIM HAL driver development and OV9655 device driver development.
    * Note: DCIM driver does not currently exist in the [Rust HAL for STM32F7](https://github.com/stm32-rs/stm32f7xx-hal).
    * Note: Can't develop the DCIM driver without using a real camera, so must develop OV9655 driver at the same time.
    * HAL driver development shall be done in [my fork](https://github.com/bbrown1867/stm32f7xx-hal) of the Rust HAL.
    * OV9655 driver development should be done in this repo, maybe moving to HAL repo as an example later.
    * Initial goal shall be to capture a low-quality image into SRAM (640x480 = 307 KB).
2. Continuous capture and frame buffering.
    * The software will need to be extended to continuously capture frames and store in a frame buffer.
    * Circular programming of DMA controller.
    * Since anything remotely high-quality will be larger than on-chip SRAM (512 KB), off-chip memory (e.g. SDRAM) is needed.
3. Write video to non-volatile memory.
    * After the frame buffer is working, will need to save it to bulk storage on a user request.
    * Seems like a filesystem will be needed here.
    * Setup push button to trigger the save action.
    * Timestamping functionality from microcontroller timer.
