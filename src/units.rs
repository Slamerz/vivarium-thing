//! Fixed-point helpers for sensor values and thresholds (one decimal place).

pub type Tenths = i16;

pub const TEMP_FAN_THRESH: Tenths = 750;
pub const TEMP_DANGER: Tenths = 850;
pub const HUMI_LOW_THRESH: Tenths = 700;
pub const HUMI_VALID_MIN: Tenths = 100;

pub fn celsius_raw_to_tenths_f(raw: u16) -> Tenths {
    let celsius_milli = -45_000i32 + (175_000i32 * i32::from(raw) / 65535);
    let fahrenheit_milli = celsius_milli * 9 / 5 + 32_000;
    (fahrenheit_milli / 100) as Tenths
}

pub fn humi_raw_to_tenths(raw: u16) -> Tenths {
    (1_000i32 * i32::from(raw) / 65535) as Tenths
}

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
