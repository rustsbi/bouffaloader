#![no_std]
#![no_main]

mod device;
mod sdcard;
mod utils;

use bouffalo_hal::{prelude::*, psram::init_psram, spi::Spi, uart::Config as UartConfig};
use bouffalo_rt::{entry, Clocks, Peripherals};
use core::ptr;
use core::{fmt::Write as _, str::FromStr};
use embedded_cli::{cli::CliBuilder, Command};
use embedded_hal::{digital::OutputPin, spi::MODE_3};
use embedded_io::{Read, Write};
use embedded_time::rate::*;
use panic_halt as _;
use utils::{format_hex, parse_hex, read_memory, write_memory};

use crate::device::{Config, Device};

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
    let mut config = Config::new();

    // Display welcome message.
    writeln!(d.tx, "Welcome to bouffaloader🦀!").ok();

    // Initialize PSRAM.
    init_psram(&p.psram, &p.glb);
    writeln!(d.tx, "PSRAM initialization success").ok();

    // Initialize sdcard and load files.
    if sdcard::load_from_sdcard(&mut d, &mut config).is_err() {
        writeln!(d.tx, "Load from sdcard fail").ok();
        run_cli(&mut d, &mut config);
    }

    // Check button states for CLI mode.
    let mut button_1 = p.gpio.io22.into_pull_up_input();
    let mut button_2 = p.gpio.io23.into_pull_up_input();
    if button_1.is_low().unwrap() && button_2.is_low().unwrap() {
        run_cli(&mut d, &mut config);
    }

    // Run payload.
    run_payload();
}

/// Executes the loaded payload
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

/// Runs the Command Line Interface
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
                        let _ = sdcard::load_from_sdcard(d, c);
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
