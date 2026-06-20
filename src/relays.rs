//! Relay timing, lockouts, and environmental control logic.

use crate::config::{MIN_LOCKOUT_MS, MIN_ON_FAN_MS, MIN_ON_PUMP_MS};
use crate::sensor::{Reading, SensorBank};
use crate::units::{HUMI_LOW_THRESH, HUMI_VALID_MIN, TEMP_DANGER, TEMP_FAN_THRESH};

#[derive(Clone, Copy, Debug, Default)]
struct TimedRelay {
    active: bool,
    turned_on_at: u32,
    lockout_until: u32,
}

impl TimedRelay {
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

    fn lockout_done(self, now: u32) -> bool {
        now >= self.lockout_until
    }

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

    pub fn update(&mut self, sensors: &SensorBank, now: u32) {
        let readings = sensors.readings();
        let left = readings[1];
        let right = readings[2];

        let left_humi_valid = humi_valid(left);
        let right_humi_valid = humi_valid(right);

        let left_humi_error = left.is_error();
        let right_humi_error = right.is_error();
        let any_humi_error = left_humi_error || right_humi_error;

        let left_temp_above_fan = left.temp_tenths_f() > TEMP_FAN_THRESH;
        let right_temp_above_fan = right.temp_tenths_f() > TEMP_FAN_THRESH;

        let left_temp_danger = left.temp_tenths_f() > TEMP_DANGER;
        let right_temp_danger = right.temp_tenths_f() > TEMP_DANGER;

        let left_temp_good_for_fan_off = left.temp_tenths_f() <= TEMP_FAN_THRESH;
        let right_temp_good_for_fan_off = right.temp_tenths_f() <= TEMP_FAN_THRESH;

        let left_temp_good_for_pump_off = left.temp_tenths_f() <= TEMP_DANGER;
        let right_temp_good_for_pump_off = right.temp_tenths_f() <= TEMP_DANGER;

        let left_humidity_low = left.humi_tenths() < HUMI_LOW_THRESH && left_humi_valid;
        let right_humidity_low = right.humi_tenths() < HUMI_LOW_THRESH && right_humi_valid;

        let left_humidity_good = left.humi_tenths() >= HUMI_LOW_THRESH && left_humi_valid;
        let right_humidity_good = right.humi_tenths() >= HUMI_LOW_THRESH && right_humi_valid;

        if left_temp_above_fan && self.left_fan.is_off() && self.left_fan.lockout_done(now) {
            self.left_fan.turn_on(now);
        }

        if right_temp_above_fan && self.right_fan.is_off() && self.right_fan.lockout_done(now) {
            self.right_fan.turn_on(now);
        }

        if self.pump.is_off()
            && self.pump.lockout_done(now)
            && (left_temp_danger || right_temp_danger || left_humidity_low || right_humidity_low)
        {
            self.pump.turn_on(now);
        }

        if self.left_fan.is_on()
            && left_temp_good_for_fan_off
            && self.left_fan.ran_long_enough(now, MIN_ON_FAN_MS)
        {
            self.left_fan.turn_off(now);
        }

        if self.right_fan.is_on()
            && right_temp_good_for_fan_off
            && self.right_fan.ran_long_enough(now, MIN_ON_FAN_MS)
        {
            self.right_fan.turn_off(now);
        }

        let both_vivarium_good_for_pump = left_temp_good_for_pump_off
            && right_temp_good_for_pump_off
            && left_humidity_good
            && right_humidity_good;

        let should_turn_off_pump_normally = self.pump.is_on()
            && both_vivarium_good_for_pump
            && self.pump.ran_long_enough(now, MIN_ON_PUMP_MS);

        let should_turn_off_pump_error = self.pump.is_on() && any_humi_error;

        if should_turn_off_pump_normally || should_turn_off_pump_error {
            self.pump.turn_off(now);
        }
    }
}

fn humi_valid(reading: Reading) -> bool {
    !reading.is_error() && reading.humi_tenths() > HUMI_VALID_MIN
}
