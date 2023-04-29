use crate::error::{Result, WmError};

use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::i2c::I2c;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::{i2c, units};

use std::thread::sleep;
use std::time::Duration;

const DEFAULT_BAUD_RATE: units::Hertz = units::Hertz(1000000);
const I2C_ADDRESS: u8 = 0x38;
const REQUEST_TIMEOUT: u32 = 10;

pub struct DHT20<'a> {
    device: i2c::I2cDriver<'a>,
}

impl<'a> DHT20<'a> {
    pub fn new<I2C: I2c>(
        i2c: impl Peripheral<P = I2C> + 'a,
        sda: impl Peripheral<P = impl InputPin + OutputPin> + 'a,
        scl: impl Peripheral<P = impl InputPin + OutputPin> + 'a,
    ) -> Result<Self> {
        let config = i2c::config::Config::new()
            .baudrate(DEFAULT_BAUD_RATE)
            .scl_enable_pullup(true)
            .sda_enable_pullup(true);
        let device = i2c::I2cDriver::new(i2c, sda, scl, &config)?;
        Ok(DHT20 { device })
    }

    pub fn read(&mut self) -> Result<(f32, f32)> {
        self.reset_sensor()?;
        let bytes: [u8; 3] = [0xAC, 0x33, 0x00];
        self.device.write(I2C_ADDRESS, &bytes, REQUEST_TIMEOUT)?;
        while self.is_measuring()? {
            sleep(Duration::from_millis(10));
        }
        let mut buffer = Vec::new();
        buffer.resize(7, 0);
        self.device
            .read(I2C_ADDRESS, &mut buffer, REQUEST_TIMEOUT)?;
        let mut raw: u32 = buffer[1] as u32;
        raw <<= 8;
        raw += buffer[2] as u32;
        raw <<= 4;
        raw += buffer[3] as u32 >> 4;
        let humidity = raw as f32 * 9.5367431640625e-5; // ==> / 1048576.0 * 100%;

        raw = buffer[3] as u32 & 0x0F;
        raw <<= 8;
        raw += buffer[4] as u32;
        raw <<= 8;
        raw += buffer[5] as u32;
        let temperature = raw as f32 * 1.9073486328125e-4 - 50.0;

        if crc_check(&buffer) {
            Ok((temperature, humidity))
        } else {
            Err(WmError::InternalError)
        }
    }

    fn read_status(&mut self) -> Result<u8> {
        let mut buffer = Vec::new();
        buffer.resize(1, 0);
        self.device
            .read(I2C_ADDRESS, &mut buffer, REQUEST_TIMEOUT)?;
        Ok(buffer[0])
    }

    fn _is_calibrated(&mut self) -> Result<bool> {
        Ok((self.read_status()? & 0x08) == 0x08)
    }

    fn is_measuring(&mut self) -> Result<bool> {
        Ok((self.read_status()? & 0x80) == 0x80)
    }

    fn _is_idle(&mut self) -> Result<bool> {
        Ok((self.read_status()? & 0x80) == 0x00)
    }

    fn reset_sensor(&mut self) -> Result<()> {
        if (self.read_status()? & 0x18) != 0x18 {
            self.reset_register(0x1B)?;
            self.reset_register(0x1C)?;
            self.reset_register(0x1E)?;
        }
        Ok(())
    }

    fn reset_register(&mut self, reg: u8) -> Result<()> {
        let bytes: [u8; 3] = [reg, 0, 0];
        let mut buffer = Vec::new();
        buffer.resize(3, 0);
        self.device
            .write_read(I2C_ADDRESS, &bytes, &mut buffer, REQUEST_TIMEOUT)?;
        let bytes: [u8; 3] = [0xB0 | reg, buffer[1], buffer[2]];
        self.device.write(I2C_ADDRESS, &bytes, REQUEST_TIMEOUT)?;
        Ok(())
    }
}

fn crc_check(buffer: &[u8]) -> bool {
    let mut crc: u8 = 0xFF;
    for idx in 0..buffer.len() - 1 {
        crc ^= buffer[idx];
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc <<= 1;
                crc ^= 0x31;
            } else {
                crc <<= 1;
            }
        }
    }
    return crc == buffer[6];
}
