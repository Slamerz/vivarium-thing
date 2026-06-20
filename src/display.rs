//! HD44780 20x4 LCD over a PCF8574 I2C backpack.

use crate::config::{LCD1_ADDR, LCD1_TITLE, LCD2_ADDR, LCD2_TITLE, LCD_SUBTITLE};
use crate::units;
use crate::sensor::{Reading, SensorBank};
use embedded_hal::i2c::I2c;

const LCD_BACKLIGHT: u8 = 0x08;
const LCD_ENABLE: u8 = 0x04;
const LCD_COMMAND: u8 = 0x00;
const LCD_DATA: u8 = 0x01;

const ROW_OFFSETS: [u8; 4] = [0x00, 0x40, 0x14, 0x54];

const PUMP_ICON: [u8; 8] =
    [0b00000, 0b01110, 0b11111, 0b11111, 0b01010, 0b01010, 0b01010, 0b01110];
const PUMP_SPRAY_L: [u8; 8] =
    [0b00100, 0b00010, 0b01011, 0b00011, 0b00110, 0b01010, 0b00100, 0b00000];
const PUMP_SPRAY_R: [u8; 8] =
    [0b00100, 0b01000, 0b11010, 0b11000, 0b01100, 0b01010, 0b00100, 0b00000];
const FROG: [u8; 8] =
    [0b00000, 0b00011, 0b00111, 0b01111, 0b01111, 0b11111, 0b10001, 0b11101];
const FLY: [u8; 8] =
    [0b00000, 0b00000, 0b00000, 0b01100, 0b01101, 0b01110, 0b01110, 0b01010];

pub fn init_all<I2C>(i2c: &mut I2C) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    init_lcd(i2c, LCD1_ADDR)?;
    let _ = write_str(i2c, LCD1_ADDR, 0, 0, "Please Wait");

    init_lcd(i2c, LCD2_ADDR)?;
    let _ = write_str(i2c, LCD2_ADDR, 0, 0, "Loading...");
    Ok(())
}

pub fn update_all<I2C>(
    i2c: &mut I2C,
    sensors: &SensorBank,
    pump_on: bool,
    now_ms: u32,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    let readings = sensors.readings();
    let spray_on = pump_on && (now_ms / 1_000) % 2 == 0;

    render_lcd(
        i2c,
        LCD1_ADDR,
        LCD1_TITLE,
        readings[0],
        readings[1],
        true,
        spray_on,
    )?;
    render_lcd(
        i2c,
        LCD2_ADDR,
        LCD2_TITLE,
        readings[2],
        readings[3],
        false,
        spray_on,
    )
}

fn init_lcd<I2C>(i2c: &mut I2C, addr: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    arduino_hal::delay_ms(50);
    write_nibble(i2c, addr, 0x03, LCD_COMMAND)?;
    arduino_hal::delay_ms(5);
    write_nibble(i2c, addr, 0x03, LCD_COMMAND)?;
    arduino_hal::delay_us(200);
    write_nibble(i2c, addr, 0x03, LCD_COMMAND)?;
    arduino_hal::delay_us(200);
    write_nibble(i2c, addr, 0x02, LCD_COMMAND)?;
    command(i2c, addr, 0x28)?;
    command(i2c, addr, 0x0C)?;
    command(i2c, addr, 0x06)?;
    command(i2c, addr, 0x01)?;
    arduino_hal::delay_ms(2);
    create_char(i2c, addr, 0, &PUMP_ICON)?;
    create_char(i2c, addr, 1, &PUMP_SPRAY_L)?;
    create_char(i2c, addr, 2, &PUMP_SPRAY_R)?;
    create_char(i2c, addr, 3, &FROG)?;
    create_char(i2c, addr, 4, &FLY)?;
    Ok(())
}

fn render_lcd<I2C>(
    i2c: &mut I2C,
    addr: u8,
    title: &str,
    primary: Reading,
    secondary: Reading,
    two_frog_pair: bool,
    spray_on: bool,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    write_padded(i2c, addr, 0, 0, title, 20)?;
    write_padded(i2c, addr, 0, 1, LCD_SUBTITLE, 20)?;

    render_primary(i2c, addr, primary)?;
    render_secondary(i2c, addr, secondary, two_frog_pair)?;

    set_cursor(i2c, addr, 18, 3)?;
    write_data(i2c, addr, 0)?;

    if two_frog_pair {
        set_cursor(i2c, addr, 15, 2)?;
        write_data(i2c, addr, 3)?;
        set_cursor(i2c, addr, 14, 2)?;
        write_data(i2c, addr, 3)?;
    } else {
        set_cursor(i2c, addr, 15, 2)?;
        write_data(i2c, addr, 3)?;
        set_cursor(i2c, addr, 14, 2)?;
        write_data(i2c, addr, 3)?;
        set_cursor(i2c, addr, 13, 2)?;
        write_data(i2c, addr, 3)?;
    }

    set_cursor(i2c, addr, 17, 2)?;
    write_data(i2c, addr, 4)?;

    if spray_on {
        set_cursor(i2c, addr, 17, 3)?;
        write_data(i2c, addr, 1)?;
        set_cursor(i2c, addr, 19, 3)?;
        write_data(i2c, addr, 2)?;
    } else {
        write_padded(i2c, addr, 17, 3, "  ", 2)?;
    }

    Ok(())
}

