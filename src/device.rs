use bouffalo_hal::spi::Spi;
use embedded_hal::digital::OutputPin;
use embedded_io::{Read, Write};

/// Device structure containing all hardware interfaces
pub struct Device<
    W: Write,
    R: Read,
    L: OutputPin,
    SPI: core::ops::Deref<Target = bouffalo_hal::spi::RegisterBlock>,
    PADS,
    const I: usize,
> {
    pub tx: W,
    pub rx: R,
    pub led: L,
    pub spi: Spi<SPI, PADS, I>,
}

/// Configuration structure for storing system settings
pub struct Config {
    pub bootargs: heapless::String<128>,
}

impl Config {
    /// Creates a new Config instance with default values
    pub fn new() -> Self {
        Self {
            bootargs: heapless::String::new(),
        }
    }
}
