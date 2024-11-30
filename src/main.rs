#![no_std]
#![no_main]

use bouffalo_hal::{prelude::*, psram::init_psram, spi::Spi, uart::Config as UartConfig};
use bouffalo_rt::{entry, Clocks, Peripherals};
use core::ptr;
use core::{fmt::Write as _, str::FromStr};
use embedded_cli::{cli::CliBuilder, Command};
use embedded_hal::{digital::OutputPin, spi::MODE_3};
use embedded_io::{Read, Write};
use embedded_sdmmc::{Mode, SdCard, VolumeManager};
use embedded_time::rate::*;
use panic_halt as _;

struct MyTimeSource {}

impl embedded_sdmmc::TimeSource for MyTimeSource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp::from_calendar(2023, 1, 1, 0, 0, 0).unwrap()
    }
}

struct Device<
    W: Write,
    R: Read,
    L: OutputPin,
    SPI: core::ops::Deref<Target = bouffalo_hal::spi::RegisterBlock>,
    PADS,
    const I: usize,
> {
    tx: W,
    rx: R,
    led: L,
    spi: Spi<SPI, PADS, I>,
}

struct Config {
    bootargs: heapless::String<128>,
}

#[entry]
fn main(p: Peripherals, c: Clocks) -> ! {
    // Initialize devices.
    let (tx, rx) = {
        let tx = p.gpio.io14.into_uart();
        let rx = p.gpio.io15.into_uart();
        let sig2 = p.uart_muxes.sig2.into_transmit::<0>();
        let sig3 = p.uart_muxes.sig3.into_receive::<0>();

        let config = UartConfig::default().set_baudrate(2000000.Bd());
        let serial = p.uart0.freerun(config, ((tx, sig2), (rx, sig3)), &c);

        serial.split()
    };
    let led = p.gpio.io8.into_floating_output();
    let spi = {
        let spi_clk = p.gpio.io3.into_spi::<1>();
        let spi_mosi = p.gpio.io1.into_spi::<1>();
        let spi_miso = p.gpio.io2.into_spi::<1>();
        let spi_cs = p.gpio.io0.into_spi::<1>();
        Spi::new(
            p.spi1,
            (spi_clk, spi_mosi, spi_miso, spi_cs),
            MODE_3,
            &p.glb,
        )
    };
    let mut d = Device { tx, rx, led, spi };

    // Display bouffaloader banner.
    // TODO
    writeln!(d.tx, "Welcome to bouffaloader🦀!").ok();

    // Initialize PSRAM.
    init_psram(&p.psram, &p.glb);
    writeln!(d.tx, "PSRAM initialization success").ok();

    // Initialize sdcard and load files.
    let mut config = Config {
        bootargs: heapless::String::new(),
    };
    if load_from_sdcard(&mut d, &mut config).is_err() {
        writeln!(d.tx, "Load from sdcard fail").ok();
        run_cli(&mut d, &mut config);
    }

    // Skip run_payload if both buttons are pressed.
    let mut button_1 = p.gpio.io22.into_pull_up_input();
    let mut button_2 = p.gpio.io23.into_pull_up_input();
    let button_1_pressed = button_1.is_low().unwrap();
    let button_2_pressed = button_2.is_low().unwrap();
    if button_1_pressed && button_2_pressed {
        run_cli(&mut d, &mut config);
    };

    // Run payload.
    run_payload();
}

