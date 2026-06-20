#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Built-in Uno/SunFounder LED: digital pin 13.
    let mut led = pins.d13.into_output();
    
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    for byte in b"Vivarium booted!\r\n" {
        nb::block!(serial.write(*byte)).unwrap_infallible();
    }
    
    loop {
        led.toggle();
        arduino_hal::delay_ms(1000);
        for byte in b"running...\r\n" {
            nb::block!(serial.write(*byte)).unwrap_infallible();
        }
    }
}