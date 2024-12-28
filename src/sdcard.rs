use core::str::FromStr;

use crate::device::{Config, Device};
use embedded_hal::digital::OutputPin;
use embedded_io::{Read, Write};
use embedded_sdmmc::{Mode, SdCard, TimeSource, Timestamp, VolumeManager};
use riscv::delay::McycleDelay;

/// Time source implementation for SD card filesystem
pub struct MyTimeSource {}

impl TimeSource for MyTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        // Returns a fixed timestamp for simplicity
        Timestamp::from_calendar(2023, 1, 1, 0, 0, 0).unwrap()
    }
}

/// Loads necessary files from SD card into memory
pub fn load_from_sdcard<
    W: Write,
    R: Read,
    L: OutputPin,
    SPI: core::ops::Deref<Target = bouffalo_hal::spi::RegisterBlock>,
    PADS,
    const I: usize,
>(
    d: &mut Device<W, R, L, SPI, PADS, I>,
    c: &mut Config,
) -> Result<(), ()> {
    // SD card initialization
    let sdcard = SdCard::new(&mut d.spi, McycleDelay::new(40_000_000));
    writeln!(d.tx, "initializing sdcard...").ok();
    const MAX_RETRY_TIME: usize = 3;
    let mut retry_time = 0;
    while sdcard.get_card_type().is_none() {
        retry_time += 1;
        if retry_time == MAX_RETRY_TIME {
            writeln!(d.tx, "error: failed to initialize sdcard.").ok();
            return Err(());
        }
    }

    // Display SD card information
    writeln!(
        d.tx,
        "sdcard initialized success: size = {:.2} GB",
        sdcard.num_bytes().unwrap() as f32 / (1024.0 * 1024.0 * 1024.0)
    )
    .ok();

    // Initialize filesystem and open root directory
    let mut volume_mgr = VolumeManager::new(sdcard, MyTimeSource {});
    let volume0 = volume_mgr
        .open_raw_volume(embedded_sdmmc::VolumeIdx(0))
        .map_err(|_| ())?;
    let root_dir = volume_mgr.open_root_dir(volume0).map_err(|_| ())?;

    // Read configuration from `config.toml`.
    // TODO: Use toml crate to parse config file in later versions.
    let bl808_cfg = "CONFIG~1.TOM";
    let buffer = &mut [0u8; 128];
    if load_file_into_memory(
        &mut volume_mgr,
        root_dir,
        bl808_cfg,
        buffer.as_mut_ptr() as usize,
        128,
    )
    .is_err()
    {
        writeln!(d.tx, "error: cannot load config file `config.toml`.").ok();
        return Err(());
    }

    // Parse configuration
    if let Ok(config_str) = core::str::from_utf8(buffer) {
        if let Some(start_pos) = config_str.find("bootargs = ") {
            c.bootargs =
                heapless::String::from_str(&config_str[start_pos + 11..]).map_err(|_| ())?;
            writeln!(d.tx, "read config success: bootargs = {}", c.bootargs).ok();
        } else {
            writeln!(d.tx, "error: invalid config format.").ok();
            return Err(());
        }
    } else {
        writeln!(d.tx, "error: invalid config encoding.").ok();
        return Err(());
    }

    // Load `bl808.dtb` and `zImage`
    for (filename, addr, size) in [
        ("BL808.DTB", 0x51ff_8000, 64 * 1024),
        ("ZIMAGE", 0x5000_0000, 32 * 1024 * 1024),
    ] {
        match load_file_into_memory(&mut volume_mgr, root_dir, filename, addr, size) {
            Ok(bytes) => {
                writeln!(d.tx, "load {} success, size = {} bytes", filename, bytes).ok();
            }
            Err(_) => {
                writeln!(d.tx, "error: cannot load file `{}`.", filename).ok();
                return Err(());
            }
        }
    }

    volume_mgr.close_dir(root_dir).unwrap();
    writeln!(d.tx, "load files from sdcard success.").ok();
    Ok(())
}

/// Loads a file from SD card into specified memory address
pub fn load_file_into_memory<T: embedded_sdmmc::BlockDevice>(
    volume_mgr: &mut VolumeManager<T, MyTimeSource>,
    dir: embedded_sdmmc::RawDirectory,
    file_name: &str,
    addr: usize,
    max_size: u32,
) -> Result<usize, ()> {
    // Find and open the file
    volume_mgr
        .find_directory_entry(dir, file_name)
        .map_err(|_| ())?;

    let file = volume_mgr
        .open_file_in_dir(dir, file_name, Mode::ReadOnly)
        .map_err(|_| ())?;

    // Check file size
    let file_size = volume_mgr.file_length(file).map_err(|_| ())?;
    if file_size > max_size {
        return Err(());
    }

    // Read file content into memory
    let target = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, file_size as usize) };
    let size = volume_mgr.read(file, target).map_err(|_| ())?;
    volume_mgr.close_file(file).ok();

    Ok(size)
}