fn load_from_sdcard<
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
    // TODO: return error message
    let delay = riscv::delay::McycleDelay::new(40_000_000);
    let sdcard = SdCard::new(&mut d.spi, delay);

    // Initialize sdcard.
    writeln!(d.tx, "initializing sdcard...").ok();
    const MAX_RETRY_TIME: usize = 3;
    let mut retry_time = 0;
    while sdcard.get_card_type().is_none() {
        retry_time = retry_time + 1;
        if retry_time == MAX_RETRY_TIME {
            writeln!(d.tx, "error: failed to initialize sdcard.").ok();
            return Err(());
        }
    }

    // Display sdcard information.
    writeln!(
        d.tx,
        "sdcard initialized success: size = {:.2} GB",
        sdcard.num_bytes().unwrap() as f32 / (1024.0 * 1024.0 * 1024.0)
    )
    .ok();

    // Initialize filesystem.
    let mut volume_mgr = VolumeManager::new(sdcard, MyTimeSource {});
    let volume0 = volume_mgr
        .open_raw_volume(embedded_sdmmc::VolumeIdx(0))
        .map_err(|_| ())?;
    let root_dir = volume_mgr.open_root_dir(volume0).map_err(|_| ())?;
    writeln!(d.tx, "filesystem initialized success.").ok();

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
    let config_str = core::str::from_utf8(buffer).map_err(|_| ())?;
    let start_pos = config_str.find("bootargs = ").ok_or(())?;
    c.bootargs = heapless::String::from_str(&config_str[start_pos + 11..config_str.len()])
        .map_err(|_| ())?;
    writeln!(d.tx, "read config success: bootargs = {}", c.bootargs,).ok();

    // Load `bl808.dtb` to memory.
    let bl808_dtb = "BL808.DTB";
    let dtb_addr = 0x51ff_8000;
    let res = load_file_into_memory(&mut volume_mgr, root_dir, bl808_dtb, dtb_addr, 64 * 1024);
    if res.is_err() {
        writeln!(d.tx, "error: cannot load dtb file `bl808.dtb`.").ok();
        return Err(());
    }
    writeln!(d.tx, "load dtb file success, size = {} bytes", res.unwrap()).ok();

    // Load `zImage` to memory.
    let bl808_z_img = "ZIMAGE";
    let z_image_addr = 0x5000_0000;
    let res = load_file_into_memory(
        &mut volume_mgr,
        root_dir,
        bl808_z_img,
        z_image_addr,
        32 * 1024 * 1024,
    );
    if res.is_err() {
        writeln!(d.tx, "error: cannot load zImage file `zImage`.").ok();
        return Err(());
    }
    writeln!(
        d.tx,
        "load zImage file success, size = {} bytes",
        res.unwrap()
    )
    .ok();

    volume_mgr.close_dir(root_dir).unwrap();
    writeln!(d.tx, "load files from sdcard success.").ok();
    Ok(())
}

fn load_file_into_memory<T: embedded_sdmmc::BlockDevice>(
    volume_mgr: &mut VolumeManager<T, MyTimeSource>,
    dir: embedded_sdmmc::RawDirectory,
    file_name: &str,
    addr: usize,
    max_size: u32,
) -> Result<usize, ()> {
    // TODO: return error message
    volume_mgr
        .find_directory_entry(dir, file_name)
        .map_err(|_| ())?;
    let file = volume_mgr
        .open_file_in_dir(dir, file_name, Mode::ReadOnly)
        .map_err(|_| ())?;
    let file_size = volume_mgr.file_length(file).map_err(|_| ())?;
    if file_size > max_size {
        return Err(());
    }
    let target = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, file_size as usize) };
    let size = volume_mgr.read(file, target).map_err(|_| ())?;
    volume_mgr.close_file(file).ok();
    Ok(size)
}

fn run_payload() -> ! {
    const ZIMAGE_ADDRESS: usize = 0x5000_0000; // Load address of Linux zImage
    const DTB_ADDRESS: usize = 0x51FF_8000; // Address of the device tree blob
    const HART_ID: usize = 0; // Hartid of the current core

    type KernelEntry = unsafe extern "C" fn(hart_id: usize, dtb_addr: usize);

    let kernel_entry: KernelEntry = unsafe { core::mem::transmute(ZIMAGE_ADDRESS) };
    unsafe {
        kernel_entry(HART_ID, DTB_ADDRESS);
    }

    loop {}
}

fn run_cli<
    W: Write,
    R: Read,
    L: OutputPin,
    SPI: core::ops::Deref<Target = bouffalo_hal::spi::RegisterBlock>,
    PADS,
    const I: usize,
