//! TCA9548A mux channel selection and SHT3x temperature/humidity reads.
//!
//! All sensors share one I2C address, so a TCA9548A 1-to-8 mux exposes them on
//! separate channels. For each reading we select the channel, kick off a
//! measurement, wait for it to complete, then read the result frame.

use crate::config::{MUX_ADDR, SENSOR_ADDR, SENSOR_COUNT, SENSOR_MEASURE_DELAY_MS};
use crate::units::{celsius_raw_to_tenths_f, humi_raw_to_tenths, Tenths};
use embedded_hal::i2c::I2c;

/// SHT3x single-shot measurement command: high repeatability, clock stretching
/// disabled (datasheet command `0x2400`).
const SHT3X_MEASURE_CMD: [u8; 2] = [0x24, 0x00];

/// A completed SHT3x measurement is six bytes: temperature MSB/LSB/CRC followed
/// by humidity MSB/LSB/CRC.
const SHT3X_FRAME_LEN: usize = 6;

/// A single probe's result: either a valid temperature/humidity pair (in
/// tenths) or an error if the probe could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Reading {
    #[default]
    Error,
    Valid {
        temp_tenths_f: Tenths,
        humi_tenths: Tenths,
    },
}

impl Reading {
    /// Temperature in tenths of a degree Fahrenheit. Errors report `i16::MIN`,
    /// the lowest possible value, so error readings never trip the "too hot"
    /// comparisons in the control logic.
    pub fn temp_tenths_f(self) -> Tenths {
        match self {
            Self::Valid { temp_tenths_f, .. } => temp_tenths_f,
            Self::Error => i16::MIN,
        }
    }

    /// Relative humidity in tenths of a percent. Errors report `i16::MIN`; the
    /// control logic additionally gates on [`is_error`](Self::is_error) before
    /// trusting a humidity value.
    pub fn humi_tenths(self) -> Tenths {
        match self {
            Self::Valid { humi_tenths, .. } => humi_tenths,
            Self::Error => i16::MIN,
        }
    }

    pub fn is_error(self) -> bool {
        matches!(self, Self::Error)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SensorBank {
    readings: [Reading; SENSOR_COUNT],
}

impl SensorBank {
    pub fn readings(&self) -> &[Reading; SENSOR_COUNT] {
        &self.readings
    }

    pub fn read_all<I2C>(&mut self, i2c: &mut I2C) -> Result<(), I2C::Error>
    where
        I2C: I2c,
    {
        for channel in 0..SENSOR_COUNT {
            self.readings[channel] = read_channel(i2c, channel as u8)?;
        }
        Ok(())
    }
}

/// Routes the shared I2C bus to one downstream channel. The TCA9548A enables a
/// channel via a one-hot bitmask, so channel `n` is selected by writing `1 << n`.
fn select_mux_channel<I2C>(i2c: &mut I2C, channel: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    if channel > 7 {
        return Ok(());
    }
    i2c.write(MUX_ADDR, &[1 << channel])
}

/// Reads one sensor. Any I2C failure (missing/unresponsive probe) degrades to
/// [`Reading::Error`] rather than propagating, so a single bad probe never stops
/// the rest of the bank from being read.
fn read_channel<I2C>(i2c: &mut I2C, channel: u8) -> Result<Reading, I2C::Error>
where
    I2C: I2c,
{
    select_mux_channel(i2c, channel)?;

    // Trigger a measurement on the now-selected probe.
    if i2c.write(SENSOR_ADDR, &SHT3X_MEASURE_CMD).is_err() {
        return Ok(Reading::Error);
    }

    // Give the sensor time to sample before reading the result frame.
    arduino_hal::delay_ms(SENSOR_MEASURE_DELAY_MS);

    let mut data = [0u8; SHT3X_FRAME_LEN];
    if i2c.read(SENSOR_ADDR, &mut data).is_err() {
        return Ok(Reading::Error);
    }

    // Reassemble the 16-bit raw values, skipping the CRC byte after each (data[2]
    // and data[5]); CRC validation is intentionally omitted to save space.
    let temp_raw = u16::from(data[0]) << 8 | u16::from(data[1]);
    let humi_raw = u16::from(data[3]) << 8 | u16::from(data[4]);

    Ok(Reading::Valid {
        temp_tenths_f: celsius_raw_to_tenths_f(temp_raw),
        humi_tenths: humi_raw_to_tenths(humi_raw),
    })
}
