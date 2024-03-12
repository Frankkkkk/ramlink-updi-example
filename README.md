# Rust ramlink example (uni-directionnal communication over UPDI)

This repo contains an example of the rust [ramlink](https://github.com/Frankkkkk/rust-ramlink) crate. It enables one way communication from an embedded device (microcontroller) to the host using a JTAG link (in my case, AVR's UPDI interface).

The repo ist divided into two parts:
- producer: the code that runs into the AVR device. In my case an ATtiny402
- consumer: the code that reads the values from the embedded device, using the host debugging interface (using `jtag2updi`)


## Jtag2updi

Please note that the consumer uses the excellent [jtag2updi code](https://github.com/ElTangas/jtag2updi) in order to "talk UPDI" with the AVR device.

**HOWEVER**, the current firmware does not allow to read the SRAM without halting the CPU. If you want to read the ramlink buffer without halting your code, you must [use my fork](https://github.com/ElTangas/jtag2updi/pull/75) (or hope that the [pull request](https://github.com/ElTangas/jtag2updi/pull/75) is merged :)).