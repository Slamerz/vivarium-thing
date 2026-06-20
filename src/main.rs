#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod config;
mod display;
mod millis;
mod relays;
mod sensor;
mod units;

use arduino_hal::prelude::*;
use panic_halt as _;

use config::LOOP_DELAY_MS;
use relays::RelayController;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    for byte in b"Vivarium booted!\r\n" {
        nb::block!(serial.write(*byte)).unwrap_infallible();
    }

    millis::init(dp.TC0);
    unsafe {
        avr_device::interrupt::enable();
    }

    let mut fan_left = pins.d9.into_output();
    let mut fan_right = pins.d10.into_output();
    let mut pump = pins.d8.into_output();
    fan_left.set_low();
    fan_right.set_low();
    pump.set_low();

    let mut i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        50_000,
    );

    arduino_hal::delay_ms(100);
    let _ = display::init_all(&mut i2c);

    let boot_now = millis::now();
    let mut relay_controller = RelayController::new_boot(boot_now);
    let mut sensors = sensor::SensorBank::default();

    loop {
        let _ = sensors.read_all(&mut i2c);

        let now = millis::now();
        relay_controller.update(&sensors, now);

        let outputs = relay_controller.outputs();
        if outputs.left_fan {
            fan_left.set_high();
        } else {
            fan_left.set_low();
        }
        if outputs.right_fan {
            fan_right.set_high();
        } else {
            fan_right.set_low();
        }
        if outputs.pump {
            pump.set_high();
        } else {
            pump.set_low();
        }

        let _ = display::update_all(&mut i2c, &sensors, outputs.pump, now);

        arduino_hal::delay_ms(LOOP_DELAY_MS);
    }
}
