use crate::error::{Result, WmError};
use embedded_graphics::{
    pixelcolor::raw::{RawData, RawU2},
    pixelcolor::PixelColor,
    prelude::*,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Color {
    White,
    Black,
    Red,
}

impl Default for Color {
    fn default() -> Self {
        Self::White
    }
}

impl PixelColor for Color {
    type Raw = RawU2;
}

impl From<RawU2> for Color {
    fn from(data: RawU2) -> Self {
        match data.into_inner() {
            0 => Color::White,
            1 => Color::Black,
            _ => Color::Red,
        }
    }
}

impl From<Color> for RawU2 {
    fn from(color: Color) -> Self {
        match color {
            Color::White => RawU2::new(0),
            Color::Black => RawU2::new(1),
            Color::Red => RawU2::new(2),
        }
    }
}

pub struct Display {
    width: usize,
    height: usize,
    black_bitmap: Vec<u8>,
    red_bitmap: Vec<u8>,
    border_color: Color,
}

impl Display {
    pub fn new(width: usize, height: usize, border_color: Color) -> Self {
        let mut black_bitmap = Vec::new();
        black_bitmap.resize(height * width / 8, 0);
        let mut red_bitmap = Vec::new();
        red_bitmap.resize(height * width / 8, 0);
        match border_color {
            Color::Black => black_bitmap.fill(0xff),
            Color::Red => red_bitmap.fill(0xff),
            _ => {}
        };
        Display {
            height,
            width,
            black_bitmap,
            red_bitmap,
            border_color,
        }
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_border_color(&self) -> Color {
        self.border_color
    }

    pub fn clear(&mut self, color: Color) {
        self.black_bitmap.fill(0);
        self.red_bitmap.fill(0);
        match color {
            Color::Black => self.black_bitmap.fill(0xff),
            Color::Red => self.red_bitmap.fill(0xff),
            _ => {}
        };
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Err(WmError::InvalidArgument);
        }
        let pos = x + y * self.width;
        match color {
            Color::Black => self.black_bitmap[pos / 8] |= 1u8 << (pos % 8),
            Color::Red => self.red_bitmap[pos / 8] |= 1u8 << (pos % 8),
            Color::White => {
                self.black_bitmap[pos / 8] &= !(1u8 << (pos % 8));
                self.red_bitmap[pos / 8] &= !(1u8 << (pos % 8));
            }
        };
        Ok(())
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Result<Color> {
        if x >= self.width || y >= self.height {
            return Err(WmError::InvalidArgument);
        }
        let pos = x + y * self.width;
        if self.black_bitmap[pos / 8] & (1u8 << (pos % 8)) != 0 {
            return Ok(Color::Black);
        } else if self.red_bitmap[pos / 8] & (1u8 << (pos % 8)) != 0 {
            return Ok(Color::Red);
        } else {
            return Ok(Color::White);
        }
    }

    pub fn bitmap(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        bitmap: &[u8],
        color: Color,
    ) -> Result<()> {
        if height * width / 8 != bitmap.len() || x + width > self.width || y + height > self.height
        {
            return Err(WmError::InvalidArgument);
        }
        for bmp_x in 0..width {
            for bmp_y in 0..height {
                let pos = bmp_x / 8 + bmp_y * (width / 8);
                let pattern = 1u8 << (7 - (bmp_x % 8));
                if bitmap[pos] & pattern != 0 {
                    self.set_pixel(x + bmp_x, y + bmp_y, color)?;
                }
            }
        }
        Ok(())
    }
}

impl DrawTarget for Display {
    type Color = Color;
    type Error = WmError;

    fn draw_iter<I>(&mut self, pixels: I) -> core::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=399, y @ 0..=299)) = coord.try_into() {
                self.set_pixel(x as usize, y as usize, color)?;
            }
        }
        Ok(())
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}
