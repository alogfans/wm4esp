use super::screen::Screen;
use crate::error::Result;

pub trait Device {
    fn draw(&mut self, screen: &Screen) -> Result<()>;
}
