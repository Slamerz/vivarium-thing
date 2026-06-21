//! HD44780 20x4 LCD over a PCF8574 I2C backpack.
//!
//! The PCF8574 is an 8-bit I2C port expander wired to the LCD's 4-bit bus, so
//! every byte sent to the controller is split into two nibbles and clocked in
//! with the enable line. The low bits of each port write carry the control
//! signals (register select, enable, backlight).

use crate::config::{LCD1_ADDR, LCD1_TITLE, LCD2_ADDR, LCD2_TITLE, LCD_SUBTITLE};
use crate::sensor::{Reading, SensorBank};
use crate::units;
use embedded_hal::i2c::I2c;

// PCF8574 control bits packed alongside the data nibble.
const LCD_BACKLIGHT: u8 = 0x08;
const LCD_ENABLE: u8 = 0x04;
const LCD_COMMAND: u8 = 0x00; // RS low: byte is an instruction
const LCD_DATA: u8 = 0x01; // RS high: byte is character data

// HD44780 instruction set.
const LCD_CLEAR: u8 = 0x01; // clear display, home cursor
const LCD_ENTRY_MODE: u8 = 0x06; // increment cursor, no display shift
const LCD_DISPLAY_ON: u8 = 0x0C; // display on, cursor off, blink off
const LCD_FUNCTION_SET: u8 = 0x28; // 4-bit bus, 2 lines, 5x8 font
const LCD_SET_CGRAM: u8 = 0x40; // base address for custom-character RAM
const LCD_SET_DDRAM: u8 = 0x80; // base address for display RAM (cursor move)

// Start address of each visible row in the controller's display RAM. The 20x4
// panel is internally laid out as two 40-char lines, hence the non-contiguous
// offsets.
const ROW_OFFSETS: [u8; 4] = [0x00, 0x40, 0x14, 0x54];

// CGRAM slots for the custom glyphs created in `init_lcd`.
const GLYPH_PUMP: u8 = 0;
const GLYPH_SPRAY_L: u8 = 1;
const GLYPH_SPRAY_R: u8 = 2;
const GLYPH_FROG: u8 = 3;
const GLYPH_FLY: u8 = 4;

// Custom 5x8 character bitmaps (one byte per row, low 5 bits = pixels).
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

/// Per-panel configuration. The two LCDs differ only in address, title, how many
/// decorative frogs they draw, and where the secondary "ERROR" message lands
/// (the latter is a quirk carried over from the original layout).
struct Panel {
    addr: u8,
    title: &'static str,
    frog_count: u8,
    secondary_error_col: u8,
    secondary_error_row: u8,
}

const LCD1: Panel = Panel {
    addr: LCD1_ADDR,
    title: LCD1_TITLE,
    frog_count: 2,
    secondary_error_col: 6,
    secondary_error_row: 2,
};

const LCD2: Panel = Panel {
    addr: LCD2_ADDR,
    title: LCD2_TITLE,
    frog_count: 3,
    secondary_error_col: 0,
    secondary_error_row: 3,
};

/// Initialises both panels and shows a placeholder message while the first
/// sensor sweep completes.
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

/// Redraws both panels from the latest readings. LCD1 shows sensors 0 and 1,
/// LCD2 shows sensors 2 and 3.
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

    // Non-blocking spray animation: while the pump runs, the spray glyphs blink
    // on every other second. The original C++ used blocking `delay(1000)` calls
    // here, which stalled the whole control loop; this keys off the clock instead.
    let spray_on = pump_on && (now_ms / 1_000) % 2 == 0;

    render_lcd(i2c, &LCD1, readings[0], readings[1], spray_on)?;
    render_lcd(i2c, &LCD2, readings[2], readings[3], spray_on)
}

fn init_lcd<I2C>(i2c: &mut I2C, addr: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    // Standard HD44780 4-bit initialisation handshake: three "function set"
    // pulses to force 8-bit mode, then switch the bus to 4-bit operation.
    arduino_hal::delay_ms(50);
    write_nibble(i2c, addr, 0x03, LCD_COMMAND)?;
    arduino_hal::delay_ms(5);
    write_nibble(i2c, addr, 0x03, LCD_COMMAND)?;
    arduino_hal::delay_us(200);
    write_nibble(i2c, addr, 0x03, LCD_COMMAND)?;
    arduino_hal::delay_us(200);
    write_nibble(i2c, addr, 0x02, LCD_COMMAND)?;

    // Configure display mode and clear it.
    command(i2c, addr, LCD_FUNCTION_SET)?;
    command(i2c, addr, LCD_DISPLAY_ON)?;
    command(i2c, addr, LCD_ENTRY_MODE)?;
    command(i2c, addr, LCD_CLEAR)?;
    arduino_hal::delay_ms(2);

    // Upload the custom glyphs into CGRAM.
    create_char(i2c, addr, GLYPH_PUMP, &PUMP_ICON)?;
    create_char(i2c, addr, GLYPH_SPRAY_L, &PUMP_SPRAY_L)?;
    create_char(i2c, addr, GLYPH_SPRAY_R, &PUMP_SPRAY_R)?;
    create_char(i2c, addr, GLYPH_FROG, &FROG)?;
    create_char(i2c, addr, GLYPH_FLY, &FLY)?;
    Ok(())
}

