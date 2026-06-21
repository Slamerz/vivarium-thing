//! Hardware addresses, timing, and control thresholds.
//!
//! Central place for the values most likely to need tuning when the rig
//! changes: I2C addresses, how often things run, and the temperature/humidity
//! comparison thresholds live in [`crate::units`].

// --- I2C device addresses (7-bit) ---
/// TCA9548A 1-to-8 channel mux that fans the bus out to the sensors.
pub const MUX_ADDR: u8 = 0x70;
/// SHT3x temperature/humidity probe (shared address behind the mux).
pub const SENSOR_ADDR: u8 = 0x44;
/// PCF8574 backpack for the first LCD panel.
pub const LCD1_ADDR: u8 = 0x27;
/// PCF8574 backpack for the second LCD panel.
pub const LCD2_ADDR: u8 = 0x26;

/// Number of sensor channels read each loop. Channels 1 and 2 are the interior
/// control probes (left/right vivarium); 0 and 3 are display-only.
pub const SENSOR_COUNT: usize = 4;

// --- Relay timing (anti-short-cycle guards) ---
/// Minimum time a fan stays on once switched on.
pub const MIN_ON_FAN_MS: u32 = 3 * 60 * 1_000;
/// Minimum time the pump stays on once switched on.
pub const MIN_ON_PUMP_MS: u32 = 60 * 1_000;
/// Cool-down after any relay switches off before it may switch on again.
pub const MIN_LOCKOUT_MS: u32 = 5 * 60 * 1_000;

// --- Loop / sensor timing ---
/// Delay between control-loop iterations.
pub const LOOP_DELAY_MS: u32 = 5_000;
/// Wait between issuing an SHT3x measurement and reading the result.
pub const SENSOR_MEASURE_DELAY_MS: u32 = 20;

// --- LCD static text ---
pub const LCD1_TITLE: &str = "Kin'iro & Gin'iro";
pub const LCD2_TITLE: &str = "Marble,Granite,Onyx";
pub const LCD_SUBTITLE: &str = "Temp and Humidity:";
