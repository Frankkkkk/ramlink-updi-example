#![no_std]
#![no_main]

use avr_device::interrupt::{self, Mutex};
use core::cell::{Cell, RefCell};
use core::fmt::Write;

use panic_halt as _;

use ramlink::producer::RB;

static RING_BUF: Mutex<RefCell<RB<5>>> = Mutex::new(RefCell::new(RB::<5>::new()));

#[avr_device::entry]
fn main() -> ! {
    let dp = avr_device::attiny402::Peripherals::take().unwrap();

    dp.PORTA.dir.write(|w| w.pa1().set_bit());

    interrupt::free(|cs| {
        RING_BUF
            .borrow(cs)
            .borrow_mut()
            .send_bytes_blocking(&[0x55, 0x66]);
    });

    let mut i: u8 = 1;
    loop {
        dp.PORTA.outtgl.write(|w| w.pa1().set_bit());
        for _ in 0..10 {
            avr_device::asm::delay_cycles(1000);
        }
        i = i + 1;
        if i >= 250 {
            i = 0;
            interrupt::free(|cs| {
                let _b = RING_BUF
                    .borrow(cs)
                    .borrow_mut()
                    .send_bytes_blocking(&[1, 2, 3]);

                let mut rb = RING_BUF.borrow(cs).borrow_mut();

                write!(rb, "Hello\n");
            });
        }
    }
}
