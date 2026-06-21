#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod actuators;
mod config;
mod display;
mod millis;
mod relays;
mod sensor;
mod units;

use arduino_hal::prelude::*;
use panic_halt as _;

use actuators::Actuators;
use config::LOOP_DELAY_MS;
use relays::RelayController;

#[arduino_hal::entry]
fn main() -> ! {
    // Acquire the singleton peripheral block and split it into individual pins.
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // UART console used only for boot/diagnostic logging.
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    for byte in b"Vivarium booted!\r\n" {
        nb::block!(serial.write(*byte)).unwrap_infallible();
    }

    // Bring up the Arduino-style `millis()` clock on Timer0, then globally enable
    // interrupts so the timer can start accumulating elapsed time.
    millis::init(dp.TC0);
    unsafe {
        avr_device::interrupt::enable();
    }

    // Relay outputs (active-high). See `actuators` for the pin map:
    // D9 = left fan, D10 = right fan, D8 = pump. Start with everything off.
    let mut left_fan = pins.d9.into_output();
    let mut right_fan = pins.d10.into_output();
    let mut pump = pins.d8.into_output();
    left_fan.set_low();
    right_fan.set_low();
    pump.set_low();
    let mut actuators = Actuators::new(left_fan, right_fan, pump);

    // Shared I2C bus (A4 = SDA, A5 = SCL) carrying the TCA9548A sensor mux and
    // both LCD backpacks. 50 kHz keeps the long sensor wiring reliable.
    let mut i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        50_000,
    );

    // Let the LCDs settle after power-up before initialising them.
    arduino_hal::delay_ms(100);
    let _ = display::init_all(&mut i2c);

    // Pre-date the relay lockouts to boot time so they may fire immediately.
    let boot_now = millis::now();
    let mut relay_controller = RelayController::new_boot(boot_now);
    let mut sensors = sensor::SensorBank::default();

    loop {
        // 1. Sample every sensor channel through the mux.
        let _ = sensors.read_all(&mut i2c);

        // 2. Run the control logic against the latest readings.
        let now = millis::now();
        relay_controller.update(&sensors, now);

        // 3. Push the controller's decisions out to the physical relays.
        let outputs = relay_controller.outputs();
        actuators.apply(outputs);

        // 4. Refresh both LCD panels (pump state drives the spray animation).
        let _ = display::update_all(&mut i2c, &sensors, outputs.pump, now);

        arduino_hal::delay_ms(LOOP_DELAY_MS);
    }
}