fn render_primary<I2C>(i2c: &mut I2C, addr: u8, reading: Reading) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    if reading.is_error() {
        write_padded(i2c, addr, 0, 2, "ERROR", 5)
    } else {
        write_reading_pair(i2c, addr, 0, reading)
    }
}

fn render_secondary<I2C>(
    i2c: &mut I2C,
    addr: u8,
    reading: Reading,
    two_frog_pair: bool,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    if reading.is_error() {
        if two_frog_pair {
            write_padded(i2c, addr, 6, 2, "ERROR", 5)
        } else {
            write_padded(i2c, addr, 0, 3, "ERROR", 5)
        }
    } else {
        set_cursor(i2c, addr, 6, 2)?;
        write_data(i2c, addr, b'|')?;
        set_cursor(i2c, addr, 6, 3)?;
        write_data(i2c, addr, b'|')?;
        write_reading_pair(i2c, addr, 7, reading)
    }
}

fn write_reading_pair<I2C>(
    i2c: &mut I2C,
    addr: u8,
    col: u8,
    reading: Reading,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    let mut temp_buf = [0u8; 12];
    let temp_len = units::write_tenths(&mut temp_buf, reading.temp_tenths_f(), b'F');
    write_padded(
        i2c,
        addr,
        col,
        2,
        core::str::from_utf8(&temp_buf[..temp_len]).unwrap_or("ERR"),
        7,
    )?;

    let mut humi_buf = [0u8; 12];
    let humi_len = units::write_tenths(&mut humi_buf, reading.humi_tenths(), b'%');
    write_padded(
        i2c,
        addr,
        col,
        3,
        core::str::from_utf8(&humi_buf[..humi_len]).unwrap_or("ERR"),
        7,
    )
}

fn command<I2C>(i2c: &mut I2C, addr: u8, value: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    write_byte(i2c, addr, value, LCD_COMMAND)
}

fn write_data<I2C>(i2c: &mut I2C, addr: u8, value: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    write_byte(i2c, addr, value, LCD_DATA)
}

fn write_byte<I2C>(i2c: &mut I2C, addr: u8, value: u8, mode: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    write_nibble(i2c, addr, value & 0xF0, mode)?;
    write_nibble(i2c, addr, (value << 4) & 0xF0, mode)
}

fn write_nibble<I2C>(i2c: &mut I2C, addr: u8, value: u8, mode: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    let data = value | mode | LCD_BACKLIGHT;
    i2c.write(addr, &[data | LCD_ENABLE])?;
    arduino_hal::delay_us(1);
    i2c.write(addr, &[data & !LCD_ENABLE])?;
    arduino_hal::delay_us(50);
    Ok(())
}

fn set_cursor<I2C>(i2c: &mut I2C, addr: u8, col: u8, row: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    let row = row.min(3);
    command(i2c, addr, 0x80 | col.wrapping_add(ROW_OFFSETS[row as usize]))
}

fn write_str<I2C>(i2c: &mut I2C, addr: u8, col: u8, row: u8, text: &str) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    set_cursor(i2c, addr, col, row)?;
    for byte in text.bytes() {
        write_data(i2c, addr, byte)?;
    }
    Ok(())
}

fn write_padded<I2C>(
    i2c: &mut I2C,
    addr: u8,
    col: u8,
    row: u8,
    text: &str,
    width: u8,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    set_cursor(i2c, addr, col, row)?;
    let mut written = 0;
    for byte in text.bytes().take(width as usize) {
        write_data(i2c, addr, byte)?;
        written += 1;
    }
    while written < width as usize {
        write_data(i2c, addr, b' ')?;
        written += 1;
    }
    Ok(())
}

fn create_char<I2C>(
    i2c: &mut I2C,
    addr: u8,
    slot: u8,
    glyph: &[u8; 8],
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    command(i2c, addr, 0x40 | (slot << 3))?;
    for row in glyph {
        write_data(i2c, addr, *row)?;
    }
    Ok(())
}
