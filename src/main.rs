#![no_std]
#![no_main]

use bouffalo_hal::{prelude::*, spi::Spi};
use bouffalo_rt::{entry, Clocks, Peripherals};
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
    // TODO

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
    // TODO
    loop {}
}

fn run_cli<W: Write, R: Read, L: OutputPin, SPI, PADS, const I: usize>(
    d: &mut Device<W, R, L, SPI, PADS, I>,
    _c: &mut Config,
) -> ! {
    #[derive(Command)]
    enum Base {
        /// Print out 'Hello world!'.
        Hello,
        /// LED control command.
        Led {
            #[command(subcommand)]
            command: Option<LedCommand>,
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
                }
                Ok(())
            }),
        );
    }
}
