use super::Screen;
use super::{Color, Device};
use crate::error::{Result, WmError};
use esp_idf_hal::{gpio, spi, units};
use std::thread::sleep;
use std::time::Duration;

const DRIVER_CONTROL: u8 = 0x01;
const WRITE_DUMMY: u8 = 0x3A;
const WRITE_GATELINE: u8 = 0x3B;
const DATA_MODE: u8 = 0x11;
const SET_RAMXPOS: u8 = 0x44;
const SET_RAMYPOS: u8 = 0x45;
const WRITE_VCOM: u8 = 0x2C;
const WRITE_BORDER: u8 = 0x3C;
const SET_RAMXCOUNT: u8 = 0x4E;
const SET_RAMYCOUNT: u8 = 0x4F;
const WRITE_RAM: u8 = 0x24;
const WRITE_ALTRAM: u8 = 0x26;
const MASTER_ACTIVATE: u8 = 0x20;
const SOFT_RESET: u8 = 0x12;

pub struct SSD1683<'a> {
    device: spi::SpiSingleDeviceDriver<'a>,
    dc_pin: gpio::PinDriver<'a, gpio::Gpio13, gpio::Output>,
    reset_pin: gpio::PinDriver<'a, gpio::Gpio14, gpio::Output>,
    busy_pin: gpio::PinDriver<'a, gpio::Gpio12, gpio::Input>,
}

pub struct SSD1683Gpio {
    // * BUSY -- GPIO12
    // * RST  -- GPIO14
    // * DC   -- GPIO13
    // * CS   -- GPIO5
    // * SCK  -- GPIO18
    // * SDA  -- GPIO23
    pub gpio5: gpio::Gpio5,
    pub gpio12: gpio::Gpio12,
    pub gpio13: gpio::Gpio13,
    pub gpio14: gpio::Gpio14,
    pub gpio18: gpio::Gpio18,
    pub gpio23: gpio::Gpio23,
}

impl SSD1683<'_> {
    pub fn new(gpio: SSD1683Gpio, spi2: spi::SPI2) -> Result<Self> {
        let dc_pin = gpio::PinDriver::output(gpio.gpio13)?;
        let reset_pin = gpio::PinDriver::output(gpio.gpio14)?;
        let busy_pin = gpio::PinDriver::input(gpio.gpio12)?;
        let dummy: Option<gpio::AnyIOPin> = None;

        let spi_driver =
            spi::SpiDriver::new(spi2, gpio.gpio18, gpio.gpio23, dummy, spi::Dma::Disabled)?;

        let config = spi::SpiConfig::new().baudrate(units::Hertz(20000000));

        let device = spi::SpiSingleDeviceDriver::new(spi_driver, Some(gpio.gpio5), &config)?;

        let context = SSD1683 {
            device,
            dc_pin,
            reset_pin,
            busy_pin,
        };

        Ok(context)
    }

    fn wait_for_busy(&self) {
        while self.busy_pin.is_high() {
            sleep(Duration::from_millis(10));
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.reset_pin.set_low()?;
        sleep(Duration::from_millis(10));
        self.reset_pin.set_high()?;
        sleep(Duration::from_millis(10));
        self.send_command(SOFT_RESET)?;
        sleep(Duration::from_secs(1));
        self.wait_for_busy();
        Ok(())
    }

    fn send_command(&mut self, cmd: u8) -> Result<()> {
        self.dc_pin.set_low()?;
        self.device.write(&[cmd])?;
        self.dc_pin.set_high()?;
        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> Result<()> {
        self.dc_pin.set_high()?;
        self.device.write(data)?;
        Ok(())
    }

    fn send_command_data(&mut self, cmd: u8, data: u8) -> Result<()> {
        self.send_command(cmd)?;
        self.send_data(&[data])?;
        Ok(())
    }

    fn build_ram_data(&self, screen: &Screen, color: Color) -> Vec<u8> {
        let mut data = Vec::<u8>::new();
        data.resize(screen.get_width() * screen.get_height() / 8, 0);
        for x in 0..screen.get_width() {
            for y in 0..screen.get_height() {
                let pos = x + y * screen.get_width();
                if screen.get_pixel(x, y).unwrap() == color {
                    data[pos / 8] |= 1u8 << (7 - (pos % 8));
                }
            }
        }
        data
    }
}

impl Device for SSD1683<'_> {
    fn draw(&mut self, screen: &Screen) -> Result<()> {
        self.reset()?;

        self.send_command(DRIVER_CONTROL)?;
        self.send_data(&[
            (screen.get_height() - 1) as u8,
            ((screen.get_height() - 1) >> 8) as u8,
            0,
        ])?;
        self.send_command_data(WRITE_DUMMY, 0x1B)?;
        self.send_command_data(WRITE_GATELINE, 0x0B)?;
        self.send_command_data(DATA_MODE, 0x03)?;
        self.send_command(SET_RAMXPOS)?;
        self.send_data(&[0, (screen.get_width() / 8 - 1) as u8])?;
        self.send_command(SET_RAMYPOS)?;
        self.send_data(&[
            0,
            0,
            (screen.get_height() - 1) as u8,
            ((screen.get_height() - 1) >> 8) as u8,
        ])?;
        self.send_command_data(WRITE_VCOM, 0x70)?;
        self.send_command(WRITE_BORDER)?;
        match screen.get_border_color() {
            Color::White => self.send_data(&[0b00000001])?,
            Color::Black => self.send_data(&[0b00000000])?,
            Color::Red => self.send_data(&[0b00000110])?,
            _ => {
                return Err(WmError::InvalidArgument);
            }
        }

        self.send_command_data(SET_RAMXCOUNT, 0x00)?;
        self.send_command(SET_RAMYCOUNT)?;
        self.send_data(&[0x00, 0x00])?;

        let data = self.build_ram_data(screen, Color::White);
        self.send_command(WRITE_RAM)?;
        self.send_data(&data)?;

        let data = self.build_ram_data(screen, Color::Red);
        self.send_command(WRITE_ALTRAM)?;
        self.send_data(&data)?;

        self.wait_for_busy();
        self.send_command(MASTER_ACTIVATE)?;

        Ok(())
    }
}
