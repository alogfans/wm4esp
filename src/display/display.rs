use super::{font_16, font_32, font_64};
use crate::error::{Result, WmError};

#[derive(Clone, PartialEq, Copy)]
pub enum Color {
    White,
    Black,
    Red,
    InvBlack,
    InvRed,
}

pub struct Screen {
    width: usize,
    height: usize,
    black_bitmap: Vec<u8>,
    red_bitmap: Vec<u8>,
    border_color: Color,
}

impl Screen {
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
        Screen {
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
            Color::Black | Color::InvBlack => self.black_bitmap[pos / 8] |= 1u8 << (pos % 8),
            Color::Red | Color::InvRed => self.red_bitmap[pos / 8] |= 1u8 << (pos % 8),
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
                let condition = if color == Color::InvBlack || color == Color::InvRed {
                    bitmap[pos] & pattern == 0
                } else {
                    bitmap[pos] & pattern != 0
                };
                if condition {
                    self.set_pixel(x + bmp_x, y + bmp_y, color)?;
                }
            }
        }
        Ok(())
    }

    pub fn rectangle(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: Color,
    ) -> Result<()> {
        if x + width > self.width || y + height > self.height {
            return Err(WmError::InvalidArgument);
        }
        for bmp_x in 0..width {
            for bmp_y in 0..height {
                self.set_pixel(x + bmp_x, y + bmp_y, color)?;
            }
        }
        Ok(())
    }

    pub fn text(
        &mut self,
        x: usize,
        y: usize,
        fontsize: usize,
        text: &str,
        color: Color,
    ) -> Result<usize> {
        let extract_font = match fontsize {
            16 => font_16::extract_font,
            32 => font_32::extract_font,
            64 => font_64::extract_font,
            _ => {
                return Err(WmError::InvalidArgument);
            }
        };

        let mut cursor_x = x;
        let mut cursor_y = y;
        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x;
                cursor_y += fontsize;
                continue;
            }
            let font = extract_font(ch);
            let width = font.len() / fontsize * 8;
            if cursor_x + width > self.get_width() {
                cursor_x = x;
                cursor_y += fontsize;
            }
            self.bitmap(cursor_x, cursor_y, width, fontsize, &font, color)?;
            cursor_x += width;
        }
        Ok(cursor_x)
    }
}