>(
    d: &mut Device<W, R, L, SPI, PADS, I>,
    c: &mut Config,
) -> ! {
    #[derive(Command)]
    enum Base<'a> {
        /// Print out 'Hello world!'.
        Hello,
        /// LED control command.
        Led {
            #[command(subcommand)]
            command: Option<LedCommand>,
        },
        /// Reload from sdcard.
        Reload,
        /// Fetch data from a specified address.
        Read { addr: &'a str },
        /// Write a value to a specified address.
        Write { addr: &'a str, val: &'a str },
        /// Boot Linux kernel.
        Boot,
        /// Bootargs command.
        Bootargs {
            #[command(subcommand)]
            command: Option<BootargsCommand<'a>>,
        },
        ///print the infomation in configs.bootargs
        Print,
    }

    #[derive(Command)]
    enum LedCommand {
        /// Turn on LED.
        On,
        /// Turn off LED.
        Off,
        /// Switch LED state.
        Switch,
    }

    #[derive(Command)]
    enum BootargsCommand<'a> {
        /// Print the bootargs variable in memory.
        Get,
        /// Set the bootargs variable in memory to the parameter content.
        Set {
            /// Bootargs.
            bootarg: Option<&'a str>,
        },
    }

    // TODO: more commands.

    writeln!(d.tx, "Welcome to bouffaloader-cli!").ok();
    writeln!(d.tx, "For command helps, type 'help'.").ok();

    let (command_buffer, history_buffer) = ([0; 128], [0; 128]);
    let writer = unsafe { ptr::read(&d.tx as *const _) };
    let mut cli = CliBuilder::default()
        .writer(writer)
        .command_buffer(command_buffer)
        .history_buffer(history_buffer)
        .prompt("> ")
        .build()
        .unwrap();

    let mut led_state = PinState::Low;
    loop {
        d.led.set_state(led_state).ok();
        let mut slice = [0];
        d.rx.read_exact(&mut slice).ok();
        let _ = cli.process_byte::<Base, _>(
            slice[0],
            &mut Base::processor(|cli, command| {
                match command {
                    Base::Hello => {
                        writeln!(d.tx, "Hello world!").ok();
                    }
                    Base::Led { command } => match command {
                        Some(LedCommand::On) => led_state = PinState::Low,
                        Some(LedCommand::Off) => led_state = PinState::High,
                        Some(LedCommand::Switch) => led_state = !led_state,
                        None => match led_state {
                            PinState::High => cli.writer().write_str("LED state: Off").unwrap(),
                            PinState::Low => cli.writer().write_str("LED state: On").unwrap(),
                        },
                    },
                    Base::Reload => {
                        let _ = load_from_sdcard(d, c);
                    }
                    Base::Read { addr } => match parse_hex(addr) {
                        Some(a) => {
                            let val = read_memory(a);
                            let mut buf = heapless::String::<48>::new();
                            let addr_fmt = format_hex(a, false);
                            let val_fmt = format_hex(val, false);
                            write!(&mut buf, "Read value from {}: {}", addr_fmt, val_fmt).unwrap();
                            cli.writer().write_str(buf.as_str()).unwrap();
                        }
                        None => cli.writer().write_str("Error: Invalid address!").unwrap(),
                    },
                    Base::Write { addr, val } => match (parse_hex(addr), parse_hex(val)) {
                        (Some(a), Some(v)) => {
                            write_memory(a, v);
                        }
                        _ => cli
                            .writer()
                            .write_str("Error: Invalid address or value!")
                            .unwrap(),
                    },
                    Base::Bootargs { command } => match command {
                        Some(BootargsCommand::Get) => {
                            writeln!(d.tx, "Bootargs: {:?}", c.bootargs).ok();
                        }
                        Some(BootargsCommand::Set { bootarg }) => match bootarg {
                            Some(bootarg) => {
                                c.bootargs = heapless::String::from_str(bootarg).unwrap();
                                writeln!(d.tx, "Bootargs set to: {:?}", c.bootargs).ok();
                            }
                            None => {
                                writeln!(d.tx, "Please enter the parameters of bootargs set").ok();
                            }
                        },
                        None => {
                            writeln!(d.tx, "Please enter the parameters of bootargs").ok();
                        }
                    },
                    Base::Boot => {
                        run_payload();
                    }
                    Base::Print => {
                        // Print the information about the configs.bootargs variable
                        writeln!(d.tx, "configs.bootargs = {:?}", c.bootargs).ok();
                    }
                }
                Ok(())
            }),
        );
    }
}

/// Convert a 32-bit unsigned integer to a hexadecimal string,
/// The string starts with "0x", and the `uppercase` parameter determines whether the letters are uppercase.
pub fn format_hex(num: u32, uppercase: bool) -> heapless::String<10> {
    let mut buf = heapless::String::<10>::new();
    let _ = buf.push_str("0x");
    for i in (0..8).rev() {
        let digit = (num >> (i * 4)) & 0xF;
        let c = match digit {
            0x0..=0x9 => (b'0' + digit as u8) as char,
            0xA..=0xF => {
                if uppercase {
                    (b'A' + (digit as u8 - 10)) as char
                } else {
                    (b'a' + (digit as u8 - 10)) as char
                }
            }
            _ => unreachable!(),
        };
        let _ = buf.push(c);
    }

    buf
}

/// Parses a hexadecimal string in the format "0xXXXXXXXX" and converts it to a 32-bit unsigned integer.
pub fn parse_hex(hex_str: &str) -> Option<u32> {
    if !hex_str.starts_with("0x") || hex_str.len() != 10 {
        return None;
    }
    let mut result = 0u32;
    for c in hex_str[2..].chars() {
        let digit = c.to_digit(16)?;
        result = result << 4 | digit;
    }

    Some(result)
}

/// Reads a 32-bit unsigned integer from the specified memory address using a volatile operation.
#[inline]
pub(crate) fn read_memory(addr: u32) -> u32 {
    unsafe { ptr::read_volatile(addr as *const u32) }
}

/// Writes a 32-bit unsigned integer value to the specified memory address using a volatile operation.
#[inline]
pub(crate) fn write_memory(addr: u32, val: u32) {
    unsafe { ptr::write_volatile(addr as *mut u32, val) }
}
