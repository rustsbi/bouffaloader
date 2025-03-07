#![no_std]

mod config;
pub mod error;
pub mod sdcard;
pub mod utils;

pub use error::Error;

use bouffalo_hal::spi::Spi;
use core::clone::Clone;
use core::fmt::Debug;
use core::marker::Copy;
use core::option::Option;
use embedded_hal::digital::OutputPin;
use embedded_io::{Read, Write};
use heapless::String;
use serde::Deserialize;

/// Device structure containing all hardware interfaces.
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

/// Configuration settings for bouffaloader.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub configs: Configs,
}

/// Including boot arguments, firmware path and an optional opaque value.
///
/// * `bootargs` - An optional string for containing the boot arguments.
/// * `firmware` - An optional string for specifying the path to the firmware.
/// * `opaque` - An optional string for additional opaque data.
#[derive(Debug, Deserialize)]
pub struct Configs {
    pub bootargs: Option<String<128>>,
    pub firmware: Option<String<128>>,
    pub opaque: Option<String<128>>,
}

/// M-mode firmware dynamic information.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DynamicInfo {
    /// Dynamic information magic value.
    pub magic: usize,
    /// Version of dynamic information.
    pub version: usize,
    /// Address of the next boot-loading stage.
    pub next_addr: usize,
    /// RISC-V privilege mode of the next boot-loading stage.
    pub next_mode: usize,
    /// M-mode firmware options; its definition varies between SBI implementations.
    pub options: usize,
    /// Boot hart ID of current environment.
    pub boot_hart: usize,
}
