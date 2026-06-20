#![no_std]
#![no_main]

use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Built-in Uno/SunFounder LED: digital pin 13.
    let mut led = pins.d13.into_output();
    led.toggle();
    loop {
    
        arduino_hal::delay_ms(1000);
    }
}