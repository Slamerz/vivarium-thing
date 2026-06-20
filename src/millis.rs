//! Millisecond counter backed by Timer0, matching Arduino `millis()`.

use arduino_hal::pac::TC0;
use core::cell::Cell;

const PRESCALER: u32 = 1024;
const TIMER_COUNTS: u32 = 125;
const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16_000;

static MILLIS_COUNTER: avr_device::interrupt::Mutex<Cell<u32>> =
    avr_device::interrupt::Mutex::new(Cell::new(0));

pub fn init(tc0: TC0) {
    tc0.tccr0a().write(|w| w.wgm0().ctc());
    tc0.ocr0a().write(|w| w.set(TIMER_COUNTS as u8));
    tc0.tccr0b().write(|w| w.cs0().prescale_1024());
    tc0.timsk0().write(|w| w.ocie0a().set_bit());

    avr_device::interrupt::free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

pub fn now() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter = MILLIS_COUNTER.borrow(cs);
        counter.set(counter.get().wrapping_add(MILLIS_INCREMENT));
    });
}
