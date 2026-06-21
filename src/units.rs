//! Fixed-point helpers for sensor values and thresholds.
//!
//! The AVR has no FPU, so floating point is slow and bloats the binary. Instead
//! every temperature/humidity value is stored as [`Tenths`] - an integer scaled
//! by 10, i.e. one decimal place (e.g. `750` means `75.0`). All conversions and
//! comparisons stay in integer arithmetic.

/// A value scaled by 10 (one decimal place). `750` == `75.0`.
pub type Tenths = i16;

/// Above this temperature (75.0 F) the side's fan switches on.
pub const TEMP_FAN_THRESH: Tenths = 750;
/// Above this temperature (85.0 F) the side is "dangerously hot" and triggers the pump.
pub const TEMP_DANGER: Tenths = 850;
/// Below this humidity (70.0 %) a side is too dry and triggers the pump.
pub const HUMI_LOW_THRESH: Tenths = 700;
/// Humidity at or below this (10.0 %) is treated as implausible and ignored for control.
pub const HUMI_VALID_MIN: Tenths = 100;

/// Full-scale value of the SHT3x's 16-bit raw output (`2^16 - 1`).
const SHT3X_FULL_SCALE: i32 = 65535;

/// Converts a raw SHT3x reading to tenths of a degree Fahrenheit.
///
/// Datasheet: `T_celsius = -45 + 175 * raw / 65535`, then the usual
/// `F = C * 9/5 + 32`. We carry the intermediate values in milli-units
/// (thousandths) for precision, then scale down to tenths at the end.
pub fn celsius_raw_to_tenths_f(raw: u16) -> Tenths {
    let celsius_milli = -45_000i32 + (175_000i32 * i32::from(raw) / SHT3X_FULL_SCALE);
    let fahrenheit_milli = celsius_milli * 9 / 5 + 32_000;
    (fahrenheit_milli / 100) as Tenths
}

/// Converts a raw SHT3x reading to tenths of a percent relative humidity.
///
/// Datasheet: `RH = 100 * raw / 65535`; scaled here to tenths (`1000 * ...`).
pub fn humi_raw_to_tenths(raw: u16) -> Tenths {
    (1_000i32 * i32::from(raw) / SHT3X_FULL_SCALE) as Tenths
}

/// Formats a [`Tenths`] value as ASCII into `dest` (e.g. `752` -> `"75.2F"`),
/// appending `suffix` (such as `b'F'` or `b'%'`). Returns the number of bytes
/// written. Used instead of `format!` to avoid heap allocation on the AVR.
pub fn write_tenths(dest: &mut [u8], value: Tenths, suffix: u8) -> usize {
    let (sign, abs) = if value < 0 {
        (b'-', (-value) as u16)
    } else {
        (0, value as u16)
    };

    let whole = abs / 10;
    let frac = abs % 10;

    let mut len = 0;
    if sign != 0 && len < dest.len() {
        dest[len] = sign;
        len += 1;
    }

    len += write_u16(&mut dest[len..], whole);
    if len < dest.len() {
        dest[len] = b'.';
        len += 1;
    }
    if len < dest.len() {
        dest[len] = b'0' + frac as u8;
        len += 1;
    }
    if len < dest.len() {
        dest[len] = suffix;
        len += 1;
    }
    len
}

/// Writes the decimal digits of `value` into `dest`, returning the count. Digits
/// are generated least-significant-first into a scratch buffer, then emitted in
/// the correct order.
fn write_u16(dest: &mut [u8], mut value: u16) -> usize {
    if dest.is_empty() {
        return 0;
    }

    if value == 0 {
        dest[0] = b'0';
        return 1;
    }

    let mut digits = [0u8; 5];
    let mut count = 0;
    while value > 0 {
        digits[count] = b'0' + (value % 10) as u8;
        value /= 10;
        count += 1;
    }

    let mut len = 0;
    while count > 0 {
        count -= 1;
        if len >= dest.len() {
            break;
        }
        dest[len] = digits[count];
        len += 1;
    }
    len
}