/// Draws one full panel: titles, both sensor readings, and the decorative
/// pump/frog/fly/spray glyphs.
fn render_lcd<I2C>(
    i2c: &mut I2C,
    panel: &Panel,
    primary: Reading,
    secondary: Reading,
    spray_on: bool,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    let addr = panel.addr;

    write_padded(i2c, addr, 0, 0, panel.title, 20)?;
    write_padded(i2c, addr, 0, 1, LCD_SUBTITLE, 20)?;

    render_primary(i2c, addr, primary)?;
    render_secondary(i2c, addr, panel, secondary)?;

    // Pump icon, bottom-right.
    set_cursor(i2c, addr, 18, 3)?;
    write_data(i2c, addr, GLYPH_PUMP)?;

    // A little row of frogs marching left from column 15.
    for i in 0..panel.frog_count {
        set_cursor(i2c, addr, 15 - i, 2)?;
        write_data(i2c, addr, GLYPH_FROG)?;
    }

    // A fly for the frogs to chase.
    set_cursor(i2c, addr, 17, 2)?;
    write_data(i2c, addr, GLYPH_FLY)?;

    // Spray glyphs flank the pump while it is misting; otherwise clear them.
    if spray_on {
        set_cursor(i2c, addr, 17, 3)?;
        write_data(i2c, addr, GLYPH_SPRAY_L)?;
        set_cursor(i2c, addr, 19, 3)?;
        write_data(i2c, addr, GLYPH_SPRAY_R)?;
    } else {
        write_padded(i2c, addr, 17, 3, "  ", 2)?;
    }

    Ok(())
}

/// Primary reading occupies the left half (columns 0-5), temperature on row 2
/// and humidity on row 3.
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

/// Secondary reading sits to the right of a `|` divider (columns 7+). Its error
/// placement differs per panel, mirroring the original layout.
fn render_secondary<I2C>(
    i2c: &mut I2C,
    addr: u8,
    panel: &Panel,
    reading: Reading,
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    if reading.is_error() {
        write_padded(
            i2c,
            addr,
            panel.secondary_error_col,
            panel.secondary_error_row,
            "ERROR",
            5,
        )
    } else {
        set_cursor(i2c, addr, 6, 2)?;
        write_data(i2c, addr, b'|')?;
        set_cursor(i2c, addr, 6, 3)?;
        write_data(i2c, addr, b'|')?;
        write_reading_pair(i2c, addr, 7, reading)
    }
}

/// Writes a temperature (row 2) and humidity (row 3) pair starting at `col`,
/// each padded to a fixed 7-character field so stale digits get overwritten.
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

/// Sends a full byte as two 4-bit nibbles (high nibble first), as required by
/// the 4-bit bus wiring.
fn write_byte<I2C>(i2c: &mut I2C, addr: u8, value: u8, mode: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    write_nibble(i2c, addr, value & 0xF0, mode)?;
    write_nibble(i2c, addr, (value << 4) & 0xF0, mode)
}

/// Clocks a single nibble into the controller by pulsing the enable line: the
/// data is presented with enable high, then latched on the falling edge. The
/// backlight bit is kept set throughout so the display stays lit.
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

/// Moves the cursor to (`col`, `row`) by writing the matching DDRAM address.
fn set_cursor<I2C>(i2c: &mut I2C, addr: u8, col: u8, row: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    let row = row.min(3);
    command(i2c, addr, LCD_SET_DDRAM | col.wrapping_add(ROW_OFFSETS[row as usize]))
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

/// Writes `text` at (`col`, `row`), truncating or space-padding to exactly
/// `width` characters so previously drawn content is fully overwritten.
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

/// Uploads an 8-row bitmap into one of the eight CGRAM custom-character slots.
fn create_char<I2C>(
    i2c: &mut I2C,
    addr: u8,
    slot: u8,
    glyph: &[u8; 8],
) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    command(i2c, addr, LCD_SET_CGRAM | (slot << 3))?;
    for row in glyph {
        write_data(i2c, addr, *row)?;
    }
    Ok(())
}
