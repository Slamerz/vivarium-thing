//! TCA9548A mux channel selection and SHT3x temperature/humidity reads.

use crate::config::{MUX_ADDR, SENSOR_ADDR, SENSOR_COUNT, SENSOR_MEASURE_DELAY_MS};
use crate::units::{celsius_raw_to_tenths_f, humi_raw_to_tenths, Tenths};
use embedded_hal::i2c::I2c;

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
    pub fn temp_tenths_f(self) -> Tenths {
        match self {
            Self::Valid { temp_tenths_f, .. } => temp_tenths_f,
            Self::Error => i16::MIN,
        }
    }

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

fn select_mux_channel<I2C>(i2c: &mut I2C, channel: u8) -> Result<(), I2C::Error>
where
    I2C: I2c,
{
    if channel > 7 {
        return Ok(());
    }
    i2c.write(MUX_ADDR, &[1 << channel])
}

fn read_channel<I2C>(i2c: &mut I2C, channel: u8) -> Result<Reading, I2C::Error>
where
    I2C: I2c,
{
    select_mux_channel(i2c, channel)?;

    if i2c.write(SENSOR_ADDR, &[0x24, 0x00]).is_err() {
        return Ok(Reading::Error);
    }

    arduino_hal::delay_ms(SENSOR_MEASURE_DELAY_MS);

    let mut data = [0u8; 6];
    if i2c.read(SENSOR_ADDR, &mut data).is_err() {
        return Ok(Reading::Error);
    }

    let temp_raw = u16::from(data[0]) << 8 | u16::from(data[1]);
    let humi_raw = u16::from(data[3]) << 8 | u16::from(data[4]);

    Ok(Reading::Valid {
        temp_tenths_f: celsius_raw_to_tenths_f(temp_raw),
        humi_tenths: humi_raw_to_tenths(humi_raw),
    })
}
