//! Hardware addresses, timing, and control thresholds.

pub const MUX_ADDR: u8 = 0x70;
pub const SENSOR_ADDR: u8 = 0x44;
pub const LCD1_ADDR: u8 = 0x27;
pub const LCD2_ADDR: u8 = 0x26;

pub const SENSOR_COUNT: usize = 4;

pub const MIN_ON_FAN_MS: u32 = 3 * 60 * 1_000;
pub const MIN_ON_PUMP_MS: u32 = 60 * 1_000;
pub const MIN_LOCKOUT_MS: u32 = 5 * 60 * 1_000;

pub const LOOP_DELAY_MS: u32 = 5_000;
pub const SENSOR_MEASURE_DELAY_MS: u32 = 20;

pub const LCD1_TITLE: &str = "Kin'iro & Gin'iro";
pub const LCD2_TITLE: &str = "Marble,Granite,Onyx";
pub const LCD_SUBTITLE: &str = "Temp and Humidity:";
