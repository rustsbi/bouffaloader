#![no_std]
#![no_main]

mod psram;

use crate::psram::uhs_psram_init;
use bouffalo_hal::{prelude::*, spi::Spi};
use bouffalo_rt::{entry, Clocks, Peripherals};
use core::arch::asm;
use core::fmt::Write as _;
use core::ptr;
use embedded_cli::{cli::CliBuilder, Command};
use embedded_hal::{digital::OutputPin, spi::MODE_3};
use embedded_io::Write;
use embedded_time::rate::*;
use panic_halt as _;

struct Device<W: Write, R: Read, L: OutputPin, SPI, PADS, const I: usize> {
    tx: W,
    rx: R,
    led: L,
    spi: Spi<SPI, PADS, I>,
}

struct Config {
    bootargs: [u8; 128],
}

#[entry]
fn main(p: Peripherals, c: Clocks) -> ! {
    // Initialize devices.
    let (tx, rx) = {
        let tx = p.gpio.io14.into_uart();
        let rx = p.gpio.io15.into_uart();
        let sig2 = p.uart_muxes.sig2.into_transmit::<0>();
        let sig3 = p.uart_muxes.sig3.into_receive::<0>();

        let config = Default::default();
        let serial = p
            .uart0
            .freerun(config, 2000000.Bd(), ((tx, sig2), (rx, sig3)), &c);

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
    uhs_psram_init();
    writeln!(d.tx, "UHS PSRAM initialization success").ok();

    // Initialize sdcard and load files.
    let mut config = Config { bootargs: [0; 128] };
    if !load_from_sdcard(&mut d, &mut config) {
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

fn load_from_sdcard<W: Write, R: Read, L: OutputPin, SPI, PADS, const I: usize>(
    d: &mut Device<W, R, L, SPI, PADS, I>,
    _c: &mut Config,
) -> bool {
    // Initialize sdcard.
    // TODO

    // Initialize filesystem.
    // TODO

    // Read configuration from `config.toml`.
    // TODO

    // Load `bl808.dtb` to memory.
    // TODO

    // Load `zImage` to memory.
    // TODO

    writeln!(d.tx, "load_from_sdcard success").ok();
    true
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

fn run_cli<W: Write, R: Read, L: OutputPin, SPI, PADS, const I: usize>(
    d: &mut Device<W, R, L, SPI, PADS, I>,
    _c: &mut Config,
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
                        load_from_sdcard(d, _c);
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
                    }
                    Base::Bootargs { command } => match command {
                        Some(BootargsCommand::Get) => {
                            writeln!(d.tx, "Bootargs: {:?}", _c.bootargs).ok();
                        }
                        Some(BootargsCommand::Set { bootarg }) => match bootarg {
                            Some(bootarg) => {
                                let bootarg = bootarg.as_bytes();
                                let len = core::cmp::min(bootarg.len(), _c.bootargs.len());
                                _c.bootargs[..len].copy_from_slice(&bootarg[..len]);
                                writeln!(d.tx, "Bootargs set to: {:?}", _c.bootargs).ok();
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

/// Sets a sequence of bits in a 32-bit unsigned integer.
///
/// # Arguments
///
/// * `val` - The original value where bits will be set.
/// * `pos` - The position to start setting bits.
/// * `len` - The number of bits to be set.
/// * `val_in` - The value to be inserted into `val` at the specified position.
///
/// # Returns
///
/// A new `u32` value with the specified bits set.
#[inline]
pub(crate) fn set_bits(val: u32, pos: u32, len: u32, val_in: u32) -> u32 {
    let mask = ((1 << len) - 1) << pos;
    (val & !mask) | ((val_in << pos) & mask)
}

/// A function to perform a busy-wait loop for approximately the given number of microseconds.
/// Note: The actual delay may vary depending on the system's processing speed.
#[inline]
pub(crate) fn sleep_us(_: u32) {
    for _ in 0..1000 {
        unsafe { asm!("nop") }
    }
}

/// A function to perform a busy-wait loop for approximately the given number of milliseconds.
/// Note: It internally calls `sleep_us` to achieve the delay.
///
/// # Arguments
///
/// * `n` - The number of milliseconds to wait.
#[inline]
pub(crate) fn sleep_ms(n: u32) {
    sleep_us(n);
}
