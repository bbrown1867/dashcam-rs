# Conceptual Design

## Basic Operation
The dash cam will have 3 main modes of operation described below. It will also have some way of uploading video from the dash cam to a host (e.g. SD card, USB).
    * `OFF` - Self explanatory.
    * `ON` - Continuous recording mode.
        * This is not the expected normal mode of operation, but may be useful for recording specific events.
        * The user can `START` recording by pressing the push button and `STOP` recording by pressing the same push button.
        * After stopping the recording, the dash cam will save the buffer to bulk storage and timestamp it.
    * `STANDBY` - Continuous buffering mode.
        * This is the expected mode of operation of the dash cam, when you want to record an incident or event.
        * The dashcam will buffer N minutes of video (TBD or configurable).
        * When the push button is pressed, the dash cam will `CAPTURE` the buffer to bulk storage and timestamp it.

## Future Enhancements
* Custom PCB.
* Professional packaging.
* Use of a GPS to record location.
* Use of an accelerometer to record speed and/or transition from `OFF` to `STANDBY` automatically.
* Host software for uploading and organizing videos.
