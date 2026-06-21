//! Environmental control: decides when the fans and misting pump run.
//!
//! The controller is pure logic - it reads sensor values and the current time
//! and produces a [`RelayOutputs`]. Driving the physical pins lives in
//! [`crate::actuators`]. Of the four sensors, channels 1 and 2 are the interior
//! probes for the left and right vivarium and are the only ones used for
//! control; channels 0 and 3 are display-only.

use crate::config::{MIN_LOCKOUT_MS, MIN_ON_FAN_MS, MIN_ON_PUMP_MS};
use crate::sensor::{Reading, SensorBank};
use crate::units::{HUMI_LOW_THRESH, HUMI_VALID_MIN, TEMP_DANGER, TEMP_FAN_THRESH};

/// A relay plus the timing state needed to avoid rapid on/off cycling.
///
/// Two guards protect the hardware:
/// - a minimum on-time, so a relay that just switched on stays on for a while;
/// - a post-off lockout, so a relay that just switched off cannot immediately
///   switch back on.
#[derive(Clone, Copy, Debug, Default)]
struct TimedRelay {
    active: bool,
    turned_on_at: u32,
    lockout_until: u32,
}

impl TimedRelay {
    /// Builds a relay whose lockout already expired at boot, so it is free to
    /// turn on as soon as conditions call for it.
    fn new_boot(now: u32) -> Self {
        Self {
            active: false,
            turned_on_at: 0,
            lockout_until: now.saturating_sub(MIN_LOCKOUT_MS),
        }
    }

    fn is_off(self) -> bool {
        !self.active
    }

    fn is_on(self) -> bool {
        self.active
    }

    /// True once the post-off lockout window has elapsed.
    fn lockout_done(self, now: u32) -> bool {
        now >= self.lockout_until
    }

    /// True once the relay has been on for at least `min_on_ms`.
    fn ran_long_enough(self, now: u32, min_on_ms: u32) -> bool {
        now.saturating_sub(self.turned_on_at) >= min_on_ms
    }

    fn turn_on(&mut self, now: u32) {
        self.active = true;
        self.turned_on_at = now;
    }

    fn turn_off(&mut self, now: u32) {
        self.active = false;
        self.lockout_until = now.saturating_add(MIN_LOCKOUT_MS);
    }
}

/// Desired relay states for the current loop iteration.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelayOutputs {
    pub left_fan: bool,
    pub right_fan: bool,
    pub pump: bool,
}

pub struct RelayController {
    left_fan: TimedRelay,
    right_fan: TimedRelay,
    pump: TimedRelay,
}

impl RelayController {
    pub fn new_boot(now: u32) -> Self {
        Self {
            left_fan: TimedRelay::new_boot(now),
            right_fan: TimedRelay::new_boot(now),
            pump: TimedRelay::new_boot(now),
        }
    }

    pub fn outputs(&self) -> RelayOutputs {
        RelayOutputs {
            left_fan: self.left_fan.active,
            right_fan: self.right_fan.active,
            pump: self.pump.active,
        }
    }

    /// Recomputes every relay state from the latest readings.
    ///
    /// Each side has its own fan, driven purely by that side's temperature. The
    /// single shared pump mists whenever *either* side is dangerously hot or too
    /// dry, and only stops once *both* sides are comfortable again.
    pub fn update(&mut self, sensors: &SensorBank, now: u32) {
        let readings = sensors.readings();
        let left = readings[1];
        let right = readings[2];

        // Fans: each side independently reacts to its own temperature.
        update_fan(&mut self.left_fan, left, now);
        update_fan(&mut self.right_fan, right, now);

        // Pump: turn on if either vivarium is dangerously hot or its (valid)
        // humidity has dropped below the comfort threshold.
        let left_humidity_low = left.humi_tenths() < HUMI_LOW_THRESH && humi_valid(left);
        let right_humidity_low = right.humi_tenths() < HUMI_LOW_THRESH && humi_valid(right);
        let any_temp_danger =
            left.temp_tenths_f() > TEMP_DANGER || right.temp_tenths_f() > TEMP_DANGER;

        if self.pump.is_off()
            && self.pump.lockout_done(now)
            && (any_temp_danger || left_humidity_low || right_humidity_low)
        {
            self.pump.turn_on(now);
        }

        // Pump turns off normally once both sides are safe (temp no longer in the
        // danger zone and humidity back above the comfort threshold) and the
        // minimum run time has elapsed...
        let both_sides_comfortable = left.temp_tenths_f() <= TEMP_DANGER
            && right.temp_tenths_f() <= TEMP_DANGER
            && left.humi_tenths() >= HUMI_LOW_THRESH
            && humi_valid(left)
            && right.humi_tenths() >= HUMI_LOW_THRESH
            && humi_valid(right);
        let turn_off_normally = self.pump.is_on()
            && both_sides_comfortable
            && self.pump.ran_long_enough(now, MIN_ON_PUMP_MS);

        // ...or immediately, ignoring the run time, if a control probe has failed
        // (we don't want to keep misting blind).
        let turn_off_on_error = self.pump.is_on() && (left.is_error() || right.is_error());

        if turn_off_normally || turn_off_on_error {
            self.pump.turn_off(now);
        }
    }
}

/// Drives a single fan from one probe's temperature.
///
/// Turns on above [`TEMP_FAN_THRESH`] (respecting the lockout) and back off once
/// the temperature is at or below the threshold and the minimum on-time has
/// passed. Short-cycling is prevented by the relay's own timing guards.
fn update_fan(fan: &mut TimedRelay, reading: Reading, now: u32) {
    let temp = reading.temp_tenths_f();

    if temp > TEMP_FAN_THRESH && fan.is_off() && fan.lockout_done(now) {
        fan.turn_on(now);
    } else if temp <= TEMP_FAN_THRESH && fan.is_on() && fan.ran_long_enough(now, MIN_ON_FAN_MS) {
        fan.turn_off(now);
    }
}

/// A humidity reading is usable for control only if the probe responded and the
/// value clears the implausibly-low floor (guards against a sensor reading 0%).
fn humi_valid(reading: Reading) -> bool {
    !reading.is_error() && reading.humi_tenths() > HUMI_VALID_MIN
}
