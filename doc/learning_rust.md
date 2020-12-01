# Rust Notes

## Documentation
* Book
    * Primary resource for learning the language
    * Generate locally: `rustup doc --book`
    * Same as the one I ordered online
* Embedded
    * Discusses using Rust for embedded software
    * https://rust-embedded.github.io/book/
* Generate and open HTML API documentation for current crate and dependencies
    * `cargo doc --open`

## Build System and Tools
* Check toolchains installed: `rustup show`
* Turn off auto-generated Git files when running `cargo new` by using:
    * `cargo new <name> --vcs=none`
* Install packages in binary form using `cargo install <name>`
    * Local package with `cargo install --path <name>`
* Check installed packages: `cargo install --list`
* Atom integration
    * Install `ide-rust` and `language-rust` packages
    * Disable the Atom built-in Rust language support package
    * Download `rust-analyzer` binary and place in `PATH`
        * May need to reopen/close Atom or disable/enable `ide-rust` to start it
        * Changed package settings to use absolute path for `rust-analyzer`, seems more reliable now

## Ownership
* Languages have different paradigms of memory management
    * Garbage collection (Java) - Constantly runs in background, freeing unused resources
    * Manual (C/C++) - Programmer must insert malloc/free code at correct places
    * Rust - Compiler inserts free code at correct places
* Rules
    * Each value has a variable that is called it's owner
    * There only can be one owner at a time
    * When owner goes out of scope, the value will be dropped (`drop` function is called)
* Stack/Heap
    * Much like C/C++, the need to `drop` values only applies to data on the heap
    * String literals are constant, but the `String` type is on the heap:
        * `let s1 = String::from("Hello");`
    * Passing around stack variables (e.g. `i32`) will make copies
    * Passing around heap variables (e.g. `String`) will just adjust pointers, must use `s1.clone()` to copy
* __Ownership changes when pointers are passed around__
    * If we pass a pointer into a function, that function is now the owner of the pointer, it will be dropped at end of function if it is not returned
    * To avoid having to return the pointer and still keep it from being dropped, a function must __borrow__ the pointer
    * Borrowing is achieved with __references__, same syntax as C++ (`&` is reference, `*` is dereference)
    * There can only be one mutable reference to a variable at a time

## Error Handling
* Two kinds of errors: recoverable and unrecoverable
* For unrecoverable errors, use the `panic!` macro to end the program
* For recoverable errors, use `Result<T, E>` to return an error code in type `E` or valid value in type `T`
```
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```
* Can use `unwrap()` and `expect("...")` functions to call `panic!` if `Ok` is not returned
* Can use the `?` syntax to propagate errors returned from functions, which is shortcut for a `match` statement as shown below
```
let mut f = match File::open("hello.txt") {
    Ok(file) => file,
    Err(e) => return Err(e),
};
```
```
let mut f = File::open("hello.txt")?;
```

## Oxidize 2020 Embedded Workshop Notes
* Workshop starter code is located [here](https://github.com/ferrous-systems/embedded-trainings-2020)
* Workshop content is located [here](https://embedded-trainings.ferrous-systems.com/)
    * Content also appears to be located in the `embedded-workshop-book` folder
* Needed to install the 5 packages in the `tools` folder
* `beginner/apps/src/bin/hello.rs` example
    * Breakdown of binary size: `cargo size --bin hello -- -A`
    * Use `cargo run --bin hello` to see run (using debugger?)
    * Use `cargo embed --bin hello` to see logs (using serial?)
* `beginner/apps/src/bin/panic.rs` example
    * When using `no_std`, need `#[panic_handler]` function
    * Can import or add your own
* `beginner/apps/src/bin/led.rs` example
    * Basic example of how to use a HAL
    * Generate and open HAL docs: `cargo doc -p dk --open`
* `beginner/apps/src/bin/blinky.rs` example
    * Uses `timer.wait` to delay between loop iterations
    * The module `core::time::Duration` provides delay values
* `boards/dongle` prebuilt binary files for the dongle
    * Make sure it is in bootloader mode (it has no debugger) using RESET button
    * Run `dongle-flash loopback.hex`
    ```
    packaging iHex using nrfutil ...
    DONE
      [####################################]  100%
    Device programmed.
    ```
    * Run `serial-term` to display the Dongle's logs
    * Can change the radio channel using the `change-channel` tool
    * Not supposed to see anything since we aren't sending yet
* `beginner/apps/src/bin/radio-{send,recv}.rs`
    * `radio-send.rs`: Sends a single packet when run
    * Now should see the output on the `serial-term` program
    * Ensure the channel in source code for this matches the dongle
    * Same idea for `radio-recv.rs`, but it will listen for a packet
